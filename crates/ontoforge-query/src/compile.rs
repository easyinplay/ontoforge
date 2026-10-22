use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use ontoforge_auth::scope::{RowVisibility, TypeScope};
use ontoforge_engine::printer::ast::{
    CompareOp, Expr, Literal, MatchKind, OrderItem, Select, SelectItem,
};
use ontoforge_expr::{Attrs, Tri};
use ontoforge_schema::ir::{
    BaseType, Collation, ObjectType, Ontology, PropertyDef, PropertyName, SourceBinding,
};
use ontoforge_storage::value::PropertyValue;
use serde_json::Value;

use crate::ast::{
    Direction, Nulls, ObjectSet, OrderBy, PageSpec, QueryLiteral, TermMode, WhereClause,
};
use crate::{QueryError, QueryRequest};

#[derive(Debug, Clone, Copy)]
pub struct CompileLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_in_values: usize,
    pub max_selected: usize,
    pub max_order: usize,
    pub max_page: u64,
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_nodes: 4096,
            max_in_values: 256,
            max_selected: 256,
            max_order: 8,
            max_page: 10_000,
        }
    }
}

#[derive(Debug)]
pub struct ValidatedQuery {
    pub request: QueryRequest,
    pub order_by: Vec<OrderBy>,
    pub page: PageSpec,
}

impl ValidatedQuery {
    #[allow(
        clippy::too_many_lines,
        reason = "keeps the existing query validation path together"
    )]
    pub fn new(
        request: QueryRequest,
        order_by: Vec<OrderBy>,
        page: PageSpec,
        ontology: &Ontology,
        scope: &TypeScope,
        limits: CompileLimits,
    ) -> Result<Self, QueryError> {
        if limits.max_page == 0
            || page.first > limits.max_page
            || [
                limits.max_depth,
                limits.max_nodes,
                limits.max_in_values,
                limits.max_selected,
                limits.max_order,
            ]
            .contains(&0)
            || request.select.len() > limits.max_selected
            || order_by.len() > limits.max_order
            || page.first == 0
        {
            return Err(QueryError::QueryTooComplex);
        }
        let ty = ontology
            .object_types
            .get(request.set.object_type())
            .ok_or(QueryError::InvalidQuery)?;
        for property in request
            .select
            .iter()
            .chain(order_by.iter().map(|o| &o.property))
        {
            check_property(ty, scope, property)?;
        }
        match &request.set {
            ObjectSet::Pk { pk, .. } => {
                let def = ty
                    .properties
                    .get(&ty.primary_key)
                    .ok_or(QueryError::InvalidQuery)?;
                PropertyValue::from_json(def, pk).map_err(|_| QueryError::InvalidQuery)?;
            }
            ObjectSet::Type { .. } => {}
            ObjectSet::Search {
                property, query, ..
            } => {
                validate_string(ty, scope, property)?;
                check_terms(query)?;
            }
            ObjectSet::Where { predicate, .. } => {
                let mut stack = vec![(predicate, 1usize)];
                let mut nodes = 0usize;
                while let Some((part, depth)) = stack.pop() {
                    nodes = nodes.checked_add(1).ok_or(QueryError::QueryTooComplex)?;
                    if depth > limits.max_depth || nodes > limits.max_nodes {
                        return Err(QueryError::QueryTooComplex);
                    }
                    match part {
                        WhereClause::Eq { property, value } => {
                            validate_atom(ty, scope, property, value, true)?;
                        }
                        WhereClause::Gt { property, value }
                        | WhereClause::Gte { property, value }
                        | WhereClause::Lt { property, value }
                        | WhereClause::Lte { property, value } => {
                            validate_atom(ty, scope, property, value, false)?;
                        }
                        WhereClause::IsNull { property } => {
                            check_property(ty, scope, property)?;
                        }
                        WhereClause::In { property, values } => {
                            if values.len() > limits.max_in_values {
                                return Err(QueryError::QueryTooComplex);
                            }
                            check_property(ty, scope, property)?;
                            for value in values {
                                validate_atom(ty, scope, property, value, true)?;
                            }
                        }
                        WhereClause::StartsWith { property, value } => {
                            validate_string(ty, scope, property)?;
                            check_text(value)?;
                        }
                        WhereClause::ContainsAnyTerm { property, value }
                        | WhereClause::ContainsAllTerms { property, value } => {
                            validate_string(ty, scope, property)?;
                            check_terms(value)?;
                        }
                        WhereClause::And { clauses } | WhereClause::Or { clauses } => {
                            if clauses.is_empty() {
                                return Err(QueryError::InvalidQuery);
                            }
                            if clauses.len() > limits.max_nodes.saturating_sub(nodes) {
                                return Err(QueryError::QueryTooComplex);
                            }
                            stack.extend(clauses.iter().map(|p| (p, depth + 1)));
                        }
                        WhereClause::Not { clause } => stack.push((clause, depth + 1)),
                    }
                }
            }
            ObjectSet::Union { .. } | ObjectSet::Intersect { .. } | ObjectSet::Subtract { .. } => {
                validate_nested_set(&request, &order_by, &page, ontology, scope, limits)?;
            }
        }
        Ok(Self {
            request,
            order_by,
            page,
        })
    }
}

