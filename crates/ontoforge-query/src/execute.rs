use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use ontoforge_auth::scope::TypeScope;
use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{Engine, QueryContext, QueryOutput};
use ontoforge_schema::ir::{Backing, ObjectType, PropertyName, SourceBinding};
use ontoforge_storage::query_read::{EditScanCursor, EditScanPage, MAX_EDIT_READ, ObjectEdit};
use ontoforge_storage::value::PropertyValue;
use serde_json::Value;

use crate::ast::Nulls;
use crate::compile::{ValidatedQuery, compile_source_select};
use crate::cursor::{CursorCodec, CursorContext, CursorPosition};
use crate::overlay::{
    NormalizedOrder, compare_rows_normalized, eligible_row, normalize_order, overlay_object,
};
use crate::{ExecutionLimits, ListPage, ObjectRow, QueryError, QueryService, engine_json};

const SOURCE_WINDOW: usize = 256;
const FINAL_ROW_CAP: usize = 10_000;
const FINAL_BYTE_CAP: usize = 16 * 1024 * 1024;

async fn join_initial_candidates<S: Future, E: Future>(
    source: S,
    edits: E,
) -> (S::Output, E::Output) {
    tokio::join!(source, edits)
}
type SourceRow = (PropertyValue, BTreeMap<PropertyName, Value>);

struct EligibleCandidate<'a> {
    candidate: &'a Candidate,
    masked: BTreeMap<PropertyName, Value>,
    sort_row: BTreeMap<PropertyName, Value>,
}

#[derive(Default)]
struct CandidateSet(BTreeMap<String, Candidate>);

struct Candidate {
    pk: PropertyValue,
    source: Option<BTreeMap<PropertyName, Value>>,
    edit: Option<ObjectEdit>,
}

impl CandidateSet {
    fn add_source(
        &mut self,
        pk: PropertyValue,
        row: BTreeMap<PropertyName, Value>,
    ) -> Result<(), QueryError> {
        let entry = self
            .0
            .entry(pk.to_json().to_string())
            .or_insert_with(|| Candidate {
                pk,
                source: None,
                edit: None,
            });
        if entry
            .source
            .as_ref()
            .is_some_and(|existing| existing != &row)
        {
            return Err(QueryError::AmbiguousObject);
        }
        entry.source = Some(row);
        Ok(())
    }

    fn add_edit(&mut self, edit: ObjectEdit) -> Result<(), QueryError> {
        let entry = self
            .0
            .entry(edit.pk.to_json().to_string())
            .or_insert_with(|| Candidate {
                pk: edit.pk.clone(),
                source: None,
                edit: None,
            });
        if entry
            .edit
            .as_ref()
            .is_some_and(|existing| existing != &edit)
        {
            return Err(QueryError::Store);
        }
        entry.edit = Some(edit);
        Ok(())
    }
}

fn decode_source(
    output: QueryOutput,
    columns: &[PropertyName],
    ty: &ObjectType,
    max_rows: usize,
    budget: &mut WorkBudget,
) -> Result<Vec<SourceRow>, QueryError> {
    if output.truncated || output.rows.len() > max_rows {
        return Err(QueryError::WorkBudgetExceeded);
    }
    budget.charge_raw_candidates(output.rows.len())?;
    let pk_def = ty
        .properties
        .get(&ty.primary_key)
        .ok_or(QueryError::InvalidQuery)?;
    output
        .rows
        .into_iter()
        .map(|values| {
            if values.len() != columns.len() {
                return Err(QueryError::Source);
            }
            let row = columns
                .iter()
                .cloned()
                .zip(values.into_iter().map(engine_json))
                .map(|(property, value)| value.map(|value| (property, value)))
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            budget.charge_decoded_bytes(
                serde_json::to_vec(&row)
                    .map_err(|_| QueryError::Source)?
                    .len(),
            )?;
            let pk = PropertyValue::from_json(
                pk_def,
                row.get(&ty.primary_key).ok_or(QueryError::Source)?,
            )
            .map_err(|_| QueryError::Source)?;
            Ok((pk, row))
        })
        .collect()
}

