//! DDL of the edit tables (ADR-0024).
//!
//! Statements are produced by `sea-query`, which quotes every identifier. Two dialects:
//!
//! - SQLite: integer types, boolean and the `__set` flags are `INTEGER`; float and double are
//!   `REAL`; string, date, array and geopoint are `TEXT` (arrays and geopoints hold JSON);
//!   decimal and timestamp are `TEXT` with the engine's collations, so comparisons follow numeric
//!   and instant order.
//! - PostgreSQL: native types (see `pg_type`); tables are qualified with the edit schema; every
//!   statement is re-runnable (`IF NOT EXISTS`), so an interrupted reload can be repeated.
//!
//! Both dialects give the system columns the same defaults (`0` / `false`), so a table that lacks
//! one can be completed with `ADD COLUMN`.

use ontoforge_engine::sqlite::{DECIMAL_COLLATION, TIMESTAMP_COLLATION};
use sea_query::{
    Alias, ColumnDef, Index, IntoTableRef, PostgresQueryBuilder, SqliteQueryBuilder, Table,
    TableRef,
};

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
    match step {
        DdlStep::AddPropertyColumn { table, column } => property_columns(column, backend)
            .iter()
            .map(|col| add_column_sql(table, col, backend, schema_edit))
            .collect(),
        DdlStep::CreateObjectTable(_) | DdlStep::CreateLinkTable(_) => {
            create_sql(step, backend, schema_edit).into_iter().collect()
        }
    }
}

/// Every column of the table of `step`, or the added columns of an `AddPropertyColumn`.
pub(crate) fn step_columns(step: &DdlStep, backend: Backend) -> (&str, Vec<Col>) {
    match step {
        DdlStep::CreateObjectTable(table) => (&table.table, object_columns(table, backend)),
        DdlStep::CreateLinkTable(table) => (&table.table, link_columns(table, backend)),
        DdlStep::AddPropertyColumn { table, column } => (table, property_columns(column, backend)),
    }
}

/// The statement that creates the table of a create step, `None` for other steps.
pub(crate) fn create_sql(
    step: &DdlStep,
    backend: Backend,
    schema_edit: Option<&str>,
) -> Option<String> {
    match step {
        DdlStep::CreateObjectTable(table) => Some(create_table_sql(
            &table.table,
            &object_columns(table, backend),
            &[&table.pk.column],
            backend,
            schema_edit,
        )),
        DdlStep::CreateLinkTable(table) => Some(create_table_sql(
            &table.table,
            &link_columns(table, backend),
            &[&table.a.column, &table.b.column],
            backend,
            schema_edit,
        )),
        DdlStep::AddPropertyColumn { .. } => None,
    }
}

/// `ALTER TABLE .. ADD COLUMN ..` for one column. On PostgreSQL the column is added only if
/// missing.
pub(crate) fn add_column_sql(
    table: &str,
    col: &Col,
    backend: Backend,
    schema_edit: Option<&str>,
) -> String {
    let mut alter = Table::alter();
    alter.table(table_ref(table, backend, schema_edit));
    match backend {
        Backend::Sqlite => alter
            .add_column(col.def.clone())
            .to_string(SqliteQueryBuilder),
        Backend::Postgres => alter
            .add_column_if_not_exists(col.def.clone())
            .to_string(PostgresQueryBuilder),
    }
}

/// The table, qualified with the edit schema on PostgreSQL.
fn table_ref(table: &str, backend: Backend, schema_edit: Option<&str>) -> TableRef {
    match (backend, schema_edit) {
        (Backend::Postgres, Some(schema)) => {
            (Alias::new(schema), Alias::new(table)).into_table_ref()
        }
        _ => Alias::new(table).into_table_ref(),
    }
}

fn create_table_sql(
    table: &str,
    columns: &[Col],
    primary_key: &[&String],
    backend: Backend,
    schema_edit: Option<&str>,
) -> String {
    let mut create = Table::create();
    create
        .table(table_ref(table, backend, schema_edit))
        .if_not_exists();
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
    match backend {
        Backend::Sqlite => create.to_string(SqliteQueryBuilder),
        Backend::Postgres => create.to_string(PostgresQueryBuilder),
    }
}

/// The PostgreSQL type of a column (ADR-0024 point 7). Strings use the `C` collation so that
/// ordering is bytewise on every server, whatever its locale. An array is its element type with
/// `[]`; the element carries no collation.
fn pg_type(ty: &ColumnType) -> String {
    match ty {
        ColumnType::Byte | ColumnType::Short => "smallint".to_owned(),
        ColumnType::Integer => "integer".to_owned(),
        ColumnType::Long => "bigint".to_owned(),
        ColumnType::Float => "real".to_owned(),
        ColumnType::Double => "double precision".to_owned(),
        ColumnType::Boolean => "boolean".to_owned(),
        ColumnType::String => "text COLLATE \"C\"".to_owned(),
        ColumnType::Decimal { precision, scale } => format!("numeric({precision}, {scale})"),
        ColumnType::Date => "date".to_owned(),
        ColumnType::Timestamp => "timestamptz".to_owned(),
        ColumnType::Array(element) => format!("{}[]", pg_element_type(element)),
        ColumnType::Geopoint => "jsonb".to_owned(),
    }
}

fn pg_element_type(ty: &ColumnType) -> String {
    match ty {
        ColumnType::String => "text".to_owned(),
        other => pg_type(other),
    }
}

