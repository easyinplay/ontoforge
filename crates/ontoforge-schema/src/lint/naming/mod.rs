//! Naming rules ONT-W01 to ONT-W05. Plan 01-10 registers them here, so plans 01-10
//! and 01-11 never edit `lint/mod.rs` at the same time.

use super::LintRule;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    Vec::new()
}