fn charge_edit_page(page: &EditScanPage, budget: &mut WorkBudget) -> Result<(), QueryError> {
    budget.charge_raw_candidates(page.edits.len())?;
    for edit in &page.edits {
        let encoded = edit
            .row
            .set
            .iter()
            .map(|(property, value)| (property.as_str(), value.to_json()))
            .collect::<BTreeMap<_, _>>();
        let bytes = serde_json::to_vec(&encoded)
            .map_err(|_| QueryError::Store)?
            .len()
            .checked_add(edit.pk.to_json().to_string().len())
            .ok_or(QueryError::WorkBudgetExceeded)?;
        budget.charge_decoded_bytes(bytes)?;
    }
    Ok(())
}

fn value_literal(
    value: &Value,
    def: &ontoforge_schema::ir::PropertyDef,
) -> Result<Literal, QueryError> {
    match PropertyValue::from_json(def, value).map_err(|_| QueryError::InvalidQuery)? {
        PropertyValue::Null => Ok(Literal::Null),
        PropertyValue::Bool(value) => Ok(Literal::Boolean(value)),
        PropertyValue::Int(value) => Ok(Literal::Long(value)),
        PropertyValue::Double(value) => Ok(Literal::Double(value)),
        PropertyValue::Decimal(value) => Ok(Literal::Decimal(value)),
        PropertyValue::String(value) => Ok(Literal::String(value)),
        PropertyValue::Date(value) => Ok(Literal::Date(value)),
        PropertyValue::Timestamp(value) => Ok(Literal::Timestamp(value)),
        PropertyValue::Array(_) | PropertyValue::Geopoint { .. } => Err(QueryError::InvalidQuery),
    }
}

fn seek_expr(
    items: &[OrderItem],
    values: &[Value],
    descriptors: &[NormalizedOrder],
    ty: &ObjectType,
) -> Result<Expr, QueryError> {
    if items.len() != values.len() || items.len() != descriptors.len() + 1 {
        return Err(QueryError::Source);
    }
    let mut alternatives = Vec::with_capacity(items.len());
    for index in 0..items.len() {
        let mut and = Vec::new();
        for before in 0..index {
            let def = ty
                .properties
                .get(if before == descriptors.len() {
                    &ty.primary_key
                } else {
                    &descriptors[before].order.property
                })
                .ok_or(QueryError::InvalidQuery)?;
            and.push(if values[before].is_null() {
                Expr::IsNull {
                    expr: Box::new(items[before].expr.clone()),
                    negated: false,
                }
            } else {
                Expr::Compare {
                    left: Box::new(items[before].expr.clone()),
                    op: CompareOp::Eq,
                    right: Box::new(Expr::Literal(value_literal(&values[before], def)?)),
                }
            });
        }
        let (nulls, descending, def) = if index == descriptors.len() {
            (
                Nulls::Last,
                false,
                ty.properties
                    .get(&ty.primary_key)
                    .ok_or(QueryError::InvalidQuery)?,
            )
        } else {
            let descriptor = &descriptors[index];
            (
                descriptor.order.nulls,
                descriptor.order.direction == crate::ast::Direction::Desc,
                ty.properties
                    .get(&descriptor.order.property)
                    .ok_or(QueryError::InvalidQuery)?,
            )
        };
        let cmp = if values[index].is_null() {
            if nulls == Nulls::First {
                Some(Expr::IsNull {
                    expr: Box::new(items[index].expr.clone()),
                    negated: true,
                })
            } else {
                None
            }
        } else {
            let ordered = Expr::Compare {
                left: Box::new(items[index].expr.clone()),
                op: if descending {
                    CompareOp::Lt
                } else {
                    CompareOp::Gt
                },
                right: Box::new(Expr::Literal(value_literal(&values[index], def)?)),
            };
            Some(if nulls == Nulls::Last {
                Expr::Or(vec![
                    ordered,
                    Expr::IsNull {
                        expr: Box::new(items[index].expr.clone()),
                        negated: false,
                    },
                ])
            } else {
                ordered
            })
        };
        if let Some(cmp) = cmp {
            and.push(cmp);
            alternatives.push(if and.len() == 1 {
                and.pop().expect("one item")
            } else {
                Expr::And(and)
            });
        }
    }
    Ok(Expr::Or(alternatives))
}

