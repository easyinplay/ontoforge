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
use ontoforge_expr::{
    AttrValue, Attrs, CmpOp, ListSource, MatchKind, Operand, Pred, PredContext, Scalar, Tri,
    compile_row_filter, eval,
};
use ontoforge_schema::ir::{AttrType, ObjectType, PropertyName};
use ontoforge_schema::load_from_memory;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
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

// ---------------------------------------------------------------------------------------
// Strategies: type-correct predicates, rows and principal attributes.
// ---------------------------------------------------------------------------------------

/// The four value families a comparison can have (ADR-0025 section 4).
#[derive(Debug, Clone, Copy)]
pub enum Fam {
    Bool,
    Int,
    Dbl,
    Str,
}

fn pick<T: Clone + std::fmt::Debug + 'static>(items: &[T]) -> BoxedStrategy<T> {
    prop::sample::select(items.to_vec()).boxed()
}

fn maybe<T: std::fmt::Debug + 'static>(
    s: impl Strategy<Value = T> + 'static,
) -> BoxedStrategy<Option<T>> {
    prop::option::weighted(0.75, s).boxed()
}

/// Strings built from pieces that matter to `LIKE` / `GLOB` / quoting / case folding.
pub fn string_value() -> BoxedStrategy<String> {
    let fragments = pick(&[
        "",
        "a",
        "A",
        "ab",
        "AB",
        "acme",
        "ACME",
        "eu",
        "us",
        "%",
        "_",
        "\\",
        "[",
        "]",
        "*",
        "?",
        "'",
        "\"",
        "é",
        "É",
        "ß",
        "\u{1F600}",
        " ",
        "a%b",
        "a_b",
        "a[b]",
        "a*b",
        "a?b",
    ]);
    prop_oneof![
        4 => fragments.clone().prop_map(str::to_owned),
        4 => prop::collection::vec(fragments, 0..4).prop_map(|v| v.concat()),
        1 => any::<String>().prop_map(|s| s.replace('\0', "")),
    ]
    .boxed()
}

pub fn int_value() -> BoxedStrategy<i64> {
    prop_oneof![
        6 => -3i64..4,
        1 => Just(i64::MIN),
        1 => Just(i64::MAX),
        1 => any::<i64>(),
    ]
    .boxed()
}

/// Finite doubles only: NaN and the infinities are outside the guarantee (ADR-0025).
pub fn double_value() -> BoxedStrategy<f64> {
    prop_oneof![
        4 => pick(&[-1.5f64, 0.0, -0.0, 1.5, 2.0, 1e21, 1e-7, 0.1]),
        2 => prop::num::f64::POSITIVE
            | prop::num::f64::NEGATIVE
            | prop::num::f64::NORMAL
            | prop::num::f64::SUBNORMAL
            | prop::num::f64::ZERO,
    ]
    .boxed()
}

fn literal(f: Fam) -> BoxedStrategy<Scalar> {
    match f {
        Fam::Str => string_value().prop_map(Scalar::String).boxed(),
        Fam::Int => int_value().prop_map(Scalar::Int).boxed(),
        Fam::Dbl => double_value().prop_map(Scalar::Double).boxed(),
        Fam::Bool => any::<bool>().prop_map(Scalar::Bool).boxed(),
    }
}

