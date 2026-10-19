//! Row predicate scope: the Rust side of "one definition, two executions" (ADR-0018).
//!
//! A role's `rowFilter` is compiled once into an own `Pred` (`ontoforge_expr`). The Rust
//! evaluator (`ontoforge_expr::eval`) runs it against one row; this module lowers the same
//! `Pred` into the printer AST (`ontoforge_engine::printer::ast`), with the caller's
//! principal attributes substituted as literals, so that Trino and the SQLite stand-in see
//! an equivalent filter (ADR-0025). The semantics are three-valued and `Unknown` is
//! invisible; the differential tests in `tests/rowpred_diff.rs` keep both sides equal.

pub mod lower;

pub use lower::lower;
