//! Action type structural checks (hard errors): references (V09) and parameter,
//! rule and value-source consistency (V12).
//!
//! A reference that does not resolve is reported once; the checks that need what
//! it points at are skipped so one typo does not produce a chain of errors.

use crate::diag::{Diagnostic, SourceLocation, codes, did_you_mean};
use crate::ir::basetype::{check_literal, widens_to};
use crate::ir::{
    ActionType, BaseType, CreateObjectRule, LinkRule, LinkTypeName, ModifyObjectRule, ObjectType,
    ObjectTypeName, Ontology, ParamType, ParameterName, PropertyDef, PropertyName, Rule,
    ValueSource,
};
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

struct Cx<'a> {
    ontology: &'a Ontology,
    action: &'a ActionType,
    key: DefKey,
    sources: &'a SourceMap,
}

impl Cx<'_> {
    fn at(&self, path: &[Seg], part: Part) -> SourceLocation {
        self.sources.locate(&self.key, path, part)
    }

    /// Location of `rules[i]` followed by the given keys.
    fn rule_at(&self, i: usize, rest: &[&str], part: Part) -> SourceLocation {
        let mut path = vec![key("rules"), Seg::Index(i)];
        path.extend(rest.iter().map(|s| key(s)));
        self.at(&path, part)
    }
}

fn key(name: &str) -> Seg {
    Seg::Key(name.to_owned())
}

pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for action in ontology.action_types.values() {
        let cx = Cx {
            ontology,
            action,
            key: DefKey {
                kind: DefKind::ActionType,
                api_name: action.api_name.to_string(),
            },
            sources,
        };
        parameters(&cx, out);
        criteria(&cx, out);
        if action.rules.is_empty() {
            out.push(
                Diagnostic::error(
                    codes::V12,
                    format!("action `{}` has no rules", action.api_name),
                    &cx.at(&[key("rules")], Part::Value),
                )
                .with_help("add at least one rule, for example `kind: createObject`"),
            );
        }
        for (i, rule) in action.rules.iter().enumerate() {
            match rule {
                Rule::CreateObject(r) => create_object(&cx, i, r, out),
                Rule::ModifyObject(r) => modify_object(&cx, i, r, out),
                Rule::DeleteObject(r) => {
                    object_param(&cx, i, &r.object, out);
                }
                Rule::AddLink(r) | Rule::RemoveLink(r) => link_rule(&cx, i, r, out),
            }
        }
    }
}

/// `d` with a "did you mean" help when `input` is close to one of `names`.
fn suggest<'a>(d: Diagnostic, input: &str, names: impl IntoIterator<Item = &'a str>) -> Diagnostic {
    match did_you_mean(input, names) {
        Some(s) => d.with_help(format!("did you mean `{s}`?")),
        None => d,
    }
}

fn type_name(t: BaseType, items: Option<BaseType>) -> String {
    match (t, items) {
        (BaseType::Array, Some(inner)) => format!("array of {}", inner.as_str()),
        _ => t.as_str().to_owned(),
    }
}

fn parameters(cx: &Cx, out: &mut Vec<Diagnostic>) {
    for (name, p) in &cx.action.parameters {
        let name = name.as_str();
        let at = |field: &str, part: Part| cx.at(&[key("parameters"), key(name), key(field)], part);
        let base = p.param_type.base_type();
        match (&p.param_type, base, p.items) {
            (ParamType::ObjectReference(target), _, _) => {
                if !cx.ontology.object_types.contains_key(target.as_str()) {
                    let here = cx.at(
                        &[
                            key("parameters"),
                            key(name),
                            key("type"),
                            key("objectReference"),
                        ],
                        Part::Value,
                    );
                    let d = Diagnostic::error(
                        codes::V09,
                        format!("parameter `{name}` refers to unknown object type `{target}`"),
                        &here,
                    );
                    let names = cx.ontology.object_types.keys().map(ObjectTypeName::as_str);
                    out.push(suggest(d, target.as_str(), names));
                }
            }
            (_, Some(BaseType::Array), None) => out.push(
                Diagnostic::error(
                    codes::V05,
                    format!("array parameter `{name}` must declare `items`"),
                    &at("type", Part::Value),
                )
                .with_help("add `items: <element type>`, for example `items: string`"),
            ),
            _ => {}
        }
        if p.items.is_some() && base != Some(BaseType::Array) {
            out.push(
                Diagnostic::error(
                    codes::V06,
                    format!(
                        "`items` is only allowed on array parameters, but `{name}` is not an array"
                    ),
                    &at("items", Part::Key),
                )
                .with_help("remove `items`, or change the type to `array`"),
            );
        }
        let Some(default) = &p.default else {
            continue;
        };
        match base {
            None => out.push(
                Diagnostic::error(
                    codes::V12,
                    format!("object reference parameter `{name}` cannot have a default"),
                    &at("default", Part::Value),
                )
                .with_help("remove `default`; callers must pass the object"),
            ),
            Some(t) => {
                if let Err(reason) = check_literal(default, t, p.items) {
                    out.push(Diagnostic::error(
                        codes::V12,
                        format!(
                            "default of parameter `{name}` is not a valid `{}` value: {reason}",
                            type_name(t, p.items)
                        ),
                        &at("default", Part::Value),
                    ));
                }
            }
        }
    }
}

