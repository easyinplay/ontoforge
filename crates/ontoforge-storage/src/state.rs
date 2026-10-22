//! The per-property patch state machine of an edit row (STORE-07, RESEARCH Pattern 2).
//!
//! `next_state` and `next_link_state` are pure functions: `commit` reads the current row, asks
//! them for the new row and writes the result back, and `replay` does the same with the logged
//! operations, so both always agree.
//!
//! Object rows (`absent` is a row no edit touched, version 0):
//!
//! | from \ op | set | unset | tombstone | release |
//! |---|---|---|---|---|
//! | absent | create (editOnly needs every required property) | editOnly `NotFound`, else invalid | source backed: tombstoned; editOnly `NotFound` | invalid |
//! | live | patch | clear the named properties (editOnly: not a required one) | tombstoned | source backed: released; editOnly `ReleaseOnEditOnly` |
//! | tombstoned | `Tombstoned` | invalid | invalid | source backed: released; editOnly `ReleaseOnEditOnly` |
//! | released | live again | invalid | tombstoned | invalid |
//!
//! `release` on an editOnly type is `ReleaseOnEditOnly` from every state. Every successful
//! transition adds one to the version; a tombstone keeps the properties that were set, a release
//! clears all of them.
//!
//! Link rows: `add` and `remove` are allowed from every state and add one to the version, except
//! that `remove` on an absent editOnly link is `NotFound` (a source backed link row may suppress
//! a join row it has never seen).

use std::collections::BTreeMap;

use ontoforge_schema::ir::{Backing, ObjectType, PropertyName};

use crate::commit::{EditError, LinkOp, ObjectOp};
use crate::value::PropertyValue;

/// The overlay an edit row holds for one object.
#[derive(Debug, Clone, PartialEq)]
pub struct EditRow {
    /// The properties an edit set, with their values.
    pub set: BTreeMap<PropertyName, PropertyValue>,
    pub deleted: bool,
    pub released: bool,
    /// `0` for a row no edit has touched.
    pub version: i64,
}

/// The state of a link row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRow {
    pub deleted: bool,
    pub version: i64,
}

/// The row after applying `op` to `prev`. `None` and a row of version 0 both mean "no edit has
/// touched this object yet".
pub fn next_state(
    prev: Option<&EditRow>,
    op: &ObjectOp,
    ty: &ObjectType,
) -> Result<EditRow, EditError> {
    let prev = prev.filter(|row| row.version > 0);
    let edit_only = ty.backing == Backing::EditOnly;
    if edit_only && matches!(op, ObjectOp::Release) {
        return Err(EditError::ReleaseOnEditOnly);
    }
    match (prev, op) {
        (None, ObjectOp::Set(values)) => {
            if edit_only
                && let Some(missing) = ty.properties.iter().find(|(name, def)| {
                    def.required && **name != ty.primary_key && !values.contains_key(*name)
                })
            {
                return Err(EditError::MissingRequired {
                    property: missing.0.as_str().to_owned(),
                });
            }
            Ok(EditRow {
                set: values.clone(),
                deleted: false,
                released: false,
                version: 1,
            })
        }
        (None, ObjectOp::Tombstone) if !edit_only => Ok(EditRow {
            set: BTreeMap::new(),
            deleted: true,
            released: false,
            version: 1,
        }),
        (None, ObjectOp::Tombstone | ObjectOp::Unset(_)) if edit_only => Err(EditError::NotFound),
        (Some(row), ObjectOp::Set(_)) if row.deleted => Err(EditError::Tombstoned),
        (Some(row), ObjectOp::Set(values)) => {
            let mut set = row.set.clone();
            set.extend(
                values
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
            Ok(EditRow {
                set,
                deleted: false,
                released: false,
                version: row.version + 1,
            })
        }
        (Some(row), ObjectOp::Unset(names)) if !row.deleted && !row.released => {
            if edit_only
                && let Some(required) = names
                    .iter()
                    .find(|name| ty.properties.get(*name).is_some_and(|def| def.required))
            {
                return Err(EditError::MissingRequired {
                    property: required.as_str().to_owned(),
                });
            }
            let mut set = row.set.clone();
            set.retain(|name, _| !names.contains(name));
            Ok(EditRow {
                set,
                deleted: false,
                released: false,
                version: row.version + 1,
            })
        }
        (Some(row), ObjectOp::Tombstone) if !row.deleted => Ok(EditRow {
            set: row.set.clone(),
            deleted: true,
            released: false,
            version: row.version + 1,
        }),
        (Some(row), ObjectOp::Release) if !row.released => Ok(EditRow {
            set: BTreeMap::new(),
            deleted: false,
            released: true,
            version: row.version + 1,
        }),
        (prev, op) => Err(EditError::InvalidTransition {
            from: object_state(prev),
            op: object_op(op),
        }),
    }
}

/// The link row after applying `op` to `prev`. `backing` is the backing of the link type.
pub fn next_link_state(
    prev: Option<&LinkRow>,
    op: LinkOp,
    backing: Backing,
) -> Result<LinkRow, EditError> {
    let prev = prev.filter(|row| row.version > 0);
    match (prev, op) {
        (None, LinkOp::Remove) if backing == Backing::EditOnly => Err(EditError::NotFound),
        (None, op) => Ok(LinkRow {
            deleted: op == LinkOp::Remove,
            version: 1,
        }),
        (Some(row), op) => Ok(LinkRow {
            deleted: op == LinkOp::Remove,
            version: row.version + 1,
        }),
    }
}

fn object_state(prev: Option<&EditRow>) -> &'static str {
    match prev {
        None => "absent",
        Some(row) if row.deleted => "tombstoned",
        Some(row) if row.released => "released",
        Some(_) => "live",
    }
}

