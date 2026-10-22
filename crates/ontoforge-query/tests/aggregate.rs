use ontoforge_query::aggregate::AggregateLimits;
use ontoforge_query::ast::AggregateRequest;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef, Value as EngineValue,
};
use ontoforge_expr::{Attrs, CmpOp, Operand, Pred, Scalar};
use ontoforge_query::aggregate::{AggregateBucket, AggregateCapabilities, AggregateResult};
use ontoforge_query::ast::{AggregateMeasure, ObjectSet};
use ontoforge_query::compile::compile_aggregate_partial;
use ontoforge_query::{ExecutionLimits, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;

fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}

fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

struct RecordingEngine {
    inner: SqliteEngine,
    sql: Mutex<Vec<String>>,
}

struct ScriptedTopNEngine {
    columns: Vec<PropertyName>,
    calls: AtomicUsize,
}

#[allow(
    clippy::unused_async_trait_impl,
    reason = "scripted test engine returns immediate futures"
)]
impl Engine for ScriptedTopNEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        ontoforge_engine::Dialect::Sqlite
    }

    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        let window = self.calls.fetch_add(1, AtomicOrdering::SeqCst);
        let range = match window.cmp(&3) {
            std::cmp::Ordering::Less => window * 256..(window + 1) * 256,
            std::cmp::Ordering::Equal => 768..769,
            std::cmp::Ordering::Greater => 769..769,
        };
        let rows = range
            .map(|index| {
                self.columns
                    .iter()
                    .map(|property| match property.as_str() {
                        "flightId" => EngineValue::String(format!("F-{index:04}")),
                        "delayMinutes" => EngineValue::Long(i64::try_from(index).unwrap()),
                        "flightNumber" => EngineValue::String(format!("FN-{index:04}")),
                        "departureAirportCode" => EngineValue::String("FRA".to_owned()),
                        "arrivalAirportCode" => EngineValue::String("JFK".to_owned()),
                        "aircraftTailNumber" => EngineValue::String("D-AAAA".to_owned()),
                        "scheduledDeparture" => {
                            EngineValue::Timestamp("2026-03-28T09:15:00".to_owned())
                        }
                        "status" => EngineValue::String("scheduled".to_owned()),
                        "tags" => EngineValue::Array(Vec::new()),
                        _ => EngineValue::Null,
                    })
                    .collect()
            })
            .collect();
        Ok(QueryOutput {
            columns: Vec::new(),
            rows,
            truncated: false,
        })
    }

    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        panic!("aggregate TopN does not describe tables")
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

impl Engine for RecordingEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.sql.lock().unwrap().push(sql.as_str().to_owned());
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

#[allow(
    clippy::cast_precision_loss,
    reason = "the two-digit fixture sum is exactly representable"
)]
fn full_materialization_reference() -> AggregateResult {
    let csv =
        include_str!("../../../examples/ontologies/aviation/bindings/dev/fixtures/flights.csv");
    let mut rows = csv
        .lines()
        .skip(1)
        .map(|line| {
            let mut cells = line.split(',');
            let key = cells.next().unwrap();
            let delay = cells.nth(6).unwrap().trim().parse::<i64>().ok();
            (key, delay)
        })
        .collect::<BTreeMap<_, _>>();
    rows.insert("F-1001", Some(10));
    rows.remove("F-1002");
    let values = rows.values().copied().flatten().collect::<Vec<_>>();
    let count = i64::try_from(rows.len()).unwrap();
    let sum: i64 = values.iter().sum();
    AggregateResult {
        buckets: vec![AggregateBucket {
            group: vec![],
            values: vec![
                json!(count),
                json!(sum),
                json!(sum as f64 / values.len() as f64),
            ],
        }],
    }
}