fn source_position(
    row: &BTreeMap<PropertyName, Value>,
    order: &[NormalizedOrder],
    pk: &PropertyName,
) -> Vec<Value> {
    let mut values = order
        .iter()
        .map(|item| {
            row.get(&item.order.property)
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect::<Vec<_>>();
    values.push(row.get(pk).cloned().unwrap_or(Value::Null));
    values
}

fn raw_backfill_select(
    ty: &ObjectType,
    binding: &SourceBinding,
    keys: &[PropertyValue],
) -> Result<Select, QueryError> {
    let pk = binding.column_for(ty.primary_key.as_str()).to_owned();
    let list = keys
        .iter()
        .map(|value| {
            value_literal(
                &value.to_json(),
                ty.properties
                    .get(&ty.primary_key)
                    .ok_or(QueryError::InvalidQuery)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Select {
        projection: ty
            .properties
            .keys()
            .map(|property| SelectItem {
                expr: Expr::Column(binding.column_for(property.as_str()).to_owned()),
                alias: Some(property.as_str().to_owned()),
            })
            .collect(),
        from: Some(ontoforge_engine::TableRef {
            catalog: binding.source.catalog.clone(),
            schema: binding.source.schema.clone(),
            table: binding.source.table.clone(),
        }),
        filter: Some(Expr::InList {
            expr: Box::new(Expr::Column(pk)),
            list,
            negated: false,
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: Some(u64::try_from(keys.len() + 1).map_err(|_| QueryError::InvalidQuery)?),
    })
}

pub struct WorkBudget {
    limits: ExecutionLimits,
    backend_calls: usize,
    raw_candidates: usize,
    backfill_keys: usize,
    decoded_bytes: usize,
}

impl WorkBudget {
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            limits,
            backend_calls: 0,
            raw_candidates: 0,
            backfill_keys: 0,
            decoded_bytes: 0,
        }
    }

    pub fn charge_backend_calls(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.backend_calls,
            count,
            self.limits.max_backend_calls,
        )
    }
    pub fn charge_raw_candidates(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.raw_candidates,
            count,
            self.limits.max_raw_candidates,
        )
    }
    pub fn charge_backfill_keys(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.backfill_keys,
            count,
            self.limits.max_backfill_keys,
        )
    }
    pub fn charge_decoded_bytes(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.decoded_bytes,
            count,
            self.limits.max_decoded_bytes,
        )
    }
}

fn charge(current: &mut usize, count: usize, limit: usize) -> Result<(), QueryError> {
    let next = current
        .checked_add(count)
        .ok_or(QueryError::WorkBudgetExceeded)?;
    if next > limit {
        return Err(QueryError::WorkBudgetExceeded);
    }
    *current = next;
    Ok(())
}

