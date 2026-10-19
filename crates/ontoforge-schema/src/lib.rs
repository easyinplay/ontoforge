//! Ontology definition IR, YAML/JSON loader, source map and diagnostics.

pub mod diag;
pub mod ir;
pub mod lint;
pub mod loader;
pub mod schema_export;
pub mod validate;

pub use loader::{LoadOutput, load_dir, load_from_memory};
