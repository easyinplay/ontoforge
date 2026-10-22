//! A naive reference model of the overlay semantics of the edit store (STORE-07, SC3).
//!
//! The model is written from the transition table in the 03-13 plan, not from
//! `ontoforge_storage::state`: it keeps rows in plain maps, applies a commit to a copy and
//! throws the copy away on the first error. It must not call into the storage state machine, so
//! that one mistake cannot be made in both places at once.
//!
//! The ontology it models (see `model_prop.rs`):
//!
//! - `Gate`: editOnly, key `code`, required `name`, optional `capacity`.
//! - `Stand`: source backed, key `id`, optional `label` and `gate`; `gate` is the foreign key of
//!   the one-to-one link `gateStand`.
//! - `gateStands`: editOnly many-to-many link `Gate` - `Stand`; `gateSlots`: the same, source
//!   backed.
#![allow(dead_code, reason = "each test target uses a subset of the model")]

use std::collections::BTreeMap;

use ontoforge_schema::ir::PropertyName;
use ontoforge_storage::commit::{Commit, Edit, LinkOp, ObjectOp};
use ontoforge_storage::value::PropertyValue;

/// The kind of failure, one per `EditError` variant the overlay can produce. Messages are not
/// compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    NotFound,
    Tombstoned,
    InvalidTransition,
    ReleaseOnEditOnly,
    MissingRequired,
    CardinalityViolation,
}

/// What a commit did: the version of every edited row after its edit, or why it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelOutcome {
    Ok(Vec<i64>),
    Err(ErrorClass),
}

/// A row as the tests compare it: what the model holds and what the SQL tables hold, in one
/// shape. `table` is the object type or link type name, `key` the primary key (`a|b` for a link),
/// `props` only the properties that are set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedRow {
    pub table: String,
    pub key: String,
    pub props: BTreeMap<String, String>,
    pub deleted: bool,
    pub released: bool,
    pub version: i64,
}

/// How a type behaves, from its definition.
struct TypeSpec {
    edit_only: bool,
    required: &'static [&'static str],
    /// The property that is the foreign key of a one-to-one link.
    one_to_one: Option<&'static str>,
}

fn object_spec(name: &str) -> TypeSpec {
    match name {
        "Gate" => TypeSpec {
            edit_only: true,
            required: &["name"],
            one_to_one: None,
        },
        "Stand" => TypeSpec {
            edit_only: false,
            required: &[],
            one_to_one: Some("gate"),
        },
        other => panic!("the model does not know type {other}"),
    }
}

