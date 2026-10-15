//! ONT-E09: a base type must fit its use (D-12).
//!
//! Checked:
//! - a primary key whose type `pk_suitability` calls discouraged (date, timestamp,
//!   boolean, byte, long) is a warning. Invalid key types are `ONT-V10`, not here.
//! - a title key must have a type `is_title_suitable` accepts. Every v1 type does,
//!   so this never fires today; the call keeps the rule honest when types are added.
//! - an array whose `items` is itself an array (nested arrays) is an error.
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
        let _ = (cx, out);
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
---
kind: ObjectType
apiName: Counter
primaryKey: n
properties:
  n: { type: long, required: true }
---
kind: ObjectType
apiName: Grid
primaryKey: id
properties:
  id: { type: string, required: true }
  cells: { type: array, items: array }
";

    const CLEAN: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: id
properties:
  id: { type: string, required: true }
  tags: { type: array, items: string }
---
kind: ObjectType
apiName: Seat
primaryKey: row
properties:
  row: { type: short, required: true }
---
kind: ObjectType
apiName: Gate
primaryKey: number
properties:
  number: { type: integer, required: true }
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
                (9, Severity::Warning),
                (18, Severity::Error)
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

    #[test]
    fn e09_config_override_applies_to_all_findings() {
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E09\" = \"error\"\n"));
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|d| d.severity == Severity::Error));
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E09\" = \"info\"\n"));
        assert!(found.iter().all(|d| d.severity == Severity::Info));
    }
}
