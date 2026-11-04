//! Real Trino/PostgreSQL Phase 4 timing evidence. Run only with dedicated services.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::aggregate::{AggregateCapabilities, AggregateError, AggregateLimits};
use ontoforge_query::ast::{
    AggregateMeasure, AggregateRequest, Direction, Nulls, ObjectSet, OrderBy, PageSpec,
    QueryLiteral, WhereClause,
};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ExecutionLimits, QueryError, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{ActiveDefinition, BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::Connection;
use sqlx::postgres::{PgConnectOptions, PgConnection};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const ROWS: usize = 10_000;
const PATCHES: usize = 800;
const TOMBSTONES: usize = 100;
const RELEASES: usize = 100;

fn required(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{name} is required for the real query benchmark"))
}

fn object_type() -> ObjectTypeName {
    ObjectTypeName::try_from("Flight".to_owned()).unwrap()
}

fn prop(value: &str) -> PropertyName {
    PropertyName::try_from(value.to_owned()).unwrap()
}

fn key(index: usize) -> String {
    format!("B-{index:05}")
}

struct RecordingEngine {
    inner: AnyEngine,
    statements: Mutex<Vec<PrintedSql>>,
}

impl RecordingEngine {
    fn take(&self) -> Vec<PrintedSql> {
        std::mem::take(&mut *self.statements.lock().unwrap())
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
        self.statements.lock().unwrap().push(sql.clone());
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
    pg_version: String,
    trino_version: String,
    ontology_tree_hash: String,
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

    async fn plans(&self, path: &str, iteration: usize) -> PlanCapture {
        let statements = self.engine.take();
        let mut capture = PlanCapture {
            source_calls: statements.len(),
            plans: Vec::with_capacity(statements.len()),
            error: None,
        };
        if statements.is_empty() {
            capture.error = Some(format!("{path}[{iteration}]: no source SQL executed"));
            return capture;
        }
        let mut ctx = QueryContext::new(format!("phase4-bench-{path}-{iteration}"), vec![]);
        ctx.timeout = Duration::from_secs(60);
        for (index, sql) in statements.iter().enumerate() {
            let explain = match print_explain_distributed_sql(sql, Dialect::Trino) {
                Ok(explain) => explain,
                Err(error) => {
                    capture.error = Some(format!("{path}[{iteration}][{index}]: {error}"));
                    break;
                }
            };
            let output = match self.engine.inner.execute(&explain, &ctx).await {
                Ok(output) => output,
                Err(error) => {
                    capture.error = Some(format!("{path}[{iteration}][{index}]: {error}"));
                    break;
                }
            };
            let plan = match complete_plan(&output) {
                Ok(plan) => plan,
                Err(error) => {
                    capture.error = Some(format!("{path}[{iteration}][{index}]: {error}"));
                    break;
                }
            };
            let fragment_count = match parse_fragments(&plan) {
                Ok(count) => count,
                Err(error) => {
                    capture.error = Some(format!("{path}[{iteration}][{index}]: {error}"));
                    0
                }
            };
            capture.plans.push(PlanEvidence {
                statement_index: index,
                fragment_count,
                plan,
            });
            if capture.error.is_some() {
                break;
            }
        }
        capture
    }
}

fn complete_plan(output: &QueryOutput) -> Result<String, &'static str> {
    if output.truncated {
        return Err("distributed EXPLAIN output was truncated");
    }
    output
        .rows
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| match value {
            EngineValue::String(text) => Ok(text.as_str()),
            _ => Err("distributed EXPLAIN returned a non-text cell"),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|cells| cells.join("\n"))
}

fn parse_fragments(plan: &str) -> Result<usize, String> {
    let mut ids = BTreeSet::new();
    for line in plan.lines().map(str::trim_start) {
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
            || !id.bytes().all(|byte| byte.is_ascii_digit())
            || (id.len() > 1 && id.starts_with('0'))
            || !kind.starts_with('[')
            || !kind.ends_with(']')
            || kind.len() < 3
            || !kind[1..kind.len() - 1]
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
            || !ids.insert(id.to_owned())
        {
            return Err(format!("malformed or duplicate fragment header: {line}"));
        }
    }
    if !(1..=2).contains(&ids.len()) {
        return Err(format!(
            "{} distributed fragments; expected 1 or 2",
            ids.len()
        ));
    }
    Ok(ids.len())
}

fn trino_version(url: &str) -> String {
    let info = std::process::Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            &format!("{url}/v1/info"),
        ])
        .output()
        .expect("curl is required to identify the Trino version");
    assert!(info.status.success(), "Trino /v1/info was unavailable");
    let value: Value = serde_json::from_slice(&info.stdout).expect("Trino /v1/info JSON");
    value["nodeVersion"]["version"]
        .as_str()
        .expect("Trino /v1/info nodeVersion.version")
        .to_owned()
}