fn validate_nested_set(
    request: &QueryRequest,
    order_by: &[OrderBy],
    page: &PageSpec,
    ontology: &Ontology,
    scope: &TypeScope,
    limits: CompileLimits,
) -> Result<(), QueryError> {
    let root_type = request.set.object_type();
    let mut stack = vec![(&request.set, 1usize)];
    let mut nodes = 0usize;
    while let Some((set, depth)) = stack.pop() {
        nodes = nodes.checked_add(1).ok_or(QueryError::QueryTooComplex)?;
        if nodes > limits.max_nodes || depth > limits.max_depth {
            return Err(QueryError::QueryTooComplex);
        }
        if set.object_type() != root_type {
            return Err(QueryError::InvalidQuery);
        }
        match set {
            ObjectSet::Union { sets, .. } | ObjectSet::Intersect { sets, .. } => {
                if sets.len() > limits.max_in_values {
                    return Err(QueryError::QueryTooComplex);
                }
                stack.extend(sets.iter().map(|child| (child, depth + 1)));
            }
            ObjectSet::Subtract { left, right, .. } => {
                stack.push((left, depth + 1));
                stack.push((right, depth + 1));
            }
            leaf => {
                ValidatedQuery::new(
                    QueryRequest {
                        set: leaf.clone(),
                        select: request.select.clone(),
                    },
                    order_by.to_vec(),
                    page.clone(),
                    ontology,
                    scope,
                    limits,
                )?;
            }
        }
    }
    Ok(())
}

fn check_property<'a>(
    ty: &'a ObjectType,
    scope: &TypeScope,
    property: &PropertyName,
) -> Result<&'a PropertyDef, QueryError> {
    if !scope.columns.contains_key(property) {
        return Err(QueryError::InvalidQuery);
    }
    ty.properties.get(property).ok_or(QueryError::InvalidQuery)
}

fn validate_atom(
    ty: &ObjectType,
    scope: &TypeScope,
    property: &PropertyName,
    value: &QueryLiteral,
    eq: bool,
) -> Result<(), QueryError> {
    let def = check_property(ty, scope, property)?;
    if matches!(def.base_type, BaseType::Array | BaseType::Geopoint)
        || (!eq && def.base_type == BaseType::Boolean)
        || value.0.is_null()
    {
        return Err(QueryError::InvalidQuery);
    }
    PropertyValue::from_json(def, &value.0).map_err(|_| QueryError::InvalidQuery)?;
    Ok(())
}

fn validate_string(
    ty: &ObjectType,
    scope: &TypeScope,
    property: &PropertyName,
) -> Result<(), QueryError> {
    if check_property(ty, scope, property)?.base_type != BaseType::String {
        return Err(QueryError::InvalidQuery);
    }
    Ok(())
}

fn check_text(text: &str) -> Result<(), QueryError> {
    if text.contains('\0') {
        Err(QueryError::InvalidQuery)
    } else {
        Ok(())
    }
}

fn check_terms(text: &str) -> Result<(), QueryError> {
    check_text(text)?;
    if text.split_whitespace().next().is_none() {
        Err(QueryError::InvalidQuery)
    } else {
        Ok(())
    }
}

