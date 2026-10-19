//! DDL of the edit tables (ADR-0024).
//!
//! Statements are produced by `sea-query`, which quotes every identifier. This plan renders the
//! SQLite dialect; the PostgreSQL dialect and the snapshot tests of both come with plan 03-04.
//!
//! SQLite columns: integer types, boolean and the `__set` flags are `INTEGER`; float and double
//! are `REAL`; string, date, array and geopoint are `TEXT` (arrays and geopoints hold JSON);
//! decimal and timestamp are `TEXT` with the engine's collations, so comparisons follow numeric
//! and instant order.

use ontoforge_engine::sqlite::{DECIMAL_COLLATION, TIMESTAMP_COLLATION};
use sea_query::{Alias, ColumnDef, Index, SqliteQueryBuilder, Table};

use crate::physical::{ColumnType, KeyColumn, LinkTable, ObjectTable, PropertyColumn};
use crate::store::Backend;

/// One schema change `reload` can apply without losing data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdlStep {
    CreateObjectTable(ObjectTable),
    AddPropertyColumn {
        table: String,
        column: PropertyColumn,
    },
    CreateLinkTable(LinkTable),
}

/// A column a step needs. `key` columns cannot be added to an existing table.
#[derive(Debug, Clone)]
pub(crate) struct Col {
    pub(crate) name: String,
    pub(crate) key: bool,
    def: ColumnDef,
}

/// The statements of `step`, one operation each.
///
/// `schema_edit` is the PostgreSQL schema of the edit tables; SQLite has none.
pub fn render(step: &DdlStep, backend: Backend, schema_edit: Option<&str>) -> Vec<String> {
    match backend {
        Backend::Sqlite => match step {
            DdlStep::AddPropertyColumn { table, column } => property_columns(column)
                .iter()
                .map(|col| add_column_sql(table, col))
                .collect(),
            DdlStep::CreateObjectTable(_) | DdlStep::CreateLinkTable(_) => {
                create_sql(step).into_iter().collect()
            }
        },
        Backend::Postgres => {
            // Added with the PostgreSQL backend in plan 03-04; no PostgreSQL store can be
            // opened before that.
            let _ = schema_edit;
            Vec::new()
        }
    }
}

/// Every column of the table of `step`, or the added columns of an `AddPropertyColumn`.
pub(crate) fn step_columns(step: &DdlStep) -> (&str, Vec<Col>) {
    match step {
        DdlStep::CreateObjectTable(table) => (&table.table, object_columns(table)),
        DdlStep::CreateLinkTable(table) => (&table.table, link_columns(table)),
        DdlStep::AddPropertyColumn { table, column } => (table, property_columns(column)),
    }
}

/// The statements that create the table of a create step, `None` for other steps.
pub(crate) fn create_sql(step: &DdlStep) -> Option<String> {
    match step {
        DdlStep::CreateObjectTable(table) => Some(create_table_sql(
            &table.table,
            &object_columns(table),
            &[&table.pk.column],
        )),
        DdlStep::CreateLinkTable(table) => Some(create_table_sql(
            &table.table,
            &link_columns(table),
            &[&table.a.column, &table.b.column],
        )),
        DdlStep::AddPropertyColumn { .. } => None,
    }
}

/// `ALTER TABLE .. ADD COLUMN ..` for one column.
pub(crate) fn add_column_sql(table: &str, col: &Col) -> String {
    Table::alter()
        .table(Alias::new(table))
        .add_column(col.def.clone())
        .to_string(SqliteQueryBuilder)
}

fn create_table_sql(table: &str, columns: &[Col], primary_key: &[&String]) -> String {
    let mut create = Table::create();
    create.table(Alias::new(table)).if_not_exists();
    for col in columns {
        create.col(col.def.clone());
    }
    if primary_key.len() > 1 {
        let mut key = Index::create();
        for column in primary_key {
            key.col(Alias::new(column.as_str()));
        }
        create.primary_key(&mut key);
    }
    create.to_string(SqliteQueryBuilder)
}

fn apply_type(def: &mut ColumnDef, ty: &ColumnType) {
    match ty {
        ColumnType::Byte
        | ColumnType::Short
        | ColumnType::Integer
        | ColumnType::Long
        | ColumnType::Boolean => def.integer(),
        ColumnType::Float | ColumnType::Double => def.double(),
        ColumnType::String | ColumnType::Date | ColumnType::Array(_) | ColumnType::Geopoint => {
            def.text()
        }
        ColumnType::Decimal { .. } => def.text().extra(format!("COLLATE {DECIMAL_COLLATION}")),
        ColumnType::Timestamp => def.text().extra(format!("COLLATE {TIMESTAMP_COLLATION}")),
    };
}