fn object_op(op: &ObjectOp) -> &'static str {
    match op {
        ObjectOp::Set(_) => "set",
        ObjectOp::Unset(_) => "unset",
        ObjectOp::Tombstone => "tombstone",
        ObjectOp::Release => "release",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use ontoforge_schema::load_from_memory;

    fn gate(backing: &str) -> ObjectType {
        let text = format!(
            "kind: ObjectType\napiName: Gate\nbacking: {backing}\nprimaryKey: code\nproperties:\n  \
             code: {{ type: string, required: true }}\n  name: {{ type: string, required: true }}\n  \
             capacity: {{ type: integer }}\n"
        );
        let out = load_from_memory(&[
            ("ontology.yaml", "apiName: demo\n"),
            ("defs.yaml", text.as_str()),
        ]);
        let ontology = out.ontology.expect("definition loads");
        ontology
            .object_types
            .values()
            .next()
            .cloned()
            .expect("type")
    }

    fn name(s: &str) -> PropertyName {
        PropertyName::try_from(s.to_owned()).expect("property name")
    }

    fn set(pairs: &[(&str, PropertyValue)]) -> ObjectOp {
        ObjectOp::Set(pairs.iter().map(|(n, v)| (name(n), v.clone())).collect())
    }

    fn unset(list: &[&str]) -> ObjectOp {
        ObjectOp::Unset(list.iter().map(|n| name(n)).collect::<BTreeSet<_>>())
    }

    fn text(s: &str) -> PropertyValue {
        PropertyValue::String(s.to_owned())
    }

    fn live(ty: &ObjectType) -> EditRow {
        next_state(None, &set(&[("name", text("N"))]), ty).expect("created")
    }

    #[test]
    fn set_creates_a_row_with_version_one() {
        let row =
            next_state(None, &set(&[("name", text("North"))]), &gate("editOnly")).expect("created");
        assert_eq!(row.version, 1);
        assert!(!row.deleted && !row.released);
        assert_eq!(row.set.get(&name("name")), Some(&text("North")));
    }

    #[test]
    fn an_edit_only_object_needs_its_required_properties() {
        let err = next_state(
            None,
            &set(&[("capacity", PropertyValue::Int(1))]),
            &gate("editOnly"),
        )
        .expect_err("name is missing");
        assert!(matches!(err, EditError::MissingRequired { property } if property == "name"));
    }

    #[test]
    fn a_source_backed_object_may_be_patched_without_required_properties() {
        let row = next_state(
            None,
            &set(&[("capacity", PropertyValue::Int(1))]),
            &gate("source"),
        )
        .expect("the source row supplies the rest");
        assert_eq!(row.version, 1);
    }

    #[test]
    fn a_version_zero_row_counts_as_absent() {
        let skeleton = EditRow {
            set: BTreeMap::new(),
            deleted: false,
            released: false,
            version: 0,
        };
        let row = next_state(
            Some(&skeleton),
            &set(&[("name", text("North"))]),
            &gate("editOnly"),
        )
        .expect("created");
        assert_eq!(row.version, 1);
    }

    #[test]
    fn set_patches_given_properties_and_bumps_the_version() {
        let ty = gate("editOnly");
        let first = next_state(None, &set(&[("name", text("North"))]), &ty).expect("created");
        let second = next_state(
            Some(&first),
            &set(&[("capacity", PropertyValue::Int(9))]),
            &ty,
        )
        .expect("patched");
        assert_eq!(second.version, 2);
        assert_eq!(second.set.len(), 2);
        let third =
            next_state(Some(&second), &set(&[("name", text("South"))]), &ty).expect("overwritten");
        assert_eq!(third.version, 3);
        assert_eq!(third.set.get(&name("name")), Some(&text("South")));
        assert_eq!(
            third.set.get(&name("capacity")),
            Some(&PropertyValue::Int(9))
        );
    }

    #[test]
    fn a_tombstoned_object_is_not_revived_by_set() {
        let ty = gate("editOnly");
        let dead = EditRow {
            set: BTreeMap::new(),
            deleted: true,
            released: false,
            version: 3,
        };
        let err = next_state(Some(&dead), &set(&[("name", text("N"))]), &ty).expect_err("dead");
        assert!(matches!(err, EditError::Tombstoned));
    }

    #[test]
    fn unset_clears_only_the_named_properties() {
        let ty = gate("editOnly");
        let first = next_state(
            None,
            &set(&[("name", text("N")), ("capacity", PropertyValue::Int(3))]),
            &ty,
        )
        .expect("created");
        let row = next_state(Some(&first), &unset(&["capacity"]), &ty).expect("unset");
        assert_eq!(row.version, 2);
        assert!(!row.set.contains_key(&name("capacity")));
        assert_eq!(row.set.get(&name("name")), Some(&text("N")));
        let again = next_state(Some(&row), &unset(&["capacity"]), &ty).expect("a no-op unset");
        assert_eq!(again.version, 3, "unsetting what is not set still counts");
    }

    #[test]
    fn an_edit_only_object_keeps_its_required_properties_on_unset() {
        let ty = gate("editOnly");
        let err = next_state(Some(&live(&ty)), &unset(&["name"]), &ty).expect_err("required");
        assert!(matches!(err, EditError::MissingRequired { property } if property == "name"));
        let ty = gate("source");
        let row = next_state(Some(&live(&ty)), &unset(&["name"]), &ty).expect("the source has it");
        assert!(row.set.is_empty());
    }

    #[test]
    fn tombstone_keeps_the_set_properties_and_blocks_everything_but_release() {
        let ty = gate("source");
        let dead = next_state(Some(&live(&ty)), &ObjectOp::Tombstone, &ty).expect("tombstone");
        assert!(dead.deleted && !dead.released);
        assert_eq!(dead.version, 2);
        assert_eq!(dead.set.len(), 1);
        for op in [ObjectOp::Tombstone, unset(&["name"])] {
            let err = next_state(Some(&dead), &op, &ty).expect_err("dead");
            assert!(matches!(
                err,
                EditError::InvalidTransition {
                    from: "tombstoned",
                    ..
                }
            ));
        }
        let back = next_state(Some(&dead), &ObjectOp::Release, &ty).expect("release");
        assert!(!back.deleted && back.released);
        assert!(back.set.is_empty());
        assert_eq!(back.version, 3);
    }

    #[test]
    fn a_released_object_can_be_set_or_tombstoned_but_not_released_or_unset() {
        let ty = gate("source");
        let released = next_state(Some(&live(&ty)), &ObjectOp::Release, &ty).expect("release");
        for op in [ObjectOp::Release, unset(&["name"])] {
            let err = next_state(Some(&released), &op, &ty).expect_err("already pure source");
            assert!(matches!(
                err,
                EditError::InvalidTransition {
                    from: "released",
                    ..
                }
            ));
        }
        let live_again =
            next_state(Some(&released), &set(&[("name", text("M"))]), &ty).expect("set");
        assert!(!live_again.released && !live_again.deleted);
        assert_eq!(live_again.version, 3);
        let dead = next_state(Some(&released), &ObjectOp::Tombstone, &ty).expect("tombstone");
        assert!(dead.deleted && !dead.released);
    }

    #[test]
    fn a_source_backed_object_can_be_tombstoned_without_an_edit_row() {
        let ty = gate("source");
        let dead = next_state(None, &ObjectOp::Tombstone, &ty).expect("suppress the source row");
        assert!(dead.deleted);
        assert_eq!(dead.version, 1);
        for op in [ObjectOp::Release, unset(&["capacity"])] {
            let err = next_state(None, &op, &ty).expect_err("nothing to undo");
            assert!(matches!(
                err,
                EditError::InvalidTransition { from: "absent", .. }
            ));
        }
    }

    #[test]
    fn an_edit_only_object_that_does_not_exist_cannot_be_changed() {
        let ty = gate("editOnly");
        for op in [ObjectOp::Tombstone, unset(&["capacity"])] {
            let err = next_state(None, &op, &ty).expect_err("absent");
            assert!(matches!(err, EditError::NotFound));
        }
        for from in [None, Some(live(&ty))] {
            let err = next_state(from.as_ref(), &ObjectOp::Release, &ty).expect_err("editOnly");
            assert!(matches!(err, EditError::ReleaseOnEditOnly));
        }
        let dead = next_state(Some(&live(&ty)), &ObjectOp::Tombstone, &ty).expect("tombstone");
        let err = next_state(Some(&dead), &ObjectOp::Release, &ty).expect_err("no way back");
        assert!(matches!(err, EditError::ReleaseOnEditOnly));
    }

    #[test]
    fn link_rows_toggle_and_count_every_write() {
        let added = next_link_state(None, LinkOp::Add, Backing::EditOnly).expect("added");
        assert_eq!(
            added,
            LinkRow {
                deleted: false,
                version: 1
            }
        );
        let again = next_link_state(Some(&added), LinkOp::Add, Backing::EditOnly).expect("again");
        assert_eq!(again.version, 2);
        assert!(!again.deleted);
        let removed =
            next_link_state(Some(&again), LinkOp::Remove, Backing::EditOnly).expect("removed");
        assert_eq!(
            removed,
            LinkRow {
                deleted: true,
                version: 3
            }
        );
        let twice =
            next_link_state(Some(&removed), LinkOp::Remove, Backing::EditOnly).expect("again");
        assert_eq!((twice.deleted, twice.version), (true, 4));
        let restored =
            next_link_state(Some(&twice), LinkOp::Add, Backing::EditOnly).expect("restored");
        assert_eq!((restored.deleted, restored.version), (false, 5));
    }

    #[test]
    fn removing_an_absent_link_depends_on_its_backing() {
        let err = next_link_state(None, LinkOp::Remove, Backing::EditOnly).expect_err("absent");
        assert!(matches!(err, EditError::NotFound));
        let suppressed =
            next_link_state(None, LinkOp::Remove, Backing::Source).expect("suppress a join row");
        assert_eq!(
            suppressed,
            LinkRow {
                deleted: true,
                version: 1
            }
        );
    }
}