fn apply_type(def: &mut ColumnDef, ty: &ColumnType, backend: Backend) {
    if backend == Backend::Postgres {
        // `custom` writes the name as it is; the collation has to sit right after the type,
        // before any column constraint.
        def.custom(Alias::new(pg_type(ty)));
        return;
    }
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

fn key_col(key: &KeyColumn, primary: bool, backend: Backend) -> Col {
    let mut def = ColumnDef::new(Alias::new(key.column.as_str()));
    apply_type(&mut def, &key.ty, backend);
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

fn value_col(name: &str, ty: &ColumnType, backend: Backend) -> Col {
    let mut def = ColumnDef::new(Alias::new(name));
    apply_type(&mut def, ty, backend);
    Col {
        name: name.to_owned(),
        key: false,
        def,
    }
}

/// `NOT NULL` boolean: `DEFAULT false` on PostgreSQL, `INTEGER DEFAULT 0` on SQLite.
fn flag_col(name: &str, backend: Backend) -> Col {
    let mut def = ColumnDef::new(Alias::new(name));
    match backend {
        Backend::Sqlite => def.integer().not_null().default(0),
        Backend::Postgres => def.boolean().not_null().default(false),
    };
    Col {
        name: name.to_owned(),
        key: false,
        def,
    }
}

/// `NOT NULL` counter with `DEFAULT 0`: `bigint` on PostgreSQL, `INTEGER` on SQLite.
fn counter_col(name: &str, backend: Backend) -> Col {
    let mut def = ColumnDef::new(Alias::new(name));
    match backend {
        Backend::Sqlite => def.integer(),
        Backend::Postgres => def.big_integer(),
    };
    def.not_null().default(0);
    Col {
        name: name.to_owned(),
        key: false,
        def,
    }
}

fn property_columns(column: &PropertyColumn, backend: Backend) -> Vec<Col> {
    vec![
        value_col(&column.column, &column.ty, backend),
        flag_col(&column.set_column, backend),
    ]
}

/// System columns. They carry defaults so that a table that lacks one can be repaired with
/// `ADD COLUMN`; writers always set them. Link tables have no `of_released`.
fn system_columns(released: bool, backend: Backend) -> Vec<Col> {
    let mut columns = vec![
        counter_col("of_version", backend),
        flag_col("of_deleted", backend),
    ];
    if released {
        columns.push(flag_col("of_released", backend));
    }
    columns.push(counter_col("of_def_version", backend));
    let mut log_seq = ColumnDef::new(Alias::new("of_log_seq"));
    match backend {
        Backend::Sqlite => log_seq.integer().null(),
        Backend::Postgres => log_seq.big_integer().null(),
    };
    columns.push(Col {
        name: "of_log_seq".to_owned(),
        key: false,
        def: log_seq,
    });
    columns
}

fn object_columns(table: &ObjectTable, backend: Backend) -> Vec<Col> {
    let mut columns = vec![key_col(&table.pk, true, backend)];
    for column in table.properties.values() {
        columns.extend(property_columns(column, backend));
    }
    columns.extend(system_columns(true, backend));
    columns
}

fn link_columns(table: &LinkTable, backend: Backend) -> Vec<Col> {
    // The composite primary key is declared on the table, not on a column.
    let mut columns = vec![
        key_col(&table.a, false, backend),
        key_col(&table.b, false, backend),
    ];
    columns.extend(system_columns(false, backend));
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use camino::Utf8Path;
    use ontoforge_schema::ir::PropertyName;

    use crate::diff::diff;
    use crate::physical::PhysicalSchema;

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

    #[test]
    fn postgres_collates_the_key_before_its_constraints() {
        let sql = render(
            &DdlStep::CreateObjectTable(table()),
            Backend::Postgres,
            Some("ontoforge_edit"),
        );
        assert_eq!(sql.len(), 1);
        assert!(
            sql[0].contains("\"code\" text COLLATE \"C\" NOT NULL PRIMARY KEY"),
            "{}",
            sql[0]
        );
    }

    #[test]
    fn postgres_add_column_is_qualified_and_idempotent() {
        let column = table().properties.values().next().cloned().expect("column");
        let sql = render(
            &DdlStep::AddPropertyColumn {
                table: "of_obj_item".to_owned(),
                column,
            },
            Backend::Postgres,
            Some("ontoforge_edit"),
        );
        assert_eq!(sql.len(), 2);
        for statement in &sql {
            assert!(
                statement.contains("\"ontoforge_edit\".\"of_obj_item\""),
                "{statement}"
            );
            assert!(
                statement.contains("ADD COLUMN IF NOT EXISTS"),
                "{statement}"
            );
        }
    }

    /// Every create statement of the aviation example, one per line group.
    fn aviation_ddl(backend: Backend, schema_edit: Option<&str>) -> String {
        let root = Utf8Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation"
        ));
        let out = ontoforge_schema::load_dir(root);
        let ontology = out.ontology.expect("aviation loads");
        let physical = PhysicalSchema::derive(&ontology).expect("physical schema");
        let plan = diff(None, &physical);
        assert!(plan.destructive.is_empty());
        let mut text = String::new();
        for step in &plan.steps {
            for statement in render(step, backend, schema_edit) {
                text.push_str(&statement);
                text.push_str(";\n\n");
            }
        }
        text
    }

    #[test]
    fn ddl_snapshots() {
        insta::assert_snapshot!("ddl_sqlite", aviation_ddl(Backend::Sqlite, None));
        insta::assert_snapshot!(
            "ddl_postgres",
            aviation_ddl(Backend::Postgres, Some("ontoforge_edit"))
        );
    }
}
