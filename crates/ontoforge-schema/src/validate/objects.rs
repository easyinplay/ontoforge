//! Object type structural checks (hard errors): primary key, title key, `items`.
//!
//! Discouraged primary key types and nested arrays are lint territory (E09), not
//! errors here; both read [`pk_suitability`].

use crate::diag::{Diagnostic, SourceLocation, codes, did_you_mean};
use crate::ir::basetype::{PkSuitability, pk_suitability};
use crate::ir::{BaseType, ObjectType, Ontology, PropertyName};
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for object in ontology.object_types.values() {
        let key = DefKey {
            kind: DefKind::ObjectType,
            api_name: object.api_name.to_string(),
        };
        let at = |path: &[&str], part: Part| {
            let path: Vec<Seg> = path.iter().map(|s| Seg::Key((*s).to_owned())).collect();
            sources.locate(&key, &path, part)
        };
        primary_key(object, &at, out);
        title_key(object, &at, out);
        items(object, &at, out);
    }
}

type At<'a> = dyn Fn(&[&str], Part) -> SourceLocation + 'a;

fn suggest(d: Diagnostic, name: &PropertyName, object: &ObjectType) -> Diagnostic {
    let candidates = object.properties.keys().map(PropertyName::as_str);
    match did_you_mean(name.as_str(), candidates) {
        Some(s) => d.with_help(format!("did you mean `{s}`?")),
        None => d,
    }
}

fn primary_key(object: &ObjectType, at: &At, out: &mut Vec<Diagnostic>) {
    let pk = &object.primary_key;
    let Some(prop) = object.properties.get(pk.as_str()) else {
        let d = Diagnostic::error(
            codes::V10,
            format!(
                "primaryKey `{pk}` is not a property of `{}`",
                object.api_name
            ),
            &at(&["primaryKey"], Part::Value),
        );
        out.push(suggest(d, pk, object));
        return;
    };
    if !prop.required {
        out.push(
            Diagnostic::error(
                codes::V10,
                format!("primary key property `{pk}` must be required"),
                &at(&["properties", pk.as_str()], Part::Key),
            )
            .with_help("declare `required: true` on the primary key property"),
        );
    }
    if pk_suitability(prop.base_type) == PkSuitability::Invalid {
        out.push(
            Diagnostic::error(
                codes::V10,
                format!(
                    "primary key property `{pk}` has type `{}`, which cannot identify an object",
                    prop.base_type.as_str()
                ),
                &at(&["primaryKey"], Part::Value),
            )
            .with_help("use a `string`, `integer` or `short` property as the primary key"),
        );
    }
}

fn title_key(object: &ObjectType, at: &At, out: &mut Vec<Diagnostic>) {
    let Some(title) = &object.title_key else {
        return;
    };
    if object.properties.contains_key(title.as_str()) {
        return;
    }
    let d = Diagnostic::error(
        codes::V10,
        format!(
            "titleKey `{title}` is not a property of `{}`",
            object.api_name
        ),
        &at(&["titleKey"], Part::Value),
    );
    out.push(suggest(d, title, object));
}

fn items(object: &ObjectType, at: &At, out: &mut Vec<Diagnostic>) {
    for (name, prop) in &object.properties {
        let name = name.as_str();
        match (prop.base_type, prop.items) {
            (BaseType::Array, None) => out.push(
                Diagnostic::error(
                    codes::V05,
                    format!("array property `{name}` must declare `items`"),
                    &at(&["properties", name, "type"], Part::Value),
                )
                .with_help("add `items: <element type>`, for example `items: string`"),
            ),
            (t, Some(_)) if t != BaseType::Array => out.push(
                Diagnostic::error(
                    codes::V06,
                    format!(
                        "`items` is only allowed on array properties, but `{name}` is `{}`",
                        t.as_str()
                    ),
                    &at(&["properties", name, "items"], Part::Key),
                )
                .with_help("remove `items`, or change the type to `array`"),
            ),
            _ => {}
        }
    }
}