fn criteria(cx: &Cx, out: &mut Vec<Diagnostic>) {
    for (i, c) in cx.action.submission_criteria.iter().enumerate() {
        for (field, text) in [
            ("condition", &c.condition),
            ("failureMessage", &c.failure_message),
        ] {
            if text.trim().is_empty() {
                let here = cx.at(
                    &[key("submissionCriteria"), Seg::Index(i), key(field)],
                    Part::Value,
                );
                out.push(Diagnostic::error(
                    codes::V12,
                    format!("submission criterion `{field}` must not be empty"),
                    &here,
                ));
            }
        }
    }
}

fn create_object(cx: &Cx, i: usize, r: &CreateObjectRule, out: &mut Vec<Diagnostic>) {
    let Some(object) = cx.ontology.object_types.get(r.object_type.as_str()) else {
        let d = Diagnostic::error(
            codes::V09,
            format!(
                "createObject refers to unknown object type `{}`",
                r.object_type
            ),
            &cx.rule_at(i, &["objectType"], Part::Value),
        );
        let names = cx.ontology.object_types.keys().map(ObjectTypeName::as_str);
        out.push(suggest(d, r.object_type.as_str(), names));
        return;
    };
    properties(cx, i, object, &r.properties, out);
    let missing: Vec<String> = object
        .properties
        .iter()
        .filter(|(name, p)| p.required || **name == object.primary_key)
        .filter(|(name, _)| !r.properties.contains_key(name.as_str()))
        .map(|(name, _)| format!("`{name}`"))
        .collect();
    if !missing.is_empty() {
        out.push(
            Diagnostic::error(
                codes::V12,
                format!(
                    "createObject of `{}` does not assign the primary key or required properties: {}",
                    object.api_name,
                    missing.join(", ")
                ),
                &cx.rule_at(i, &["properties"], Part::Key),
            )
            .with_help("add each missing property under `properties` with a value source"),
        );
    }
}

fn modify_object(cx: &Cx, i: usize, r: &ModifyObjectRule, out: &mut Vec<Diagnostic>) {
    let Some(object) = object_param(cx, i, &r.object, out) else {
        return;
    };
    for name in r.properties.keys() {
        if *name == object.primary_key {
            out.push(
                Diagnostic::error(
                    codes::V12,
                    format!(
                        "modifyObject cannot assign the primary key `{name}` of `{}`",
                        object.api_name
                    ),
                    &cx.rule_at(i, &["properties", name.as_str()], Part::Key),
                )
                .with_help("an action cannot change an object's primary key"),
            );
        }
    }
    properties(cx, i, object, &r.properties, out);
}

fn link_rule(cx: &Cx, i: usize, r: &LinkRule, out: &mut Vec<Diagnostic>) {
    let Some(link) = cx.ontology.link_types.get(r.link.as_str()) else {
        let d = Diagnostic::error(
            codes::V09,
            format!("rule refers to unknown link type `{}`", r.link),
            &cx.rule_at(i, &["link"], Part::Value),
        );
        let names = cx.ontology.link_types.keys().map(LinkTypeName::as_str);
        out.push(suggest(d, r.link.as_str(), names));
        return;
    };
    for (field, param, end) in [("from", &r.from, &link.from), ("to", &r.to, &link.to)] {
        let Some(target) = reference_param(cx, i, field, param, out) else {
            continue;
        };
        let exists = cx.ontology.object_types.contains_key(target.as_str());
        if exists && *target != end.object_type {
            out.push(Diagnostic::error(
                codes::V12,
                format!(
                    "parameter `{param}` references `{target}`, but the `{field}` end of link `{}` is `{}`",
                    link.api_name, end.object_type
                ),
                &cx.rule_at(i, &[field], Part::Value),
            ));
        }
    }
}

/// The object type a rule's `object` parameter refers to. Reports an unknown
/// parameter (V09) or one that is not an object reference (V12).
fn object_param<'a>(
    cx: &Cx<'a>,
    i: usize,
    name: &ParameterName,
    out: &mut Vec<Diagnostic>,
) -> Option<&'a ObjectType> {
    let target = reference_param(cx, i, "object", name, out)?;
    cx.ontology.object_types.get(target.as_str())
}

