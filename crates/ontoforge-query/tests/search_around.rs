use std::collections::BTreeSet;

use ontoforge_query::link::{LinkEdge, merge_edges};
use ontoforge_storage::query_read::LinkEdit;
use ontoforge_storage::state::LinkRow;
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;

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
        row: LinkRow { deleted, version: 1 },
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
            (PropertyValue::Int(from), PropertyValue::Int(to)) => (*from as u8, *to as u8),
            _ => panic!("unexpected endpoint type"),
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, reference(&source, &edits));
}

#[test]
fn duplicate_edits_use_highest_version_independent_of_input_order() {
    let removed = LinkEdit {
        row: LinkRow { deleted: true, version: 2 },
        ..edit(1, 2, true)
    };
    let restored = LinkEdit {
        row: LinkRow { deleted: false, version: 3 },
        ..edit(1, 2, false)
    };
    for edits in [vec![removed.clone(), restored.clone()], vec![restored.clone(), removed.clone()]] {
        assert_eq!(merge_edges(vec![edge(1, 2)], &edits).unwrap(), vec![edge(1, 2)]);
    }
    let conflict = LinkEdit {
        row: LinkRow { deleted: true, version: 3 },
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
            (PropertyValue::Int(from), PropertyValue::Int(to)) => (*from as u8, *to as u8),
            _ => panic!("unexpected endpoint type"),
        }).collect::<BTreeSet<_>>();
        prop_assert_eq!(actual, reference(&source, &logical_edits));
    }
}
