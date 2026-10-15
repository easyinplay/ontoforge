//! ONT-W08: Rule of three.
//!
//! When at least three object types share at least three properties with the same
//! name and base type, the shared set is a candidate for an interface or a shared
//! property (a v2 capability). Each shared property set is reported once, on the
//! first object type by `apiName`.

use super::super::{Finding, Level, LintCx, LintRule};

/// Types that must share a property set.
const MIN_TYPES: usize = 3;
/// Properties the types must share.
const MIN_SHARED: usize = 3;

pub(super) struct W08;

pub(super) static RULE: W08 = W08;

impl LintRule for W08 {
    fn id(&self) -> &'static str {
        "ONT-W08"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {
        let _ = (MIN_TYPES, MIN_SHARED);
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run};

    const AUDIT: [(&str, &str); 3] = [
        ("createdAt", "timestamp"),
        ("updatedAt", "timestamp"),
        ("ownerId", "string"),
    ];

    #[test]
    fn triggers() {
        let text = join(&[
            object("Alpha", "alphaId", &AUDIT),
            object("Beta", "betaId", &AUDIT),
            object("Gamma", "gammaId", &AUDIT),
        ]);
        let found = run("ONT-W08", &text);
        assert_eq!(flagged(&text, &found), ["Alpha"]);
        let message = &found[0].message;
        for word in [
            "`Alpha`",
            "`Beta`",
            "`Gamma`",
            "`createdAt`",
            "`updatedAt`",
            "`ownerId`",
        ] {
            assert!(message.contains(word), "{word} missing from {message}");
        }
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            // Three types share two properties only.
            object("Alpha", "alphaId", &AUDIT[..2]),
            object("Beta", "betaId", &AUDIT[..2]),
            object("Gamma", "gammaId", &AUDIT[..2]),
            // Two types share three properties.
            object("Delta", "deltaId", &[("x", "string"), ("y", "string"), ("z", "string")]),
            object("Epsilon", "epsilonId", &[("x", "string"), ("y", "string"), ("z", "string")]),
            // Same name, different base type does not count.
            object("Zeta", "zetaId", &[("createdAt", "date"), ("updatedAt", "date")]),
        ]);
        let found = run("ONT-W08", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn a_shared_set_is_reported_once_for_four_types() {
        let text = join(&[
            object("Alpha", "alphaId", &AUDIT),
            object("Beta", "betaId", &AUDIT),
            object("Gamma", "gammaId", &AUDIT),
            object("Delta", "deltaId", &AUDIT),
        ]);
        let found = run("ONT-W08", &text);
        assert_eq!(flagged(&text, &found), ["Alpha"]);
        assert!(found[0].message.contains("`Delta`"));
    }
}