impl<E: Engine> QueryService<'_, E> {
    /// Returns a non-snapshot keyset page. Concurrent edits may move rows between requests.
    #[allow(
        clippy::too_many_lines,
        reason = "keeps the bounded source and edit state machine together"
    )]
    pub async fn execute_list(&self, query: &ValidatedQuery) -> Result<ListPage, QueryError> {
        let mut budget = WorkBudget::new(self.limits);
        if matches!(
            query.request.set,
            crate::ast::ObjectSet::Union { .. }
                | crate::ast::ObjectSet::Intersect { .. }
                | crate::ast::ObjectSet::Subtract { .. }
        ) {
            return self.execute_set_list(query, &mut budget).await;
        }
        if matches!(query.request.set, crate::ast::ObjectSet::SearchAround { .. }) {
            return self.execute_search_around(query, &mut budget).await;
        }
        self.execute_list_internal(query, &mut budget, None).await
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keeps the bounded source and edit state machine together"
    )]
    pub(crate) async fn execute_list_internal(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
        allowed_keys: Option<&BTreeSet<String>>,
    ) -> Result<ListPage, QueryError> {
        let type_name = query.request.set.object_type();
        let ty = self
            .active
            .ontology
            .object_types
            .get(type_name)
            .ok_or(QueryError::InvalidQuery)?;
        let scope = self
            .scope
            .types
            .get(type_name)
            .ok_or(QueryError::InvalidQuery)?;
        let binding = if ty.backing == Backing::Source {
            let profile = self
                .active
                .ontology
                .default_profile
                .as_ref()
                .ok_or(QueryError::Source)?;
            Some(
                self.active
                    .ontology
                    .profile_bindings(profile)
                    .and_then(|bindings| bindings.objects.get(type_name))
                    .ok_or(QueryError::Source)?,
            )
        } else {
            None
        };
        let normalized = binding.map_or_else(
            || {
                query
                    .order_by
                    .iter()
                    .cloned()
                    .map(|order| NormalizedOrder {
                        order,
                        collation: ontoforge_schema::ir::Collation::CaseSensitive,
                        final_recheck: false,
                    })
                    .collect()
            },
            |binding| normalize_order(&query.order_by, binding),
        );
        let cursor_context = CursorContext::new(
            self.principal.id.as_str(),
            self.scope,
            &self.principal.attrs,
            self.active.def_version,
            type_name.clone(),
            normalized.clone(),
        );
        let codec = CursorCodec::new(self.tokens);
        let position = query
            .page
            .after
            .as_deref()
            .map(|token| codec.decode(token, &cursor_context))
            .transpose()?;
        let mut candidates = CandidateSet::default();
        let mut source_plan = binding
            .map(|binding| {
                compile_source_select(
                    query,
                    &self.active.ontology,
                    binding,
                    scope,
                    &self.principal.attrs,
                )
            })
            .transpose()?;
        let seek_order = if normalized.iter().any(|item| item.final_recheck) {
            Vec::new()
        } else {
            normalized.clone()
        };
        if let (Some(plan), Some(binding)) = (&mut source_plan, binding) {
            plan.required_columns = ty.properties.keys().cloned().collect();
            plan.select.projection = plan
                .required_columns
                .iter()
                .map(|property| SelectItem {
                    expr: Expr::Column(binding.column_for(property.as_str()).to_owned()),
                    alias: Some(property.as_str().to_owned()),
                })
                .collect();
            plan.select.order_by.push(OrderItem {
                expr: Expr::Column(binding.column_for(ty.primary_key.as_str()).to_owned()),
                descending: false,
                nulls_first: false,
            });
            plan.select.limit = Some(SOURCE_WINDOW as u64);
            if seek_order.len() == normalized.len()
                && let Some(position) = &position
            {
                let mut values = position.sort_values.clone();
                values.push(position.pk.clone());
                let boundary = seek_expr(&plan.select.order_by, &values, &seek_order, ty)?;
                plan.select.filter = Some(Expr::And(vec![
                    plan.select.filter.take().ok_or(QueryError::Source)?,
                    boundary,
                ]));
            }
        }
        let mut source_rows = Vec::new();
        let mut edit_cursor: Option<EditScanCursor>;
        let mut edit_more: bool;
        if let Some(plan) = &source_plan {
            budget.charge_backend_calls(2)?;
            let sql = print_select(&plan.select, self.engine.dialect())
                .map_err(|_| QueryError::Source)?;
            let mut ctx = QueryContext::new("l0-list".to_owned(), Vec::new());
            ctx.max_rows = SOURCE_WINDOW + 1;
            let (source_result, edit_result) = join_initial_candidates(
                self.engine.execute(&sql, &ctx),
                self.store
                    .scan_object_edits(&self.active, type_name, None, MAX_EDIT_READ),
            )
            .await;
            source_rows = decode_source(
                source_result.map_err(|_| QueryError::Source)?,
                &plan.required_columns,
                ty,
                SOURCE_WINDOW,
                budget,
            )?;
            let edits = edit_result.map_err(|_| QueryError::Store)?;
            charge_edit_page(&edits, budget)?;
            edit_cursor = edits.next;
            edit_more = edits.has_more;
            for edit in edits.edits {
                candidates.add_edit(edit)?;
            }
        } else {
            budget.charge_backend_calls(1)?;
            let edits = self
                .store
                .scan_object_edits(&self.active, type_name, None, MAX_EDIT_READ)
                .await
                .map_err(|_| QueryError::Store)?;
            charge_edit_page(&edits, budget)?;
            edit_cursor = edits.next;
            edit_more = edits.has_more;
            for edit in edits.edits {
                candidates.add_edit(edit)?;
            }
        }
        while edit_more {
            budget.charge_backend_calls(1)?;
            let edits = self
                .store
                .scan_object_edits(&self.active, type_name, edit_cursor.as_ref(), MAX_EDIT_READ)
                .await
                .map_err(|_| QueryError::Store)?;
            charge_edit_page(&edits, budget)?;
            edit_cursor = edits.next;
            edit_more = edits.has_more;
            for edit in edits.edits {
                candidates.add_edit(edit)?;
            }
        }
        if let Some(binding) = binding {
            let edit_keys = candidates
                .0
                .values()
                .filter(|item| item.edit.is_some())
                .map(|item| item.pk.clone())
                .collect::<Vec<_>>();
            for keys in edit_keys.chunks(MAX_EDIT_READ) {
                budget.charge_backfill_keys(keys.len())?;
                let select = raw_backfill_select(ty, binding, keys)?;
                let rows = self
                    .read_source_select(
                        &select,
                        &ty.properties.keys().cloned().collect::<Vec<_>>(),
                        ty,
                        keys.len() + 1,
                        budget,
                    )
                    .await?;
                for (pk, row) in rows {
                    candidates.add_source(pk, row)?;
                }
            }
        }
        let mut source_exhausted = source_rows.len() < SOURCE_WINDOW;
        let mut source_frontier = source_rows
            .last()
            .map(|(_, row)| source_sort_view(row, ty, scope, &self.principal.attrs))
            .transpose()?;
        for (pk, row) in source_rows {
            candidates.add_source(pk, row)?;
        }
        while !source_exhausted {
            if seek_order.len() == normalized.len()
                && self.can_stop_list(
                    query,
                    ty,
                    scope,
                    &normalized,
                    position.as_ref(),
                    &candidates,
                    source_frontier.as_ref().ok_or(QueryError::Source)?,
                    allowed_keys,
                )?
            {
                break;
            }
            let plan = source_plan.as_mut().ok_or(QueryError::Source)?;
            let before = source_frontier
                .as_ref()
                .map(|row| source_position(row, &seek_order, &ty.primary_key))
                .ok_or(QueryError::Source)?;
            let boundary = seek_expr(&plan.select.order_by, &before, &seek_order, ty)?;
            let base_filter = plan.select.filter.clone().ok_or(QueryError::Source)?;
            plan.select.filter = Some(Expr::And(vec![base_filter.clone(), boundary]));
            let rows = self
                .read_source_select(
                    &plan.select,
                    &plan.required_columns,
                    ty,
                    SOURCE_WINDOW,
                    budget,
                )
                .await?;
            plan.select.filter = Some(base_filter);
            source_exhausted = rows.len() < SOURCE_WINDOW;
            source_frontier = rows
                .last()
                .map(|(_, row)| source_sort_view(row, ty, scope, &self.principal.attrs))
                .transpose()?;
            for (pk, row) in rows {
                candidates.add_source(pk, row)?;
            }
        }
        self.finish_list(
            query,
            ty,
            scope,
            &normalized,
            position.as_ref(),
            &cursor_context,
            &codec,
            &candidates,
            allowed_keys,
        )
    }

    async fn read_source_select(
        &self,
        select: &Select,
        columns: &[PropertyName],
        ty: &ObjectType,
        limit: usize,
        budget: &mut WorkBudget,
    ) -> Result<Vec<SourceRow>, QueryError> {
        budget.charge_backend_calls(1)?;
        let sql = print_select(select, self.engine.dialect()).map_err(|_| QueryError::Source)?;
        let mut ctx = QueryContext::new("l0-list".to_owned(), Vec::new());
        ctx.max_rows = limit + 1;
        let output = self
            .engine
            .execute(&sql, &ctx)
            .await
            .map_err(|_| QueryError::Source)?;
        decode_source(output, columns, ty, limit, budget)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "finalization needs the frozen query and cursor context"
    )]
    fn finish_list(
        &self,
        query: &ValidatedQuery,
        ty: &ObjectType,
        scope: &TypeScope,
        order: &[NormalizedOrder],
        after: Option<&CursorPosition>,
        cursor_context: &CursorContext,
        codec: &CursorCodec<'_>,
        candidates: &CandidateSet,
        allowed_keys: Option<&BTreeSet<String>>,
    ) -> Result<ListPage, QueryError> {
        let eligible =
            self.eligible_candidates(query, ty, scope, order, after, candidates, allowed_keys)?;
        let total = eligible.len();
        let page_size = usize::try_from(query.page.first).map_err(|_| QueryError::InvalidQuery)?;
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        let mut last_position = None;
        let mut truncated = false;
        for EligibleCandidate {
            candidate, masked, ..
        } in eligible
        {
            if rows.len() == page_size {
                break;
            }
            if rows.len() == FINAL_ROW_CAP {
                truncated = true;
                break;
            }
            let properties = query
                .request
                .select
                .iter()
                .filter_map(|property| {
                    masked
                        .get(property)
                        .map(|value| (property.clone(), value.clone()))
                })
                .collect();
            let row = ObjectRow {
                object_type: ty.api_name.clone(),
                pk: candidate.pk.to_json(),
                properties,
                version: self.version_for(
                    &ty.api_name,
                    &candidate.pk,
                    candidate.source.as_ref(),
                    candidate.edit.as_ref(),
                )?,
            };
            let row_bytes = serde_json::to_vec(&row)
                .map_err(|_| QueryError::Source)?
                .len();
            let next_bytes = bytes
                .checked_add(row_bytes)
                .ok_or(QueryError::WorkBudgetExceeded)?;
            if next_bytes > FINAL_BYTE_CAP {
                truncated = true;
                break;
            }
            bytes = next_bytes;
            last_position = Some(CursorPosition {
                sort_values: order
                    .iter()
                    .map(|item| {
                        masked
                            .get(&item.order.property)
                            .cloned()
                            .unwrap_or(Value::Null)
                    })
                    .collect(),
                pk: candidate.pk.to_json(),
            });
            rows.push(row);
        }
        let has_more = total > rows.len();
        let cursor = if has_more {
            last_position
                .as_ref()
                .map(|position| codec.encode(cursor_context, position))
                .transpose()?
        } else {
            None
        };
        Ok(ListPage {
            rows,
            has_more,
            truncated,
            cursor,
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "uses the same frozen list evaluation as finalization"
    )]
    fn can_stop_list(
        &self,
        query: &ValidatedQuery,
        ty: &ObjectType,
        scope: &TypeScope,
        order: &[NormalizedOrder],
        after: Option<&CursorPosition>,
        candidates: &CandidateSet,
        frontier: &BTreeMap<PropertyName, Value>,
        allowed_keys: Option<&BTreeSet<String>>,
    ) -> Result<bool, QueryError> {
        let eligible =
            self.eligible_candidates(query, ty, scope, order, after, candidates, allowed_keys)?;
        let page_size = usize::try_from(query.page.first).map_err(|_| QueryError::InvalidQuery)?;
        Ok(eligible.get(page_size).is_some_and(|item| {
            compare_rows_normalized(&item.sort_row, frontier, order, &ty.primary_key, ty)
                != Ordering::Greater
        }))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "shares final eligibility with set membership"
    )]
    fn eligible_candidates<'a>(
        &self,
        query: &ValidatedQuery,
        ty: &ObjectType,
        scope: &TypeScope,
        order: &[NormalizedOrder],
        after: Option<&CursorPosition>,
        candidates: &'a CandidateSet,
        allowed_keys: Option<&BTreeSet<String>>,
    ) -> Result<Vec<EligibleCandidate<'a>>, QueryError> {
        let mut eligible = Vec::new();
        let after_row = after.map(|position| {
            let mut row = order
                .iter()
                .zip(&position.sort_values)
                .map(|(key, value)| (key.order.property.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>();
            row.insert(ty.primary_key.clone(), position.pk.clone());
            row
        });
        for candidate in candidates.0.values() {
            if allowed_keys.is_some_and(|keys| !keys.contains(&candidate.pk.to_json().to_string()))
            {
                continue;
            }
            if let Some(merged) = overlay_object(
                candidate.source.as_ref(),
                candidate.edit.as_ref(),
                &ty.primary_key,
                &candidate.pk,
            )? && let Some(masked) = eligible_row(
                &merged,
                &query.request.set,
                ty,
                scope,
                &self.principal.attrs,
            ) {
                let mut sort_row = masked.clone();
                sort_row.insert(ty.primary_key.clone(), candidate.pk.to_json());
                if after_row.as_ref().is_none_or(|row| {
                    compare_rows_normalized(&sort_row, row, order, &ty.primary_key, ty)
                        == Ordering::Greater
                }) {
                    eligible.push(EligibleCandidate {
                        candidate,
                        masked,
                        sort_row,
                    });
                }
            }
        }
        eligible.sort_by(|left, right| {
            compare_rows_normalized(&left.sort_row, &right.sort_row, order, &ty.primary_key, ty)
        });
        Ok(eligible)
    }
}

