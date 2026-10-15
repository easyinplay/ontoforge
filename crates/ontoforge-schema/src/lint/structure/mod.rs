//! Structure rules ONT-W06 to ONT-W11. Plan 01-11 registers them here, so plans
//! 01-10 and 01-11 never edit `lint/mod.rs` at the same time.

use super::Finding;
use super::LintRule;
use crate::ir::ObjectType;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

mod w06;
mod w07;
mod w08;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    vec![&w06::RULE, &w07::RULE, &w08::RULE]
}

/// Appended to every structure help text: these rules are heuristics, so the way out
/// is part of the message.
fn off_hint(rule_id: &str) -> String {
    format!("if this is intentional, set \"{rule_id}\" = \"off\" under [lint] in ontoforge.toml")
}

/// A finding on the `apiName` value of `object`.
fn type_finding(object: &ObjectType, message: String, help: String) -> Finding {
    Finding {
        def: DefKey {
            kind: DefKind::ObjectType,
            api_name: object.api_name.to_string(),
        },
        path: vec![Seg::Key("apiName".to_owned())],
        part: Part::Value,
        message,
        help: Some(help),
        severity: None,
    }
}

/// Fixture helpers shared by the rule tests.
#[cfg(test)]
pub(super) mod testing {
    use crate::diag::Diagnostic;
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    /// One object type document. `pk` is added as a required string property, then
    /// `props` as `(name, base type)` pairs.
    pub fn object(name: &str, pk: &str, props: &[(&str, &str)]) -> String {
        let mut lines = vec![
            "kind: ObjectType".to_owned(),
            format!("apiName: {name}"),
            format!("primaryKey: {pk}"),
            "properties:".to_owned(),
            format!("  {pk}: {{ type: string, required: true }}"),
        ];
        lines.extend(
            props
                .iter()
                .map(|(prop, ty)| format!("  {prop}: {{ type: {ty} }}")),
        );
        lines.join("\n") + "\n"
    }

    /// Join documents into one multi-document file.
    pub fn join(docs: &[String]) -> String {
        docs.join("---\n")
    }

    pub fn run(rule_id: &str, text: &str) -> Vec<Diagnostic> {
        run_with(rule_id, text, None)
    }

    pub fn run_with(rule_id: &str, text: &str, config: Option<&str>) -> Vec<Diagnostic> {
        lint_rule_on(
            rule_id,
            &[("ontology.yaml", MANIFEST), ("defs.yaml", text)],
            config,
        )
    }

    /// The source text each diagnostic points at, in report order.
    pub fn flagged<'a>(text: &'a str, found: &[Diagnostic]) -> Vec<&'a str> {
        found
            .iter()
            .map(|d| &text[d.span.offset..d.span.offset + d.span.length])
            .collect()
    }
}
