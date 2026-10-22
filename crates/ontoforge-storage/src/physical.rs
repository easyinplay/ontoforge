//! The physical schema of the edit store, derived from an ontology (D-17, ADR-0024).
//!
//! `PhysicalSchema` is what `reload` compares between definition versions and what it stores
//! in `of_def_version.physical_json`, so every type here is serializable and stays
//! backwards compatible (new fields use `#[serde(default)]`).

use std::collections::BTreeMap;

use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{
    BaseType, Cardinality, LinkType, LinkTypeName, ObjectType, ObjectTypeName, Ontology,
    PropertyDef, PropertyName,
};
use serde::{Deserialize, Serialize};

use crate::naming;

/// Column type, one to one with [`BaseType`] (byte and short stay distinct). Any change of a
/// column type is a destructive definition change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnType {
    Byte,
    Short,
    Integer,
    Long,
    Float,
    Double,
    Boolean,
    String,
    Decimal { precision: u32, scale: u32 },
    Date,
    Timestamp,
    Array(Box<ColumnType>),
    Geopoint,
}

/// The primary key column of an object table, or an end column of a link table. It has no
/// `__set` column: a row exists only because its key was set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyColumn {
    pub property: PropertyName,
    pub column: String,
    pub ty: ColumnType,
}

/// A non-key property: its value column and the boolean column that records whether an edit
/// set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyColumn {
    pub column: String,
    pub set_column: String,
    pub ty: ColumnType,
    pub required: bool,
}

/// A column that migration moved out of the live definition but keeps in the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivedColumn {
    pub property: PropertyName,
    pub column: String,
    pub set_column: String,
    pub ty: ColumnType,
    pub def_version: i64,
}

/// A table that migration moved out of the live definition but keeps in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivedTable {
    pub type_name: String,
    pub table: String,
    pub def_version: i64,
}

/// What an index enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexKind {
    /// A lookup index, from `indexed: true`.
    Plain,
    /// Unique among live rows whose column was set: `WHERE <col>__set AND NOT of_deleted`. It
    /// is how a `oneToOne` link is enforced by the database (STORE-05).
    UniqueWhenSet,
}

/// An index on the value column of one property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
    pub name: String,
    /// The value column; the `__set` column of the partial index is derived from it.
    pub column: String,
    pub kind: IndexKind,
}

/// The table of one object type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectTable {
    pub table: String,
    pub pk: KeyColumn,
    pub properties: BTreeMap<PropertyName, PropertyColumn>,
    #[serde(default)]
    pub archived_columns: Vec<ArchivedColumn>,
    /// Indexes other than the primary key, sorted by name. Absent in a schema stored before
    /// indexes existed.
    #[serde(default)]
    pub indexes: Vec<IndexDef>,
}

/// The join table of one many-to-many link type. `a` holds the key of the `from` object type,
/// `b` the key of the `to` object type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTable {
    pub table: String,
    pub from_type: ObjectTypeName,
    pub to_type: ObjectTypeName,
    pub a: KeyColumn,
    pub b: KeyColumn,
}

/// Every table of the edit store for one definition version.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PhysicalSchema {
    pub objects: BTreeMap<ObjectTypeName, ObjectTable>,
    pub links: BTreeMap<LinkTypeName, LinkTable>,
    #[serde(default)]
    pub archived_tables: Vec<ArchivedTable>,
}

impl PhysicalSchema {
    /// Derives the physical schema of `ontology`.
    ///
    /// A validated ontology always derives; the diagnostics cover a definition that skipped
    /// validation.
    pub fn derive(ontology: &Ontology) -> Result<Self, Vec<Diagnostic>> {
        let mut errors = Vec::new();
        let mut schema = Self::default();

        for (name, object) in &ontology.object_types {
            match object_table(object) {
                Ok(table) => {
                    schema.objects.insert(name.clone(), table);
                }
                Err(message) => errors.push(derive_error(&format!("{name}: {message}"))),
            }
        }

        for (name, link) in &ontology.link_types {
            if link.cardinality == Cardinality::OneToOne {
                add_one_to_one_index(&mut schema, name, link, &mut errors);
            }
            if link.cardinality != Cardinality::ManyToMany {
                continue;
            }
            let (Some(from), Some(to)) = (
                schema.objects.get(&link.from.object_type),
                schema.objects.get(&link.to.object_type),
            ) else {
                errors.push(derive_error(&format!(
                    "link type `{name}` refers to an object type that has no table"
                )));
                continue;
            };
            let table = LinkTable {
                table: naming::link_table(name),
                from_type: link.from.object_type.clone(),
                to_type: link.to.object_type.clone(),
                a: end_column(&from.pk, "a_pk"),
                b: end_column(&to.pk, "b_pk"),
            };
            schema.links.insert(name.clone(), table);
        }

        for table in schema.objects.values_mut() {
            table.indexes.sort_by(|a, b| a.name.cmp(&b.name));
        }
        errors.extend(schema.name_collisions());

        if errors.is_empty() {
            Ok(schema)
        } else {
            Err(errors)
        }
    }

