use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{Engine, QueryContext, TableRef};
use ontoforge_schema::ir::{Cardinality, ObjectTypeName, PropertyName};
use ontoforge_storage::query_read::{LinkEdit, LinkEndpoint, MAX_EDIT_READ};
use ontoforge_storage::value::PropertyValue;

use crate::ast::{LinkDirection, ObjectSet, PageSpec, QueryLiteral, WhereClause};
use crate::compile::{CompileLimits, ValidatedQuery};
use crate::execute::{WorkBudget, join_initial_candidates};
use crate::{ListPage, ObjectRow, QueryError, QueryRequest, QueryService, engine_json};

pub struct SearchAroundPlan {
    pub target_type: ObjectTypeName,
    pub fk: Option<PropertyName>,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkEdge {
    pub from: PropertyValue,
    pub to: PropertyValue,
}

pub fn merge_edges(source: Vec<LinkEdge>, edits: &[LinkEdit]) -> Result<Vec<LinkEdge>, QueryError> {
    let mut live = BTreeMap::new();
    for edge in source {
        live.insert(edge_key(&edge), edge);
    }
    let mut latest = BTreeMap::new();
    for edit in edits {
        if edit.row.version <= 0 {
            return Err(QueryError::Store);
        }
        let key = (
            edit.from.to_json().to_string(),
            edit.to.to_json().to_string(),
        );
        if let Some(previous) = latest.get(&key) {
            let previous: &LinkEdit = previous;
            if previous.row.version == edit.row.version && previous != edit {
                return Err(QueryError::Store);
            }
            if previous.row.version >= edit.row.version {
                continue;
            }
        }
        latest.insert(key, edit.clone());
    }
    for edit in latest.into_values() {
        let edge = LinkEdge {
            from: edit.from.clone(),
            to: edit.to.clone(),
        };
        if edit.row.deleted {
            live.remove(&edge_key(&edge));
        } else {
            live.insert(edge_key(&edge), edge);
        }
    }
    Ok(live.into_values().collect())
}

fn edge_key(edge: &LinkEdge) -> (String, String) {
    (
        edge.from.to_json().to_string(),
        edge.to.to_json().to_string(),
    )
}

impl<E: Engine> QueryService<'_, E> {
    #[allow(
        clippy::too_many_lines,
        reason = "keeps relationship authorization and target finalization together"
    )]
    pub(crate) async fn execute_search_around(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<ListPage, QueryError> {
        let ObjectSet::SearchAround {
            object_type,
            base,
            link,
            direction,
        } = &query.request.set
        else {
            return Err(QueryError::InvalidQuery);
        };
        let definition = self
            .active
            .ontology
            .link_types
            .get(link)
            .ok_or(QueryError::InvalidQuery)?;
        let (base_type, target_type) = match direction {
            LinkDirection::FromTo => (&definition.from.object_type, &definition.to.object_type),
            LinkDirection::ToFrom => (&definition.to.object_type, &definition.from.object_type),
        };
        if base.object_type() != base_type || object_type != target_type {
            return Err(QueryError::InvalidQuery);
        }
        let base_scope = self
            .scope
            .types
            .get(base_type)
            .ok_or(QueryError::NotFound)?;
        let target_scope = self
            .scope
            .types
            .get(target_type)
            .ok_or(QueryError::NotFound)?;
        let fk = definition.to.foreign_key.as_ref();
        let base_select = if *direction == LinkDirection::ToFrom
            && definition.cardinality != Cardinality::ManyToMany
        {
            vec![fk.ok_or(QueryError::InvalidQuery)?.clone()]
        } else {
            Vec::new()
        };
        let base_query = ValidatedQuery::new(
            QueryRequest {
                set: *base.clone(),
                select: base_select,
            },
            Vec::new(),
            PageSpec {
                first: self.limits.max_raw_candidates.min(10_000) as u64,
                after: None,
            },
            &self.active.ontology,
            base_scope,
            CompileLimits::default(),
        )?;
        let base_rows = self.collect_rows(&base_query, budget).await?;
        let mut keys = BTreeSet::new();
        match definition.cardinality {
            Cardinality::OneToOne | Cardinality::OneToMany => match direction {
                LinkDirection::ToFrom => {
                    let fk = fk.ok_or(QueryError::InvalidQuery)?;
                    for row in base_rows {
                        if let Some(value) = row.properties.get(fk).filter(|value| !value.is_null())
                        {
                            keys.insert(value.to_string());
                        }
                    }
                }
                LinkDirection::FromTo => {
                    let fk = fk.ok_or(QueryError::InvalidQuery)?;
                    if !target_scope.columns.contains_key(fk) {
                        return Err(QueryError::NotFound);
                    }
                    for row in base_rows {
                        let child = ValidatedQuery::new(
                            QueryRequest {
                                set: ObjectSet::Where {
                                    object_type: target_type.clone(),
                                    predicate: WhereClause::Eq {
                                        property: fk.clone(),
                                        value: QueryLiteral(row.pk),
                                    },
                                },
                                select: Vec::new(),
                            },
                            Vec::new(),
                            PageSpec {
                                first: self.limits.max_raw_candidates.min(10_000) as u64,
                                after: None,
                            },
                            &self.active.ontology,
                            target_scope,
                            CompileLimits::default(),
                        )?;
                        keys.extend(
                            self.collect_rows(&child, budget)
                                .await?
                                .into_iter()
                                .map(|row| row.pk.to_string()),
                        );
                    }
                }
            },
            Cardinality::ManyToMany => {
                for row in base_rows {
                    let endpoint_type = &self.active.ontology.object_types[base_type];
                    let pk_def = &endpoint_type.properties[&endpoint_type.primary_key];
                    let endpoint = PropertyValue::from_json(pk_def, &row.pk)
                        .map_err(|_| QueryError::InvalidQuery)?;
                    let edges = self
                        .read_many_to_many_edges(link, *direction, &endpoint, budget)
                        .await?;
                    for edge in edges {
                        let other = match direction {
                            LinkDirection::FromTo => edge.to,
                            LinkDirection::ToFrom => edge.from,
                        };
                        keys.insert(other.to_json().to_string());
                    }
                }
            }
        }
        let final_query = ValidatedQuery {
            request: QueryRequest {
                set: ObjectSet::Type {
                    object_type: target_type.clone(),
                },
                select: query.request.select.clone(),
            },
            order_by: query.order_by.clone(),
            page: query.page.clone(),
        };
        self.execute_list_internal(&final_query, budget, Some(&keys))
            .await
    }

    async fn collect_rows(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<Vec<ObjectRow>, QueryError> {
        let mut rows = Vec::new();
        let mut after = None;
        loop {
            let current = ValidatedQuery {
                request: query.request.clone(),
                order_by: query.order_by.clone(),
                page: PageSpec {
                    first: query.page.first,
                    after,
                },
            };
            let page = match &current.request.set {
                ObjectSet::Union { .. }
                | ObjectSet::Intersect { .. }
                | ObjectSet::Subtract { .. } => {
                    Box::pin(self.execute_set_list(&current, budget)).await?
                }
                ObjectSet::SearchAround { .. } => {
                    Box::pin(self.execute_search_around(&current, budget)).await?
                }
                _ => self.execute_list_internal(&current, budget, None).await?,
            };
            if page.truncated {
                return Err(QueryError::WorkBudgetExceeded);
            }
            rows.extend(page.rows);
            if !page.has_more {
                return Ok(rows);
            }
            after = Some(page.cursor.ok_or(QueryError::WorkBudgetExceeded)?);
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keeps bounded source and edit edge scans together"
    )]
    async fn read_many_to_many_edges(
        &self,
        link: &ontoforge_schema::ir::LinkTypeName,
        direction: LinkDirection,
        endpoint: &PropertyValue,
        budget: &mut WorkBudget,
    ) -> Result<Vec<LinkEdge>, QueryError> {
        let definition = &self.active.ontology.link_types[link];
        let profile = self
            .active
            .ontology
            .default_profile
            .as_ref()
            .ok_or(QueryError::Source)?;
        let binding = self
            .active
            .ontology
            .profile_bindings(profile)
            .and_then(|profile| profile.links.get(link));
        let mut source = Vec::new();
        let edit_endpoint = match direction {
            LinkDirection::FromTo => LinkEndpoint::From(endpoint.clone()),
            LinkDirection::ToFrom => LinkEndpoint::To(endpoint.clone()),
        };
        let mut first_edit_page = None;
        if let Some(binding) = binding {
            let from_column = binding.column_for("from").to_owned();
            let to_column = binding.column_for("to").to_owned();
            let (fixed_column, other_column) = match direction {
                LinkDirection::FromTo => (&from_column, &to_column),
                LinkDirection::ToFrom => (&to_column, &from_column),
            };
            let other_type = match direction {
                LinkDirection::FromTo => &definition.to.object_type,
                LinkDirection::ToFrom => &definition.from.object_type,
            };
            let other_ty = &self.active.ontology.object_types[other_type];
            let other_def = &other_ty.properties[&other_ty.primary_key];
            let mut after: Option<PropertyValue> = None;
            loop {
                let mut filters = vec![Expr::Compare {
                    left: Box::new(Expr::Column(fixed_column.clone())),
                    op: CompareOp::Eq,
                    right: Box::new(Expr::Literal(link_literal(endpoint)?)),
                }];
                if let Some(value) = &after {
                    filters.push(Expr::Compare {
                        left: Box::new(Expr::Column(other_column.clone())),
                        op: CompareOp::Gt,
                        right: Box::new(Expr::Literal(link_literal(value)?)),
                    });
                }
                let select = Select {
                    projection: [from_column.as_str(), to_column.as_str()]
                        .into_iter()
                        .map(|column| SelectItem {
                            expr: Expr::Column(column.to_owned()),
                            alias: None,
                        })
                        .collect(),
                    from: Some(TableRef {
                        catalog: binding.source.catalog.clone(),
                        schema: binding.source.schema.clone(),
                        table: binding.source.table.clone(),
                    }),
                    filter: Some(Expr::And(filters)),
                    group_by: Vec::new(),
                    having: None,
                    order_by: vec![OrderItem {
                        expr: Expr::Column(other_column.clone()),
                        descending: false,
                        nulls_first: false,
                    }],
                    limit: Some(MAX_EDIT_READ as u64),
                };
                let sql =
                    print_select(&select, self.engine.dialect()).map_err(|_| QueryError::Source)?;
                let mut ctx = QueryContext::new("l0-link".to_owned(), Vec::new());
                ctx.max_rows = MAX_EDIT_READ + 1;
                let output = if first_edit_page.is_none() {
                    budget.charge_backend_calls(2)?;
                    let (source_result, edit_result) = join_initial_candidates(
                        self.engine.execute(&sql, &ctx),
                        self.store.read_link_edits(
                            &self.active,
                            link,
                            edit_endpoint.clone(),
                            None,
                            MAX_EDIT_READ,
                        ),
                    )
                    .await;
                    first_edit_page = Some(edit_result.map_err(|_| QueryError::Store)?);
                    source_result.map_err(|_| QueryError::Source)?
                } else {
                    budget.charge_backend_calls(1)?;
                    self.engine
                        .execute(&sql, &ctx)
                        .await
                        .map_err(|_| QueryError::Source)?
                };
                if output.truncated || output.rows.len() > MAX_EDIT_READ {
                    return Err(QueryError::WorkBudgetExceeded);
                }
                let len = output.rows.len();
                budget.charge_raw_candidates(len)?;
                for row in output.rows {
                    if row.len() != 2 {
                        return Err(QueryError::Source);
                    }
                    let mut row = row.into_iter();
                    let from_json = engine_json(row.next().ok_or(QueryError::Source)?)?;
                    let to_json = engine_json(row.next().ok_or(QueryError::Source)?)?;
                    budget.charge_decoded_bytes(
                        from_json.to_string().len() + to_json.to_string().len(),
                    )?;
                    let from_ty = &self.active.ontology.object_types[&definition.from.object_type];
                    let to_ty = &self.active.ontology.object_types[&definition.to.object_type];
                    let from = PropertyValue::from_json(
                        &from_ty.properties[&from_ty.primary_key],
                        &from_json,
                    )
                    .map_err(|_| QueryError::Source)?;
                    let to =
                        PropertyValue::from_json(&to_ty.properties[&to_ty.primary_key], &to_json)
                            .map_err(|_| QueryError::Source)?;
                    after = Some(
                        PropertyValue::from_json(
                            other_def,
                            match direction {
                                LinkDirection::FromTo => &to_json,
                                LinkDirection::ToFrom => &from_json,
                            },
                        )
                        .map_err(|_| QueryError::Source)?,
                    );
                    source.push(LinkEdge { from, to });
                }
                if len < MAX_EDIT_READ {
                    break;
                }
            }
        }
        let mut edits = Vec::new();
        let mut cursor = None;
        loop {
            let page = if let Some(page) = first_edit_page.take() {
                page
            } else {
                budget.charge_backend_calls(1)?;
                self.store
                    .read_link_edits(
                        &self.active,
                        link,
                        edit_endpoint.clone(),
                        cursor.as_ref(),
                        MAX_EDIT_READ,
                    )
                    .await
                    .map_err(|_| QueryError::Store)?
            };
            budget.charge_raw_candidates(page.edits.len())?;
            for edit in &page.edits {
                budget.charge_decoded_bytes(
                    edit.from.to_json().to_string().len() + edit.to.to_json().to_string().len(),
                )?;
            }
            edits.extend(page.edits);
            if !page.has_more {
                break;
            }
            cursor = page.next;
        }
        merge_edges(source, &edits)
    }
}

fn link_literal(value: &PropertyValue) -> Result<Literal, QueryError> {
    match value {
        PropertyValue::Bool(value) => Ok(Literal::Boolean(*value)),
        PropertyValue::Int(value) => Ok(Literal::Long(*value)),
        PropertyValue::Double(value) => Ok(Literal::Double(*value)),
        PropertyValue::Decimal(value) => Ok(Literal::Decimal(value.clone())),
        PropertyValue::String(value) => Ok(Literal::String(value.clone())),
        PropertyValue::Date(value) => Ok(Literal::Date(value.clone())),
        PropertyValue::Timestamp(value) => Ok(Literal::Timestamp(value.clone())),
        PropertyValue::Null | PropertyValue::Array(_) | PropertyValue::Geopoint { .. } => {
            Err(QueryError::InvalidQuery)
        }
    }
}
