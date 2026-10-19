//! ONT-E09: a base type must fit its use (D-12).
//!
//! Checked:
//! - a primary key whose type `pk_suitability` calls discouraged (date, timestamp,
//!   boolean, byte, long) is a warning. Invalid key types are `ONT-V10`, not here.
//! - a title key must have a type `is_title_suitable` accepts. Every v1 type does,
//!   so this never fires today; the call keeps the rule honest when types are added.
//! - an array whose `items` is itself an array (nested arrays) is an error, for
//!   object type properties and action parameters alike.
//!
//! One rule id carries both severities: a configured level in `ontoforge.toml`
//! replaces both, an unconfigured run keeps each finding's own severity.

use super::{Finding, Level, LintCx, LintRule};
use crate::diag::Severity;
use crate::ir::BaseType;
use crate::ir::basetype::{PkSuitability, is_title_suitable, pk_suitability};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

pub(super) struct E09;

pub(super) static RULE: E09 = E09;

impl LintRule for E09 {
    fn id(&self) -> &'static str {
        "ONT-E09"
    }

    fn default_level(&self) -> Level {
        Level::Error
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            let def = DefKey {
                kind: DefKind::ObjectType,
                api_name: object.api_name.to_string(),
            };
            let finding =
                |path: Vec<Seg>, message: String, help: Option<String>, severity| Finding {
                    def: def.clone(),
                    path,
                    part: Part::Value,
                    message,
                    help,
                    severity,
                };
            let key = |name: &str| Seg::Key(name.to_owned());

            if let Some(pk) = object.properties.get(&object.primary_key)
                && pk_suitability(pk.base_type) == PkSuitability::Discouraged
            {
                out.push(finding(
                    vec![key("primaryKey")],
                    format!(
                        "primary key `{}` is a {}, which is discouraged",
                        object.primary_key,
                        pk.base_type.as_str()
                    ),
                    Some("prefer string, integer or short for a primary key".to_owned()),
                    Some(Severity::Warning),
                ));
            }
            if let Some(title) = &object.title_key
                && let Some(prop) = object.properties.get(title)
                && !is_title_suitable(prop.base_type, prop.items)
            {
                out.push(finding(
                    vec![key("titleKey")],
                    format!(
                        "title key `{title}` is a {}, which cannot serve as a title",
                        prop.base_type.as_str()
                    ),
                    None,
                    None,
                ));
            }
            for (name, prop) in &object.properties {
                if prop.base_type == BaseType::Array && prop.items == Some(BaseType::Array) {
                    out.push(finding(
                        vec![key("properties"), key(name.as_str()), key("items")],
                        format!("array property `{name}` has array items; nested arrays are not supported"),
                        Some("use a scalar element type, or model the inner list as its own object type".to_owned()),
                        None,
                    ));
                }
            }
        }
        // Parameters hold array elements too (WR-09).
        for action in cx.ontology.action_types.values() {
            for (name, param) in &action.parameters {
                if param.param_type.base_type() == Some(BaseType::Array)
                    && param.items == Some(BaseType::Array)
                {
                    out.push(Finding {
                        def: DefKey {
                            kind: DefKind::ActionType,
                            api_name: action.api_name.to_string(),
                        },
                        path: ["parameters", name.as_str(), "items"]
                            .map(|k| Seg::Key(k.to_owned()))
                            .to_vec(),
                        part: Part::Value,
                        message: format!(
                            "array parameter `{name}` has array items; nested arrays are not supported"
                        ),
                        help: Some("use a scalar element type".to_owned()),
                        severity: None,
                    });
                }
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
apiName: Event
primaryKey: at
properties:
  at: { type: timestamp, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Counter
primaryKey: n
properties:
  n: { type: long, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Grid
primaryKey: id
properties:
  id: { type: string, required: true }
  cells: { type: array, items: array }
backing: editOnly
";

    const CLEAN: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: id
properties:
  id: { type: string, required: true }
  tags: { type: array, items: string }
backing: editOnly
---
kind: ObjectType
apiName: Seat
primaryKey: row
properties:
  row: { type: short, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Gate
primaryKey: number
properties:
  number: { type: integer, required: true }
backing: editOnly
";

    fn run(objects: &str, config: Option<&str>) -> Vec<Diagnostic> {
        lint_rule_on(
            "ONT-E09",
            &[("ontology.yaml", MANIFEST), ("objects.yaml", objects)],
            config,
        )
    }

    #[test]
    fn triggers() {
        let found = run(TRIGGERS, None);
        assert_eq!(found.len(), 3, "{found:#?}");
        let severities: Vec<_> = found.iter().map(|d| (d.line, d.severity)).collect();
        assert_eq!(
            severities,
            [
                (3, Severity::Warning),
                (10, Severity::Warning),
                (20, Severity::Error)
            ]
        );
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let found = run(CLEAN, None);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    /// WR-09: action parameters are array elements too.
    #[test]
    fn nested_array_parameter_is_an_error() {
        let text = format!(
            "{CLEAN}---
kind: ActionType
apiName: fill
parameters:
  name: {{ type: string, required: true }}
  grid: {{ type: array, items: array }}
  tags: {{ type: array, items: string }}
rules:
  - kind: createObject
    objectType: Flight
    properties:
      id: {{ param: name }}
"
        );
        let found = run(&text, None);
        assert_eq!(found.len(), 1, "{found:#?}");
        let d = &found[0];
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("parameter `grid`"), "{}", d.message);
        assert_eq!(&text[d.span.offset..d.span.offset + d.span.length], "array");
        assert!(text[..d.span.offset].ends_with("grid: { type: array, items: "));
    }

    #[test]
    fn e09_config_override_applies_to_all_findings() {
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E09\" = \"error\"\n"));
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|d| d.severity == Severity::Error));
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E09\" = \"info\"\n"));
        assert!(found.iter().all(|d| d.severity == Severity::Info));
    }
}
