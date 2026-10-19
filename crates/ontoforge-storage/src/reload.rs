//! Definition reload (D-15).

use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::Ontology;

use crate::error::StoreError;
use crate::store::EditStore;

#[derive(Debug, Clone, Copy, Default)]
pub struct ReloadOptions {
    /// Return the DDL that would run; change nothing.
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadOutcome {
    pub previous_def_version: Option<i64>,
    pub def_version: Option<i64>,
    pub changed: bool,
    pub ddl: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    #[error("the definition cannot be applied")]
    Definition(Vec<Diagnostic>),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Brings the edit tables in line with `ontology`.
pub async fn reload(
    store: &EditStore,
    ontology: &Ontology,
    opts: &ReloadOptions,
) -> Result<ReloadOutcome, ReloadError> {
    let _ = (store, ontology, opts);
    Err(ReloadError::Store(StoreError::Config(
        "reload is not implemented yet".to_owned(),
    )))
}