#[test]
fn count_sum_avg_connector_capability_selects_supported_plan() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let ontology = load_dir(root).ontology.unwrap();
    let flight = name("Flight");
    let binding = &ontology
        .profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap()
        .objects[&flight];
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: ontology.object_types[&flight]
            .properties
            .keys()
            .cloned()
            .map(|property| (property, Pred::Const(true)))
            .collect(),
    };
    let request = AggregateRequest {
        set: ObjectSet::Type {
            object_type: flight,
        },
        group_by: vec![prop("status")],
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
    let plan = compile_aggregate_partial(
        &request,
        &ontology,
        binding,
        &scope,
        &Attrs::default(),
        AggregateCapabilities::all_supported(),
    )
    .unwrap()
    .unwrap();
    assert!(plan.from.is_some());
    assert_eq!(plan.group_by.len(), 1);
    assert_eq!(plan.projection.len(), 7);
    let sql =
        ontoforge_engine::printer::print_select(&plan, ontoforge_engine::printer::Dialect::Trino)
            .unwrap();
    assert_eq!(sql.as_str().matches(" FROM ").count(), 1);
    assert!(sql.as_str().contains("CASE WHEN"));
    let mut no_integer_sum = AggregateCapabilities::all_supported();
    no_integer_sum.sum_integer = false;
    assert!(
        compile_aggregate_partial(
            &request,
            &ontology,
            binding,
            &scope,
            &Attrs::default(),
            no_integer_sum
        )
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "integration fixture, edit setup and oracle are kept together"
)]
async fn count_sum_avg_match_full_materialization_reference() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = RecordingEngine {
        inner: SqliteEngine::from_fixtures(root, &fixtures.tables)
            .await
            .unwrap(),
        sql: Mutex::new(Vec::new()),
    };
    let dir = tempfile::tempdir().unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite {
            path: Utf8PathBuf::from_path_buf(dir.path().join("edits.db")).unwrap(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = store.active_definition().await.unwrap();
    let flight = name("Flight");
    let scope = PrincipalScope {
        types: std::iter::once((
            flight.clone(),
            TypeScope {
                row: Pred::Const(true),
                columns: ontology.object_types[&flight]
                    .properties
                    .keys()
                    .cloned()
                    .map(|property| (property, Pred::Const(true)))
                    .collect(),
            },
        ))
        .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "aggregate-reader"),
        issuer: "test".into(),
        subject: "aggregate-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "aggregateCorpus".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Object {
                    type_name: flight.clone(),
                    pk: PropertyValue::String("F-1001".to_owned()),
                    op: ObjectOp::Set(BTreeMap::from([(
                        prop("delayMinutes"),
                        PropertyValue::Int(10),
                    )])),
                },
                Edit::Object {
                    type_name: flight.clone(),
                    pk: PropertyValue::String("F-1002".to_owned()),
                    op: ObjectOp::Tombstone,
                },
            ],
        })
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"aggregate-test".to_vec()).unwrap();
    let service = QueryService {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let request = AggregateRequest {
        set: ObjectSet::Type {
            object_type: flight,
        },
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
    assert_eq!(
        service
            .execute_aggregate(
                request,
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default(),
            )
            .await
            .unwrap(),
        full_materialization_reference(),
    );
    let statements = engine.sql.lock().unwrap();
    assert!(
        statements.iter().any(|sql| sql.contains("sum(")),
        "{statements:?}"
    );
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "fixture and winner-changing edits exercise both directions"
)]
async fn min_max_edited_values_can_win_or_remove_winner() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = RecordingEngine {
        inner: SqliteEngine::from_fixtures(root, &fixtures.tables)
            .await
            .unwrap(),
        sql: Mutex::new(Vec::new()),
    };
    let dir = tempfile::tempdir().unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite {
            path: Utf8PathBuf::from_path_buf(dir.path().join("edits.db")).unwrap(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = store.active_definition().await.unwrap();
    let flight = name("Flight");
    let scope = PrincipalScope {
        types: std::iter::once((
            flight.clone(),
            TypeScope {
                row: Pred::Const(true),
                columns: ontology.object_types[&flight]
                    .properties
                    .keys()
                    .cloned()
                    .map(|property| (property, Pred::Const(true)))
                    .collect(),
            },
        ))
        .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "minmax-reader"),
        issuer: "test".into(),
        subject: "minmax-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let csv =
        include_str!("../../../examples/ontologies/aviation/bindings/dev/fixtures/flights.csv");
    let mut rows = csv
        .lines()
        .skip(1)
        .map(|line| {
            let mut cells = line.split(',');
            let key = cells.next().unwrap().to_owned();
            let delay = cells.nth(6).unwrap().trim().parse::<i64>().ok();
            (key, delay)
        })
        .collect::<BTreeMap<_, _>>();
    let source_max = rows
        .iter()
        .filter_map(|(key, value)| value.map(|v| (key.clone(), v)))
        .max_by_key(|(_, value)| *value)
        .unwrap()
        .0;
    assert_ne!(source_max, "F-1001");
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "minmaxCorpus".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Object {
                    type_name: flight.clone(),
                    pk: PropertyValue::String("F-1001".to_owned()),
                    op: ObjectOp::Set(BTreeMap::from([(
                        prop("delayMinutes"),
                        PropertyValue::Int(-10),
                    )])),
                },
                Edit::Object {
                    type_name: flight.clone(),
                    pk: PropertyValue::String(source_max.clone()),
                    op: ObjectOp::Tombstone,
                },
            ],
        })
        .await
        .unwrap();
    rows.insert("F-1001".to_owned(), Some(-10));
    rows.remove(&source_max);
    let expected_min = rows.values().copied().flatten().min().unwrap();
    let expected_max = rows.values().copied().flatten().max().unwrap();
    let tokens = TokenCodec::new(b"minmax-test".to_vec()).unwrap();
    let service = QueryService {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let result = service
        .execute_aggregate(
            AggregateRequest {
                set: ObjectSet::Type {
                    object_type: flight,
                },
                group_by: vec![],
                measures: vec![
                    AggregateMeasure::Min {
                        property: prop("delayMinutes"),
                    },
                    AggregateMeasure::Max {
                        property: prop("delayMinutes"),
                    },
                ],
            },
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(
        result.buckets[0].values,
        vec![json!(expected_min), json!(expected_max)]
    );
    let statements = engine.sql.lock().unwrap();
    assert!(
        statements
            .iter()
            .any(|sql| sql.contains("ORDER BY") && sql.contains("LIMIT"))
    );
    assert!(
        statements
            .iter()
            .all(|sql| !sql.contains("min(") && !sql.contains("max("))
    );
}

#[tokio::test]
async fn min_max_refill_past_invalid_top_rows() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let ontology = load_dir(root).ontology.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite {
            path: Utf8PathBuf::from_path_buf(dir.path().join("edits.db")).unwrap(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = store.active_definition().await.unwrap();
    let flight = name("Flight");
    let visible_key = "F-0768";
    let type_scope = TypeScope {
        row: Pred::Cmp {
            left: Operand::Column(prop("flightId")),
            op: CmpOp::Eq,
            right: Operand::Lit(Scalar::String(visible_key.to_owned())),
        },
        columns: ontology.object_types[&flight]
            .properties
            .keys()
            .cloned()
            .map(|property| (property, Pred::Const(true)))
            .collect(),
    };
    let scope = PrincipalScope {
        types: std::iter::once((flight.clone(), type_scope)).collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "topn-reader"),
        issuer: "test".into(),
        subject: "topn-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let engine = ScriptedTopNEngine {
        columns: ontology.object_types[&flight]
            .properties
            .keys()
            .cloned()
            .collect(),
        calls: AtomicUsize::new(0),
    };
    let tokens = TokenCodec::new(b"topn-test".to_vec()).unwrap();
    let request = AggregateRequest {
        set: ObjectSet::Type {
            object_type: flight,
        },
        group_by: vec![],
        measures: vec![AggregateMeasure::Min {
            property: prop("delayMinutes"),
        }],
    };
    let service = QueryService {
        principal: &principal,
        scope: &scope,
        active: active.clone(),
        engine: &engine,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let result = service
        .execute_aggregate(
            request.clone(),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(result.buckets[0].values, vec![json!(768)]);
    assert_eq!(engine.calls.load(AtomicOrdering::SeqCst), 4);

    let limited_engine = ScriptedTopNEngine {
        columns: engine.columns.clone(),
        calls: AtomicUsize::new(0),
    };
    let limited = QueryService {
        engine: &limited_engine,
        limits: ExecutionLimits::try_new(4, 10_000, 2_048, 64 * 1024 * 1024).unwrap(),
        ..service
    };
    assert!(matches!(
        limited
            .execute_aggregate(
                request,
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default()
            )
            .await,
        Err(ontoforge_query::aggregate::AggregateError::AggregateCostExceeded)
    ));
}

#[test]
fn count_sum_avg_limits_phase4_defaults_are_exact() {
    let limits = AggregateLimits::phase4_default();
    assert_eq!(limits.max_groups, 1_024);
    assert_eq!(limits.max_buckets, 4_096);
    assert_eq!(limits.max_distinct_values, 10_000);
    assert_eq!(limits.max_distinct_memory_bytes, 16 * 1024 * 1024);
    assert!(AggregateLimits::try_new(0, 1, 1, 1).is_err());
    assert!(AggregateLimits::try_new(1, 0, 1, 1).is_err());
    assert!(AggregateLimits::try_new(1, 1, 0, 1).is_err());
    assert!(AggregateLimits::try_new(1, 1, 1, 0).is_err());
    assert!(AggregateLimits::try_new(usize::MAX, 1, 1, 1).is_err());
    let injected = json!({
        "set": {"kind": "type", "object_type": "Flight"},
        "groupBy": [],
        "measures": [{"op": "count", "property": null}],
        "maxGroups": 999_999
    });
    assert!(serde_json::from_value::<AggregateRequest>(injected).is_err());
}
