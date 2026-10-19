//! Shared pieces of the Rust-vs-SQLite differential tests (plan 03-11).
//!
//! `Harness` holds one in-memory SQLite connection with the table `"p__t"."rows"`. The
//! predicate SQL it executes always comes from `print_select`, never from test-written text.

// Each test target uses a different part of this module.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{Dialect, TableRef};
use ontoforge_expr::{PredContext, Scalar, Tri};
use ontoforge_schema::ir::{AttrType, ObjectType, PropertyName};
use ontoforge_schema::load_from_memory;
use sqlx::sqlite::SqliteConnection;
use sqlx::{AssertSqlSafe, Connection, Row};

pub fn prop(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

/// The SQLite column of a property: the property name itself.
pub fn column_name(p: &PropertyName) -> String {
    p.as_str().to_owned()
}

/// One row of `"p__t"."rows"`. The property `b` is a boolean stored as 0 / 1.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRow {
    pub id: i64,
    pub b: Option<bool>,
    pub i: Option<i64>,
    pub d: Option<f64>,
    pub s: Option<String>,
    pub t: Option<String>,
}

impl TestRow {
    /// The values `ontoforge_expr::eval` reads.
    pub fn values(&self) -> BTreeMap<PropertyName, Scalar> {
        let mut out = BTreeMap::new();
        out.insert(prop("b"), self.b.map_or(Scalar::Null, Scalar::Bool));
        out.insert(prop("i"), self.i.map_or(Scalar::Null, Scalar::Int));
        out.insert(prop("d"), self.d.map_or(Scalar::Null, Scalar::Double));
        out.insert(
            prop("s"),
            self.s.clone().map_or(Scalar::Null, Scalar::String),
        );
        out.insert(
            prop("t"),
            self.t.clone().map_or(Scalar::Null, Scalar::String),
        );
        out
    }
}

fn rows_table() -> TableRef {
    TableRef {
        catalog: "p".to_owned(),
        schema: "t".to_owned(),
        table: "rows".to_owned(),
    }
}

fn id_column() -> Expr {
    Expr::Column("id".to_owned())
}

pub struct Harness {
    // Declared first so the connection is dropped before the runtime.
    conn: SqliteConnection,
    rt: tokio::runtime::Runtime,
}

impl Harness {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let conn = rt.block_on(async {
            let mut conn = SqliteConnection::connect("sqlite::memory:")
                .await
                .expect("connect");
            sqlx::query("ATTACH DATABASE ':memory:' AS \"p__t\"")
                .execute(&mut conn)
                .await
                .expect("attach");
            sqlx::query(
                "CREATE TABLE \"p__t\".\"rows\" \
                 (id INTEGER PRIMARY KEY, b INTEGER, i INTEGER, d REAL, s TEXT, t TEXT)",
            )
            .execute(&mut conn)
            .await
            .expect("create table");
            conn
        });
        Self { conn, rt }
    }

    /// Replaces the table contents with `rows`.
    pub fn load_rows(&mut self, rows: &[TestRow]) {
        let Self { conn, rt } = self;
        rt.block_on(async {
            sqlx::query("DELETE FROM \"p__t\".\"rows\"")
                .execute(&mut *conn)
                .await
                .expect("delete");
            for r in rows {
                sqlx::query(
                    "INSERT INTO \"p__t\".\"rows\" (id, b, i, d, s, t) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(r.id)
                .bind(r.b)
                .bind(r.i)
                .bind(r.d)
                .bind(r.s.as_deref())
                .bind(r.t.as_deref())
                .execute(&mut *conn)
                .await
                .expect("insert");
            }
        });
    }

    /// The truth value of `expr` on `row`, executed by SQLite.
    pub fn tri_of(&mut self, expr: &Expr, row: &TestRow) -> Tri {
        self.load_rows(std::slice::from_ref(row));
        let filter = Expr::Compare {
            left: Box::new(id_column()),
            op: CompareOp::Eq,
            right: Box::new(Expr::Literal(Literal::Long(row.id))),
        };
        let out = self.run(expr, Some(filter));
        assert_eq!(out.len(), 1, "one row expected for id {}", row.id);
        out[0].1
    }

    /// The truth value of `expr` on every loaded row, in `id` order.
    pub fn tri_all(&mut self, expr: &Expr) -> Vec<(i64, Tri)> {
        self.run(expr, None)
    }

    fn run(&mut self, expr: &Expr, filter: Option<Expr>) -> Vec<(i64, Tri)> {
        let select = Select {
            projection: vec![
                SelectItem {
                    expr: id_column(),
                    alias: Some("id".to_owned()),
                },
                SelectItem {
                    expr: expr.clone(),
                    alias: Some("v".to_owned()),
                },
            ],
            from: Some(rows_table()),
            filter,
            group_by: Vec::new(),
            having: None,
            order_by: vec![OrderItem {
                expr: id_column(),
                descending: false,
                nulls_first: false,
            }],
            limit: None,
        };
        let sql = print_select(&select, Dialect::Sqlite).expect("printable");
        let Self { conn, rt } = self;
        let rows = rt
            .block_on(async {
                sqlx::query(AssertSqlSafe(sql.as_str().to_owned()))
                    .fetch_all(&mut *conn)
                    .await
            })
            .unwrap_or_else(|e| panic!("sqlite rejected {sql}: {e}"));
        rows.iter()
            .map(|r| {
                let id: i64 = r.get("id");
                let v: Option<i64> = r.get("v");
                let tri = match v {
                    None => Tri::Unknown,
                    Some(1) => Tri::True,
                    Some(0) => Tri::False,
                    Some(other) => panic!("not a truth value: {other} from {sql}"),
                };
                (id, tri)
            })
            .collect()
    }
}

thread_local! {
    static HARNESS: RefCell<Harness> = RefCell::new(Harness::new());
}

/// Runs `f` with this thread's harness (one connection per test thread).
pub fn with_harness<R>(f: impl FnOnce(&mut Harness) -> R) -> R {
    HARNESS.with(|h| f(&mut h.borrow_mut()))
}

const MANIFEST: &str = "apiName: demo\n";

const OBJECTS: &str = "kind: ObjectType
apiName: Doc
primaryKey: id
properties:
  id: { type: string, required: true }
  b: { type: boolean }
  i: { type: long }
  d: { type: double }
  s: { type: string }
  t: { type: string }
backing: editOnly
";

/// The object type the differential tests compile predicates against.
pub struct Fixture {
    object_type: ObjectType,
    attrs: BTreeMap<String, AttrType>,
}

impl Fixture {
    pub fn new() -> Self {
        let out = load_from_memory(&[("ontology.yaml", MANIFEST), ("objects.yaml", OBJECTS)]);
        assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
        let object_type = out
            .ontology
            .expect("ontology")
            .object_types
            .get("Doc")
            .expect("Doc object type")
            .clone();
        let attrs = [
            ("tenant", AttrType::String),
            ("prefix", AttrType::String),
            ("level", AttrType::Integer),
            ("active", AttrType::Boolean),
            ("regions", AttrType::StringList),
            ("nums", AttrType::IntegerList),
        ]
        .into_iter()
        .map(|(name, ty)| (name.to_owned(), ty))
        .collect();
        Self { object_type, attrs }
    }

    pub fn cx(&self) -> PredContext<'_> {
        PredContext {
            object_type: &self.object_type,
            attrs: &self.attrs,
        }
    }
}
