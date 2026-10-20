//! The per-property patch state machine of an edit row (STORE-07, RESEARCH Pattern 2).
//!
//! `next_state` is a pure function: `commit` reads the current row, asks it for the new row and
//! writes the result back. The full transition table and its reference model come with plan
//! 03-13; this module implements the creating and the patching `Set`.

use std::collections::BTreeMap;

use ontoforge_schema::ir::{ObjectType, PropertyName};

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

/// The row after applying `op` to `prev` (`None` or a version 0 row: no edit yet).
pub fn next_state(
    _prev: Option<&EditRow>,
    _op: &ObjectOp,
    _ty: &ObjectType,
) -> Result<EditRow, EditError> {
    Err(EditError::InvalidTransition {
        from: "unknown",
        op: "unknown",
    })
}

/// The link row after applying `op` to `prev`.
pub fn next_link_state(_prev: Option<&LinkRow>, _op: LinkOp) -> Result<LinkRow, EditError> {
    Err(EditError::InvalidTransition {
        from: "unknown",
        op: "unknown",
    })
}