fn key_col(key: &KeyColumn, primary: bool) -> Col {
    let mut def = ColumnDef::new(Alias::new(key.column.as_str()));
    apply_type(&mut def, &key.ty);
    def.not_null();
    if primary {
        def.primary_key();
    }
    Col {
        name: key.column.clone(),
        key: true,
        def,
    }
}

fn value_col(name: &str, ty: &ColumnType) -> Col {
    let mut def = ColumnDef::new(Alias::new(name));
    apply_type(&mut def, ty);
    Col {
        name: name.to_owned(),
        key: false,
        def,
    }
}

fn flag_col(name: &str) -> Col {
    let mut def = ColumnDef::new(Alias::new(name));
    def.integer().not_null().default(0);
    Col {
        name: name.to_owned(),
        key: false,
        def,
    }
}

fn property_columns(column: &PropertyColumn) -> Vec<Col> {
    vec![
        value_col(&column.column, &column.ty),
        flag_col(&column.set_column),
    ]
}

/// System columns. `of_def_version` carries a default so that a table that lacks it can be
/// repaired with `ADD COLUMN`; writers always set it. Link tables have no `of_released`.
fn system_columns(released: bool) -> Vec<Col> {
    let mut names = vec!["of_version", "of_deleted"];
    if released {
        names.push("of_released");
    }
    names.push("of_def_version");
    let mut columns: Vec<Col> = names.into_iter().map(flag_col).collect();
    let mut log_seq = ColumnDef::new(Alias::new("of_log_seq"));
    log_seq.integer().null();
    columns.push(Col {
        name: "of_log_seq".to_owned(),
        key: false,
        def: log_seq,
    });
    columns
}

fn object_columns(table: &ObjectTable) -> Vec<Col> {
    let mut columns = vec![key_col(&table.pk, true)];
    for column in table.properties.values() {
        columns.extend(property_columns(column));
    }
    columns.extend(system_columns(true));
    columns
}

fn link_columns(table: &LinkTable) -> Vec<Col> {
    // The composite primary key is declared on the table, not on a column.
    let mut columns = vec![key_col(&table.a, false), key_col(&table.b, false)];
    columns.extend(system_columns(false));
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ontoforge_schema::ir::PropertyName;

    fn property(name: &str) -> PropertyName {
        PropertyName::try_from(name.to_owned()).expect("property name")
    }

    fn table() -> ObjectTable {
        let mut properties = BTreeMap::new();
        properties.insert(
            property("price"),
            PropertyColumn {
                column: "price".to_owned(),
                set_column: "price__set".to_owned(),
                ty: ColumnType::Decimal {
                    precision: 12,
                    scale: 2,
                },
                required: false,
            },
        );
        ObjectTable {
            table: "of_obj_item".to_owned(),
            pk: KeyColumn {
                property: property("code"),
                column: "code".to_owned(),
                ty: ColumnType::String,
            },
            properties,
            archived_columns: Vec::new(),
        }
    }

    #[test]
    fn create_table_declares_collation_flags_and_system_columns() {
        let sql = render(&DdlStep::CreateObjectTable(table()), Backend::Sqlite, None);
        assert_eq!(sql.len(), 1);
        let sql = &sql[0];
        assert!(sql.contains("TABLE IF NOT EXISTS \"of_obj_item\""), "{sql}");
        assert!(sql.contains("\"code\" text NOT NULL PRIMARY KEY"), "{sql}");
        assert!(
            sql.contains("\"price\" text COLLATE ontoforge_decimal"),
            "{sql}"
        );
        assert!(
            sql.contains("\"price__set\" integer NOT NULL DEFAULT 0"),
            "{sql}"
        );
        assert!(sql.contains("\"of_log_seq\" integer NULL"), "{sql}");
    }

    #[test]
    fn add_property_column_is_two_single_operation_statements() {
        let column = table().properties.values().next().cloned().expect("column");
        let sql = render(
            &DdlStep::AddPropertyColumn {
                table: "of_obj_item".to_owned(),
                column,
            },
            Backend::Sqlite,
            None,
        );
        assert_eq!(sql.len(), 2);
        assert!(sql[0].contains("ADD COLUMN \"price\""), "{}", sql[0]);
        assert!(sql[1].contains("ADD COLUMN \"price__set\""), "{}", sql[1]);
    }
}