fn source_sort_view(
    row: &BTreeMap<PropertyName, Value>,
    ty: &ObjectType,
    scope: &TypeScope,
    attrs: &ontoforge_expr::Attrs,
) -> Result<BTreeMap<PropertyName, Value>, QueryError> {
    let scalars = row
        .iter()
        .map(|(property, value)| (property.clone(), crate::scope_scalar(value)))
        .collect::<BTreeMap<_, _>>();
    let mut view = row
        .iter()
        .filter(|(property, _)| {
            scope
                .columns
                .get(*property)
                .is_some_and(|grant| ontoforge_expr::eval(grant, &scalars, attrs).is_true())
        })
        .map(|(property, value)| (property.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    view.insert(
        ty.primary_key.clone(),
        row.get(&ty.primary_key)
            .cloned()
            .ok_or(QueryError::Source)?,
    );
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::join_initial_candidates;

    #[tokio::test]
    async fn list_runs_source_and_edit_lanes_before_first_merge() {
        let (source_polled, source_seen) = tokio::sync::oneshot::channel();
        let (edit_polled, edit_seen) = tokio::sync::oneshot::channel();
        let source = async move {
            source_polled.send(()).unwrap();
            edit_seen.await.unwrap();
            "source"
        };
        let edit = async move {
            edit_polled.send(()).unwrap();
            source_seen.await.unwrap();
            "edit"
        };
        let joined = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            join_initial_candidates(source, edit),
        )
        .await
        .unwrap();
        assert_eq!(joined, ("source", "edit"));
    }
}
