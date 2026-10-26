//! Validated L2 SQL is produced here, within the private `PrintedSql` boundary.

use std::collections::BTreeSet;

use super::PrintedSql;

/// Trusted grants and deployment exclusions for one L2 request.
pub struct L2Policy {
    pub raw_allowed: bool,
    pub catalogs: BTreeSet<String>,
    pub schemas: BTreeSet<String>,
    pub edit_catalog: String,
    pub max_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum L2Error {
    Denied,
    Unsupported,
}

/// Validates untrusted SQL before it can become executable engine SQL.
pub fn validate_l2(_raw: &str, _policy: &L2Policy) -> Result<PrintedSql, L2Error> {
    Err(L2Error::Unsupported)
}