fn ontology_tree_hash() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD:examples/ontologies/aviation"])
        .output()
        .expect("git is required for the ontology tree hash");
    assert!(output.status.success(), "ontology tree is not committed");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[allow(
    clippy::too_many_lines,
    reason = "real backend setup and fixed dataset share one fixture"
)]
async fn setup() -> Fixture {
    let trino_url = required("ONTOFORGE_TRINO_URL");
    let pg_url = required("ONTOFORGE_PG_URL");
    let config = parse_server_config(&format!(
        "[trino]\nurl = \"{trino_url}\"\nconnect_timeout_ms = 5000\n\n[trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"none\"\n"
    ))
    .expect("Trino configuration");
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
        statements: Mutex::new(Vec::new()),
    };
    let mut demo = PgConnection::connect_with(
        &PgConnectOptions::from_str(&pg_url)
            .expect("PostgreSQL URL")
            .database("demo"),
    )
    .await
    .expect("PostgreSQL demo database");
    let pg_version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&mut demo)
        .await
        .expect("PostgreSQL version");
    sqlx::query(
        "INSERT INTO aviation.flights (flight_id, flight_no, departure_airport, arrival_airport, tail_number, scheduled_departure, status, delay_minutes, tags) \
         SELECT 'B-' || lpad(i::text, 5, '0'), 'BF' || lpad(i::text, 5, '0'), 'FRA', 'JFK', 'D-AAAA', \
                timestamp '2026-04-01 09:00:00', CASE WHEN i % 10 = 0 THEN 'bench-hot' ELSE 'bench-cold' END, i % 100, ARRAY['bench']::text[] \
         FROM generate_series(1, 10000) AS i \
         ON CONFLICT (flight_id) DO UPDATE SET flight_no = EXCLUDED.flight_no, departure_airport = EXCLUDED.departure_airport, \
           arrival_airport = EXCLUDED.arrival_airport, tail_number = EXCLUDED.tail_number, scheduled_departure = EXCLUDED.scheduled_departure, \
           status = EXCLUDED.status, delay_minutes = EXCLUDED.delay_minutes, tags = EXCLUDED.tags",
    )
    .execute(&mut demo)
    .await
    .expect("seed fixed 10,000 source rows");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM aviation.flights WHERE flight_id BETWEEN 'B-00001' AND 'B-10000'",
    )
    .fetch_one(&mut demo)
    .await
    .expect("count fixed source rows");
    assert_eq!(
        count,
        i64::try_from(ROWS).unwrap(),
        "fixed source dataset must have exactly 10,000 rows"
    );
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let schema = format!("of_bench_{suffix}");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Postgres {
            url: secrecy::SecretString::from(pg_url),
            schema_edit: format!("{schema}_edit"),
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
    reload(
        &store,
        &loaded.ontology.expect("aviation ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload PostgreSQL edit store");
    let active = store.active_definition().await.expect("active definition");
    let scope = PrincipalScope {
        types: active
            .ontology
            .object_types
            .iter()
            .map(|(name, ty)| {
                (
                    name.clone(),
                    TypeScope {
                        row: Pred::Const(true),
                        columns: ty
                            .properties
                            .keys()
                            .cloned()
                            .map(|p| (p, Pred::Const(true)))
                            .collect(),
                    },
                )
            })
            .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("bench", "query-reader"),
        issuer: "bench".into(),
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
        tokens: TokenCodec::new(b"phase4-bench-token".to_vec()).unwrap(),
        pg_version,
        trino_version: trino_version(&trino_url),
        ontology_tree_hash: ontology_tree_hash(),
    };
    seed_edits(&fixture).await;
    fixture
}

async fn seed_edits(f: &Fixture) {
    let mut edits = Vec::with_capacity(PATCHES + TOMBSTONES + RELEASES);
    for index in 1..=PATCHES + TOMBSTONES + RELEASES {
        let op = if index > PATCHES && index <= PATCHES + TOMBSTONES {
            ObjectOp::Tombstone
        } else if index == 1 {
            ObjectOp::Set(BTreeMap::from([
                (
                    prop("flightNumber"),
                    PropertyValue::String("BF00001".to_owned()),
                ),
                (
                    prop("departureAirportCode"),
                    PropertyValue::String("FRA".to_owned()),
                ),
                (
                    prop("arrivalAirportCode"),
                    PropertyValue::String("JFK".to_owned()),
                ),
                (
                    prop("aircraftTailNumber"),
                    PropertyValue::String("D-AAAA".to_owned()),
                ),
                (
                    prop("scheduledDeparture"),
                    PropertyValue::Timestamp("2026-04-01T07:00:00.000Z".to_owned()),
                ),
                (
                    prop("status"),
                    PropertyValue::String("bench-edited".to_owned()),
                ),
                (prop("delayMinutes"), PropertyValue::Int(1)),
                (
                    prop("tags"),
                    PropertyValue::Array(vec![PropertyValue::String("bench".to_owned())]),
                ),
            ]))
        } else {
            ObjectOp::Set(BTreeMap::from([(
                prop("status"),
                PropertyValue::String("bench-edited".to_owned()),
            )]))
        };
        edits.push(Edit::Object {
            type_name: object_type(),
            pk: PropertyValue::String(key(index)),
            op,
        });
    }
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "phase4BenchCoverage".into(),
            def_version: f.active.def_version,
            params: json!({"seed": 4, "rows": ROWS}),
            edits,
        })
        .await
        .expect("seed 10% object edit coverage");
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "phase4BenchRelease".into(),
            def_version: f.active.def_version,
            params: json!({"seed": 4}),
            edits: (PATCHES + TOMBSTONES + 1..=PATCHES + TOMBSTONES + RELEASES)
                .map(|index| Edit::Object {
                    type_name: object_type(),
                    pk: PropertyValue::String(key(index)),
                    op: ObjectOp::Release,
                })
                .collect(),
        })
        .await
        .expect("seed release coverage");
}

