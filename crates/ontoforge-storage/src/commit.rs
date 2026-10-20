//! The write path of the edit store: one commit, one transaction (STORE-03).
//!
//! A [`Commit`] is validated without a transaction; then one transaction locks or creates the
//! edit rows, computes their new state, appends one log record with its entries and the history
//! snapshots, and writes the rows. Any failure rolls everything back.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};

use crate::error::StoreError;
use crate::store::EditStore;
use crate::value::PropertyValue;

/// Who commits. The reserved system actors are stored as `system:<name>`; a principal id is
/// `{iss}|{sub}` and never starts with `system:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// `{iss}|{sub}`.
    Principal(String),
    System(SystemActor),
}

/// The reserved actors that are not a principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemActor {
    /// Stored as `system:import`.
    Import,
}

impl Actor {
    /// The text stored in `of_edit_log.actor`.
    pub fn as_text(&self) -> &str {
        match self {
            Self::Principal(id) => id,
            Self::System(SystemActor::Import) => "system:import",
        }
    }
}

/// One atomic change set.
#[derive(Debug, Clone)]
pub struct Commit {
    pub actor: Actor,
    pub action: String,
    /// The definition version the caller validated against.
    pub def_version: i64,
    /// The action parameters, stored as they are (D-11).
    pub params: serde_json::Value,
    pub edits: Vec<Edit>,
}

/// One change of an object or a link.
#[derive(Debug, Clone)]
pub enum Edit {
    Object {
        type_name: ObjectTypeName,
        pk: PropertyValue,
        op: ObjectOp,
    },
    Link {
        link: LinkTypeName,
        a: PropertyValue,
        b: PropertyValue,
        op: LinkOp,
    },
}

#[derive(Debug, Clone)]
pub enum ObjectOp {
    Set(BTreeMap<PropertyName, PropertyValue>),
    Unset(BTreeSet<PropertyName>),
    Tombstone,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOp {
    Add,
    Remove,
}

/// What a successful commit returns.
#[derive(Debug, Clone)]
pub struct Receipt {
    pub seq: i64,
    pub event_id: ulid::Ulid,
    /// The version of each edited row after its edit, in the order of `Commit::edits`.
    pub versions: Vec<i64>,
}

/// Why a commit was refused. Nothing is written in any of these cases.
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("unknown type `{0}`")]
    UnknownType(String),
    #[error("type `{type_name}` has no property `{property}`")]
    UnknownProperty { type_name: String, property: String },
    #[error("invalid value for `{property}`: {reason}")]
    Value { property: String, reason: String },
    #[error("the primary key cannot be set")]
    PrimaryKeyNotSettable,
    #[error("required property `{property}` is missing")]
    MissingRequired { property: String },
    #[error("the object does not exist")]
    NotFound,
    #[error("the object is deleted")]
    Tombstoned,
    #[error("cannot {op} an object that is {from}")]
    InvalidTransition {
        from: &'static str,
        op: &'static str,
    },
    #[error("only a source backed object can be released")]
    ReleaseOnEditOnly,
    #[error("link `{link}` would violate its cardinality")]
    CardinalityViolation { link: String },
    #[error("the commit was validated against definition {expected}, but {active} is active")]
    DefinitionStale { expected: i64, active: i64 },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl EditStore {
    /// Applies `commit` in one transaction.
    pub async fn commit(&self, _commit: Commit) -> Result<Receipt, EditError> {
        Err(EditError::Store(StoreError::Config(
            "commit is not implemented".to_owned(),
        )))
    }
}
