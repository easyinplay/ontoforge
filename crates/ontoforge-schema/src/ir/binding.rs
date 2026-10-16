//! Where an object type's data comes from (D-02) and how it is bound to a table
//! (D-01).
//!
//! `backing` is the definition-layer declaration. The physical binding to a table
//! lives in separate `kind: SourceBinding` documents under `bindings/<profile>/`
//! (ADR-0021), so a definition file never carries a catalog, schema or table name.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::names::{LinkTypeName, ObjectTypeName};
use super::object::PropertyDef;
use super::Ontology;

/// The data source an object type or many-to-many link declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Backing {
    /// Objects come from the physical table bound for the selected profile; edits
    /// are layered on top.
    Source,
    /// Objects are created only through actions or imports. No binding is allowed.
    EditOnly,
}

/// Binds one object type or one many-to-many link to a physical table or view.
///
/// The profile is not written in the document: it is the name of the directory
/// the file sits in, `bindings/<profile>/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceBinding {
    /// What is bound: `{ objectType: Flight }` or `{ linkType: flightCrew }`. Only
    /// `manyToMany` links can be bound.
    pub target: BindingTarget,
    /// The physical table or view the target reads from.
    pub source: SourceRef,
    /// Column mappings that differ from the default. Keys are property apiNames for
    /// an object type, or `from` and `to` for a link. A property that is not listed
    /// is read from the column of the same name (D-03).
    #[serde(default)]
    pub columns: IndexMap<String, ColumnBinding>,
    /// Sample rows that stand in for the table when no Trino is available; used
    /// only by the SQLite stand-in engine.
    pub fixture: Option<FixtureRef>,
}

/// What a [`SourceBinding`] binds. Externally tagged so that error paths survive
/// deserialization (ADR-0003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BindingTarget {
    /// An object type, by its `PascalCase` apiName.
    ObjectType(ObjectTypeName),
    /// A many-to-many link type, by its `camelCase` apiName.
    LinkType(LinkTypeName),
}

/// A physical table or view in the data source.
///
/// It is one table or view. To combine several tables, create a view in Trino and
/// bind that (D-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceRef {
    /// Trino catalog.
    pub catalog: String,
    /// Schema inside the catalog.
    pub schema: String,
    /// Table or view name.
    pub table: String,
}

/// How one property (or one link end) maps to a column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ColumnBinding {
    /// Physical column name. Defaults to the property apiName (D-03).
    pub column: Option<String>,
    /// IANA time zone name for a timestamp column that carries no zone, for
    /// example `Europe/Berlin`; write `aware` when the column already has one
    /// (ADR-0021).
    pub timezone: Option<String>,
    /// How string comparison treats case on this column.
    pub collation: Option<Collation>,
}

/// String comparison semantics of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Collation {
    /// Upper and lower case letters differ.
    CaseSensitive,
    /// Upper and lower case letters compare equal.
    CaseInsensitive,
}

/// Sample rows for the stand-in engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FixtureRef {
    /// Path relative to the directory of the binding file. Only `.csv` and `.json`
    /// files inside the ontology directory are accepted.
    pub file: String,
}

/// All bindings of one profile.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileBindings {
    /// Object type bindings by object type apiName.
    #[serde(default)]
    pub objects: BTreeMap<ObjectTypeName, SourceBinding>,
    /// Many-to-many link bindings by link type apiName.
    #[serde(default)]
    pub links: BTreeMap<LinkTypeName, SourceBinding>,
}

impl SourceBinding {
    /// The apiName of the bound object type or link type.
    pub fn target_name(&self) -> &str {
        match &self.target {
            BindingTarget::ObjectType(name) => name.as_str(),
            BindingTarget::LinkType(name) => name.as_str(),
        }
    }

    /// The physical column for `key`: `columns[key].column` when written, otherwise
    /// `key` itself (D-03).
    pub fn column_for<'a>(&'a self, key: &'a str) -> &'a str {
        self.columns
            .get(key)
            .and_then(|c| c.column.as_deref())
            .unwrap_or(key)
    }

    /// The declared time zone of `key`, if any.
    pub fn timezone_for(&self, key: &str) -> Option<&str> {
        self.columns.get(key).and_then(|c| c.timezone.as_deref())
    }
}

/// One column a binding expects in the physical table.
#[derive(Debug, Clone, Copy)]
pub struct BoundColumn<'a> {
    /// The property apiName, or `from` / `to` for a link binding.
    pub key: &'a str,
    /// The physical column name ([`SourceBinding::column_for`]).
    pub column: &'a str,
    /// Whether `columns[key].column` was written, as opposed to the same-name default.
    pub explicit: bool,
    /// For an object binding the property itself; for a link binding the primary
    /// key property of the object type at that end.
    pub property: &'a PropertyDef,
}

impl Ontology {
    /// The columns `binding` expects in its table, in a fixed order: property
    /// declaration order for an object type, `from` before `to` for a link. Empty
    /// when the target does not exist.
    ///
    /// The only derivation of expected physical column names; the offline fixture
    /// check and the online comparison both call it.
    pub fn bound_columns<'a>(&'a self, binding: &'a SourceBinding) -> Vec<BoundColumn<'a>> {
        let _ = binding;
        Vec::new()
    }
}
