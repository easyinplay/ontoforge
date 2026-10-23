use std::collections::BTreeMap;

use ontoforge_auth::Principal;
use ontoforge_schema::ir::{ActionType, Backing, LinkRule, Rule, ValueSource};
use ontoforge_storage::commit::{Edit, LinkOp, ObjectOp};
use ontoforge_storage::query_read::{LinkEndpoint, MAX_EDIT_READ};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use ontoforge_storage::value::PropertyValue;
use serde_json::Value;

use crate::{ActionError, params::ResolvedItem};

pub(crate) async fn expand(
    store: &EditStore,
    active: &ActiveDefinition,
    action: &ActionType,
    rule: &Rule,
    item: &ResolvedItem,
    principal: &Principal,
    chosen_current_time: &str,
) -> Result<(Edit, i64), ActionError> {
    match rule {
        Rule::CreateObject(rule) => {
            let ty = active
                .ontology
                .object_types
                .get(&rule.object_type)
                .ok_or_else(|| ActionError::InvalidRequest("unknown object type".to_owned()))?;
            if ty.backing != Backing::EditOnly {
                return Err(ActionError::UnsupportedRule);
            }
            let mut set = properties(ty, &rule.properties, item, principal, chosen_current_time)?;
            let pk = set.remove(&ty.primary_key).ok_or_else(|| {
                ActionError::InvalidRequest("missing create primary key".to_owned())
            })?;
            Ok((
                Edit::Object {
                    type_name: rule.object_type.clone(),
                    pk,
                    op: ObjectOp::Set(set),
                },
                0,
            ))
        }
        Rule::ModifyObject(rule) => {
            let (type_name, pk, version) =
                reference(store, active, action, item, &rule.object).await?;
            let ty = active
                .ontology
                .object_types
                .get(&type_name)
                .ok_or_else(|| ActionError::InvalidRequest("unknown object type".to_owned()))?;
            let set = properties(ty, &rule.properties, item, principal, chosen_current_time)?;
            Ok((
                Edit::Object {
                    type_name,
                    pk,
                    op: ObjectOp::Set(set),
                },
                version,
            ))
        }
        Rule::DeleteObject(rule) => {
            let (type_name, pk, version) =
                reference(store, active, action, item, &rule.object).await?;
            Ok((
                Edit::Object {
                    type_name,
                    pk,
                    op: ObjectOp::Tombstone,
                },
                version,
            ))
        }
        Rule::AddLink(link_rule) => {
            expand_link(store, active, action, item, link_rule, LinkOp::Add).await
        }
        Rule::RemoveLink(link_rule) => {
            expand_link(store, active, action, item, link_rule, LinkOp::Remove).await
        }
    }
}

async fn expand_link(
    store: &EditStore,
    active: &ActiveDefinition,
    action: &ActionType,
    item: &ResolvedItem,
    rule: &LinkRule,
    op: LinkOp,
) -> Result<(Edit, i64), ActionError> {
    let link = active
        .ontology
        .link_types
        .get(&rule.link)
        .ok_or_else(|| ActionError::InvalidRequest("unknown link type".to_owned()))?;
    if link.backing != Some(Backing::EditOnly) {
        return Err(ActionError::UnsupportedRule);
    }
    let (from_type, a, _) = reference(store, active, action, item, &rule.from).await?;
    let (to_type, b, _) = reference(store, active, action, item, &rule.to).await?;
    if from_type != link.from.object_type || to_type != link.to.object_type {
        return Err(ActionError::InvalidRequest(
            "link endpoint type mismatch".to_owned(),
        ));
    }
    let observed = store
        .read_link_edits(
            active,
            &rule.link,
            LinkEndpoint::From(a.clone()),
            None,
            MAX_EDIT_READ,
        )
        .await?;
    if observed.has_more {
        return Err(ActionError::InvalidRequest(
            "link read limit exceeded".to_owned(),
        ));
    }
    let version = observed
        .edits
        .iter()
        .find(|edit| edit.to == b)
        .map_or(0, |edit| edit.row.version);
    Ok((
        Edit::Link {
            link: rule.link.clone(),
            a,
            b,
            op,
        },
        version,
    ))
}

async fn reference(
    store: &EditStore,
    active: &ActiveDefinition,
    action: &ActionType,
    item: &ResolvedItem,
    name: &ontoforge_schema::ir::ParameterName,
) -> Result<(ontoforge_schema::ir::ObjectTypeName, PropertyValue, i64), ActionError> {
    let def = action
        .parameters
        .get(name)
        .ok_or_else(|| ActionError::InvalidRequest("unknown object parameter".to_owned()))?;
    let ontoforge_schema::ir::ParamType::ObjectReference(type_name) = &def.param_type else {
        return Err(ActionError::InvalidRequest(
            "object parameter is not a reference".to_owned(),
        ));
    };
    let ty = active
        .ontology
        .object_types
        .get(type_name)
        .ok_or_else(|| ActionError::InvalidRequest("unknown object type".to_owned()))?;
    if ty.backing != Backing::EditOnly {
        return Err(ActionError::UnsupportedRule);
    }
    let pk = item
        .references
        .get(name)
        .cloned()
        .ok_or_else(|| ActionError::InvalidRequest("missing object reference".to_owned()))?;
    let observed = store
        .read_object_edit(active, type_name, &pk)
        .await?
        .ok_or(ActionError::NotFound)?;
    if observed.row.deleted || observed.row.released {
        return Err(ActionError::NotFound);
    }
    Ok((type_name.clone(), pk, observed.row.version))
}

fn properties(
    ty: &ontoforge_schema::ir::ObjectType,
    sources: &indexmap::IndexMap<ontoforge_schema::ir::PropertyName, ValueSource>,
    item: &ResolvedItem,
    principal: &Principal,
    chosen_current_time: &str,
) -> Result<BTreeMap<ontoforge_schema::ir::PropertyName, PropertyValue>, ActionError> {
    sources
        .iter()
        .map(|(name, source)| {
            let def = ty
                .properties
                .get(name)
                .ok_or_else(|| ActionError::InvalidRequest("unknown rule property".to_owned()))?;
            let raw = match source {
                ValueSource::Param(param) => {
                    item.values.get(param.as_str()).cloned().ok_or_else(|| {
                        ActionError::InvalidRequest(format!("missing parameter `{param}`"))
                    })?
                }
                ValueSource::Static(value) => value.clone(),
                ValueSource::CurrentUser(_) => Value::String(principal.id.as_str().to_owned()),
                ValueSource::CurrentTime(_) => Value::String(chosen_current_time.to_owned()),
            };
            let value = PropertyValue::from_json(def, &raw)
                .map_err(|e| ActionError::InvalidRequest(format!("invalid `{name}` value: {e}")))?;
            Ok((name.clone(), value))
        })
        .collect()
}
