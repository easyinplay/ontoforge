use std::collections::BTreeMap;

use ontoforge_query::overlay::overlay_object;
use ontoforge_schema::ir::PropertyName;
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::state::EditRow;
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use serde_json::{Value, json};

fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

// This model deliberately does not call the production merge, filter, or sort helpers.
fn reference(
    source: Option<BTreeMap<PropertyName, Value>>,
    edit: Option<&ObjectEdit>,
    pk: i64,
) -> Option<BTreeMap<PropertyName, Value>> {
    if edit.is_some_and(|edit| edit.row.deleted) {
        return None;
    }
    let mut row = source?;
    row.insert(prop("flightId"), json!(pk));
    if let Some(edit) = edit.filter(|edit| !edit.row.released) {
        for (property, value) in &edit.row.set {
            row.insert(property.clone(), value.to_json());
        }
    }
    Some(row)
}

#[test]
fn overlay_partial_patch_and_explicit_null() {
    let source = BTreeMap::from([
        (prop("flightId"), json!(7)),
        (prop("status"), json!("old")),
        (prop("delayMinutes"), json!(12)),
    ]);
    let edit = ObjectEdit {
        pk: PropertyValue::Int(7),
        row: EditRow {
            set: BTreeMap::from([
                (prop("status"), PropertyValue::String("new".into())),
                (prop("delayMinutes"), PropertyValue::Null),
            ]),
            deleted: false,
            released: false,
            version: 1,
        },
    };
    let actual = overlay_object(Some(&source), Some(&edit), &prop("flightId"), &edit.pk).unwrap();
    let expected = reference(Some(source), Some(&edit), 7);
    assert_eq!(actual, expected);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn overlay_matches_independent_reference(
        source_value in -100_i64..100,
        patch in prop::option::of(-100_i64..100),
        deleted in any::<bool>(),
        released in any::<bool>(),
    ) {
        let source = BTreeMap::from([
            (prop("flightId"), json!(7)),
            (prop("delayMinutes"), json!(source_value)),
        ]);
        let edit = ObjectEdit {
            pk: PropertyValue::Int(7),
            row: EditRow {
                set: patch.map_or_else(BTreeMap::new, |value| {
                    BTreeMap::from([(prop("delayMinutes"), PropertyValue::Int(value))])
                }),
                deleted,
                released,
                version: 1,
            },
        };
        let actual = overlay_object(Some(&source), Some(&edit), &prop("flightId"), &edit.pk).unwrap();
        let expected = reference(Some(source), Some(&edit), 7);
        prop_assert_eq!(actual, expected);
    }
}
