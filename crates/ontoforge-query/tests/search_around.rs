use std::collections::BTreeSet;

use ontoforge_query::link::{LinkEdge, merge_edges};
use ontoforge_storage::query_read::LinkEdit;
use ontoforge_storage::state::LinkRow;
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use ontoforge_query::ast::ObjectSet;
use serde_json::json;
use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ExecutionLimits, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
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
async fn outbound_fk_uses_scoped_merged_object_value() {
    let root = Utf8Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation"));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixtures.tables).await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: Utf8PathBuf::from_path_buf(dir.path().join("edits.db")).unwrap() },
        lock_timeout: Duration::from_secs(5), probe_timeout: Duration::from_secs(5), trino_catalog: None,
    }).await.unwrap();
    reload(&store, &ontology, &ReloadOptions::default()).await.unwrap();
    let active = store.active_definition().await.unwrap();
    let name = |raw: &str| ObjectTypeName::try_from(raw.to_owned()).unwrap();
    let prop = |raw: &str| PropertyName::try_from(raw.to_owned()).unwrap();
    store.commit(Commit {
        actor: Actor::Principal("test|reader".to_owned()),
        action: "airportFixture".to_owned(), def_version: active.def_version,
        params: json!({}),
        edits: vec![Edit::Object {
            type_name: name("Airport"), pk: PropertyValue::String("FRA".to_owned()),
            op: ObjectOp::Set([(prop("name"), PropertyValue::String("Frankfurt".to_owned()))].into()),
        }],
    }).await.unwrap();
    let scope = PrincipalScope {
        types: ["Flight", "Airport"].into_iter().map(|raw| {
            let ty = name(raw);
            (ty.clone(), TypeScope {
                row: Pred::Const(true),
                columns: ontology.object_types[&ty].properties.keys().cloned().map(|p| (p, Pred::Const(true))).collect(),
            })
        }).collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "reader"), issuer: "test".into(), subject: "reader".into(),
        groups: BTreeSet::new(), attrs: Attrs::default(),
    };
    let tokens = TokenCodec::new(b"link-test".to_vec()).unwrap();
    let service = QueryService {
        principal: &principal, scope: &scope, active: Arc::clone(&active), engine: &engine,
        store: &store, limits: ExecutionLimits::phase4_default(), tokens: &tokens,
    };
    let request = QueryRequest {
        set: serde_json::from_value(json!({
            "kind": "searchAround", "object_type": "Airport",
            "base": {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
            "link": "flightDepartureAirport", "direction": "toFrom"
        })).unwrap(),
        select: vec![prop("code")],
    };
    let query = ValidatedQuery::new(request, vec![], PageSpec { first: 10, after: None },
        &ontology, &scope.types[&name("Airport")], CompileLimits::default()).unwrap();
    let page = service.execute_list(&query).await.unwrap();
    assert_eq!(page.rows.iter().map(|row| row.pk.clone()).collect::<Vec<_>>(), vec![json!("FRA")]);
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
