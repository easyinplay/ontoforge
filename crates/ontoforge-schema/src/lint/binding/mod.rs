//! Source binding rules ONT-E13 to ONT-E16 and ONT-W17 (BIND-02, D-03). They check
//! the type contract between an ontology and its bindings without any connection to
//! a data source; what only the data source knows is `validate --online`.

use super::LintRule;

mod e14;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    vec![&e14::RULE]
}