/// Resolve parameter `name`, written in field `field` of `rules[i]`, to the object
/// type it references.
fn reference_param<'a>(
    cx: &Cx<'a>,
    i: usize,
    field: &str,
    name: &ParameterName,
    out: &mut Vec<Diagnostic>,
) -> Option<&'a ObjectTypeName> {
    let here = || cx.rule_at(i, &[field], Part::Value);
    let Some(def) = cx.action.parameters.get(name.as_str()) else {
        let d = Diagnostic::error(
            codes::V09,
            format!("`{field}` refers to unknown parameter `{name}`"),
            &here(),
        );
        let names = cx.action.parameters.keys().map(ParameterName::as_str);
        out.push(suggest(d, name.as_str(), names));
        return None;
    };
    if let ParamType::ObjectReference(target) = &def.param_type {
        return Some(target);
    }
    out.push(
        Diagnostic::error(
            codes::V12,
            format!("parameter `{name}` must be an objectReference parameter"),
            &here(),
        )
        .with_help("declare it as `type: { objectReference: <ObjectType> }`"),
    );
    None
}

fn properties(
    cx: &Cx,
    i: usize,
    object: &ObjectType,
    assigned: &indexmap::IndexMap<PropertyName, ValueSource>,
    out: &mut Vec<Diagnostic>,
) {
    for (name, source) in assigned {
        let Some(prop) = object.properties.get(name.as_str()) else {
            let d = Diagnostic::error(
                codes::V09,
                format!("`{}` has no property `{name}`", object.api_name),
                &cx.rule_at(i, &["properties", name.as_str()], Part::Key),
            );
            let names = object.properties.keys().map(PropertyName::as_str);
            out.push(suggest(d, name.as_str(), names));
            continue;
        };
        value_source(cx, i, name, prop, source, out);
    }
}

fn value_source(
    cx: &Cx,
    i: usize,
    name: &PropertyName,
    prop: &PropertyDef,
    source: &ValueSource,
    out: &mut Vec<Diagnostic>,
) {
    let here = cx.rule_at(i, &["properties", name.as_str(), source.key()], Part::Value);
    let prop_type = type_name(prop.base_type, prop.items);
    match source {
        ValueSource::Param(param) => {
            let Some(def) = cx.action.parameters.get(param.as_str()) else {
                let d = Diagnostic::error(
                    codes::V09,
                    format!("`param` refers to unknown parameter `{param}`"),
                    &here,
                );
                let names = cx.action.parameters.keys().map(ParameterName::as_str);
                out.push(suggest(d, param.as_str(), names));
                return;
            };
            let Some(t) = def.param_type.base_type() else {
                out.push(
                    Diagnostic::error(
                        codes::V12,
                        format!(
                            "object reference parameter `{param}` cannot be assigned to property `{name}`"
                        ),
                        &here,
                    )
                    .with_help("pass the primary key as a `string` parameter instead"),
                );
                return;
            };
            if !compatible((t, def.items), (prop.base_type, prop.items)) {
                out.push(Diagnostic::error(
                    codes::V12,
                    format!(
                        "parameter `{param}` has type `{}`, but property `{name}` has type `{prop_type}`",
                        type_name(t, def.items)
                    ),
                    &here,
                ));
            }
        }
        ValueSource::Static(value) => {
            let reason = if value.is_null() {
                Some(
                    "a static value cannot be null; leave the property out to leave it unset"
                        .to_owned(),
                )
            } else {
                check_literal(value, prop.base_type, prop.items).err()
            };
            if let Some(reason) = reason {
                out.push(Diagnostic::error(
                    codes::V12,
                    format!(
                        "static value for property `{name}` (`{prop_type}`) is invalid: {reason}"
                    ),
                    &here,
                ));
            }
        }
        ValueSource::CurrentUser(_) if prop.base_type != BaseType::String => {
            out.push(Diagnostic::error(
                codes::V12,
                format!("`currentUser` can only be assigned to a string property, but `{name}` is `{prop_type}`"),
                &here,
            ));
        }
        ValueSource::CurrentTime(_) if prop.base_type != BaseType::Timestamp => {
            out.push(Diagnostic::error(
                codes::V12,
                format!("`currentTime` can only be assigned to a timestamp property, but `{name}` is `{prop_type}`"),
                &here,
            ));
        }
        ValueSource::CurrentUser(_) | ValueSource::CurrentTime(_) => {}
    }
}

/// Whether a value of type `from` can be assigned to a property of type `to`:
/// numbers only widen ([`widens_to`]); arrays also compare their element types.
fn compatible(from: (BaseType, Option<BaseType>), to: (BaseType, Option<BaseType>)) -> bool {
    if !widens_to(from.0, to.0) {
        return false;
    }
    match (from.1, to.1) {
        (Some(a), Some(b)) => widens_to(a, b),
        _ => true,
    }
}