pub struct SourceSelect {
    pub select: Select,
    pub required_columns: Vec<PropertyName>,
    pub final_recheck: bool,
    pub logical_order: Vec<OrderBy>,
}

#[allow(clippy::too_many_lines)]
pub fn compile_source_select(
    query: &ValidatedQuery,
    ontology: &Ontology,
    binding: &SourceBinding,
    scope: &TypeScope,
    attrs: &Attrs,
) -> Result<SourceSelect, QueryError> {
    let ty = ontology
        .object_types
        .get(query.request.set.object_type())
        .ok_or(QueryError::InvalidQuery)?;
    let column = |property: &PropertyName| binding.column_for(property.as_str()).to_owned();
    let lower = scope.lower(attrs, &column);
    let mut required = BTreeSet::from([ty.primary_key.clone()]);
    required.extend(query.request.select.iter().cloned());
    if let ObjectSet::Where { predicate, .. } = &query.request.set {
        collect_properties(predicate, &mut required);
    }
    if let ObjectSet::Search { property, .. } = &query.request.set {
        required.insert(property.clone());
    }
    required.extend(query.order_by.iter().map(|o| o.property.clone()));
    let projection = required
        .iter()
        .map(|property| {
            let raw = Expr::Column(column(property));
            let expr = if property == &ty.primary_key {
                raw
            } else {
                Expr::CaseWhen {
                    when: Box::new(
                        lower
                            .columns
                            .get(property)
                            .cloned()
                            .ok_or(QueryError::InvalidQuery)?,
                    ),
                    then: Box::new(raw),
                    otherwise: Box::new(Expr::Literal(Literal::Null)),
                }
            };
            Ok(SelectItem {
                expr,
                alias: Some(property.as_str().to_owned()),
            })
        })
        .collect::<Result<Vec<_>, QueryError>>()?;
    let candidate = match &query.request.set {
        ObjectSet::Pk { pk, .. } => Some(Expr::Compare {
            left: Box::new(Expr::Column(column(&ty.primary_key))),
            op: CompareOp::Eq,
            right: Box::new(Expr::Literal(to_literal(
                &PropertyValue::from_json(
                    ty.properties
                        .get(&ty.primary_key)
                        .ok_or(QueryError::InvalidQuery)?,
                    pk,
                )
                .map_err(|_| QueryError::InvalidQuery)?,
            ))),
        }),
        ObjectSet::Where { predicate, .. } if can_push(predicate, binding) => {
            Some(lower_where(predicate, ty, binding, &lower.columns)?)
        }
        ObjectSet::Where { .. }
        | ObjectSet::Search { .. }
        | ObjectSet::Type { .. }
        | ObjectSet::Union { .. }
        | ObjectSet::Intersect { .. }
        | ObjectSet::Subtract { .. } => None,
    };
    let filter = Some(match candidate {
        Some(candidate) => Expr::And(vec![lower.row, candidate]),
        None => lower.row,
    });
    let sortable = query.order_by.iter().all(|o| !is_ci(binding, &o.property));
    let order_by = if sortable {
        query
            .order_by
            .iter()
            .map(|o| OrderItem {
                expr: Expr::CaseWhen {
                    when: Box::new(
                        lower
                            .columns
                            .get(&o.property)
                            .expect("validated column")
                            .clone(),
                    ),
                    then: Box::new(Expr::Column(column(&o.property))),
                    otherwise: Box::new(Expr::Literal(Literal::Null)),
                },
                descending: o.direction == Direction::Desc,
                nulls_first: o.nulls == Nulls::First,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(SourceSelect {
        select: Select {
            projection,
            from: Some(ontoforge_engine::TableRef {
                catalog: binding.source.catalog.clone(),
                schema: binding.source.schema.clone(),
                table: binding.source.table.clone(),
            }),
            filter,
            group_by: Vec::new(),
            having: None,
            order_by,
            limit: Some(if matches!(query.request.set, ObjectSet::Pk { .. }) {
                2
            } else {
                query.page.first
            }),
        },
        required_columns: required.into_iter().collect(),
        final_recheck: true,
        logical_order: query.order_by.clone(),
    })
}

fn is_ci(binding: &SourceBinding, property: &PropertyName) -> bool {
    binding
        .columns
        .get(property.as_str())
        .and_then(|c| c.collation)
        == Some(Collation::CaseInsensitive)
}

fn can_push(part: &WhereClause, binding: &SourceBinding) -> bool {
    match part {
        WhereClause::ContainsAnyTerm { .. } | WhereClause::ContainsAllTerms { .. } => false,
        WhereClause::Eq { property, .. }
        | WhereClause::Gt { property, .. }
        | WhereClause::Gte { property, .. }
        | WhereClause::Lt { property, .. }
        | WhereClause::Lte { property, .. }
        | WhereClause::IsNull { property }
        | WhereClause::In { property, .. }
        | WhereClause::StartsWith { property, .. } => !is_ci(binding, property),
        WhereClause::And { clauses } | WhereClause::Or { clauses } => {
            clauses.iter().all(|p| can_push(p, binding))
        }
        WhereClause::Not { clause } => can_push(clause, binding),
    }
}

fn lower_where(
    part: &WhereClause,
    ty: &ObjectType,
    binding: &SourceBinding,
    grants: &BTreeMap<PropertyName, Expr>,
) -> Result<Expr, QueryError> {
    let wrap = |property: &PropertyName, body: Expr| -> Result<Expr, QueryError> {
        Ok(Expr::CaseWhen {
            when: Box::new(
                grants
                    .get(property)
                    .cloned()
                    .ok_or(QueryError::InvalidQuery)?,
            ),
            then: Box::new(body),
            otherwise: Box::new(Expr::Literal(Literal::Null)),
        })
    };
    let col =
        |property: &PropertyName| Expr::Column(binding.column_for(property.as_str()).to_owned());
    let compare = |property: &PropertyName, value: &QueryLiteral, op| -> Result<Expr, QueryError> {
        let def = ty
            .properties
            .get(property)
            .ok_or(QueryError::InvalidQuery)?;
        let literal = to_literal(
            &PropertyValue::from_json(def, &value.0).map_err(|_| QueryError::InvalidQuery)?,
        );
        wrap(
            property,
            Expr::Compare {
                left: Box::new(col(property)),
                op,
                right: Box::new(Expr::Literal(literal)),
            },
        )
    };
    match part {
        WhereClause::Eq { property, value } => compare(property, value, CompareOp::Eq),
        WhereClause::Gt { property, value } => compare(property, value, CompareOp::Gt),
        WhereClause::Gte { property, value } => compare(property, value, CompareOp::GtEq),
        WhereClause::Lt { property, value } => compare(property, value, CompareOp::Lt),
        WhereClause::Lte { property, value } => compare(property, value, CompareOp::LtEq),
        WhereClause::IsNull { property } => wrap(
            property,
            Expr::IsNull {
                expr: Box::new(col(property)),
                negated: false,
            },
        ),
        WhereClause::In { property, values } => {
            let def = ty
                .properties
                .get(property)
                .ok_or(QueryError::InvalidQuery)?;
            let body = if values.is_empty() {
                Expr::Literal(Literal::Boolean(false))
            } else {
                Expr::InList {
                    expr: Box::new(col(property)),
                    list: values
                        .iter()
                        .map(|v| {
                            PropertyValue::from_json(def, &v.0)
                                .map(|p| to_literal(&p))
                                .map_err(|_| QueryError::InvalidQuery)
                        })
                        .collect::<Result<_, _>>()?,
                    negated: false,
                }
            };
            wrap(property, body)
        }
        WhereClause::StartsWith { property, value } => wrap(
            property,
            Expr::StringMatch {
                expr: Box::new(col(property)),
                needle: value.clone(),
                kind: MatchKind::StartsWith,
            },
        ),
        WhereClause::ContainsAnyTerm { .. } | WhereClause::ContainsAllTerms { .. } => {
            Err(QueryError::InvalidQuery)
        }
        WhereClause::And { clauses } => Ok(Expr::And(
            clauses
                .iter()
                .map(|p| lower_where(p, ty, binding, grants))
                .collect::<Result<_, _>>()?,
        )),
        WhereClause::Or { clauses } => Ok(Expr::Or(
            clauses
                .iter()
                .map(|p| lower_where(p, ty, binding, grants))
                .collect::<Result<_, _>>()?,
        )),
        WhereClause::Not { clause } => Ok(Expr::Not(Box::new(lower_where(
            clause, ty, binding, grants,
        )?))),
    }
}

fn to_literal(value: &PropertyValue) -> Literal {
    match value {
        PropertyValue::Null | PropertyValue::Array(_) | PropertyValue::Geopoint { .. } => {
            Literal::Null
        }
        PropertyValue::Bool(v) => Literal::Boolean(*v),
        PropertyValue::Int(v) => Literal::Long(*v),
        PropertyValue::Double(v) => Literal::Double(*v),
        PropertyValue::Decimal(v) => Literal::Decimal(v.clone()),
        PropertyValue::String(v) => Literal::String(v.clone()),
        PropertyValue::Date(v) => Literal::Date(v.clone()),
        PropertyValue::Timestamp(v) => {
            Literal::Timestamp(v.replace('T', " ").trim_end_matches('Z').to_owned())
        }
    }
}

fn collect_properties(part: &WhereClause, out: &mut BTreeSet<PropertyName>) {
    match part {
        WhereClause::Eq { property, .. }
        | WhereClause::Gt { property, .. }
        | WhereClause::Gte { property, .. }
        | WhereClause::Lt { property, .. }
        | WhereClause::Lte { property, .. }
        | WhereClause::IsNull { property }
        | WhereClause::In { property, .. }
        | WhereClause::StartsWith { property, .. }
        | WhereClause::ContainsAnyTerm { property, .. }
        | WhereClause::ContainsAllTerms { property, .. } => {
            out.insert(property.clone());
        }
        WhereClause::And { clauses } | WhereClause::Or { clauses } => {
            for part in clauses {
                collect_properties(part, out);
            }
        }
        WhereClause::Not { clause } => collect_properties(clause, out),
    }
}

pub fn evaluate_where(
    predicate: &WhereClause,
    row: &BTreeMap<PropertyName, Value>,
    visibility: &RowVisibility,
    ty: &ObjectType,
) -> Tri {
    if !visibility.visible {
        return Tri::False;
    }
    match predicate {
        WhereClause::Eq { property, value } => {
            compare_atom(property, &value.0, CompareOp::Eq, row, visibility, ty)
        }
        WhereClause::Gt { property, value } => {
            compare_atom(property, &value.0, CompareOp::Gt, row, visibility, ty)
        }
        WhereClause::Gte { property, value } => {
            compare_atom(property, &value.0, CompareOp::GtEq, row, visibility, ty)
        }
        WhereClause::Lt { property, value } => {
            compare_atom(property, &value.0, CompareOp::Lt, row, visibility, ty)
        }
        WhereClause::Lte { property, value } => {
            compare_atom(property, &value.0, CompareOp::LtEq, row, visibility, ty)
        }
        WhereClause::IsNull { property } => visible_value(property, row, visibility)
            .map_or(Tri::Unknown, |value| Tri::from(value.is_null())),
        WhereClause::In { property, values } => {
            let Some(value) = visible_value(property, row, visibility) else {
                return Tri::Unknown;
            };
            if values.is_empty() {
                return Tri::False;
            }
            let Some(def) = ty.properties.get(property) else {
                return Tri::Unknown;
            };
            values.iter().fold(Tri::False, |acc, item| {
                acc.or(compare_value(value, &item.0, CompareOp::Eq, def))
            })
        }
        WhereClause::StartsWith { property, value } => visible_value(property, row, visibility)
            .and_then(Value::as_str)
            .map_or(Tri::Unknown, |text| Tri::from(text.starts_with(value))),
        WhereClause::ContainsAnyTerm { property, value } => {
            evaluate_search(property, value, TermMode::Any, row, visibility)
        }
        WhereClause::ContainsAllTerms { property, value } => {
            evaluate_search(property, value, TermMode::All, row, visibility)
        }
        WhereClause::And { clauses } => clauses.iter().fold(Tri::True, |acc, p| {
            acc.and(evaluate_where(p, row, visibility, ty))
        }),
        WhereClause::Or { clauses } => clauses.iter().fold(Tri::False, |acc, p| {
            acc.or(evaluate_where(p, row, visibility, ty))
        }),
        WhereClause::Not { clause } => evaluate_where(clause, row, visibility, ty).not(),
    }
}

pub fn evaluate_search(
    property: &PropertyName,
    query: &str,
    mode: TermMode,
    row: &BTreeMap<PropertyName, Value>,
    visibility: &RowVisibility,
) -> Tri {
    let Some(text) = visible_value(property, row, visibility).and_then(Value::as_str) else {
        return Tri::Unknown;
    };
    let haystack = text.to_lowercase();
    let mut terms = query.split_whitespace().map(str::to_lowercase);
    Tri::from(match mode {
        TermMode::Any => terms.any(|term| haystack.contains(&term)),
        TermMode::All => terms.all(|term| haystack.contains(&term)),
    })
}

fn visible_value<'a>(
    property: &PropertyName,
    row: &'a BTreeMap<PropertyName, Value>,
    visibility: &RowVisibility,
) -> Option<&'a Value> {
    static NULL: Value = Value::Null;
    if !visibility.columns.contains(property) {
        return None;
    }
    Some(row.get(property).unwrap_or(&NULL))
}

fn compare_atom(
    property: &PropertyName,
    literal: &Value,
    op: CompareOp,
    row: &BTreeMap<PropertyName, Value>,
    visibility: &RowVisibility,
    ty: &ObjectType,
) -> Tri {
    let Some(def) = ty.properties.get(property) else {
        return Tri::Unknown;
    };
    visible_value(property, row, visibility)
        .map_or(Tri::Unknown, |value| compare_value(value, literal, op, def))
}

fn compare_value(a: &Value, b: &Value, op: CompareOp, def: &PropertyDef) -> Tri {
    if a.is_null() || b.is_null() {
        return Tri::Unknown;
    }
    let (Ok(a), Ok(b)) = (
        PropertyValue::from_json(def, a),
        PropertyValue::from_json(def, b),
    ) else {
        return Tri::Unknown;
    };
    let order = match (&a, &b) {
        (PropertyValue::Bool(a), PropertyValue::Bool(b)) => Some(a.cmp(b)),
        (PropertyValue::Int(a), PropertyValue::Int(b)) => Some(a.cmp(b)),
        (PropertyValue::Double(a), PropertyValue::Double(b)) => a.partial_cmp(b),
        (PropertyValue::Decimal(a), PropertyValue::Decimal(b)) => decimal_order(a, b),
        (PropertyValue::String(a), PropertyValue::String(b))
        | (PropertyValue::Date(a), PropertyValue::Date(b))
        | (PropertyValue::Timestamp(a), PropertyValue::Timestamp(b)) => Some(a.cmp(b)),
        _ => None,
    };
    let Some(order) = order else {
        return Tri::Unknown;
    };
    Tri::from(match op {
        CompareOp::Eq => order == Ordering::Equal,
        CompareOp::NotEq => order != Ordering::Equal,
        CompareOp::Lt => order == Ordering::Less,
        CompareOp::LtEq => order != Ordering::Greater,
        CompareOp::Gt => order == Ordering::Greater,
        CompareOp::GtEq => order != Ordering::Less,
    })
}

fn decimal_order(a: &str, b: &str) -> Option<Ordering> {
    let a = ontoforge_engine::value::split_decimal(a)?;
    let b = ontoforge_engine::value::split_decimal(b)?;
    if a.negative != b.negative {
        return Some(if a.negative {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let ai = a.integer.trim_start_matches('0');
    let bi = b.integer.trim_start_matches('0');
    let magnitude = ai
        .len()
        .cmp(&bi.len())
        .then_with(|| ai.cmp(bi))
        .then_with(|| {
            let len = a.fraction.len().max(b.fraction.len());
            a.fraction
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(len)
                .cmp(b.fraction.bytes().chain(std::iter::repeat(b'0')).take(len))
        });
    Some(if a.negative {
        magnitude.reverse()
    } else {
        magnitude
    })
}
