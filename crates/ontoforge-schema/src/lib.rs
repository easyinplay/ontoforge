//! Ontology definition IR, YAML/JSON loader, source map and diagnostics.

pub mod diag;
pub mod ir;
pub mod lint;
pub mod loader;
pub mod schema_export;
pub mod validate;

pub use loader::{
    LoadOutput, ScopeLoadOutput, load_dir, load_from_memory, load_scope_dir, load_scope_from_memory,
};
