//! Row predicate scope: the Rust side of "one definition, two executions" (ADR-0018).
//!
//! A role's `rowFilter` is compiled once into an own `Pred` (`ontoforge_expr`). The Rust
//! evaluator (`ontoforge_expr::eval`) runs it against one row; [`lower`] turns the same
//! `Pred` into the printer AST (`ontoforge_engine::printer::ast`), with the caller's
//! principal attributes substituted as literals, so that Trino and the SQLite stand-in see
//! an equivalent filter (ADR-0025). The semantics are three-valued and `Unknown` is
//! invisible; the differential tests in `tests/rowpred_diff.rs` keep both sides equal.
//!
//! [`merge`] combines the roles of one principal on one object type into a [`TypeScope`]
//! that can be evaluated in Rust or lowered to SQL.
//!
//! [`compile_scope`] is the single offline check of a scope directory against an ontology
//! (D-01): it validates every reference, compiles the row predicates and produces a
//! [`CompiledScope`], from which [`CompiledScope::for_principal`] resolves one caller.

pub mod compile;
mod coverage;
pub mod lower;
pub mod merge;

pub use compile::{
    CompiledRole, CompiledScope, PrincipalScope, ScopeInputs, attr_types, compile_scope,
};
pub use lower::lower;
pub use merge::{LoweredScope, RoleTypeGrant, RowVisibility, TypeScope, merge};
