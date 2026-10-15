//! ONT-W03: a property name must not be an encoded abbreviation (D-13).
//!
//! Patterns, each linear-time under the `regex` crate (T-01-33):
//! - `^dt[A-Z]`: a Hungarian date prefix (`dtCreated`);
//! - `^n[A-Z]{2,}`: a Hungarian count prefix (`nTOTAL`; `nTotal` and `nCount` have one capital and pass);
//! - `\d{2,}$`: two or more trailing digits (`code01`; `level2` passes);
//! - `^field[A-Z0-9]`: a spreadsheet column name (`field1`).
//!
//! The last pattern is anchored at the start (Claude's Discretion, do not revert).
//! Unanchored, it would flag `airfieldCode`. The `snake_case` and leading underscore
//! branches of the original table are covered by `ONT-V07` and are not repeated.

use std::sync::LazyLock;

use regex::Regex;

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, property_finding};

static ENCODED: LazyLock<[Regex; 4]> = LazyLock::new(|| {
    [r"^dt[A-Z]", r"^n[A-Z]{2,}", r"\d{2,}$", r"^field[A-Z0-9]"]
        .map(|pattern| Regex::new(pattern).expect("fixed pattern is valid"))
});

pub(super) struct W03;

pub(super) static RULE: W03 = W03;

impl LintRule for W03 {
    fn id(&self) -> &'static str {
        "ONT-W03"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            for name in object.properties.keys() {
                if let Some(pattern) = ENCODED.iter().find(|re| re.is_match(name.as_str())) {
                    out.push(property_finding(
                        object,
                        name.as_str(),
                        format!(
                            "property `{name}` looks like an encoded abbreviation (matches `{}`)",
                            pattern.as_str()
                        ),
                        format!(
                            "spell the name out, such as `createdAt` or `totalCount`; {}",
                            off_hint(self.id())
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::naming::testing::{flagged, objects, run};

    #[test]
    fn triggers() {
        let text = objects(&[(
            "Record",
            &[
                ("dtCreated", "date"),
                ("nTOTAL", "integer"),
                ("code01", "string"),
                ("field1", "string"),
                ("fieldA", "string"),
            ],
        )]);
        let found = run("ONT-W03", &text);
        assert_eq!(
            flagged(&text, &found),
            ["dtCreated", "nTOTAL", "code01", "field1", "fieldA"]
        );
        assert!(found[0].message.contains("^dt[A-Z]"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[(
            "Record",
            &[
                ("createdAt", "timestamp"),
                ("level2", "integer"),
                ("airfieldCode", "string"),
                ("nCount", "integer"),
            ],
        )]);
        let found = run("ONT-W03", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
