//! Phase 4 source/edit conformance against real Trino and PostgreSQL.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::Utf8Path;
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::print_explain_distributed_sql;
use ontoforge_engine::{
    AnyEngine, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql,
    QueryContext, QueryOutput, TableRef, Value as EngineValue,
};
use ontoforge_expr::{Attrs, CmpOp, Operand, Pred, Scalar};
use ontoforge_query::aggregate::{AggregateCapabilities, AggregateLimits};
use ontoforge_query::ast::{
    AggregateMeasure, AggregateRequest, Direction, LinkDirection, Nulls, ObjectSet, OrderBy,
    PageSpec, QueryLiteral, TermMode, WhereClause,
};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ExecutionLimits, QueryError, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, LinkOp, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{ActiveDefinition, BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde_json::{Value, json};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!("{name} is required for the real Trino/PostgreSQL query corpus"),
    }
}

fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}

fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

fn pv(raw: &str) -> PropertyValue {
    PropertyValue::String(raw.to_owned())
}

fn unique() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
}

struct RecordingEngine {
    inner: AnyEngine,
    executed: Mutex<Vec<PrintedSql>>,
}

impl RecordingEngine {
    fn take(&self) -> Vec<PrintedSql> {
        std::mem::take(&mut *self.executed.lock().unwrap())
    }
}

impl Engine for RecordingEngine {
    fn dialect(&self) -> Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.executed.lock().unwrap().push(sql.clone());
        self.inner.execute(sql, ctx).await
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }

    async fn status(&self) -> EngineStatus {
        self.inner.status().await
    }
}

struct Fixture {
    engine: RecordingEngine,
    store: EditStore,
    active: Arc<ActiveDefinition>,
    scope: PrincipalScope,
    principal: Principal,
    tokens: TokenCodec,
    edit_schema: String,
}

impl Fixture {
    fn service(&self) -> QueryService<'_, RecordingEngine> {
        QueryService {
            principal: &self.principal,
            scope: &self.scope,
            active: Arc::clone(&self.active),
            engine: &self.engine,
            store: &self.store,
            limits: ExecutionLimits::phase4_default(),
            tokens: &self.tokens,
        }
    }

    async fn explain_captured(&self, case: &str) {
        let statements = self.engine.take();
        assert!(!statements.is_empty(), "{case}: no source SQL was executed");
        let mut ctx = QueryContext::new(format!("phase4-explain-{case}"), vec![]);
        ctx.timeout = Duration::from_secs(60);
        for (index, sql) in statements.iter().enumerate() {
            let explain = print_explain_distributed_sql(sql, Dialect::Trino).unwrap();
            assert_eq!(
                explain.as_str(),
                format!("EXPLAIN (TYPE DISTRIBUTED) {}", sql.as_str()),
                "{case}: EXPLAIN must wrap the exact executed PrintedSql"
            );
            let output = self
                .engine
                .inner
                .execute(&explain, &ctx)
                .await
                .unwrap_or_else(|e| panic!("{case}[{index}]: distributed EXPLAIN failed: {e}"));
            let plan = output
                .rows
                .iter()
                .flat_map(|row| row.iter())
                .map(|value| match value {
                    EngineValue::String(text) => text.as_str(),
                    other => panic!("{case}[{index}]: non-text plan cell: {other:?}"),
                })
                .collect::<Vec<_>>()
                .join("\n");
            let fragments =
                parse_fragments(&plan).unwrap_or_else(|e| panic!("{case}[{index}]: {e}\n{plan}"));
            assert!(
                (1..=2).contains(&fragments),
                "{case}[{index}]: {fragments} fragments, expected 1..=2\n{plan}"
            );
            let directory = std::env::var("ONTOFORGE_QUERY_PLAN_DIR")
                .unwrap_or_else(|_| {
                    concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/query-plans").to_owned()
                });
            std::fs::create_dir_all(&directory).expect("create plan artifact directory");
            let safe_case = case.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
            std::fs::write(format!("{directory}/{safe_case}_{index}.txt"), &plan)
                .expect("write distributed plan");
            println!("{case}[{index}]: {fragments} distributed fragments");
        }
    }
}