    /// ONT-V24 for every physical name two definitions share, or that a property shares with a
    /// system column. Truncation (`naming::bounded`) can fold two long names into one, and snake
    /// casing maps `ABTest` and `AbTest` to the same name, so apiNames that differ can still
    /// collide. Names only need to be unique within their namespace: tables and indexes of the
    /// edit schema, columns of one table.
    fn name_collisions(&self) -> Vec<Diagnostic> {
        let mut tables: Vec<(String, String)> = Vec::new();
        let mut indexes: Vec<(String, String)> = Vec::new();
        let mut diagnostics = Vec::new();

        for (type_name, table) in &self.objects {
            tables.push((table.table.clone(), format!("object type `{type_name}`")));
            let mut columns = vec![(
                table.pk.column.clone(),
                format!("primary key `{type_name}.{}`", table.pk.property),
            )];
            for (property, column) in &table.properties {
                columns.push((
                    column.column.clone(),
                    format!("property `{type_name}.{property}`"),
                ));
                columns.push((
                    column.set_column.clone(),
                    format!("flag column of `{type_name}.{property}`"),
                ));
            }
            columns.extend(system_column_entries(&naming::OBJECT_SYSTEM_COLUMNS));
            diagnostics.extend(duplicates(
                columns,
                &format!("column of table `{}`", table.table),
            ));
            indexes.extend(table.indexes.iter().map(|ix| {
                (
                    ix.name.clone(),
                    format!("index on `{type_name}.{}`", ix.column),
                )
            }));
        }
        for (link_name, table) in &self.links {
            tables.push((table.table.clone(), format!("link type `{link_name}`")));
            let mut columns = vec![
                (table.a.column.clone(), "link end column `a_pk`".to_owned()),
                (table.b.column.clone(), "link end column `b_pk`".to_owned()),
            ];
            columns.extend(system_column_entries(&naming::LINK_SYSTEM_COLUMNS));
            diagnostics.extend(duplicates(
                columns,
                &format!("column of table `{}`", table.table),
            ));
        }
        diagnostics.extend(duplicates(tables, "table"));
        diagnostics.extend(duplicates(indexes, "index"));
        diagnostics
    }

