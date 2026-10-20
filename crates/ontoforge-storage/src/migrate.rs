//! Destructive definition changes through `ontoforge migrate` (STORE-02, D-14, D-16).
//!
//! `reload` refuses a change that could lose or reinterpret data (ONT-V25). `migrate` is the way
//! out: `plan_migration` turns the difference into a fixed list of steps with a hash, and
//! `apply_migration` runs exactly that list, in one transaction, only if the hash still matches.
//! Nothing is deleted: an old column or table is renamed (archived) and stays in the database
//! until `plan_drop_archived` / `apply_drop_archived` remove it on explicit request.

use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::Ontology;
use serde::{Deserialize, Serialize};

use crate::ddl::DdlStep;
use crate::error::StoreError;
use crate::store::{ActiveDefinition, EditStore};

/// A rename the deployer declares, because a rename shows up in the diff as a removal plus an
/// addition (D-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenameDecl {
    /// `Type.old=new`: a property of the object type `type_name` (its name in the new definition).
    Property {
        type_name: String,
        from: String,
        to: String,
    },
    /// `OldType=NewType`.
    ObjectType { from: String, to: String },
}

impl std::str::FromStr for RenameDecl {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Err(format!("not implemented: {text}"))
    }
}

/// One step of a migration. A step does one thing; the SQL of a step may be several statements
/// (SQLite allows one operation per `ALTER TABLE`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MigrationStep {
    /// A step `reload` could also run: new tables, new columns, indexes.
    Ddl(DdlStep),
    /// The value column and its `__set` column get the archive names; the data stays.
    ArchiveColumn {
        type_name: String,
        property: String,
        table: String,
        column: String,
        archived: String,
        set_archived: String,
    },
    /// The value column and its `__set` column move to the new property name.
    RenameColumn {
        type_name: String,
        table: String,
        /// Property names.
        from: String,
        to: String,
        from_column: String,
        to_column: String,
    },
    RenameTable {
        from_type: String,
        to_type: String,
        from: String,
        to: String,
    },
    ArchiveTable {
        type_name: String,
        table: String,
        archived: String,
    },
    DropArchivedColumn {
        table: String,
        column: String,
    },
    DropArchivedTable {
        table: String,
    },
}

/// The steps of a migration and the hash that identifies them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub from_def_version: i64,
    pub renames: Vec<RenameDecl>,
    pub steps: Vec<MigrationStep>,
    /// FNV-1a 64 bit, 16 lower case hex digits, over the other fields and the new definition.
    pub hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    /// The migration cannot be planned; the diagnostics say why.
    #[error("the migration cannot be planned")]
    Definition(Vec<Diagnostic>),
    /// The plan that `apply` computed is not the plan that was reviewed.
    #[error("the migration plan changed: expected {expected}, now {actual}")]
    PlanChanged { expected: String, actual: String },
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Plans the migration from the `active` definition to `new`.
pub fn plan_migration(
    active: &ActiveDefinition,
    _new: &Ontology,
    renames: &[RenameDecl],
) -> Result<MigrationPlan, Vec<Diagnostic>> {
    Ok(MigrationPlan {
        from_def_version: active.def_version,
        renames: renames.to_vec(),
        steps: Vec::new(),
        hash: String::new(),
    })
}

/// Runs the migration to `new` if its plan still has the hash `expected_hash`.
pub async fn apply_migration(
    _store: &EditStore,
    _new: &Ontology,
    _renames: &[RenameDecl],
    expected_hash: &str,
) -> Result<i64, MigrateError> {
    Err(MigrateError::PlanChanged {
        expected: expected_hash.to_owned(),
        actual: String::new(),
    })
}
