use std::collections::{BTreeMap, BTreeSet};

use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::Engine;
use ontoforge_expr::{analyze, member_paths};
use ontoforge_query::{QueryError, read_pk_source};
use ontoforge_schema::ir::{ActionType, Backing, ObjectTypeName, ParamType, PropertyName, Rule};
use ontoforge_storage::{
    commit::ActionGuardRead,
    store::{ActiveDefinition, EditStore},
    value::PropertyValue,
};
use serde_json::Value;

use crate::{ActionError, ActionItem, ActionLimits, criteria, params::ResolvedItem};

pub(crate) type RowKey = (ObjectTypeName, String);
pub(crate) type SourceSnapshots = BTreeMap<RowKey, BTreeMap<PropertyName, Value>>;

pub(crate) fn row_key(type_name: &ObjectTypeName, pk: &PropertyValue) -> RowKey {
    (type_name.clone(), pk.to_json().to_string())
}

pub(crate) struct PreparedSources {
    pub reads: Vec<ActionGuardRead>,
    pub snapshots: SourceSnapshots,
}

#[expect(
    clippy::too_many_arguments,
    reason = "source preparation receives the Action read context"
)]
pub(crate) async fn prepare<E: Engine>(
    store: &EditStore,
    engine: Option<&E>,
    active: &ActiveDefinition,
    action: &ActionType,
    scope: &PrincipalScope,
    requests: &[ActionItem],
    items: &[ResolvedItem],
    limits: ActionLimits,
) -> Result<PreparedSources, ActionError> {
    let mut reads = Vec::new();
    let mut snapshots = SourceSnapshots::new();
    let mut seen = BTreeSet::new();
    let mut source_calls = 0usize;
    for (request, item) in requests.iter().zip(items) {
        for (name, pk) in &item.references {
            let Some(ParamType::ObjectReference(type_name)) =
                action.parameters.get(name).map(|def| &def.param_type)
            else {
                continue;
            };
            let key = row_key(type_name, pk);
            if !seen.insert(key.clone()) {
                continue;
            }
            let ty = active
                .ontology
                .object_types
                .get(type_name)
                .ok_or_else(|| ActionError::InvalidRequest("unknown object type".to_owned()))?;
            let edit = store.read_object_edit(active, type_name, pk).await?;
            let row = edit.as_ref().map(|e| &e.row);
            let denied = || {
                if criteria::is_target_ref(action, name) {
                    ActionError::NotFound
                } else {
                    ActionError::ScopeDenied
                }
            };
            if row.is_some_and(|row| row.deleted) {
                return Err(denied());
            }
            if ty.backing == Backing::EditOnly && row.is_none_or(|row| row.released) {
                return Err(denied());
            }
            let required = required_properties(action, scope, type_name, name)?;
            let token_requires_source = request.expected_versions.contains_key(name.as_str())
                && row.is_none_or(|row| {
                    row.released
                        || ty.properties.keys().any(|property| {
                            property != &ty.primary_key && !row.set.contains_key(property)
                        })
                });
            let release = action
                .rules
                .iter()
                .any(|rule| matches!(rule, Rule::ReleaseObject(r) if &r.object == name));
            let missing = required.iter().any(|property| {
                property != &ty.primary_key && row.is_none_or(|row| !row.set.contains_key(property))
            });
            if ty.backing == Backing::Source
                && (row.is_none_or(|row| row.released)
                    || missing
                    || token_requires_source
                    || release)
            {
                source_calls = source_calls.checked_add(1).ok_or_else(|| {
                    ActionError::InvalidRequest("source call count exceeds host limits".to_owned())
                })?;
                if source_calls > limits.source_calls {
                    return Err(ActionError::InvalidRequest(
                        "source call count exceeds host limits".to_owned(),
                    ));
                }
                let engine = engine.ok_or(ActionError::EngineUnavailable)?;
                let sampled = read_pk_source(active, engine, type_name, pk, limits.request_bytes)
                    .await
                    .map_err(|error| match error {
                        QueryError::AmbiguousObject => ActionError::AmbiguousObject,
                        _ => ActionError::EngineUnavailable,
                    })?
                    .ok_or_else(denied)?;
                snapshots.insert(key, sampled);
            }
            reads.push(ActionGuardRead {
                type_name: type_name.clone(),
                pk: pk.clone(),
                observed_version: row.map_or(0, |row| row.version),
            });
        }
    }
    Ok(PreparedSources { reads, snapshots })
}

fn required_properties(
    action: &ActionType,
    scope: &PrincipalScope,
    type_name: &ObjectTypeName,
    reference: &ontoforge_schema::ir::ParameterName,
) -> Result<BTreeSet<PropertyName>, ActionError> {
    let mut required = BTreeSet::new();
    if let Some(grant) = scope.types.get(type_name) {
        required.extend(grant.row.referenced_properties());
        for pred in grant.columns.values() {
            required.extend(pred.referenced_properties());
        }
    }
    for criterion in &action.submission_criteria {
        let paths = analyze(&criterion.condition, |program| {
            member_paths(program, &criterion.condition)
        })
        .map_err(|_| ActionError::ScopeDenied)?;
        for path in paths {
            if path.segments.first().map(String::as_str) == Some("objects")
                && path.segments.get(1).map(String::as_str) == Some(reference.as_str())
                && let Some(property) = path.segments.get(2)
            {
                required.insert(
                    property
                        .clone()
                        .try_into()
                        .map_err(|_| ActionError::ScopeDenied)?,
                );
            }
        }
    }
    Ok(required)
}
