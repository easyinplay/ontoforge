//! Naming rules ONT-W01 to ONT-W05. Plan 01-10 registers them here, so plans 01-10
//! and 01-11 never edit `lint/mod.rs` at the same time.

use super::LintRule;

mod w01;
mod w02;
mod w03;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    vec![&w01::RULE, &w02::RULE, &w03::RULE]
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
        let mut out = String::new();
        for (name, props) in specs {
            if !out.is_empty() {
                out.push_str("---\n");
            }
            out.push_str(&format!(
                "kind: ObjectType\napiName: {name}\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n"
            ));
            for (prop, ty) in *props {
                out.push_str(&format!("  {prop}: {{ type: {ty} }}\n"));
            }
        }
        out
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
