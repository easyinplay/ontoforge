//! ONT-E13: a decimal needs a valid precision and scale (BIND-02).
//!
//! Applies to every decimal property (and every array of decimals), whatever the
//! backing: the precision and scale are part of the type, not of the binding. The
//! effective values come from [`PropertyDef::effective_decimal`], the only place the
//! defaults are defined.

use super::element_type;
use crate::ir::{BaseType, PropertyDef};
use crate::lint::{Finding, Level, LintCx, LintRule};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

/// The largest precision a decimal may have.
const MAX_PRECISION: u32 = 38;

pub(super) struct E13;

pub(super) static RULE: E13 = E13;

impl LintRule for E13 {
    fn id(&self) -> &'static str {
        "ONT-E13"
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
            for (name, property) in &object.properties {
                if element_type(property) != BaseType::Decimal {
                    continue;
                }
                let Some((field, message, help)) = problem(name.as_str(), property) else {
                    continue;
                };
                out.push(Finding {
                    def: def.clone(),
                    path: ["properties", name.as_str(), field]
                        .map(|k| Seg::Key(k.to_owned()))
                        .to_vec(),
                    part: Part::Value,
                    message,
                    help: Some(help),
                    severity: None,
                });
            }
        }
    }
}

/// What is wrong with the decimal `property`, as the field to point at, a message and
/// a help text. At most one problem per property: an out-of-range precision first.
fn problem(name: &str, property: &PropertyDef) -> Option<(&'static str, String, String)> {
    let (precision, scale) = property.effective_decimal();
    if !(1..=MAX_PRECISION).contains(&precision) {
        return Some((
            "precision",
            format!(
                "decimal property `{name}` has precision {precision}, which is outside 1 to {MAX_PRECISION}"
            ),
            format!("write a `precision` between 1 and {MAX_PRECISION}"),
        ));
    }
    if scale <= precision {
        return None;
    }
    if property.scale.is_some() {
        Some((
            "scale",
            format!(
                "decimal property `{name}` has scale {scale}, which is greater than its precision {precision}"
            ),
            format!("write a `scale` between 0 and {precision}"),
        ))
    } else {
        // Only the precision is written, so the default scale is what is too large.
        Some((
            "precision",
            format!(
                "decimal property `{name}` has precision {precision}, which is smaller than the default scale {scale}"
            ),
            format!("the default scale is {scale}; declare `scale` explicitly"),
        ))
    }
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
