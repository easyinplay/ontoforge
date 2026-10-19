//! Difference between two physical schemas.

use ontoforge_schema::diag::Diagnostic;

use crate::ddl::DdlStep;
use crate::physical::PhysicalSchema;

/// What `reload` would do, and what it refuses to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<DdlStep>,
    pub destructive: Vec<DestructiveChange>,
}

/// A definition change that would lose or reinterpret data. `reload` never applies one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestructiveChange {
    ObjectTypeRemoved { type_name: String },
    LinkTypeRemoved { link: String },
    PropertyRemoved { type_name: String, property: String },
    PropertyTypeChanged { type_name: String, property: String },
    PrimaryKeyChanged { type_name: String },
    RequiredPropertyAdded { type_name: String, property: String },
    LinkEndsChanged { link: String },
}

impl DestructiveChange {
    /// The ONT-V25 diagnostic of this change.
    pub fn diagnostic(&self) -> Diagnostic {
        unimplemented!("diagnostic")
    }
}

/// Compares the active physical schema `old` (none before the first reload) with `new`.
pub fn diff(old: Option<&PhysicalSchema>, new: &PhysicalSchema) -> Plan {
    let _ = (old, new);
    Plan::default()
}
