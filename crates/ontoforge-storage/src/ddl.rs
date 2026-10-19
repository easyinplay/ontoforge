//! DDL of the edit tables.

use crate::physical::{LinkTable, ObjectTable, PropertyColumn};
use crate::store::Backend;

/// One schema change `reload` can apply without losing data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdlStep {
    CreateObjectTable(ObjectTable),
    AddPropertyColumn {
        table: String,
        column: PropertyColumn,
    },
    CreateLinkTable(LinkTable),
}

/// The statements of `step`, one operation each.
pub fn render(step: &DdlStep, backend: Backend, schema_edit: Option<&str>) -> Vec<String> {
    let _ = (step, backend, schema_edit);
    Vec::new()
}
