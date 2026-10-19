//! The physical schema of the edit store, derived from an ontology (D-17, ADR-0024).
//!
//! `PhysicalSchema` is what `reload` compares between definition versions and what it stores
//! in `of_def_version.physical_json`, so every type here is serializable and stays
//! backwards compatible (new fields use `#[serde(default)]`).

use std::collections::BTreeMap;

use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{
    BaseType, Cardinality, LinkTypeName, ObjectType, ObjectTypeName, Ontology, PropertyDef,
    PropertyName,
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

/// The table of one object type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectTable {
    pub table: String,
    pub pk: KeyColumn,
    pub properties: BTreeMap<PropertyName, PropertyColumn>,
    #[serde(default)]
    pub archived_columns: Vec<ArchivedColumn>,
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

        if errors.is_empty() {
            Ok(schema)
        } else {
            Err(errors)
        }
    }

    pub fn object_table(&self, name: &ObjectTypeName) -> Option<&ObjectTable> {
        self.objects.get(name)
    }

    pub fn link_table(&self, name: &LinkTypeName) -> Option<&LinkTable> {
        self.links.get(name)
    }
}

fn derive_error(message: &str) -> Diagnostic {
    Diagnostic::without_location(
        codes::V09,
        Severity::Error,
        format!("cannot derive the edit store schema: {message}"),
        "ontology",
    )
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
    let mut properties = BTreeMap::new();
    for (name, def) in &object.properties {
        if *name == object.primary_key {
            continue;
        }
        let column = naming::property_column(name);
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
        table: naming::object_table(&object.api_name),
        pk,
        properties,
        archived_columns: Vec::new(),
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
