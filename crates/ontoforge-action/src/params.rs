use std::collections::BTreeMap;

use ontoforge_schema::ir::{ActionType, ParamType, ParameterName, basetype::check_literal};
use ontoforge_storage::store::ActiveDefinition;
use ontoforge_storage::value::PropertyValue;
use serde_json::{Map, Value};

use crate::{ActionError, ActionItem, ParameterProblem};

pub(crate) struct ResolvedItem {
    pub(crate) values: Map<String, Value>,
    pub(crate) references: BTreeMap<ParameterName, PropertyValue>,
}

pub(crate) fn resolve(
    active: &ActiveDefinition,
    action: &ActionType,
    item: &ActionItem,
    item_ordinal: usize,
) -> Result<ResolvedItem, ActionError> {
    let mut problems = BTreeMap::new();
    let mut values = item.params.clone();
    let mut references = BTreeMap::new();
    for name in item.params.keys() {
        if !action.parameters.keys().any(|known| known.as_str() == name) {
            problems.insert(name.clone(), "unknown parameter".to_owned());
        }
    }
    for (name, def) in &action.parameters {
        let key = name.as_str();
        let Some(value) = item.params.get(key) else {
            if def.required {
                problems.insert(key.to_owned(), "required parameter is missing".to_owned());
            } else if let Some(default) = &def.default {
                values.insert(key.to_owned(), default.clone());
            }
            continue;
        };
        let validation = match &def.param_type {
            ParamType::ObjectReference(type_name) => active
                .ontology
                .object_types
                .get(type_name)
                .and_then(|ty| ty.properties.get(&ty.primary_key))
                .ok_or_else(|| "unknown reference type".to_owned())
                .and_then(|pk_def| {
                    PropertyValue::from_json(pk_def, value).map_err(|e| e.to_string())
                })
                .map(|pk| {
                    references.insert(name.clone(), pk);
                }),
            _ => def
                .param_type
                .base_type()
                .ok_or_else(|| "unknown parameter type".to_owned())
                .and_then(|base| check_literal(value, base, def.items)),
        };
        if let Err(reason) = validation {
            problems.insert(key.to_owned(), reason);
        }
    }
    if !problems.is_empty() {
        return Err(ActionError::ParameterErrors(
            problems
                .into_iter()
                .map(|(name, reason)| ParameterProblem {
                    item_ordinal,
                    name,
                    reason,
                })
                .collect(),
        ));
    }
    Ok(ResolvedItem { values, references })
}
