//! Ontology definition IR, YAML/JSON loader, source map and diagnostics.

pub mod diag;
pub mod ir;
pub mod loader;

pub use loader::{LoadOutput, load_dir, load_from_memory};
