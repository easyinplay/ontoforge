//! Naming rules ONT-W01 to ONT-W05. Plan 01-10 registers them here, so plans 01-10
//! and 01-11 never edit `lint/mod.rs` at the same time.

use super::Finding;
use super::LintRule;
use super::words::split_camel;
use crate::ir::ObjectType;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

mod w01;
mod w02;
mod w03;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    vec![&w01::RULE, &w02::RULE, &w03::RULE]
}

/// Appended to every naming help text: these rules are heuristics, so the way out is
/// part of the message.
fn off_hint(rule_id: &str) -> String {
    format!(
        "if the name is intentional, set \"{rule_id}\" = \"off\" under [lint] in ontoforge.toml"
    )
}

/// A finding on the `apiName` value of `object`.
fn type_finding(object: &ObjectType, message: String, help: String) -> Finding {
    Finding {
        def: def_key(object),
        path: vec![Seg::Key("apiName".to_owned())],
        part: Part::Value,
        message,
        help: Some(help),
        severity: None,
    }
}

/// A finding on the key of `property` under `properties` of `object`.
fn property_finding(object: &ObjectType, property: &str, message: String, help: String) -> Finding {
    Finding {
        def: def_key(object),
        path: vec![
            Seg::Key("properties".to_owned()),
            Seg::Key(property.to_owned()),
        ],
        part: Part::Key,
        message,
        help: Some(help),
        severity: None,
    }
}

fn def_key(object: &ObjectType) -> DefKey {
    DefKey {
        kind: DefKind::ObjectType,
        api_name: object.api_name.to_string(),
    }
}

/// The last word of a name, lowercase (`shippingStatus` is `status`).
fn last_word(name: &str) -> Option<String> {
    split_camel(name).pop()
}

/// Fixture helpers shared by the rule tests.
#[cfg(test)]
pub(super) mod testing {
    use crate::diag::Diagnostic;
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    /// One YAML document per `(object type, [(property, type)])`. Every object type
    /// also gets a required string primary key `id`.
    pub fn objects(specs: &[(&str, &[(&str, &str)])]) -> String {
        let docs: Vec<String> = specs
            .iter()
            .map(|(name, props)| {
                let mut lines = vec![
                    "kind: ObjectType".to_owned(),
                    format!("apiName: {name}"),
                    "primaryKey: id".to_owned(),
                    "properties:".to_owned(),
                    "  id: { type: string, required: true }".to_owned(),
                ];
                lines.extend(
                    props
                        .iter()
                        .map(|(prop, ty)| format!("  {prop}: {{ type: {ty} }}")),
                );
                lines.join("\n") + "\n"
            })
            .collect();
        docs.join("---\n")
    }

    pub fn run(rule_id: &str, objects: &str) -> Vec<Diagnostic> {
        lint_rule_on(
            rule_id,
            &[("ontology.yaml", MANIFEST), ("objects.yaml", objects)],
            None,
        )
    }

    /// The source text each diagnostic points at, in report order.
    pub fn flagged<'a>(objects: &'a str, found: &[Diagnostic]) -> Vec<&'a str> {
        found
            .iter()
            .map(|d| &objects[d.span.offset..d.span.offset + d.span.length])
            .collect()
    }
}
