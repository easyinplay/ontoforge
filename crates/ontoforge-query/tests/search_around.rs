use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::ast::ObjectSet;
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::link::{LinkEdge, merge_edges};
use ontoforge_query::{ExecutionLimits, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, LinkOp, ObjectOp};
use ontoforge_storage::query_read::LinkEdit;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::state::LinkRow;
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn search_around_wire_uses_only_logical_names() {
    let wire = json!({
        "kind": "searchAround", "object_type": "Airport",
        "base": {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
        "link": "flightDepartureAirport", "direction": "toFrom"
    });
    let parsed: ObjectSet = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "one fixture exercises FK, M:N edits, scope and budget"
)]
async fn outbound_fk_uses_scoped_merged_object_value() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixtures.tables)
        .await
        .unwrap();
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
    let name = |raw: &str| ObjectTypeName::try_from(raw.to_owned()).unwrap();
    let prop = |raw: &str| PropertyName::try_from(raw.to_owned()).unwrap();
    store
        .commit(Commit {
            actor: Actor::Principal("test|reader".to_owned()),
            action: "airportFixture".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Airport"),
                pk: PropertyValue::String("FRA".to_owned()),
                op: ObjectOp::Set(
                    [(prop("name"), PropertyValue::String("Frankfurt".to_owned()))].into(),
                ),
            }],
        })
        .await
        .unwrap();
    let scope = PrincipalScope {
        types: ["Flight", "Airport", "Pilot"]
            .into_iter()
            .map(|raw| {
                let ty = name(raw);
                (
                    ty.clone(),
                    TypeScope {
                        row: Pred::Const(true),
                        columns: ontology.object_types[&ty]
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
        id: PrincipalId::new("test", "reader"),
        issuer: "test".into(),
        subject: "reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let tokens = TokenCodec::new(b"link-test".to_vec()).unwrap();
    let service = QueryService {
        principal: &principal,
        scope: &scope,
        active: Arc::clone(&active),
        engine: &engine,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let request = QueryRequest {
        set: serde_json::from_value(json!({
            "kind": "searchAround", "object_type": "Airport",
            "base": {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
            "link": "flightDepartureAirport", "direction": "toFrom"
        }))
        .unwrap(),
        select: vec![prop("code")],
    };
    let query = ValidatedQuery::new(
        request,
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&name("Airport")],
        CompileLimits::default(),
    )
    .unwrap();
    let page = service.execute_list(&query).await.unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row.pk.clone())
            .collect::<Vec<_>>(),
        vec![json!("FRA")]
    );

    let crew_request = QueryRequest {
        set: serde_json::from_value(json!({
            "kind": "searchAround", "object_type": "Pilot",
            "base": {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
            "link": "flightCrew", "direction": "fromTo"
        }))
        .unwrap(),
        select: vec![prop("employeeId")],
    };
    let crew_query = ValidatedQuery::new(
        crew_request.clone(),
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&name("Pilot")],
        CompileLimits::default(),
    )
    .unwrap();
    let crew = service.execute_list(&crew_query).await.unwrap();
    assert_eq!(
        crew.rows
            .iter()
            .map(|row| row.pk.clone())
            .collect::<Vec<_>>(),
        vec![json!("E-001"), json!("E-002")]
    );

    let combined_base = ValidatedQuery::new(QueryRequest {
        set: serde_json::from_value(json!({
            "kind": "searchAround", "object_type": "Pilot",
            "base": {"kind": "union", "object_type": "Flight", "sets": [
                {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
                {"kind": "pk", "object_type": "Flight", "pk": "F-1002"}
            ]},
            "link": "flightCrew", "direction": "fromTo"
        })).unwrap(), select: vec![prop("employeeId")],
    }, vec![], PageSpec { first: 10, after: None },
    &ontology, &scope.types[&name("Pilot")], CompileLimits::default()).unwrap();
    assert_eq!(service.execute_list(&combined_base).await.unwrap().rows.iter().map(|row| row.pk.clone()).collect::<Vec<_>>(),
        vec![json!("E-001"), json!("E-002"), json!("E-003")]);

    let first_crew = ValidatedQuery::new(
        crew_request.clone(),
        vec![],
        PageSpec {
            first: 1,
            after: None,
        },
        &ontology,
        &scope.types[&name("Pilot")],
        CompileLimits::default(),
    )
    .unwrap();
    let first_crew = service.execute_list(&first_crew).await.unwrap();
    assert_eq!(first_crew.rows[0].pk, json!("E-001"));
    assert!(first_crew.has_more);
    let second_crew = ValidatedQuery::new(
        crew_request.clone(),
        vec![],
        PageSpec {
            first: 1,
            after: first_crew.cursor,
        },
        &ontology,
        &scope.types[&name("Pilot")],
        CompileLimits::default(),
    )
    .unwrap();
    let second_crew = service.execute_list(&second_crew).await.unwrap();
    assert_eq!(second_crew.rows[0].pk, json!("E-002"));
    assert!(!second_crew.has_more);

    let reverse_crew = ValidatedQuery::new(
        QueryRequest {
            set: serde_json::from_value(json!({
                "kind": "searchAround", "object_type": "Flight",
                "base": {"kind": "pk", "object_type": "Pilot", "pk": "E-001"},
                "link": "flightCrew", "direction": "toFrom"
            }))
            .unwrap(),
            select: vec![prop("flightId")],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&name("Flight")],
        CompileLimits::default(),
    )
    .unwrap();
    assert_eq!(
        service.execute_list(&reverse_crew).await.unwrap().rows[0].pk,
        json!("F-1001")
    );

    let inbound = ValidatedQuery::new(
        QueryRequest {
            set: serde_json::from_value(json!({
                "kind": "searchAround", "object_type": "Flight",
                "base": {"kind": "pk", "object_type": "Airport", "pk": "FRA"},
                "link": "flightDepartureAirport", "direction": "fromTo"
            }))
            .unwrap(),
            select: vec![prop("flightId")],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&name("Flight")],
        CompileLimits::default(),
    )
    .unwrap();
    assert_eq!(
        service
            .execute_list(&inbound)
            .await
            .unwrap()
            .rows
            .iter()
            .map(|row| row.pk.clone())
            .collect::<Vec<_>>(),
        vec![json!("F-1001"), json!("F-2010"), json!("F-2020")]
    );

    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "crewEdit".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Link {
                    link: ontoforge_schema::ir::LinkTypeName::try_from("flightCrew".to_owned())
                        .unwrap(),
                    a: PropertyValue::String("F-1001".to_owned()),
                    b: PropertyValue::String("E-001".to_owned()),
                    op: LinkOp::Remove,
                },
                Edit::Link {
                    link: ontoforge_schema::ir::LinkTypeName::try_from("flightCrew".to_owned())
                        .unwrap(),
                    a: PropertyValue::String("F-1001".to_owned()),
                    b: PropertyValue::String("E-003".to_owned()),
                    op: LinkOp::Add,
                },
            ],
        })
        .await
        .unwrap();
    let edited_crew = service.execute_list(&crew_query).await.unwrap();
    assert_eq!(
        edited_crew
            .rows
            .iter()
            .map(|row| row.pk.clone())
            .collect::<Vec<_>>(),
        vec![json!("E-002"), json!("E-003")]
    );

    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "pilotTombstone".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Pilot"),
                pk: PropertyValue::String("E-003".to_owned()),
                op: ObjectOp::Tombstone,
            }],
        })
        .await
        .unwrap();
    assert_eq!(
        service
            .execute_list(&crew_query)
            .await
            .unwrap()
            .rows
            .iter()
            .map(|row| row.pk.clone())
            .collect::<Vec<_>>(),
        vec![json!("E-002")]
    );

    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "moveDeparture".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![
                Edit::Object {
                    type_name: name("Airport"),
                    pk: PropertyValue::String("MUC".to_owned()),
                    op: ObjectOp::Set(
                        [(prop("name"), PropertyValue::String("Munich".to_owned()))].into(),
                    ),
                },
                Edit::Object {
                    type_name: name("Flight"),
                    pk: PropertyValue::String("F-1001".to_owned()),
                    op: ObjectOp::Set(
                        [(
                            prop("departureAirportCode"),
                            PropertyValue::String("MUC".to_owned()),
                        )]
                        .into(),
                    ),
                },
            ],
        })
        .await
        .unwrap();
    assert_eq!(
        service.execute_list(&query).await.unwrap().rows[0].pk,
        json!("MUC")
    );
    assert!(
        !service
            .execute_list(&inbound)
            .await
            .unwrap()
            .rows
            .iter()
            .any(|row| row.pk == json!("F-1001"))
    );
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "releaseDeparture".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Flight"),
                pk: PropertyValue::String("F-1001".to_owned()),
                op: ObjectOp::Release,
            }],
        })
        .await
        .unwrap();
    assert_eq!(
        service.execute_list(&query).await.unwrap().rows[0].pk,
        json!("FRA")
    );

    let mut hidden_fk_scope = scope.clone();
    hidden_fk_scope
        .types
        .get_mut(&name("Flight"))
        .unwrap()
        .columns
        .insert(prop("departureAirportCode"), Pred::Const(false));
    let hidden_fk = QueryService {
        scope: &hidden_fk_scope,
        active: Arc::clone(&active),
        ..service
    };
    assert!(
        hidden_fk
            .execute_list(&query)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(
        hidden_fk
            .execute_list(&inbound)
            .await
            .unwrap()
            .rows
            .is_empty()
    );

    let mut hidden_base_scope = scope.clone();
    hidden_base_scope
        .types
        .get_mut(&name("Flight"))
        .unwrap()
        .row = Pred::Const(false);
    let hidden_base = QueryService {
        scope: &hidden_base_scope,
        active: Arc::clone(&active),
        ..hidden_fk
    };
    assert!(
        hidden_base
            .execute_list(&crew_query)
            .await
            .unwrap()
            .rows
            .is_empty()
    );

    let mut hidden_scope = scope.clone();
    hidden_scope.types.get_mut(&name("Pilot")).unwrap().row = Pred::Const(false);
    let hidden = QueryService {
        scope: &hidden_scope,
        active: Arc::clone(&active),
        ..hidden_base
    };
    assert!(
        hidden
            .execute_list(&crew_query)
            .await
            .unwrap()
            .rows
            .is_empty()
    );

    let limited = QueryService {
        scope: &scope,
        limits: ExecutionLimits::try_new(1, 10_000, 2_048, 64 * 1024 * 1024).unwrap(),
        active: Arc::clone(&active),
        ..hidden
    };
    assert!(matches!(
        limited.execute_list(&crew_query).await,
        Err(ontoforge_query::QueryError::WorkBudgetExceeded)
    ));
}

