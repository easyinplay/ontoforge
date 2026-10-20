//! The per-property patch state machine of an edit row (STORE-07, RESEARCH Pattern 2).
//!
//! `next_state` is a pure function: `commit` reads the current row, asks it for the new row and
//! writes the result back. This plan implements the transitions of `Set` (creating an object and
//! patching a live one) and of a link `Add`; the full transition table and its reference model
//! come with plan 03-13, until then every other combination is `InvalidTransition`.

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
    match (prev, op) {
        (None, ObjectOp::Set(values)) => {
            if ty.backing == Backing::EditOnly
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
        (prev, op) => Err(EditError::InvalidTransition {
            from: object_state(prev),
            op: object_op(op),
        }),
    }
}

/// The link row after applying `op` to `prev`.
pub fn next_link_state(prev: Option<&LinkRow>, op: LinkOp) -> Result<LinkRow, EditError> {
    let prev = prev.filter(|row| row.version > 0);
    match (prev, op) {
        (None, LinkOp::Add) => Ok(LinkRow {
            deleted: false,
            version: 1,
        }),
        (Some(row), LinkOp::Add) if row.deleted => Ok(LinkRow {
            deleted: false,
            version: row.version + 1,
        }),
        (prev, op) => Err(EditError::InvalidTransition {
            from: match prev {
                None => "absent",
                Some(row) if row.deleted => "removed",
                Some(_) => "linked",
            },
            op: match op {
                LinkOp::Add => "add",
                LinkOp::Remove => "remove",
            },
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

    fn text(s: &str) -> PropertyValue {
        PropertyValue::String(s.to_owned())
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
    fn other_operations_wait_for_plan_03_13() {
        let ty = gate("editOnly");
        let live = next_state(None, &set(&[("name", text("N"))]), &ty).expect("created");
        for op in [
            ObjectOp::Tombstone,
            ObjectOp::Release,
            ObjectOp::Unset(BTreeSet::from([name("capacity")])),
        ] {
            let err = next_state(Some(&live), &op, &ty).expect_err("not yet");
            assert!(matches!(
                err,
                EditError::InvalidTransition { from: "live", .. }
            ));
        }
    }

    #[test]
    fn link_add_creates_and_restores() {
        let added = next_link_state(None, LinkOp::Add).expect("added");
        assert_eq!(
            added,
            LinkRow {
                deleted: false,
                version: 1
            }
        );
        let again = next_link_state(Some(&added), LinkOp::Add);
        assert!(matches!(
            again,
            Err(EditError::InvalidTransition {
                from: "linked",
                op: "add"
            })
        ));
        let removed = LinkRow {
            deleted: true,
            version: 2,
        };
        let restored = next_link_state(Some(&removed), LinkOp::Add).expect("restored");
        assert_eq!(
            restored,
            LinkRow {
                deleted: false,
                version: 3
            }
        );
    }
}
