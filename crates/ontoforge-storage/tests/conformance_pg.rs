//! The conformance cases on the PostgreSQL edit store (STORE-01, STORE-03).
//!
//! Every test is `#[ignore]`d: they need a PostgreSQL server. Run them with
//! `ONTOFORGE_PG_URL=postgres://... cargo test -p ontoforge-storage --test conformance_pg -- --ignored`
//! or, on a machine that can download and run PostgreSQL, with
//! `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg -- --ignored`.
//! Without either, `support::pg_store()` panics.

mod conformance;
mod support;

conformance::all_cases!(
    conformance::pg,
    ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"
);