fn parse_fragments(plan: &str) -> Result<usize, String> {
    let mut ids = BTreeSet::new();
    for line in plan.lines() {
        let line = line.trim_start();
        let Some(tail) = line.strip_prefix("Fragment ") else {
            if line.starts_with("Fragment") {
                return Err(format!("malformed fragment header: {line}"));
            }
            continue;
        };
        let (id, kind) = tail
            .split_once(' ')
            .ok_or_else(|| format!("malformed fragment header: {line}"))?;
        if id.is_empty()
            || !id.bytes().all(|b| b.is_ascii_digit())
            || (id.len() > 1 && id.starts_with('0'))
            || !kind.starts_with('[')
            || !kind.ends_with(']')
            || kind.len() < 3
            || !kind[1..kind.len() - 1]
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b == b'_')
        {
            return Err(format!("malformed fragment header: {line}"));
        }
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate fragment id {id}"));
        }
    }
    if ids.is_empty() {
        return Err("no complete Fragment header".to_owned());
    }
    if ids.len() > 2 {
        return Err(format!("{} fragments exceed the maximum of 2", ids.len()));
    }
    Ok(ids.len())
}

#[allow(
    clippy::too_many_lines,
    reason = "real backend and fixture prerequisites share one setup"
)]
async fn fixture() -> Fixture {
    let trino_url = required("ONTOFORGE_TRINO_URL");
    let pg_url = required("ONTOFORGE_PG_URL");
    let text = format!(
        "[trino]\nurl = \"{trino_url}\"\nconnect_timeout_ms = 5000\n\n[trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"none\"\n"
    );
    let config = parse_server_config(&text).expect("ONTOFORGE_TRINO_URL configuration");
    let engine = RecordingEngine {
        inner: AnyEngine::Trino(
            build_trino_engines(
                &config.trino.expect("Trino config"),
                Utf8Path::new("."),
                &|_| None,
            )
            .expect("Trino engine")
            .ontology,
        ),
        executed: Mutex::new(Vec::new()),
    };
    let schema = format!("of_query_{}", unique());
    let edit_schema = format!("{schema}_edit");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Postgres {
            url: secrecy::SecretString::from(pg_url),
            schema_edit: edit_schema.clone(),
            schema_sys: format!("{schema}_sys"),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("real PostgreSQL edit store");
    let loaded = ontoforge_schema::load_dir(Utf8Path::new(AVIATION));
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.expect("aviation ontology");
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload real PostgreSQL edit store");
    let active = store.active_definition().await.expect("active definition");
    let scope = PrincipalScope {
        types: ontology
            .object_types
            .iter()
            .map(|(ty_name, ty)| {
                (
                    ty_name.clone(),
                    TypeScope {
                        row: Pred::Const(true),
                        columns: ty
                            .properties
                            .keys()
                            .cloned()
                            .map(|property| (property, Pred::Const(true)))
                            .collect(),
                    },
                )
            })
            .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "query-reader"),
        issuer: "test".into(),
        subject: "query-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let fixture = Fixture {
        engine,
        store,
        active,
        scope,
        principal,
        tokens: TokenCodec::new(b"phase4-real-backend".to_vec()).unwrap(),
        edit_schema,
    };
    let table = TableRef {
        catalog: "demo".into(),
        schema: "aviation".into(),
        table: "flights".into(),
    };
    let ctx = QueryContext::new("query-corpus-check".into(), vec![]);
    let columns = fixture
        .engine
        .describe_tables(std::slice::from_ref(&table), &ctx)
        .await
        .expect("Trino demo catalog unavailable");
    assert!(
        columns.contains_key(&table),
        "Trino demo.aviation.flights missing"
    );
    let query = validated(&fixture, request(corpus_set(), &["flightId"]), 20, None);
    let page = fixture
        .service()
        .execute_list(&query)
        .await
        .expect("query corpus fixture missing or unreadable");
    assert_eq!(
        keys(&page.rows),
        ["Q-0001", "Q-0002", "Q-0003", "Q-0004", "Q-0005", "Q-0006"],
        "ci/pg/query_corpus.sql must be seeded in demo"
    );
    fixture.explain_captured("fixture_source").await;
    fixture
}