fn list_query(f: &Fixture) -> ValidatedQuery {
    ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Where {
                object_type: object_type(),
                predicate: WhereClause::Eq {
                    property: prop("status"),
                    value: QueryLiteral(json!("bench-hot")),
                },
            },
            select: vec![prop("flightId"), prop("status")],
        },
        vec![OrderBy {
            property: prop("flightId"),
            direction: Direction::Asc,
            nulls: Nulls::Last,
        }],
        PageSpec {
            first: 20,
            after: None,
        },
        &f.active.ontology,
        &f.scope.types[&object_type()],
        CompileLimits::default(),
    )
    .expect("fixed list query")
}

#[derive(Serialize)]
struct PlanEvidence {
    statement_index: usize,
    fragment_count: usize,
    plan: String,
}

struct PlanCapture {
    source_calls: usize,
    plans: Vec<PlanEvidence>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Sample {
    iteration: usize,
    elapsed_ms: f64,
    error: Option<String>,
    limit_error: bool,
    source_calls: usize,
    plans: Vec<PlanEvidence>,
}

#[derive(Serialize)]
struct PathEvidence {
    name: &'static str,
    query_shape: &'static str,
    threshold_ms: f64,
    warmups: Vec<Sample>,
    measured: Vec<Sample>,
    limit_error_samples: usize,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    max_ms: Option<f64>,
    passed: bool,
}

fn nearest_rank(samples: &[Sample], percentile: usize) -> Option<f64> {
    if samples.iter().any(|sample| sample.error.is_some()) {
        return None;
    }
    let mut values = samples
        .iter()
        .map(|sample| sample.elapsed_ms)
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values
        .get((values.len() * percentile).div_ceil(100).saturating_sub(1))
        .copied()
}

#[derive(Clone, Copy)]
enum Path {
    SourcePk,
    List,
    Aggregate,
    EditedPk,
}

impl Path {
    fn name(self) -> &'static str {
        match self {
            Self::SourcePk => "source_pk",
            Self::List => "filtered_list_page",
            Self::Aggregate => "aggregate",
            Self::EditedPk => "edited_pg_direct_pk",
        }
    }

