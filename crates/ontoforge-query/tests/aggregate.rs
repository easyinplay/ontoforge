use ontoforge_query::aggregate::AggregateLimits;
use ontoforge_query::ast::AggregateRequest;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_auth::Principal;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::aggregate::{AggregateBucket, AggregateCapabilities, AggregateResult};
use ontoforge_query::ast::{AggregateMeasure, ObjectSet, QueryLiteral, WhereClause};
use ontoforge_query::compile::compile_aggregate_partial;
use ontoforge_query::{ExecutionLimits, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{reload, ReloadOptions};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;

fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}

fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

fn full_materialization_reference() -> AggregateResult {
    let mut rows = BTreeMap::from([("F-1001", 0_i64), ("F-1002", 25_i64)]);
    rows.insert("F-1001", 10);
    rows.remove("F-1002");
    let values = rows.values().copied().collect::<Vec<_>>();
    let count = i64::try_from(values.len()).unwrap();
    let sum: i64 = values.iter().sum();
    AggregateResult {
        buckets: vec![AggregateBucket {
            group: vec![],
            values: vec![json!(count), json!(sum), json!(sum as f64 / count as f64)],
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
    let binding = &ontology.profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap().objects[&flight];
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: ontology.object_types[&flight].properties.keys()
            .cloned().map(|property| (property, Pred::Const(true))).collect(),
    };
    let request = AggregateRequest {
        set: ObjectSet::Type { object_type: flight },
        group_by: vec![prop("status")],
        measures: vec![
            AggregateMeasure::Count { property: None },
            AggregateMeasure::Sum { property: prop("delayMinutes") },
            AggregateMeasure::Avg { property: prop("delayMinutes") },
        ],
    };
    let plan = compile_aggregate_partial(&request, &ontology, binding, &scope,
        &Attrs::default(), AggregateCapabilities::all_supported()).unwrap().unwrap();
    assert!(plan.from.is_some());
    assert_eq!(plan.group_by.len(), 1);
    assert_eq!(plan.projection.len(), 6);
    let sql = ontoforge_engine::printer::print_select(&plan,
        ontoforge_engine::printer::Dialect::Trino).unwrap();
    assert_eq!(sql.as_str().matches(" FROM ").count(), 1);
    assert!(sql.as_str().contains("CASE WHEN"));
    let mut no_integer_sum = AggregateCapabilities::all_supported();
    no_integer_sum.sum_integer = false;
    assert!(compile_aggregate_partial(&request, &ontology, binding, &scope,
        &Attrs::default(), no_integer_sum).unwrap().is_none());
}

#[tokio::test]
async fn count_sum_avg_match_full_materialization_reference() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixtures.tables).await.unwrap();
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
    reload(&store, &ontology, &ReloadOptions::default()).await.unwrap();
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
    store.commit(Commit {
        actor: Actor::Principal(principal.id.to_string()),
        action: "aggregateCorpus".to_owned(),
        def_version: active.def_version,
        params: json!({}),
        edits: vec![
            Edit::Object {
                type_name: flight.clone(),
                pk: PropertyValue::String("F-1001".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(prop("delayMinutes"), PropertyValue::Int(10))])),
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
        set: ObjectSet::Where {
            object_type: flight,
            predicate: WhereClause::In {
                property: prop("flightId"),
                values: vec![QueryLiteral(json!("F-1001")), QueryLiteral(json!("F-1002"))],
            },
        },
        group_by: vec![],
        measures: vec![
            AggregateMeasure::Count { property: None },
            AggregateMeasure::Sum { property: prop("delayMinutes") },
            AggregateMeasure::Avg { property: prop("delayMinutes") },
        ],
    };
    assert_eq!(
        service.execute_aggregate(
            request,
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
        ).await.unwrap(),
        full_materialization_reference(),
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
        "maxGroups": 999999
    });
    assert!(serde_json::from_value::<AggregateRequest>(injected).is_err());
}