async fn seed_edits(f: &Fixture) {
    let flight = name("Flight");
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "phase4QueryCorpus".into(),
            def_version: f.active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Object {
                    type_name: flight.clone(),
                    pk: pv("Q-0002"),
                    op: ObjectOp::Set(BTreeMap::from([(prop("status"), pv("delayed"))])),
                },
                Edit::Object {
                    type_name: flight.clone(),
                    pk: pv("Q-0003"),
                    op: ObjectOp::Tombstone,
                },
                Edit::Object {
                    type_name: flight,
                    pk: pv("Q-0004"),
                    op: ObjectOp::Set(BTreeMap::from([(prop("status"), pv("changed"))])),
                },
                Edit::Object {
                    type_name: name("Airport"),
                    pk: pv("FRA"),
                    op: ObjectOp::Set(BTreeMap::from([(prop("name"), pv("Frankfurt"))])),
                },
                Edit::Link {
                    link: LinkTypeName::try_from("flightCrew".to_owned()).unwrap(),
                    a: pv("Q-0001"),
                    b: pv("E-003"),
                    op: LinkOp::Remove,
                },
                Edit::Link {
                    link: LinkTypeName::try_from("flightCrew".to_owned()).unwrap(),
                    a: pv("Q-0001"),
                    b: pv("E-006"),
                    op: LinkOp::Add,
                },
            ],
        })
        .await
        .expect("real PG edit commit");
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "phase4Release".into(),
            def_version: f.active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Flight"),
                pk: pv("Q-0004"),
                op: ObjectOp::Release,
            }],
        })
        .await
        .expect("real PG release");
}

fn request(set: ObjectSet, select: &[&str]) -> QueryRequest {
    QueryRequest {
        set,
        select: select.iter().map(|value| prop(value)).collect(),
    }
}

fn corpus_set() -> ObjectSet {
    ObjectSet::Where {
        object_type: name("Flight"),
        predicate: WhereClause::StartsWith {
            property: prop("flightId"),
            value: "Q-".into(),
        },
    }
}

fn validated(
    f: &Fixture,
    request: QueryRequest,
    first: u64,
    after: Option<String>,
) -> ValidatedQuery {
    ValidatedQuery::new(
        request,
        vec![OrderBy {
            property: prop("flightId"),
            direction: Direction::Asc,
            nulls: Nulls::Last,
        }],
        PageSpec { first, after },
        &f.active.ontology,
        &f.scope.types[&name("Flight")],
        CompileLimits::default(),
    )
    .unwrap()
}