fn column(names: &[&'static str]) -> BoxedStrategy<Operand> {
    pick(names).prop_map(|n| Operand::Column(prop(n))).boxed()
}

fn attr(names: &[&'static str]) -> BoxedStrategy<Operand> {
    pick(names)
        .prop_map(|n| Operand::Attr(n.to_owned()))
        .boxed()
}

fn operand(f: Fam) -> BoxedStrategy<Operand> {
    let lit = literal(f).prop_map(Operand::Lit);
    match f {
        Fam::Str => prop_oneof![
            3 => column(&["s", "t"]),
            2 => lit,
            2 => attr(&["tenant", "prefix"]),
        ]
        .boxed(),
        Fam::Int => prop_oneof![3 => column(&["i"]), 2 => lit, 1 => attr(&["level"])].boxed(),
        Fam::Dbl => prop_oneof![3 => column(&["d"]), 2 => lit].boxed(),
        Fam::Bool => prop_oneof![2 => column(&["b"]), 1 => lit, 1 => attr(&["active"])].boxed(),
    }
}

fn family() -> BoxedStrategy<Fam> {
    pick(&[Fam::Str, Fam::Int, Fam::Dbl, Fam::Bool])
}

fn cmp_pred() -> BoxedStrategy<Pred> {
    family()
        .prop_flat_map(|f| {
            let ops = match f {
                Fam::Bool => vec![CmpOp::Eq, CmpOp::Ne],
                _ => vec![
                    CmpOp::Eq,
                    CmpOp::Ne,
                    CmpOp::Lt,
                    CmpOp::Le,
                    CmpOp::Gt,
                    CmpOp::Ge,
                ],
            };
            (operand(f), pick(&ops), operand(f))
        })
        .prop_map(|(left, op, right)| Pred::Cmp { left, op, right })
        .boxed()
}

fn is_null_pred() -> BoxedStrategy<Pred> {
    let target = prop_oneof![
        column(&["b", "i", "d", "s", "t"]),
        attr(&["tenant", "prefix", "level", "active"]),
    ];
    (target, any::<bool>())
        .prop_map(|(operand, negated)| Pred::IsNull { operand, negated })
        .boxed()
}

fn in_pred() -> BoxedStrategy<Pred> {
    let literal_list = family().prop_flat_map(|f| {
        (operand(f), prop::collection::vec(literal(f), 0..4)).prop_map(|(operand, items)| {
            Pred::InList {
                operand,
                list: ListSource::Literal(items),
            }
        })
    });
    let attr_list = prop_oneof![
        (operand(Fam::Str), Just("regions")),
        (operand(Fam::Int), Just("nums")),
    ]
    .prop_map(|(operand, name)| Pred::InList {
        operand,
        list: ListSource::Attr(name.to_owned()),
    });
    prop_oneof![3 => literal_list, 1 => attr_list].boxed()
}

fn match_pred() -> BoxedStrategy<Pred> {
    (
        pick(&["s", "t"]),
        prop_oneof![
            2 => string_value().prop_map(|s| Operand::Lit(Scalar::String(s))),
            1 => attr(&["tenant", "prefix"]),
        ],
        pick(&[
            MatchKind::StartsWith,
            MatchKind::EndsWith,
            MatchKind::Contains,
        ]),
    )
        .prop_map(|(name, needle, kind)| Pred::StrMatch {
            column: prop(name),
            needle,
            kind,
        })
        .boxed()
}

fn leaf() -> BoxedStrategy<Pred> {
    prop_oneof![
        1 => any::<bool>().prop_map(Pred::Const),
        4 => cmp_pred(),
        2 => is_null_pred(),
        2 => in_pred(),
        3 => match_pred(),
    ]
    .boxed()
}

/// Random type-correct predicates covering every node kind, depth at most 4.
pub fn pred() -> BoxedStrategy<Pred> {
    leaf()
        .prop_recursive(4, 32, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Pred::And(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Pred::Or(Box::new(a), Box::new(b))),
                inner.prop_map(|a| Pred::Not(Box::new(a))),
            ]
        })
        .boxed()
}

/// A row with id 1; every column may be NULL.
pub fn row_strategy() -> BoxedStrategy<TestRow> {
    (
        maybe(any::<bool>()),
        maybe(int_value()),
        maybe(double_value()),
        maybe(string_value()),
        maybe(string_value()),
    )
        .prop_map(|(b, i, d, s, t)| TestRow {
            id: 1,
            b,
            i,
            d,
            s,
            t,
        })
        .boxed()
}

