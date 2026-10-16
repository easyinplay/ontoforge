//! Fixture planning and loading for the SQLite stand-in.

use ontoforge_schema::ir::{Ontology, ProfileName};
use ontoforge_schema::loader::SourceMap;

use crate::TableRef;
use crate::bind_check::BindingKey;

/// The logical type of a fixture column, derived from the property type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalType {
    String,
    Integer { min: i64, max: i64 },
    Float,
    Decimal { precision: u32, scale: u32 },
    Boolean,
    Date,
    Timestamp { aware: bool },
    Array(Box<LogicalType>),
}

/// A column a fixture table must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureColumn {
    pub name: String,
    pub logical: LogicalType,
}

/// One physical table to build; bindings that name the same table share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureTable {
    pub def_names: Vec<String>,
    pub table: TableRef,
    /// Path relative to the ontology root.
    pub file: String,
    pub columns: Vec<FixtureColumn>,
}

/// A problem with a fixture. It never repeats cell contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureError {
    pub file: String,
    pub line: Option<u64>,
    pub message: String,
}

/// The tables to build for one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixturePlan {
    pub tables: Vec<FixtureTable>,
    pub without_fixture: Vec<BindingKey>,
}

/// Plans the fixture tables of the bindings of `profile`.
pub fn plan_fixture_tables(
    _ontology: &Ontology,
    _sources: &SourceMap,
    _profile: &ProfileName,
) -> Result<FixturePlan, Vec<FixtureError>> {
    Ok(FixturePlan {
        tables: Vec::new(),
        without_fixture: Vec::new(),
    })
}