fn link_is_edit_only(name: &str) -> bool {
    match name {
        "gateStands" => true,
        "gateSlots" => false,
        other => panic!("the model does not know link {other}"),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ModelRow {
    props: BTreeMap<String, PropertyValue>,
    deleted: bool,
    released: bool,
    version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelLink {
    deleted: bool,
    version: i64,
}

/// Rows are found by this key. The derived order is the order in which a commit locks and writes
/// rows: objects before links, then by type, then by the key's JSON text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Object { name: String, pk: String },
    Link { name: String, a: String, b: String },
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    objects: BTreeMap<(String, String), ModelRow>,
    links: BTreeMap<(String, String, String), ModelLink>,
}

fn json_text(value: &PropertyValue) -> String {
    value.to_json().to_string()
}

fn plain_text(value: &PropertyValue) -> String {
    match value {
        PropertyValue::String(s) => s.clone(),
        PropertyValue::Int(i) => i.to_string(),
        other => panic!("the model only holds strings and integers, not {other:?}"),
    }
}

/// The lifecycle state of an object row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Absent,
    Live,
    Tombstoned,
    Released,
}

fn state_of(row: Option<&ModelRow>) -> State {
    match row {
        None => State::Absent,
        Some(row) if row.deleted => State::Tombstoned,
        Some(row) if row.released => State::Released,
        Some(_) => State::Live,
    }
}

/// One edit of the commit, with the position it had in the commit.
struct Pending<'a> {
    index: usize,
    key: Key,
    edit: &'a Edit,
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies `commit` as one unit: every edit is applied to a copy in lock order, the rows are
    /// written back one at a time in the same order while the one-to-one rule is checked after
    /// each write. The first failure leaves the model as it was.
    pub fn apply(&mut self, commit: &Commit) -> ModelOutcome {
        let mut pending: Vec<Pending<'_>> = commit
            .edits
            .iter()
            .enumerate()
            .map(|(index, edit)| Pending {
                index,
                key: key_of(edit),
                edit,
            })
            .collect();
        // Stable: two edits of one row keep their order in the commit.
        pending.sort_by(|x, y| x.key.cmp(&y.key));

        let mut work = self.clone();
        let mut versions = vec![0; pending.len()];
        for item in &pending {
            match work.apply_edit(item.edit) {
                Ok(version) => versions[item.index] = version,
                Err(class) => return ModelOutcome::Err(class),
            }
        }

        // The write phase: start from the rows as they were before the commit.
        let mut db = self.clone();
        let mut written: Option<&Key> = None;
        for item in &pending {
            if written == Some(&item.key) {
                continue;
            }
            written = Some(&item.key);
            db.copy_row(&work, &item.key);
            if db.violates_one_to_one(&item.key) {
                return ModelOutcome::Err(ErrorClass::CardinalityViolation);
            }
        }
        *self = work;
        ModelOutcome::Ok(versions)
    }

    fn copy_row(&mut self, from: &Model, key: &Key) {
        match key {
            Key::Object { name, pk } => {
                let id = (name.clone(), pk.clone());
                if let Some(row) = from.objects.get(&id) {
                    self.objects.insert(id, row.clone());
                }
            }
            Key::Link { name, a, b } => {
                let id = (name.clone(), a.clone(), b.clone());
                if let Some(row) = from.links.get(&id) {
                    self.links.insert(id, row.clone());
                }
            }
        }
    }

    /// Two live rows of a type with a one-to-one foreign key may not hold the same value of it.
    /// Only the row just written can have caused a violation.
    fn violates_one_to_one(&self, key: &Key) -> bool {
        let Key::Object { name, pk } = key else {
            return false;
        };
        let Some(property) = object_spec(name).one_to_one else {
            return false;
        };
        let Some(row) = self.objects.get(&(name.clone(), pk.clone())) else {
            return false;
        };
        let Some(value) = row.props.get(property) else {
            return false;
        };
        if row.deleted {
            return false;
        }
        self.objects.iter().any(|((other_name, other_pk), other)| {
            other_name == name
                && other_pk != pk
                && !other.deleted
                && other.props.get(property) == Some(value)
        })
    }

    fn apply_edit(&mut self, edit: &Edit) -> Result<i64, ErrorClass> {
        match edit {
            Edit::Object { type_name, pk, op } => {
                let id = (type_name.as_str().to_owned(), json_text(pk));
                let next =
                    object_transition(&object_spec(type_name.as_str()), self.objects.get(&id), op)?;
                let version = next.version;
                self.objects.insert(id, next);
                Ok(version)
            }
            Edit::Link { link, a, b, op } => {
                let id = (link.as_str().to_owned(), json_text(a), json_text(b));
                let next =
                    link_transition(link_is_edit_only(link.as_str()), self.links.get(&id), *op)?;
                let version = next.version;
                self.links.insert(id, next);
                Ok(version)
            }
        }
    }

    /// Every row some edit has touched, in a fixed order.
    pub fn rows(&self) -> Vec<NormalizedRow> {
        let objects = self.objects.iter().map(|((name, pk), row)| NormalizedRow {
            table: name.clone(),
            key: unquote(pk),
            props: row
                .props
                .iter()
                .map(|(property, value)| (property.clone(), plain_text(value)))
                .collect(),
            deleted: row.deleted,
            released: row.released,
            version: row.version,
        });
        let links = self.links.iter().map(|((name, a, b), row)| NormalizedRow {
            table: name.clone(),
            key: format!("{}|{}", unquote(a), unquote(b)),
            props: BTreeMap::new(),
            deleted: row.deleted,
            released: false,
            version: row.version,
        });
        let mut all: Vec<NormalizedRow> = objects.chain(links).collect();
        all.sort();
        all
    }
}