    fn threshold(self) -> f64 {
        match self {
            Self::SourcePk => 500.0,
            Self::List => 1_000.0,
            Self::Aggregate => 1_500.0,
            Self::EditedPk => 50.0,
        }
    }

    fn shape(self) -> &'static str {
        match self {
            Self::SourcePk => {
                "Flight pk flightId=B-10000; select flightId,status; unedited source read"
            }
            Self::List => {
                "Flight where status=bench-hot; order flightId ASC NULLS LAST; first=20, after=null; select flightId,status"
            }
            Self::Aggregate => "Flight where status=bench-hot; groupBy=[]; measure=count(*)",
            Self::EditedPk => {
                "Flight pk flightId=B-00001; select flightId,status; fully patched PG-direct read, zero source calls"
            }
        }
    }
}

type TimedResult = Result<(), (String, bool)>;

#[allow(
    clippy::too_many_lines,
    reason = "all four timed paths use the same failure recording"
)]
async fn sample(f: &Fixture, path: Path, iteration: usize) -> Sample {
    let _ = f.engine.take();
    let service = f.service();
    let start = Instant::now();
    let result: TimedResult = match path {
        Path::SourcePk | Path::EditedPk => {
            let pk = if matches!(path, Path::SourcePk) {
                key(ROWS)
            } else {
                key(1)
            };
            service
                .execute(QueryRequest {
                    set: ObjectSet::Pk {
                        object_type: object_type(),
                        pk: json!(pk),
                    },
                    select: vec![prop("flightId"), prop("status")],
                })
                .await
                .map_err(|error| {
                    let limited = matches!(
                        error,
                        QueryError::CostExceeded | QueryError::WorkBudgetExceeded
                    );
                    (error.to_string(), limited)
                })
                .and_then(|page| {
                    if page.rows.len() == 1 {
                        Ok(())
                    } else {
                        Err((
                            format!("pk returned {} rows, expected 1", page.rows.len()),
                            false,
                        ))
                    }
                })
        }
        Path::List => service
            .execute_list(&list_query(f))
            .await
            .map_err(|error| {
                let limited = matches!(
                    error,
                    QueryError::CostExceeded | QueryError::WorkBudgetExceeded
                );
                (error.to_string(), limited)
            })
            .and_then(|page| {
                if page.rows.len() == 20 {
                    Ok(())
                } else {
                    Err((
                        format!("fixed list returned {} rows, expected 20", page.rows.len()),
                        false,
                    ))
                }
            }),
        Path::Aggregate => service
            .execute_aggregate(
                AggregateRequest {
                    set: ObjectSet::Where {
                        object_type: object_type(),
                        predicate: WhereClause::Eq {
                            property: prop("status"),
                            value: QueryLiteral(json!("bench-hot")),
                        },
                    },
                    group_by: vec![],
                    measures: vec![AggregateMeasure::Count { property: None }],
                },
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default(),
            )
            .await
            .map_err(|error| {
                let limited = matches!(error, AggregateError::AggregateCostExceeded);
                (error.to_string(), limited)
            })
            .and_then(|result| {
                if result.buckets.len() == 1 && result.buckets[0].values == [json!(910)] {
                    Ok(())
                } else {
                    Err((
                        format!(
                            "fixed aggregate returned {:?}, expected count 910",
                            result.buckets
                        ),
                        false,
                    ))
                }
            }),
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
    let (mut error, limit_error) = match result {
        Ok(()) => (None, false),
        Err((message, limited)) => (Some(message), limited),
    };
    let mut plans = Vec::new();
    let source_calls;
    if matches!(path, Path::List | Path::Aggregate) {
        let capture = f.plans(path.name(), iteration).await;
        source_calls = capture.source_calls;
        plans = capture.plans;
        if let Some(plan_error) = capture.error {
            error.get_or_insert(plan_error);
        }
    } else {
        source_calls = f.engine.take().len();
        if matches!(path, Path::SourcePk) && source_calls == 0 {
            error.get_or_insert("source pk made zero Trino calls".to_owned());
        }
        if matches!(path, Path::EditedPk) && source_calls != 0 {
            error.get_or_insert(format!(
                "edited PG-direct pk made {source_calls} source calls"
            ));
        }
    }
    Sample {
        iteration,
        elapsed_ms,
        error,
        limit_error,
        source_calls,
        plans,
    }
}

async fn measure(f: &Fixture, path: Path, mode: &str) -> PathEvidence {
    let (warmup_count, measured_count) = if mode == "isolated" { (5, 40) } else { (1, 1) };
    let mut warmups = Vec::with_capacity(warmup_count);
    let mut measured = Vec::with_capacity(measured_count);
    for iteration in 0..warmup_count {
        warmups.push(sample(f, path, iteration).await);
    }
    for iteration in 0..measured_count {
        measured.push(sample(f, path, iteration).await);
    }
    let p50_ms = nearest_rank(&measured, 50);
    let p95_ms = nearest_rank(&measured, 95);
    let max_ms = nearest_rank(&measured, 100);
    let limit_error_samples = warmups
        .iter()
        .chain(&measured)
        .filter(|sample| sample.limit_error)
        .count();
    let passed = measured
        .iter()
        .chain(&warmups)
        .all(|sample| sample.error.is_none())
        && (mode != "isolated" || p95_ms.is_some_and(|p95| p95 < path.threshold()));
    PathEvidence {
        name: path.name(),
        query_shape: path.shape(),
        threshold_ms: path.threshold(),
        warmups,
        measured,
        limit_error_samples,
        p50_ms,
        p95_ms,
        max_ms,
        passed,
    }
}

#[tokio::test]
#[ignore = "needs dedicated ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn phase4_real_backend_p95() {
    let mode = required("ONTOFORGE_BENCH_MODE");
    assert!(
        mode == "smoke" || mode == "isolated",
        "ONTOFORGE_BENCH_MODE must be smoke or isolated"
    );
    let fixture = setup().await;
    let mut paths = Vec::new();
    for path in [Path::SourcePk, Path::List, Path::Aggregate, Path::EditedPk] {
        paths.push(measure(&fixture, path, &mode).await);
    }
    let limits = ExecutionLimits::phase4_default();
    let aggregate_limits = AggregateLimits::phase4_default();
    let limit_error_samples: usize = paths.iter().map(|path| path.limit_error_samples).sum();
    let limit_error_known = paths
        .iter()
        .flat_map(|path| path.warmups.iter().chain(&path.measured))
        .all(|sample| sample.error.is_none() || sample.limit_error);
    let evidence = json!({
        "schema_version": 1,
        "mode": mode,
        "git_sha": std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_owned()),
        "run_url": std::env::var("GITHUB_RUN_ID").ok().map(|id| format!("https://github.com/{}/actions/runs/{id}", std::env::var("GITHUB_REPOSITORY").unwrap_or_default())),
        "runner": {"name": std::env::var("RUNNER_NAME").unwrap_or_else(|_| "local".to_owned()), "os": std::env::var("RUNNER_OS").unwrap_or_else(|_| std::env::consts::OS.to_owned()), "isolation": mode == "isolated"},
        "connector": "Trino 483 PostgreSQL connector + direct PostgreSQL 17 edit store",
        "versions": {"trino": fixture.trino_version, "postgresql": fixture.pg_version},
        "dataset": {"seed": 4, "benchmark_rows": ROWS, "benchmark_key_range": ["B-00001", "B-10000"], "other_aviation_seed_and_query_corpus_rows_share_table": true, "edit_coverage": {"total": PATCHES + TOMBSTONES + RELEASES, "patch": PATCHES, "tombstone": TOMBSTONES, "release": RELEASES}, "query_filter": "status == bench-hot", "hot_benchmark_rows": 1_000},
        "definition": {"ontology_tree_hash": fixture.ontology_tree_hash, "version": fixture.active.def_version},
        "execution_limits": {"max_backend_calls": limits.max_backend_calls, "max_raw_candidates": limits.max_raw_candidates, "max_backfill_keys": limits.max_backfill_keys, "max_decoded_bytes": limits.max_decoded_bytes},
        "aggregate_limits": {"max_groups": aggregate_limits.max_groups, "max_buckets": aggregate_limits.max_buckets, "max_distinct_values": aggregate_limits.max_distinct_values, "max_distinct_memory_bytes": aggregate_limits.max_distinct_memory_bytes},
        "guardrails_raised": false,
        "limit_error_samples": limit_error_samples,
        "limit_error_known": limit_error_known,
        "limit_error_detection": "typed QueryError::CostExceeded/WorkBudgetExceeded and AggregateError::AggregateCostExceeded; any error sample fails its path",
        "paths": paths,
    });
    let directory = std::env::var("ONTOFORGE_BENCH_ARTIFACT_DIR").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/query-bench").to_owned()
    });
    std::fs::create_dir_all(&directory).expect("create benchmark artifact directory");
    let artifact = format!("{directory}/phase4-{mode}.json");
    std::fs::write(&artifact, serde_json::to_vec_pretty(&evidence).unwrap())
        .expect("write raw benchmark evidence");
    println!("Phase 4 benchmark evidence: {artifact}");
    for path in &paths {
        println!(
            "{}: p95={:?}ms threshold={}ms passed={}",
            path.name, path.p95_ms, path.threshold_ms, path.passed
        );
    }
    assert!(
        paths.iter().all(|path| path.passed),
        "one or more benchmark paths failed; inspect raw artifact {artifact}"
    );
}

#[test]
fn nearest_rank_uses_all_samples_and_rejects_failures() {
    let values = [1.0, 2.0, 3.0, 4.0, 5.0]
        .into_iter()
        .enumerate()
        .map(|(iteration, elapsed_ms)| Sample {
            iteration,
            elapsed_ms,
            error: None,
            limit_error: false,
            source_calls: 0,
            plans: vec![],
        })
        .collect::<Vec<_>>();
    assert_eq!(nearest_rank(&values, 95), Some(5.0));
    assert_eq!(nearest_rank(&values, 50), Some(3.0));
    let mut failed = values;
    failed[0].error = Some("backend failed".to_owned());
    assert_eq!(nearest_rank(&failed, 95), None);
}

#[test]
fn truncated_distributed_explain_is_rejected_before_fragment_count() {
    let output = QueryOutput {
        columns: vec![],
        rows: vec![vec![EngineValue::String("Fragment 0 [SOURCE]".to_owned())]],
        truncated: true,
    };
    assert_eq!(
        complete_plan(&output),
        Err("distributed EXPLAIN output was truncated")
    );
}