fn keys(rows: &[ontoforge_query::ObjectRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.pk.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn distributed_fragment_parser_rejects_bad_evidence() {
    assert_eq!(parse_fragments("Fragment 0 [SOURCE]\n  Scan"), Ok(1));
    for bad in [
        "text Fragment 0 [SOURCE]",
        "Fragment x [SOURCE]",
        "Fragment 0",
        "Fragment",
        "Fragment 00 [SOURCE]",
        "Fragment 0 [SOURCE] trailing",
        "Fragment 0 [SOURCE]\nFragment 0 [SINGLE]",
        "Fragment 0 [SOURCE]\nFragment 1 [HASH]\nFragment 2 [SINGLE]",
    ] {
        assert!(parse_fragments(bad).is_err(), "{bad}");
    }
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn fixed_semantics_and_cursor_refill() {
    let f = fixture().await;
    seed_edits(&f).await;
    let service = f.service();
    let mut after = None;
    let mut actual = Vec::new();
    for page_index in 0..3 {
        let query = validated(
            &f,
            request(corpus_set(), &["status", "delayMinutes"]),
            2,
            after,
        );
        let page = service
            .execute_list(&query)
            .await
            .expect("real corpus page");
        actual.extend(keys(&page.rows));
        after = page.cursor;
        assert_eq!(page.has_more, page_index < 2);
        f.explain_captured(&format!("cursor_page_{page_index}"))
            .await;
    }
    assert_eq!(actual, ["Q-0001", "Q-0002", "Q-0004", "Q-0005", "Q-0006"]);

    for (label, pk, expected) in [
        ("source", "Q-0001", Some("Boarding")),
        ("patched", "Q-0002", Some("delayed")),
        ("tombstoned", "Q-0003", None),
        ("released", "Q-0004", Some("delayed")),
        ("missing", "Q-9999", None),
    ] {
        let outcome = service
            .execute(request(
                ObjectSet::Pk {
                    object_type: name("Flight"),
                    pk: json!(pk),
                },
                &["status"],
            ))
            .await;
        match expected {
            Some(status) => assert_eq!(
                outcome.unwrap().rows[0].properties[&prop("status")],
                json!(status),
                "{label}"
            ),
            None => assert!(matches!(outcome, Err(QueryError::NotFound)), "{label}"),
        }
        if label == "tombstoned" {
            assert!(f.engine.take().is_empty(), "tombstone must avoid source");
        } else {
            f.explain_captured(&format!("pk_{label}")).await;
        }
    }
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
#[allow(
    clippy::too_many_lines,
    reason = "the fixed operator, set, and link corpus is one case matrix"
)]
async fn where_search_set_and_relationship_corpus() {
    let f = fixture().await;
    seed_edits(&f).await;
    let service = f.service();
    let cases = [
        (
            "eq",
            WhereClause::Eq {
                property: prop("status"),
                value: QueryLiteral(json!("delayed")),
            },
            vec!["Q-0002", "Q-0004"],
        ),
        (
            "gt",
            WhereClause::Gt {
                property: prop("delayMinutes"),
                value: QueryLiteral(json!(20)),
            },
            vec!["Q-0006"],
        ),
        (
            "gte",
            WhereClause::Gte {
                property: prop("delayMinutes"),
                value: QueryLiteral(json!(20)),
            },
            vec!["Q-0005", "Q-0006"],
        ),
        (
            "lt",
            WhereClause::Lt {
                property: prop("delayMinutes"),
                value: QueryLiteral(json!(10)),
            },
            vec!["Q-0001"],
        ),
        (
            "lte",
            WhereClause::Lte {
                property: prop("delayMinutes"),
                value: QueryLiteral(json!(10)),
            },
            vec!["Q-0001", "Q-0002"],
        ),
        (
            "null",
            WhereClause::IsNull {
                property: prop("status"),
            },
            vec!["Q-0005"],
        ),
        (
            "in",
            WhereClause::In {
                property: prop("delayMinutes"),
                values: vec![QueryLiteral(json!(0)), QueryLiteral(json!(30))],
            },
            vec!["Q-0001", "Q-0006"],
        ),
        (
            "starts",
            WhereClause::StartsWith {
                property: prop("status"),
                value: "del".into(),
            },
            vec!["Q-0002", "Q-0004"],
        ),
        (
            "any",
            WhereClause::ContainsAnyTerm {
                property: prop("status"),
                value: "boarding delayed".into(),
            },
            vec!["Q-0001", "Q-0002", "Q-0004"],
        ),
        (
            "all",
            WhereClause::ContainsAllTerms {
                property: prop("status"),
                value: "delayed delay".into(),
            },
            vec!["Q-0002", "Q-0004"],
        ),
    ];
    for (label, predicate, expected) in cases {
        let restricted = ObjectSet::Intersect {
            object_type: name("Flight"),
            sets: vec![
                corpus_set(),
                ObjectSet::Where {
                    object_type: name("Flight"),
                    predicate,
                },
            ],
        };
        let query = validated(&f, request(restricted, &["status"]), 100, None);
        let page = service
            .execute_list(&query)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(keys(&page.rows), expected, "{label}");
        f.explain_captured(&format!("where_{label}")).await;
    }
    let search = ObjectSet::Search {
        object_type: name("Flight"),
        property: prop("status"),
        query: "boarding delayed".into(),
        mode: TermMode::Any,
    };
    let query = validated(
        &f,
        request(
            ObjectSet::Intersect {
                object_type: name("Flight"),
                sets: vec![corpus_set(), search],
            },
            &["status"],
        ),
        100,
        None,
    );
    let page = service.execute_list(&query).await.expect("search any");
    assert_eq!(keys(&page.rows), ["Q-0001", "Q-0002", "Q-0004"]);
    f.explain_captured("search_any").await;

    let delayed = ObjectSet::Where {
        object_type: name("Flight"),
        predicate: WhereClause::Eq {
            property: prop("status"),
            value: QueryLiteral(json!("delayed")),
        },
    };
    let union = ObjectSet::Union {
        object_type: name("Flight"),
        sets: vec![
            ObjectSet::Pk {
                object_type: name("Flight"),
                pk: json!("Q-0001"),
            },
            delayed.clone(),
        ],
    };
    let query = validated(&f, request(union.clone(), &["status"]), 100, None);
    let page = service.execute_list(&query).await.expect("union");
    assert!(keys(&page.rows).contains(&"Q-0001".to_owned()));
    assert!(keys(&page.rows).contains(&"Q-0002".to_owned()));
    f.explain_captured("set_union").await;
    let subtract = ObjectSet::Subtract {
        object_type: name("Flight"),
        left: Box::new(union),
        right: Box::new(delayed),
    };
    let query = validated(&f, request(subtract, &["status"]), 100, None);
    let page = service.execute_list(&query).await.expect("subtract");
    assert_eq!(keys(&page.rows), ["Q-0001"]);
    f.explain_captured("set_subtract").await;

    let departure: ObjectSet = serde_json::from_value(json!({
        "kind": "searchAround", "object_type": "Airport",
        "base": { "kind": "pk", "object_type": "Flight", "pk": "Q-0001" },
        "link": "flightDepartureAirport", "direction": "toFrom"
    }))
    .unwrap();
    let query = ValidatedQuery::new(
        request(departure, &["name"]),
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &f.active.ontology,
        &f.scope.types[&name("Airport")],
        CompileLimits::default(),
    )
    .unwrap();
    let page = service
        .execute_list(&query)
        .await
        .expect("FK to PG edit-only target");
    assert_eq!(keys(&page.rows), ["FRA"]);
    f.explain_captured("outbound_fk").await;

    let crew: ObjectSet = serde_json::from_value(json!({
        "kind": "searchAround", "object_type": "Pilot",
        "base": { "kind": "pk", "object_type": "Flight", "pk": "Q-0001" },
        "link": "flightCrew", "direction": "fromTo"
    }))
    .unwrap();
    let query = ValidatedQuery::new(
        request(crew, &["fullName"]),
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &f.active.ontology,
        &f.scope.types[&name("Pilot")],
        CompileLimits::default(),
    )
    .unwrap();
    let page = service
        .execute_list(&query)
        .await
        .expect("M:N with PG link edits");
    assert_eq!(keys(&page.rows), ["E-001", "E-006"]);
    f.explain_captured("many_to_many").await;
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn aggregate_corpus_and_distributed_explain() {
    let f = fixture().await;
    seed_edits(&f).await;
    let service = f.service();
    let request = AggregateRequest {
        set: corpus_set(),
        group_by: vec![],
        measures: vec![
            AggregateMeasure::Count { property: None },
            AggregateMeasure::Sum {
                property: prop("delayMinutes"),
            },
            AggregateMeasure::Avg {
                property: prop("delayMinutes"),
            },
        ],
    };
    let result = service
        .execute_aggregate(
            request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .expect("real aggregate");
    assert_eq!(result.buckets[0].values, [json!(5), json!(60), json!(15.0)]);
    f.explain_captured("aggregate_partial").await;
    for (label, measure, expected) in [
        (
            "min",
            AggregateMeasure::Min {
                property: prop("delayMinutes"),
            },
            json!(0),
        ),
        (
            "max",
            AggregateMeasure::Max {
                property: prop("delayMinutes"),
            },
            json!(30),
        ),
        (
            "distinct",
            AggregateMeasure::Distinct {
                property: prop("delayMinutes"),
            },
            json!(4),
        ),
    ] {
        let request = AggregateRequest {
            set: corpus_set(),
            group_by: vec![],
            measures: vec![measure],
        };
        let result = service
            .execute_aggregate(
                request,
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default(),
            )
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(result.buckets[0].values, [expected], "{label}");
        f.explain_captured(&format!("aggregate_{label}")).await;
    }
    let request = AggregateRequest {
        set: corpus_set(),
        group_by: vec![prop("status")],
        measures: vec![AggregateMeasure::Count { property: None }],
    };
    let result = service
        .execute_aggregate(
            request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .expect("groupBy");
    for (group, expected) in [
        (json!("Boarding"), 1),
        (json!("delayed"), 2),
        (Value::Null, 1),
        (json!("scheduled"), 1),
    ] {
        let bucket = result
            .buckets
            .iter()
            .find(|bucket| bucket.group == vec![group.clone()]);
        assert_eq!(
            bucket.map(|bucket| &bucket.values),
            Some(&vec![json!(expected)])
        );
    }
    f.explain_captured("aggregate_group_by").await;
}

fn outward_error(error: &QueryError) {
    let response = json!({ "code": error.code(), "message": error.to_string() }).to_string();
    for physical in [
        "aviation",
        "flights",
        "flight_id",
        "demo",
        "SELECT",
        "query_id",
        "Q-0003",
    ] {
        assert!(
            !response.contains(physical),
            "physical detail leaked: {response}"
        );
    }
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
#[allow(
    clippy::too_many_lines,
    reason = "cursor replay and role masking use the same real fixture"
)]
async fn real_security_roles_and_cursor_replay() {
    let f = fixture().await;
    seed_edits(&f).await;
    let first = validated(&f, request(corpus_set(), &["status"]), 1, None);
    let page = f
        .service()
        .execute_list(&first)
        .await
        .expect("first cursor page");
    let cursor = page.cursor.expect("first page cursor");
    f.explain_captured("security_first_page").await;

    let mut other_principal = f.principal.clone();
    other_principal.id = PrincipalId::new("test", "other-reader");
    let mut other_scope = f.scope.clone();
    other_scope
        .types
        .get_mut(&name("Flight"))
        .unwrap()
        .columns
        .remove(&prop("status"));
    let mut other_definition = (*f.active).clone();
    other_definition.def_version += 1;
    for (label, principal, scope, active) in [
        (
            "principal",
            &other_principal,
            &f.scope,
            Arc::clone(&f.active),
        ),
        ("scope", &f.principal, &other_scope, Arc::clone(&f.active)),
        (
            "definition",
            &f.principal,
            &f.scope,
            Arc::new(other_definition),
        ),
    ] {
        let service = QueryService {
            principal,
            scope,
            active,
            engine: &f.engine,
            store: &f.store,
            limits: ExecutionLimits::phase4_default(),
            tokens: &f.tokens,
        };
        let query = validated(
            &f,
            request(corpus_set(), &["status"]),
            1,
            Some(cursor.clone()),
        );
        let error = service.execute_list(&query).await.expect_err(label);
        assert!(
            matches!(error, QueryError::InvalidCursor),
            "{label}: {error:?}"
        );
        outward_error(&error);
        assert!(
            f.engine.take().is_empty(),
            "{label}: cursor replay reached source"
        );
    }

    let mut role_scope = f.scope.clone();
    // A row seen through the broad role must not inherit `status` from the narrow role.
    role_scope
        .types
        .get_mut(&name("Flight"))
        .unwrap()
        .columns
        .insert(
            prop("status"),
            Pred::Cmp {
                left: Operand::Column(prop("flightId")),
                op: CmpOp::Eq,
                right: Operand::Lit(Scalar::String("Q-0001".into())),
            },
        );
    let service = QueryService {
        principal: &f.principal,
        scope: &role_scope,
        active: Arc::clone(&f.active),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let query = validated(&f, request(corpus_set(), &["status"]), 20, None);
    let page = service
        .execute_list(&query)
        .await
        .expect("role scoped list");
    for row in &page.rows {
        if row.pk != json!("Q-0001") {
            assert!(
                !row.properties.contains_key(&prop("status")),
                "cross-role value leaked"
            );
        }
    }
    f.explain_captured("multi_role_mask").await;
    let filtered = ObjectSet::Intersect {
        object_type: name("Flight"),
        sets: vec![
            corpus_set(),
            ObjectSet::Where {
                object_type: name("Flight"),
                predicate: WhereClause::Eq {
                    property: prop("status"),
                    value: QueryLiteral(json!("delayed")),
                },
            },
        ],
    };
    let query = validated(&f, request(filtered, &["status"]), 20, None);
    assert!(
        service
            .execute_list(&query)
            .await
            .expect("role scoped filter")
            .rows
            .is_empty()
    );
    f.explain_captured("multi_role_filter").await;
    let query = ValidatedQuery::new(
        request(corpus_set(), &["status"]),
        vec![OrderBy {
            property: prop("status"),
            direction: Direction::Asc,
            nulls: Nulls::Last,
        }],
        PageSpec {
            first: 20,
            after: None,
        },
        &f.active.ontology,
        &role_scope.types[&name("Flight")],
        CompileLimits::default(),
    )
    .unwrap();
    let sorted = service
        .execute_list(&query)
        .await
        .expect("role scoped sort");
    assert_eq!(sorted.rows[0].pk, json!("Q-0001"));
    assert!(
        sorted
            .rows
            .iter()
            .skip(1)
            .all(|row| !row.properties.contains_key(&prop("status")))
    );
    f.explain_captured("multi_role_sort").await;
    let aggregate_request = AggregateRequest {
        set: corpus_set(),
        group_by: vec![prop("status")],
        measures: vec![AggregateMeasure::Count { property: None }],
    };
    let grouped = service
        .execute_aggregate(
            aggregate_request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .expect("role scoped group");
    assert!(
        grouped
            .buckets
            .iter()
            .all(|bucket| bucket.group == vec![json!("Boarding")]
                || bucket.group == vec![Value::Null])
    );
    f.explain_captured("multi_role_group").await;

    let mut hidden_target_scope = f.scope.clone();
    hidden_target_scope
        .types
        .get_mut(&name("Pilot"))
        .unwrap()
        .row = Pred::Cmp {
        left: Operand::Column(prop("employeeId")),
        op: CmpOp::Eq,
        right: Operand::Lit(Scalar::String("E-001".into())),
    };
    let service = QueryService {
        principal: &f.principal,
        scope: &hidden_target_scope,
        active: Arc::clone(&f.active),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let set = ObjectSet::SearchAround {
        object_type: name("Pilot"),
        base: Box::new(ObjectSet::Pk {
            object_type: name("Flight"),
            pk: json!("Q-0001"),
        }),
        link: LinkTypeName::try_from("flightCrew".to_owned()).unwrap(),
        direction: LinkDirection::FromTo,
    };
    let query = ValidatedQuery::new(
        request(set, &["fullName"]),
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &f.active.ontology,
        &hidden_target_scope.types[&name("Pilot")],
        CompileLimits::default(),
    )
    .unwrap();
    let page = service
        .execute_list(&query)
        .await
        .expect("hidden M:N target");
    assert_eq!(keys(&page.rows), ["E-001"]);
    f.explain_captured("hidden_target").await;
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn real_physical_errors_are_sanitized() {
    let f = fixture().await;
    let mut broken = (*f.active).clone();
    let profile = broken.ontology.default_profile.clone().expect("profile");
    broken
        .ontology
        .bindings
        .get_mut(&profile)
        .unwrap()
        .objects
        .get_mut(&name("Flight"))
        .unwrap()
        .source
        .table = "not_a_real_phase4_table".into();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::new(broken),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let error = service
        .execute(request(
            ObjectSet::Pk {
                object_type: name("Flight"),
                pk: json!("Q-0001"),
            },
            &["status"],
        ))
        .await
        .expect_err("Trino missing table must fail");
    assert!(matches!(error, QueryError::Source), "{error:?}");
    outward_error(&error);
    assert_eq!(
        f.engine.take().len(),
        1,
        "real Trino error path must execute SQL"
    );

    let pg_url = required("ONTOFORGE_PG_URL");
    let pool = sqlx::PgPool::connect(&pg_url)
        .await
        .expect("PG side connection");
    let drop_schema = format!("DROP SCHEMA \"{}\" CASCADE", f.edit_schema);
    sqlx::query(sqlx::AssertSqlSafe(drop_schema))
        .execute(&pool)
        .await
        .expect("drop this test's edit schema");
    let error = f
        .service()
        .execute(request(
            ObjectSet::Pk {
                object_type: name("Flight"),
                pk: json!("Q-0001"),
            },
            &["status"],
        ))
        .await
        .expect_err("PG missing edit table must fail");
    assert!(matches!(error, QueryError::Store), "{error:?}");
    outward_error(&error);
    assert!(
        f.engine.take().is_empty(),
        "PG failure must stop before source"
    );
}