fn edge(from: u8, to: u8) -> LinkEdge {
    LinkEdge {
        from: PropertyValue::Int(i64::from(from)),
        to: PropertyValue::Int(i64::from(to)),
    }
}

fn edit(from: u8, to: u8, deleted: bool) -> LinkEdit {
    LinkEdit {
        from: PropertyValue::Int(i64::from(from)),
        to: PropertyValue::Int(i64::from(to)),
        row: LinkRow {
            deleted,
            version: 1,
        },
    }
}

fn reference(source: &[(u8, u8)], edits: &[(u8, u8, bool)]) -> BTreeSet<(u8, u8)> {
    let mut live = source.iter().copied().collect::<BTreeSet<_>>();
    for &(from, to, deleted) in edits {
        if deleted {
            live.remove(&(from, to));
        } else {
            live.insert((from, to));
        }
    }
    live
}

#[test]
fn many_to_many_merges_source_and_link_edits_fixed() {
    let source = vec![(1, 2), (1, 2), (1, 3)];
    let edits = vec![(1, 2, true), (1, 4, false)];
    let merged = merge_edges(
        source.iter().map(|&(from, to)| edge(from, to)).collect(),
        &edits
            .iter()
            .map(|&(from, to, deleted)| edit(from, to, deleted))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let actual = merged
        .iter()
        .map(|edge| match (&edge.from, &edge.to) {
            (PropertyValue::Int(from), PropertyValue::Int(to)) => {
                (u8::try_from(*from).unwrap(), u8::try_from(*to).unwrap())
            }
            _ => panic!("unexpected endpoint type"),
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, reference(&source, &edits));
}

#[test]
fn duplicate_edits_use_highest_version_independent_of_input_order() {
    let removed = LinkEdit {
        row: LinkRow {
            deleted: true,
            version: 2,
        },
        ..edit(1, 2, true)
    };
    let restored = LinkEdit {
        row: LinkRow {
            deleted: false,
            version: 3,
        },
        ..edit(1, 2, false)
    };
    for edits in [
        vec![removed.clone(), restored.clone()],
        vec![restored.clone(), removed.clone()],
    ] {
        assert_eq!(
            merge_edges(vec![edge(1, 2)], &edits).unwrap(),
            vec![edge(1, 2)]
        );
    }
    let conflict = LinkEdit {
        row: LinkRow {
            deleted: true,
            version: 3,
        },
        ..removed
    };
    assert!(merge_edges(vec![], &[restored, conflict]).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn edge_overlay_matches_independent_reference(
        source in prop::collection::vec((0_u8..8, 0_u8..8), 0..32),
        edits in prop::collection::btree_map((0_u8..8, 0_u8..8), any::<bool>(), 0..16),
    ) {
        let logical_edits = edits.into_iter().map(|((from, to), deleted)| (from, to, deleted)).collect::<Vec<_>>();
        let merged = merge_edges(
            source.iter().map(|&(from, to)| edge(from, to)).collect(),
            &logical_edits.iter().map(|&(from, to, deleted)| edit(from, to, deleted)).collect::<Vec<_>>(),
        ).unwrap();
        let actual = merged.iter().map(|edge| match (&edge.from, &edge.to) {
            (PropertyValue::Int(from), PropertyValue::Int(to)) => {
                (u8::try_from(*from).unwrap(), u8::try_from(*to).unwrap())
            }
            _ => panic!("unexpected endpoint type"),
        }).collect::<BTreeSet<_>>();
        prop_assert_eq!(actual, reference(&source, &logical_edits));
    }
}
