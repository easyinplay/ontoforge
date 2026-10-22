use std::collections::BTreeMap;

use ontoforge_schema::ir::{Collation, ObjectType, PropertyName, SourceBinding};
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::value::PropertyValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::QueryError;
use crate::ast::{Direction, Nulls};
use crate::ast::{ObjectSet, OrderBy};
use crate::compile::{evaluate_search, evaluate_where};
use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::Attrs;
use std::cmp::Ordering;

pub fn overlay_object(
    source: Option<&BTreeMap<PropertyName, Value>>,
    edit: Option<&ObjectEdit>,
    pk_name: &PropertyName,
    pk: &PropertyValue,
) -> Result<Option<BTreeMap<PropertyName, Value>>, QueryError> {
    if let Some(edit) = edit {
        if edit.pk != *pk || edit.row.version <= 0 || (edit.row.deleted && edit.row.released) {
            return Err(QueryError::Store);
        }
        if edit.row.deleted {
            return Ok(None);
        }
    }
    let mut merged = match source {
        Some(row) => row.clone(),
        None if edit.is_some_and(|edit| !edit.row.released) => BTreeMap::new(),
        None => return Ok(None),
    };
    merged.insert(pk_name.clone(), pk.to_json());
    if let Some(edit) = edit.filter(|edit| !edit.row.released) {
        for (property, value) in &edit.row.set {
            if property != pk_name {
                merged.insert(property.clone(), value.to_json());
            }
        }
    }
    Ok(Some(merged))
}

pub fn overlay_candidates(
    sources: Vec<(PropertyValue, BTreeMap<PropertyName, Value>)>,
    edits: Vec<ObjectEdit>,
    pk_name: &PropertyName,
) -> Result<Vec<BTreeMap<PropertyName, Value>>, QueryError> {
    let mut by_key = BTreeMap::new();
    for (pk, source) in sources {
        let slot = by_key
            .entry(pk.to_json().to_string())
            .or_insert_with(|| (pk, None, None));
        if slot.1.as_ref().is_some_and(|existing| existing != &source) {
            return Err(QueryError::AmbiguousObject);
        }
        slot.1 = Some(source);
    }
    for edit in edits {
        let slot = by_key
            .entry(edit.pk.to_json().to_string())
            .or_insert_with(|| (edit.pk.clone(), None, None::<ObjectEdit>));
        if let Some(existing) = &slot.2 {
            if existing.row.version == edit.row.version && existing != &edit {
                return Err(QueryError::Store);
            }
            if existing.row.version > edit.row.version {
                continue;
            }
        }
        slot.2 = Some(edit);
    }
    by_key
        .into_values()
        .filter_map(|(pk, source, edit)| {
            overlay_object(source.as_ref(), edit.as_ref(), pk_name, &pk).transpose()
        })
        .collect()
}

pub fn eligible_row(
    merged: &BTreeMap<PropertyName, Value>,
    set: &ObjectSet,
    ty: &ObjectType,
    scope: &TypeScope,
    attrs: &Attrs,
) -> Option<BTreeMap<PropertyName, Value>> {
    let scalar_row = merged
        .iter()
        .map(|(property, value)| (property.clone(), crate::scope_scalar(value)))
        .collect::<BTreeMap<_, _>>();
    let visibility = scope.evaluate(&scalar_row, attrs);
    if !visibility.visible {
        return None;
    }
    let matches = match set {
        ObjectSet::Pk { pk, .. } => merged.get(&ty.primary_key) == Some(pk),
        ObjectSet::Type { .. } => true,
        ObjectSet::Where { predicate, .. } => {
            evaluate_where(predicate, merged, &visibility, ty).is_true()
        }
        ObjectSet::Search {
            property,
            query,
            mode,
            ..
        } => evaluate_search(property, query, *mode, merged, &visibility).is_true(),
    };
    matches.then(|| {
        merged
            .iter()
            .filter(|(property, _)| visibility.columns.contains(*property))
            .map(|(property, value)| (property.clone(), value.clone()))
            .collect()
    })
}

