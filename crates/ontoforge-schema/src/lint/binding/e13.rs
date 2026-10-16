//! ONT-E13: a decimal needs a valid precision and scale (BIND-02).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct E13;

pub(super) static RULE: E13 = E13;

impl LintRule for E13 {
    fn id(&self) -> &'static str {
        "ONT-E13"
    }

    fn default_level(&self) -> Level {
        Level::Error
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
}

#[cfg(test)]
mod tests {
    use crate::diag::{Diagnostic, Severity};
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    const TRIGGERS: &str = "\
kind: ObjectType
apiName: Money
primaryKey: id
properties:
  id: { type: string, required: true }
  tooWide: { type: decimal, precision: 39 }
  zero: { type: decimal, precision: 0 }
  scaleAbove: { type: decimal, precision: 12, scale: 13 }
  defaultScale: { type: decimal, precision: 5 }
  list: { type: array, items: decimal, precision: 40 }
backing: editOnly
";

    const CLEAN: &str = "\
kind: ObjectType
apiName: Money
primaryKey: id
properties:
  id: { type: string, required: true }
  plain: { type: decimal }
  full: { type: decimal, precision: 38, scale: 38 }
  integral: { type: decimal, precision: 1, scale: 0 }
  price: { type: decimal, precision: 12, scale: 2 }
  prices: { type: array, items: decimal }
  name: { type: string }
backing: source
";

    fn run(objects: &str, config: Option<&str>) -> Vec<Diagnostic> {
        lint_rule_on(
            "ONT-E13",
            &[("ontology.yaml", MANIFEST), ("objects.yaml", objects)],
            config,
        )
    }

    fn flagged<'a>(text: &'a str, found: &[Diagnostic]) -> Vec<&'a str> {
        found
            .iter()
            .map(|d| &text[d.span.offset..d.span.offset + d.span.length])
            .collect()
    }

    #[test]
    fn triggers() {
        let found = run(TRIGGERS, None);
        assert_eq!(found.len(), 5, "{found:#?}");
        assert!(found.iter().all(|d| d.severity == Severity::Error));
        assert_eq!(flagged(TRIGGERS, &found), ["39", "0", "13", "5", "40"]);
        // Only `precision: 5` relies on the default scale of 10, and says so.
        let help = |i: usize| found[i].help.as_deref().unwrap_or_default();
        assert!(
            help(3).contains("declare `scale` explicitly"),
            "{}",
            help(3)
        );
        assert!(!help(2).contains("default scale"), "{}", help(2));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let found = run(CLEAN, None);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn e13_can_be_downgraded() {
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E13\" = \"warn\"\n"));
        assert_eq!(found.len(), 5);
        assert!(found.iter().all(|d| d.severity == Severity::Warning));
    }
}
