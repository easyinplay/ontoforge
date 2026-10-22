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
    nonfinite: bool,
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
                        "delayMinutes" if self.nonfinite => EngineValue::Double(f64::NAN),
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
                request.clone(),
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default(),
            )
            .await
            .unwrap(),
        full_materialization_reference(),
    );
    let mut fallback_capabilities = AggregateCapabilities::all_supported();
    fallback_capabilities.sum_integer = false;
    assert_eq!(
        service
            .execute_aggregate(
                request,
                fallback_capabilities,
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
                    object_type: flight.clone(),
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
    let mut fallback_capabilities = AggregateCapabilities::all_supported();
    fallback_capabilities.min_max = false;
    let fallback = service
        .execute_aggregate(
            AggregateRequest {
                set: ObjectSet::Type {
                    object_type: flight.clone(),
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
            fallback_capabilities,
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(fallback, result);
    {
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
    let mut masked_scope = scope.clone();
    masked_scope.types.get_mut(&flight).unwrap().columns.insert(
        prop("delayMinutes"),
        Pred::Cmp {
            left: Operand::Column(prop("flightId")),
            op: CmpOp::Ne,
            right: Operand::Lit(Scalar::String("F-1001".to_owned())),
        },
    );
    let masked_service = QueryService {
        scope: &masked_scope,
        ..service
    };
    let minmax = |flight: ObjectTypeName| AggregateRequest {
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
    };
    let visible_min = rows
        .iter()
        .filter(|(key, _)| key.as_str() != "F-1001")
        .filter_map(|(_, value)| *value)
        .min()
        .unwrap();
    let masked = masked_service
        .execute_aggregate(
            minmax(flight.clone()),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(
        masked.buckets[0].values,
        vec![json!(visible_min), json!(expected_max)]
    );
    let mut all_hidden_scope = masked_scope.clone();
    all_hidden_scope
        .types
        .get_mut(&flight)
        .unwrap()
        .columns
        .insert(prop("delayMinutes"), Pred::Const(false));
    let all_hidden_service = QueryService {
        scope: &all_hidden_scope,
        ..masked_service
    };
    let all_hidden = all_hidden_service
        .execute_aggregate(
            minmax(flight),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(all_hidden.buckets[0].values, vec![json!(null), json!(null)]);
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "scripted windows and budget cases share one store fixture"
)]
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
        nonfinite: false,
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
        nonfinite: false,
    };
    let limited = QueryService {
        engine: &limited_engine,
        limits: ExecutionLimits::try_new(4, 10_000, 2_048, 64 * 1024 * 1024).unwrap(),
        ..service
    };
    assert!(matches!(
        limited
            .execute_aggregate(
                request.clone(),
                AggregateCapabilities::all_supported(),
                AggregateLimits::phase4_default()
            )
            .await,
        Err(ontoforge_query::aggregate::AggregateError::AggregateCostExceeded)
    ));
    let nonfinite_engine = ScriptedTopNEngine {
        columns: engine.columns.clone(),
        calls: AtomicUsize::new(0),
        nonfinite: true,
    };
    let nonfinite_service = QueryService {
        engine: &nonfinite_engine,
        limits: ExecutionLimits::phase4_default(),
        ..limited
    };
    let nonfinite_error = nonfinite_service
        .execute_aggregate(
            request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(
            nonfinite_error,
            ontoforge_query::aggregate::AggregateError::UnsupportedNumericValue
        ),
        "{nonfinite_error:?}"
    );
}

#[tokio::test]
#[allow(
    clippy::cast_precision_loss,
    reason = "the CSV fixture has 23 small integer rows"
)]
#[allow(
    clippy::too_many_lines,
    reason = "independent CSV oracle, edits and authorization share one fixture"
)]
async fn group_distinct_matches_visible_merged_reference() {
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
    let mut columns = ontology.object_types[&flight]
        .properties
        .keys()
        .cloned()
        .map(|property| (property, Pred::Const(true)))
        .collect::<BTreeMap<_, _>>();
    let visible_except = |key: &str| Pred::Cmp {
        left: Operand::Column(prop("flightId")),
        op: CmpOp::Ne,
        right: Operand::Lit(Scalar::String(key.to_owned())),
    };
    columns.insert(prop("status"), visible_except("F-1003"));
    columns.insert(prop("delayMinutes"), visible_except("F-1004"));
    let scope = PrincipalScope {
        types: std::iter::once((
            flight.clone(),
            TypeScope {
                row: Pred::Const(true),
                columns,
            },
        ))
        .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "group-reader"),
        issuer: "test".into(),
        subject: "group-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "groupCorpus".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Object {
                    type_name: flight.clone(),
                    pk: PropertyValue::String("F-1001".to_owned()),
                    op: ObjectOp::Set(BTreeMap::from([
                        (prop("status"), PropertyValue::String("delayed".to_owned())),
                        (prop("delayMinutes"), PropertyValue::Int(10)),
                    ])),
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
    let csv =
        include_str!("../../../examples/ontologies/aviation/bindings/dev/fixtures/flights.csv");
    let mut rows = csv
        .lines()
        .skip(1)
        .map(|line| {
            let mut cells = line.split(',');
            let key = cells.next().unwrap().to_owned();
            let status = cells.nth(5).unwrap().to_owned();
            let delay = cells.next().unwrap().parse::<i64>().ok();
            (key, (status, delay))
        })
        .collect::<BTreeMap<_, _>>();
    rows.insert("F-1001".to_owned(), ("delayed".to_owned(), Some(10)));
    rows.remove("F-1002");
    let mut oracle = BTreeMap::<String, (Vec<serde_json::Value>, u64, BTreeSet<i64>, u64)>::new();
    for (key, (status, delay)) in rows.clone() {
        let group = vec![if key == "F-1003" || status.is_empty() {
            json!(null)
        } else {
            json!(status)
        }];
        let group_key = serde_json::to_string(&group).unwrap();
        let entry = oracle
            .entry(group_key)
            .or_insert_with(|| (group, 0, BTreeSet::new(), 0));
        entry.1 += 1;
        if key != "F-1004"
            && let Some(value) = delay
        {
            entry.2.insert(value);
            entry.3 += 1;
        }
    }
    let expected = AggregateResult {
        buckets: oracle
            .into_values()
            .map(|(group, rows, distinct, non_null)| AggregateBucket {
                group,
                values: vec![json!(rows), json!(distinct.len()), json!(non_null)],
            })
            .collect(),
    };
    let tokens = TokenCodec::new(b"group-test".to_vec()).unwrap();
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
            object_type: flight.clone(),
        },
        group_by: vec![prop("status")],
        measures: vec![
            AggregateMeasure::Count { property: None },
            AggregateMeasure::Distinct {
                property: prop("delayMinutes"),
            },
            AggregateMeasure::Count {
                property: Some(prop("delayMinutes")),
            },
        ],
    };
    let result = service
        .execute_aggregate(
            request.clone(),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(result, expected);
    let mut partial_oracle = BTreeMap::<String, (Vec<serde_json::Value>, u64, i64, u64)>::new();
    for (key, (status, delay)) in &rows {
        let group = vec![if key == "F-1003" || status.is_empty() {
            json!(null)
        } else {
            json!(status)
        }];
        let entry = partial_oracle
            .entry(serde_json::to_string(&group).unwrap())
            .or_insert_with(|| (group, 0, 0, 0));
        entry.1 += 1;
        if key != "F-1004"
            && let Some(value) = delay
        {
            entry.2 += value;
            entry.3 += 1;
        }
    }
    let partial_expected = AggregateResult {
        buckets: partial_oracle
            .into_values()
            .map(|(group, count, sum, non_null)| AggregateBucket {
                group,
                values: vec![
                    json!(count),
                    if non_null == 0 {
                        json!(null)
                    } else {
                        json!(sum)
                    },
                    if non_null == 0 {
                        json!(null)
                    } else {
                        json!(sum as f64 / non_null as f64)
                    },
                ],
            })
            .collect(),
    };
    let partial_request = AggregateRequest {
        set: ObjectSet::Type {
            object_type: flight.clone(),
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
    let partial_result = service
        .execute_aggregate(
            partial_request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(partial_result, partial_expected);
    assert!(
        engine
            .sql
            .lock()
            .unwrap()
            .iter()
            .any(|sql| sql.contains("sum(") && sql.contains("GROUP BY"))
    );
    let union_result = service
        .execute_aggregate(
            AggregateRequest {
                set: ObjectSet::Union {
                    object_type: flight.clone(),
                    sets: vec![
                        ObjectSet::Pk {
                            object_type: flight.clone(),
                            pk: json!("F-1001"),
                        },
                        ObjectSet::Pk {
                            object_type: flight.clone(),
                            pk: json!("F-1003"),
                        },
                    ],
                },
                group_by: vec![],
                measures: vec![
                    AggregateMeasure::Count { property: None },
                    AggregateMeasure::Distinct { property: prop("delayMinutes") },
                ],
            },
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(union_result.buckets[0].values, vec![json!(2), json!(2)]);
    let limited = service
        .execute_aggregate(
            request.clone(),
            AggregateCapabilities::all_supported(),
            AggregateLimits::try_new(1, 4, 100, 1_000).unwrap(),
        )
        .await;
    assert!(matches!(
        limited,
        Err(ontoforge_query::aggregate::AggregateError::AggregateCostExceeded)
    ));

    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "hiddenValueMutation".to_owned(),
            def_version: service.active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: flight,
                pk: PropertyValue::String("F-1003".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    prop("status"),
                    PropertyValue::String("unobservable".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let mutated = service
        .execute_aggregate(
            request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(mutated).unwrap()
    );
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
