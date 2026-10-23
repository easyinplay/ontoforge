use std::collections::BTreeMap;

use cel::{Context, Value as CelValue};
use ontoforge_auth::{
    Principal,
    scope::{PrincipalScope, RowVisibility},
};
use ontoforge_expr::{Scalar, analyze, member_paths};
use ontoforge_schema::ir::{ActionType, ObjectTypeName, ParameterName, PropertyName, Rule};
use ontoforge_storage::{
    commit::{
        ActionEditPosition, ActionGuardRejection as Denied, ActionGuardView, Edit, ObjectOp,
        RowState,
    },
    state::EditRow,
    store::ActiveDefinition,
    value::PropertyValue,
};
use serde_json::{Map, Value, json};

use crate::params::ResolvedItem;

pub(crate) fn is_target_ref(action: &ActionType, name: &ParameterName) -> bool {
    action.rules.iter().any(|rule| match rule {
        Rule::ModifyObject(rule) => &rule.object == name,
        Rule::DeleteObject(rule) => &rule.object == name,
        _ => false,
    })
}

fn scalar(value: Value) -> Scalar {
    match value {
        Value::Bool(v) => Scalar::Bool(v),
        Value::Number(v) => v.as_i64().map_or_else(
            || v.as_f64().map_or(Scalar::Null, Scalar::Double),
            Scalar::Int,
        ),
        Value::String(v) => Scalar::String(v),
        _ => Scalar::Null,
    }
}

fn visibility(
    scope: &PrincipalScope,
    principal: &Principal,
    active: &ActiveDefinition,
    type_name: &ObjectTypeName,
    pk: &PropertyValue,
    row: &EditRow,
) -> Option<RowVisibility> {
    let ty = active.ontology.object_types.get(type_name)?;
    let type_scope = scope.types.get(type_name)?;
    let mut values: BTreeMap<PropertyName, Scalar> = row
        .set
        .iter()
        .map(|(name, value)| (name.clone(), scalar(value.to_json())))
        .collect();
    values.insert(ty.primary_key.clone(), scalar(pk.to_json()));
    Some(type_scope.evaluate(&values, &principal.attrs))
}

fn allowed_row(
    scope: &PrincipalScope,
    principal: &Principal,
    active: &ActiveDefinition,
    type_name: &ObjectTypeName,
    pk: &PropertyValue,
    row: &EditRow,
) -> Result<RowVisibility, Denied> {
    if row.version == 0 || row.deleted || row.released {
        return Err(Denied::NotFound);
    }
    visibility(scope, principal, active, type_name, pk, row)
        .filter(|visible| visible.visible)
        .ok_or(Denied::NotFound)
}

fn criterion(
    condition: &str,
    failure_message: &str,
    item: &ResolvedItem,
    visible: &BTreeMap<String, (RowVisibility, Map<String, Value>)>,
    principal: &Principal,
) -> Result<(), Denied> {
    let objects: Map<String, Value> = visible
        .iter()
        .map(|(name, (_, row))| (name.clone(), Value::Object(row.clone())))
        .collect();
    analyze(condition, |program| {
        let paths = member_paths(program, condition);
        for path in paths
            .iter()
            .filter(|path| path.segments.first().map(String::as_str) == Some("objects"))
        {
            if path.dynamic || path.segments.len() < 3 {
                return Err(Denied::ScopeDenied);
            }
            let Some((permission, _)) = visible.get(&path.segments[1]) else {
                return Err(Denied::ScopeDenied);
            };
            if !permission
                .columns
                .iter()
                .any(|column| column.as_str() == path.segments[2])
            {
                return Err(Denied::ScopeDenied);
            }
        }
        let mut context = Context::default();
        context
            .add_variable("params", &item.values)
            .map_err(|_| Denied::ScopeDenied)?;
        context
            .add_variable("objects", objects)
            .map_err(|_| Denied::ScopeDenied)?;
        context
            .add_variable(
                "user",
                json!({
                    "id": principal.id.as_str(), "groups": principal.groups,
                }),
            )
            .map_err(|_| Denied::ScopeDenied)?;
        match program.execute(&context) {
            Ok(CelValue::Bool(true)) => Ok(()),
            _ => Err(Denied::CriteriaFailed(failure_message.to_owned())),
        }
    })
    .map_err(|_| Denied::ScopeDenied)?
}

type VisibleObjects = BTreeMap<String, (RowVisibility, Map<String, Value>)>;

#[derive(Clone, Copy)]
pub(crate) struct Authorization<'a> {
    pub active: &'a ActiveDefinition,
    pub action: &'a ActionType,
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub items: &'a [ResolvedItem],
    pub edits: &'a [Edit],
    pub positions: &'a [ActionEditPosition],
}