    /// Keeps the plain indexes of `old` that this schema no longer declares.
    ///
    /// Dropping `indexed: true` never drops the index: a role's row predicate may rely on it. The
    /// index stays in the database, so it stays in the schema that describes the database;
    /// otherwise every reload would see a difference. A unique index is not kept: it belongs to
    /// a link and leaves with it.
    pub fn keep_plain_indexes_of(&mut self, old: &PhysicalSchema) {
        for (name, table) in &mut self.objects {
            let Some(previous) = old.objects.get(name) else {
                continue;
            };
            let live = |column: &str| {
                table.pk.column == column || table.properties.values().any(|p| p.column == column)
            };
            let kept: Vec<IndexDef> = previous
                .indexes
                .iter()
                .filter(|ix| ix.kind == IndexKind::Plain && live(&ix.column))
                .filter(|ix| !table.indexes.iter().any(|new| new.name == ix.name))
                .cloned()
                .collect();
            table.indexes.extend(kept);
            table.indexes.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    /// Keeps the archived columns and tables of `old`.
    ///
    /// `derive` knows only the live definition. Migration (D-16) leaves archived columns and
    /// tables in the database, and they are part of the schema that describes it; without this a
    /// `reload` after a migration would see a difference on every run.
    pub fn keep_archived_of(&mut self, old: &PhysicalSchema) {
        for (name, table) in &mut self.objects {
            if let Some(previous) = old.objects.get(name) {
                table
                    .archived_columns
                    .clone_from(&previous.archived_columns);
            }
        }
        self.archived_tables.clone_from(&old.archived_tables);
    }

    pub fn object_table(&self, name: &ObjectTypeName) -> Option<&ObjectTable> {
        self.objects.get(name)
    }

    pub fn link_table(&self, name: &LinkTypeName) -> Option<&LinkTable> {
        self.links.get(name)
    }
}

fn system_column_entries(names: &[&str]) -> impl Iterator<Item = (String, String)> {
    names
        .iter()
        .map(|name| ((*name).to_owned(), format!("system column `{name}`")))
}

/// One ONT-V24 per physical name that more than one definition maps to.
fn duplicates(entries: Vec<(String, String)>, what: &str) -> Vec<Diagnostic> {
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (physical, logical) in entries {
        by_name.entry(physical).or_default().push(logical);
    }
    by_name
        .into_iter()
        .filter(|(_, logical)| logical.len() > 1)
        .map(|(physical, logical)| {
            Diagnostic::without_location(
                codes::V24,
                Severity::Error,
                format!(
                    "{what} name `{physical}` is shared by {}",
                    logical.join(" and ")
                ),
                "ontology",
            )
            .with_help("rename one of them; physical names come from the apiName in snake case, cut to fit PostgreSQL's 63 byte limit")
        })
        .collect()
}

fn derive_error(message: &str) -> Diagnostic {
    Diagnostic::without_location(
        codes::V09,
        Severity::Error,
        format!("cannot derive the edit store schema: {message}"),
        "ontology",
    )
}

/// The partial unique index on the foreign key of a `oneToOne` link, on the `to` end's table.
/// A foreign key that is the primary key needs none: the key is unique already.
fn add_one_to_one_index(
    schema: &mut PhysicalSchema,
    name: &LinkTypeName,
    link: &LinkType,
    errors: &mut Vec<Diagnostic>,
) {
    let Some(foreign_key) = &link.to.foreign_key else {
        errors.push(derive_error(&format!(
            "oneToOne link type `{name}` has no foreignKey"
        )));
        return;
    };
    let Some(table) = schema.objects.get_mut(&link.to.object_type) else {
        errors.push(derive_error(&format!(
            "link type `{name}` refers to an object type that has no table"
        )));
        return;
    };
    let Some(column) = table.properties.get(foreign_key) else {
        if table.pk.property != *foreign_key {
            errors.push(derive_error(&format!(
                "foreignKey `{foreign_key}` of link type `{name}` is not a property of `{}`",
                link.to.object_type
            )));
        }
        return;
    };
    let index = IndexDef {
        name: naming::unique_index_name(&table.table, &column.column),
        column: column.column.clone(),
        kind: IndexKind::UniqueWhenSet,
    };
    if !table.indexes.contains(&index) {
        table.indexes.push(index);
    }
}

fn end_column(pk: &KeyColumn, column: &str) -> KeyColumn {
    KeyColumn {
        property: pk.property.clone(),
        column: column.to_owned(),
        ty: pk.ty.clone(),
    }
}

fn object_table(object: &ObjectType) -> Result<ObjectTable, String> {
    let pk_def = object.properties.get(&object.primary_key).ok_or_else(|| {
        format!(
            "primary key `{}` is not a declared property",
            object.primary_key
        )
    })?;
    let pk = KeyColumn {
        property: object.primary_key.clone(),
        column: naming::property_column(&object.primary_key),
        ty: column_type(pk_def)?,
    };
    let table_name = naming::object_table(&object.api_name);
    let mut properties = BTreeMap::new();
    let mut indexes = Vec::new();
    for (name, def) in &object.properties {
        if *name == object.primary_key {
            continue;
        }
        let column = naming::property_column(name);
        if def.indexed {
            indexes.push(IndexDef {
                name: naming::index_name(&table_name, &column),
                column: column.clone(),
                kind: IndexKind::Plain,
            });
        }
        properties.insert(
            name.clone(),
            PropertyColumn {
                set_column: naming::set_column(&column),
                column,
                ty: column_type(def)?,
                required: def.required,
            },
        );
    }
    Ok(ObjectTable {
        table: table_name,
        pk,
        properties,
        archived_columns: Vec::new(),
        indexes,
    })
}

fn column_type(def: &PropertyDef) -> Result<ColumnType, String> {
    Ok(match def.base_type {
        BaseType::Byte => ColumnType::Byte,
        BaseType::Short => ColumnType::Short,
        BaseType::Integer => ColumnType::Integer,
        BaseType::Long => ColumnType::Long,
        BaseType::Float => ColumnType::Float,
        BaseType::Double => ColumnType::Double,
        BaseType::Boolean => ColumnType::Boolean,
        BaseType::String => ColumnType::String,
        BaseType::Decimal => {
            let (precision, scale) = def.effective_decimal();
            ColumnType::Decimal { precision, scale }
        }
        BaseType::Date => ColumnType::Date,
        BaseType::Timestamp => ColumnType::Timestamp,
        BaseType::Geopoint => ColumnType::Geopoint,
        BaseType::Array => {
            let items = def
                .items
                .ok_or_else(|| "an array property has no `items`".to_owned())?;
            let element = PropertyDef {
                base_type: items,
                items: None,
                ..def.clone()
            };
            ColumnType::Array(Box::new(column_type(&element)?))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(base_type: BaseType, items: Option<BaseType>) -> PropertyDef {
        PropertyDef {
            base_type,
            required: false,
            display_name: None,
            description: None,
            items,
            precision: Some(12),
            scale: Some(2),
            indexed: false,
        }
    }

    #[test]
    fn array_of_decimal_keeps_precision() {
        let ty = column_type(&property(BaseType::Array, Some(BaseType::Decimal))).expect("type");
        assert_eq!(
            ty,
            ColumnType::Array(Box::new(ColumnType::Decimal {
                precision: 12,
                scale: 2
            }))
        );
    }

    #[test]
    fn array_without_items_is_refused() {
        assert!(column_type(&property(BaseType::Array, None)).is_err());
    }

    #[test]
    fn byte_and_short_stay_distinct() {
        let byte = column_type(&property(BaseType::Byte, None)).expect("byte");
        let short = column_type(&property(BaseType::Short, None)).expect("short");
        assert_ne!(byte, short);
    }
}
