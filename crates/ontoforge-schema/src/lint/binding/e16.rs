//! ONT-E16: geopoint properties cannot be read from a source (ADR-0015).
//!
//! A geopoint has no representation in the data source types a binding reads, so a
//! `backing: source` object type may not declare one (or an array of them). An
//! `editOnly` object type may.

use super::element_type;
use crate::ir::{Backing, BaseType};
use crate::lint::{Finding, Level, LintCx, LintRule};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

pub(super) struct E16;

pub(super) static RULE: E16 = E16;

impl LintRule for E16 {
    fn id(&self) -> &'static str {
        "ONT-E16"
    }

    fn default_level(&self) -> Level {
        Level::Error
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            if object.backing != Backing::Source {
                continue;
            }
            let def = DefKey {
                kind: DefKind::ObjectType,
                api_name: object.api_name.to_string(),
            };
            for (name, property) in &object.properties {
                if element_type(property) != BaseType::Geopoint {
                    continue;
                }
                // Point at the type that is the geopoint: `items` of an array.
                let field = if property.base_type == BaseType::Array {
                    "items"
                } else {
                    "type"
                };
                out.push(Finding {
                    def: def.clone(),
                    path: ["properties", name.as_str(), field]
                        .map(|k| Seg::Key(k.to_owned()))
                        .to_vec(),
                    part: Part::Value,
                    message: format!(
                        "geopoint properties cannot be read from a source binding (property `{name}` of `{}`)",
                        object.api_name
                    ),
                    help: Some(format!(
                        "give `{}` `backing: editOnly`, or remove the property",
                        object.api_name
                    )),
                    severity: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::{Diagnostic, Severity};
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    const TRIGGERS: &str = "\
kind: ObjectType
apiName: Place
primaryKey: id
properties:
  id: { type: string, required: true }
  location: { type: geopoint }
  waypoints: { type: array, items: geopoint }
backing: source
";

    const CLEAN: &str = "\
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
  location: { type: geopoint }
  approaches: { type: array, items: geopoint }
backing: editOnly
---
kind: ObjectType
apiName: Flight
primaryKey: id
properties:
  id: { type: string, required: true }
  tags: { type: array, items: string }
backing: source
";

    fn run(objects: &str) -> Vec<Diagnostic> {
        lint_rule_on(
            "ONT-E16",
            &[("ontology.yaml", MANIFEST), ("objects.yaml", objects)],
            None,
        )
    }

    #[test]
    fn triggers() {
        let found = run(TRIGGERS);
        assert_eq!(found.len(), 2, "{found:#?}");
        assert!(found.iter().all(|d| d.severity == Severity::Error));
        let texts: Vec<&str> = found
            .iter()
            .map(|d| &TRIGGERS[d.span.offset..d.span.offset + d.span.length])
            .collect();
        assert_eq!(texts, ["geopoint", "geopoint"]);
        assert_eq!((found[0].line, found[1].line), (6, 7));
        assert!(
            found[0]
                .message
                .contains("geopoint properties cannot be read from a source binding"),
            "{}",
            found[0].message
        );
        let help = found[0].help.as_deref().unwrap_or_default();
        assert!(help.contains("editOnly"), "{help}");
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let found = run(CLEAN);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