fn masked_items(
    input: Authorization<'_>,
    view: &ActionGuardView,
) -> Result<Vec<VisibleObjects>, Denied> {
    let Authorization {
        active,
        action,
        principal,
        scope,
        items,
        edits,
        positions,
    } = input;
    let mut item_objects: Vec<VisibleObjects> = vec![BTreeMap::new(); items.len()];
    for (item_ordinal, (item, objects)) in items.iter().zip(&mut item_objects).enumerate() {
        for (name, pk) in &item.references {
            let Some(ontoforge_schema::ir::ParamType::ObjectReference(type_name)) =
                action.parameters.get(name).map(|def| &def.param_type)
            else {
                return Err(Denied::ScopeDenied);
            };
            let read = view
                .reads
                .iter()
                .find(|read| &read.type_name == type_name && &read.pk == pk)
                .ok_or(Denied::NotFound)?;
            let first_edit = view.steps.iter().filter(|step| {
                positions[step.ordinal].item_ordinal == item_ordinal &&
                matches!(&edits[step.ordinal], Edit::Object { type_name: edited_type, pk: edited_pk, .. }
                    if edited_type == type_name && edited_pk == pk)
            }).min_by_key(|step| step.ordinal);
            let state = first_edit.map_or(&read.row, |step| {
                let RowState::Object(before) = &step.before else {
                    unreachable!("object edit state")
                };
                before
            });
            let permission =
                allowed_row(scope, principal, active, type_name, pk, state).map_err(|error| {
                    if is_target_ref(action, name) {
                        error
                    } else {
                        Denied::ScopeDenied
                    }
                })?;
            let mut row: Map<String, Value> = state
                .set
                .iter()
                .filter(|(name, _)| permission.columns.contains(*name))
                .map(|(name, value)| (name.as_str().to_owned(), value.to_json()))
                .collect();
            let ty = active
                .ontology
                .object_types
                .get(type_name)
                .ok_or(Denied::ScopeDenied)?;
            if permission.columns.contains(&ty.primary_key) {
                row.insert(ty.primary_key.as_str().to_owned(), pk.to_json());
            }
            objects.insert(name.as_str().to_owned(), (permission, row));
        }
    }
    Ok(item_objects)
}

pub(crate) fn authorize(input: Authorization<'_>, view: &ActionGuardView) -> Result<(), Denied> {
    let Authorization {
        active,
        action,
        principal,
        scope,
        items,
        edits,
        ..
    } = input;
    let item_objects = masked_items(input, view)?;

    for step in &view.steps {
        let edit = &edits[step.ordinal];
        if let (
            Edit::Object { type_name, pk, op },
            RowState::Object(before),
            RowState::Object(after),
        ) = (edit, &step.before, &step.after)
        {
            let pre = if before.version > 0 {
                Some(allowed_row(
                    scope, principal, active, type_name, pk, before,
                )?)
            } else {
                None
            };
            if let ObjectOp::Set(values) = op {
                let created = if pre.is_none() {
                    Some(
                        visibility(scope, principal, active, type_name, pk, after)
                            .filter(|v| v.visible)
                            .ok_or(Denied::ScopeDenied)?,
                    )
                } else {
                    None
                };
                let grant = pre
                    .as_ref()
                    .or(created.as_ref())
                    .ok_or(Denied::ScopeDenied)?;
                if values
                    .keys()
                    .any(|name| name.as_str().starts_with("of_") || !grant.columns.contains(name))
                {
                    return Err(Denied::ScopeDenied);
                }
            }
        }
    }
    for (item, objects) in items.iter().zip(&item_objects) {
        for check in &action.submission_criteria {
            criterion(
                &check.condition,
                &check.failure_message,
                item,
                objects,
                principal,
            )?;
        }
    }
    for step in &view.steps {
        if let (Edit::Object { type_name, pk, .. }, RowState::Object(after)) =
            (&edits[step.ordinal], &step.after)
            && !visibility(scope, principal, active, type_name, pk, after)
                .is_some_and(|permission| permission.visible)
        {
            return Err(Denied::ScopeDenied);
        }
    }
    check_link_endpoints(input, view)?;
    Ok(())
}

fn check_link_endpoints(input: Authorization<'_>, view: &ActionGuardView) -> Result<(), Denied> {
    let Authorization {
        active,
        principal,
        scope,
        edits,
        ..
    } = input;
    for step in &view.steps {
        if let (Edit::Link { link, a, b, .. }, RowState::Link(_)) =
            (&edits[step.ordinal], &step.after)
        {
            let definition = active
                .ontology
                .link_types
                .get(link)
                .ok_or(Denied::ScopeDenied)?;
            for (type_name, pk) in [
                (&definition.from.object_type, a),
                (&definition.to.object_type, b),
            ] {
                let read = view
                    .reads
                    .iter()
                    .find(|read| &read.type_name == type_name && &read.pk == pk)
                    .ok_or(Denied::ScopeDenied)?;
                let final_edit = view
                    .steps
                    .iter()
                    .filter(|candidate| {
                        matches!(&edits[candidate.ordinal], Edit::Object {
                        type_name: edited_type, pk: edited_pk, ..
                    } if edited_type == type_name && edited_pk == pk)
                    })
                    .max_by_key(|candidate| candidate.ordinal);
                let row = final_edit.map_or(&read.row, |candidate| {
                    let RowState::Object(after) = &candidate.after else {
                        unreachable!("object state")
                    };
                    after
                });
                allowed_row(scope, principal, active, type_name, pk, row)
                    .map_err(|_| Denied::ScopeDenied)?;
            }
        }
    }
    Ok(())
}