pub fn compare_rows(
    left: &BTreeMap<PropertyName, Value>,
    right: &BTreeMap<PropertyName, Value>,
    order: &[OrderBy],
    pk: &PropertyName,
    ty: &ObjectType,
) -> Ordering {
    let normalized = order
        .iter()
        .cloned()
        .map(|order| NormalizedOrder {
            order,
            collation: Collation::CaseSensitive,
            final_recheck: false,
        })
        .collect::<Vec<_>>();
    compare_rows_normalized(left, right, &normalized, pk, ty)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NormalizedOrder {
    pub order: OrderBy,
    pub collation: Collation,
    pub final_recheck: bool,
}

pub fn normalize_order(order: &[OrderBy], binding: &SourceBinding) -> Vec<NormalizedOrder> {
    order
        .iter()
        .map(|item| {
            let collation = binding
                .columns
                .get(item.property.as_str())
                .and_then(|column| column.collation)
                .unwrap_or(Collation::CaseSensitive);
            NormalizedOrder {
                order: item.clone(),
                collation,
                final_recheck: collation == Collation::CaseInsensitive,
            }
        })
        .collect()
}

pub fn compare_rows_normalized(
    left: &BTreeMap<PropertyName, Value>,
    right: &BTreeMap<PropertyName, Value>,
    order: &[NormalizedOrder],
    pk: &PropertyName,
    ty: &ObjectType,
) -> Ordering {
    for key in order {
        let lhs = left.get(&key.order.property).filter(|v| !v.is_null());
        let rhs = right.get(&key.order.property).filter(|v| !v.is_null());
        let result = match (lhs, rhs) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => {
                if key.order.nulls == Nulls::First {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (Some(_), None) => {
                if key.order.nulls == Nulls::First {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Some(lhs), Some(rhs)) => {
                let result = ty.properties.get(&key.order.property).map_or_else(
                    || lhs.to_string().cmp(&rhs.to_string()),
                    |def| compare_typed(lhs, rhs, def, key.collation),
                );
                if key.order.direction == Direction::Desc {
                    result.reverse()
                } else {
                    result
                }
            }
        };
        if result != Ordering::Equal {
            return result;
        }
    }
    match (left.get(pk), right.get(pk), ty.properties.get(pk)) {
        (Some(lhs), Some(rhs), Some(def)) => compare_typed(lhs, rhs, def, Collation::CaseSensitive),
        (Some(lhs), Some(rhs), None) => lhs.to_string().cmp(&rhs.to_string()),
        _ => Ordering::Equal,
    }
}

fn compare_typed(
    left: &Value,
    right: &Value,
    def: &ontoforge_schema::ir::PropertyDef,
    collation: Collation,
) -> Ordering {
    let lhs = PropertyValue::from_json(def, left);
    let rhs = PropertyValue::from_json(def, right);
    match (lhs, rhs) {
        (Ok(PropertyValue::Bool(a)), Ok(PropertyValue::Bool(b))) => a.cmp(&b),
        (Ok(PropertyValue::Int(a)), Ok(PropertyValue::Int(b))) => a.cmp(&b),
        (Ok(PropertyValue::Double(a)), Ok(PropertyValue::Double(b))) => a.total_cmp(&b),
        (Ok(PropertyValue::String(a)), Ok(PropertyValue::String(b)))
            if collation == Collation::CaseInsensitive =>
        {
            a.to_lowercase().cmp(&b.to_lowercase())
        }
        (Ok(PropertyValue::String(a)), Ok(PropertyValue::String(b)))
        | (Ok(PropertyValue::Date(a)), Ok(PropertyValue::Date(b)))
        | (Ok(PropertyValue::Timestamp(a)), Ok(PropertyValue::Timestamp(b))) => a.cmp(&b),
        _ => left.to_string().cmp(&right.to_string()),
    }
}