/// Principal attributes: each may be missing; lists may be empty.
pub fn attrs_strategy() -> BoxedStrategy<Attrs> {
    let str_list = maybe(prop::collection::vec(
        string_value().prop_map(Scalar::String),
        0..4,
    ));
    let int_list = maybe(prop::collection::vec(
        int_value().prop_map(Scalar::Int),
        0..4,
    ));
    (
        maybe(string_value()),
        maybe(string_value()),
        maybe(int_value()),
        maybe(any::<bool>()),
        str_list,
        int_list,
    )
        .prop_map(|(tenant, prefix, level, active, regions, nums)| {
            let mut out = Attrs::new();
            let mut put = |name: &str, v: Option<AttrValue>| {
                if let Some(v) = v {
                    out.insert(name, v).expect("attribute");
                }
            };
            put(
                "tenant",
                tenant.map(|s| AttrValue::Scalar(Scalar::String(s))),
            );
            put(
                "prefix",
                prefix.map(|s| AttrValue::Scalar(Scalar::String(s))),
            );
            put("level", level.map(|i| AttrValue::Scalar(Scalar::Int(i))));
            put("active", active.map(|b| AttrValue::Scalar(Scalar::Bool(b))));
            put("regions", regions.map(AttrValue::List));
            put("nums", nums.map(AttrValue::List));
            out
        })
        .boxed()
}

/// Predicates and attributes from a fixed seed: the same `n` pairs on every run and machine,
/// for the Trino side of the differential (D-09).
pub fn fixed_seed_cases(n: usize) -> Vec<(Pred, Attrs)> {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &[7u8; 32]),
    );
    let strategy = (pred(), attrs_strategy());
    (0..n)
        .map(|_| {
            strategy
                .new_tree(&mut runner)
                .expect("value tree")
                .current()
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// The fixed corpus (tests/data/rowpred_corpus.json).
// ---------------------------------------------------------------------------------------

const CORPUS: &str = include_str!("../data/rowpred_corpus.json");

pub struct Case {
    pub src: String,
    pub attrs: Attrs,
}

pub struct Corpus {
    pub rows: Vec<TestRow>,
    pub cases: Vec<Case>,
}

fn json_scalar(v: &serde_json::Value) -> Scalar {
    match v {
        serde_json::Value::Bool(b) => Scalar::Bool(*b),
        serde_json::Value::Number(n) => Scalar::Int(n.as_i64().expect("integer attribute")),
        serde_json::Value::String(s) => Scalar::String(s.clone()),
        other => panic!("unsupported attribute value {other}"),
    }
}

fn json_attrs(v: &serde_json::Value) -> Attrs {
    let mut out = Attrs::new();
    for (name, value) in v.as_object().expect("attrs object") {
        let value = match value {
            serde_json::Value::Array(items) => {
                AttrValue::List(items.iter().map(json_scalar).collect())
            }
            other => AttrValue::Scalar(json_scalar(other)),
        };
        out.insert(name.as_str(), value).expect("attribute");
    }
    out
}

fn json_row(v: &serde_json::Value) -> TestRow {
    let opt = |name: &str| v.get(name).filter(|x| !x.is_null());
    TestRow {
        id: v["id"].as_i64().expect("row id"),
        b: opt("b").map(|x| x.as_bool().expect("bool")),
        i: opt("i").map(|x| x.as_i64().expect("integer")),
        d: opt("d").map(|x| x.as_f64().expect("double")),
        s: opt("s").map(|x| x.as_str().expect("string").to_owned()),
        t: opt("t").map(|x| x.as_str().expect("string").to_owned()),
    }
}

pub fn load_corpus() -> Corpus {
    let json: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus JSON");
    Corpus {
        rows: json["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(json_row)
            .collect(),
        cases: json["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|c| Case {
                src: c["pred"].as_str().expect("pred").to_owned(),
                attrs: json_attrs(&c["attrs"]),
            })
            .collect(),
    }
}

impl Case {
    pub fn pred(&self) -> Pred {
        compile_row_filter(&self.src, &Fixture::new().cx())
            .unwrap_or_else(|e| panic!("corpus predicate {}: {e:?}", self.src))
    }
}

/// What Rust says about every row: `(id, eval)`.
pub fn rust_all(p: &Pred, attrs: &Attrs, rows: &[TestRow]) -> Vec<(i64, Tri)> {
    rows.iter()
        .map(|r| (r.id, eval(p, &r.values(), attrs)))
        .collect()
}