/// `"G1"` to `G1`, `7` to `7`.
fn unquote(json: &str) -> String {
    json.trim_matches('"').to_owned()
}

fn key_of(edit: &Edit) -> Key {
    match edit {
        Edit::Object { type_name, pk, .. } => Key::Object {
            name: type_name.as_str().to_owned(),
            pk: json_text(pk),
        },
        Edit::Link { link, a, b, .. } => Key::Link {
            name: link.as_str().to_owned(),
            a: json_text(a),
            b: json_text(b),
        },
    }
}

/// The transition table of an object row.
fn object_transition(
    spec: &TypeSpec,
    current: Option<&ModelRow>,
    op: &ObjectOp,
) -> Result<ModelRow, ErrorClass> {
    // An editOnly object has no source row to fall back to, so it has no released state.
    if spec.edit_only && matches!(op, ObjectOp::Release) {
        return Err(ErrorClass::ReleaseOnEditOnly);
    }
    let state = state_of(current);
    let version = current.map_or(0, |row| row.version) + 1;
    let props = current.map(|row| row.props.clone()).unwrap_or_default();
    let row = |props, deleted, released| ModelRow {
        props,
        deleted,
        released,
        version,
    };
    let merged = |values: &BTreeMap<PropertyName, PropertyValue>| {
        let mut merged = props.clone();
        for (name, value) in values {
            merged.insert(name.as_str().to_owned(), value.clone());
        }
        merged
    };
    match (state, op) {
        (State::Absent, ObjectOp::Set(values)) => {
            let missing = spec
                .required
                .iter()
                .any(|name| !values.keys().any(|key| key.as_str() == *name));
            if spec.edit_only && missing {
                return Err(ErrorClass::MissingRequired);
            }
            Ok(row(merged(values), false, false))
        }
        (State::Absent, ObjectOp::Tombstone) if !spec.edit_only => {
            Ok(row(BTreeMap::new(), true, false))
        }
        (State::Absent, ObjectOp::Tombstone | ObjectOp::Unset(_)) if spec.edit_only => {
            Err(ErrorClass::NotFound)
        }
        (State::Live | State::Released, ObjectOp::Set(values)) => {
            Ok(row(merged(values), false, false))
        }
        (State::Live, ObjectOp::Unset(names)) => {
            if spec.edit_only
                && names
                    .iter()
                    .any(|name| spec.required.contains(&name.as_str()))
            {
                return Err(ErrorClass::MissingRequired);
            }
            let mut kept = props.clone();
            for name in names {
                kept.remove(name.as_str());
            }
            Ok(row(kept, false, false))
        }
        (State::Live | State::Released, ObjectOp::Tombstone) => Ok(row(props, true, false)),
        (State::Live | State::Tombstoned, ObjectOp::Release) => {
            Ok(row(BTreeMap::new(), false, true))
        }

        (State::Tombstoned, ObjectOp::Set(_)) => Err(ErrorClass::Tombstoned),
        (State::Absent | State::Tombstoned | State::Released, _) => {
            Err(ErrorClass::InvalidTransition)
        }
    }
}

/// The transition table of a link row.
fn link_transition(
    edit_only: bool,
    current: Option<&ModelLink>,
    op: LinkOp,
) -> Result<ModelLink, ErrorClass> {
    if current.is_none() && edit_only && op == LinkOp::Remove {
        return Err(ErrorClass::NotFound);
    }
    Ok(ModelLink {
        deleted: op == LinkOp::Remove,
        version: current.map_or(0, |link| link.version) + 1,
    })
}
