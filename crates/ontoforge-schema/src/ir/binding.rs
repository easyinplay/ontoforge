//! Where an object type's data comes from (D-02).
//!
//! This is the definition-layer declaration only. The physical binding to a table
//! lives in separate source binding files (ADR-0021).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The data source an object type or many-to-many link declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Backing {
    /// Objects come from the physical table bound for the selected profile; edits
    /// are layered on top.
    Source,
    /// Objects are created only through actions or imports. No binding is allowed.
    EditOnly,
}
