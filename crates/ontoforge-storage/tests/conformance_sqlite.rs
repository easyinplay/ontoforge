//! The conformance cases on the SQLite edit store (STORE-01, STORE-03).
//!
//! The same cases run on PostgreSQL in `conformance_pg.rs`.

mod conformance;
mod support;

conformance::all_cases!(conformance::sqlite);
