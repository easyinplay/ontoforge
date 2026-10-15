//! Structure rules ONT-W06 to ONT-W11. Plan 01-11 registers them here, so plans
//! 01-10 and 01-11 never edit `lint/mod.rs` at the same time.

use super::LintRule;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    Vec::new()
}
