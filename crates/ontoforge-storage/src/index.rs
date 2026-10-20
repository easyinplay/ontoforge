//! Index reconciliation for the properties row predicates read (plan 03-16, orchestrator
//! decision 8: indexes are only ever added).
//!
//! RED stub.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_schema::ir::{ObjectTypeName, PropertyName};

use crate::ddl::DdlStep;
use crate::physical::PhysicalSchema;

#[allow(dead_code, reason = "RED stub of plan 03-16")]
pub(crate) fn role_index_steps(
    _physical: &PhysicalSchema,
    _props: &BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>,
) -> Vec<DdlStep> {
    Vec::new()
}
