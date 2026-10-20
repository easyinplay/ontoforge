//! Roles, role bindings, the admin audit and user records of the edit store (SCOPE-01, SCOPE-07;
//! D-01..D-04).
//!
//! RED stub of plan 03-16: the types are final, the behaviour is not there yet.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ontoforge_auth::Principal;
use ontoforge_schema::ir::{ObjectTypeName, PropertyName, Role, ScopeSet};
use serde::Serialize;

use crate::error::StoreError;
use crate::store::EditStore;

/// Who a binding names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubjectKind {
    User,
    Group,
}

/// One row of `of_role_binding`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct BindingRow {
    pub role: String,
    pub kind: SubjectKind,
    pub subject: String,
}

/// What `scope apply` would change. Every list is sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopePlan {
    pub roles_added: Vec<String>,
    pub roles_changed: Vec<String>,
    pub roles_removed: Vec<String>,
    pub bindings_added: Vec<BindingRow>,
    pub bindings_removed: Vec<BindingRow>,
    pub index_ddl: Vec<String>,
    pub before_hash: String,
    pub after_hash: String,
    pub hash: String,
}

/// A scope that was applied, and the sequence number of its audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedScope {
    pub plan: ScopePlan,
    pub audit_seq: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeApplyError {
    #[error("the plan changed since it was reviewed (expected hash {expected}, now {actual})")]
    PlanChanged { expected: String, actual: String },
    #[error("no definition is active yet; run `ontoforge reload` first")]
    NoActiveDefinition,
    #[error(transparent)]
    Store(#[from] StoreError),
}

type RowFilterProperties = BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>;

fn empty_plan() -> ScopePlan {
    ScopePlan {
        roles_added: Vec::new(),
        roles_changed: Vec::new(),
        roles_removed: Vec::new(),
        bindings_added: Vec::new(),
        bindings_removed: Vec::new(),
        index_ddl: Vec::new(),
        before_hash: String::new(),
        after_hash: String::new(),
        hash: String::new(),
    }
}

impl EditStore {
    /// The roles and bindings in the store.
    pub async fn load_scope(&self) -> Result<ScopeSet, StoreError> {
        Ok(ScopeSet::default())
    }

    /// What applying `desired` would change.
    pub async fn scope_plan(
        &self,
        _desired: &ScopeSet,
        _row_filter_properties: &RowFilterProperties,
    ) -> Result<ScopePlan, StoreError> {
        Ok(empty_plan())
    }

    /// Writes `desired` if its plan still has the hash `expected_hash`.
    pub async fn scope_apply(
        &self,
        _desired: &ScopeSet,
        _row_filter_properties: &RowFilterProperties,
        _expected_hash: &str,
        _actor: &str,
    ) -> Result<AppliedScope, ScopeApplyError> {
        Ok(AppliedScope {
            plan: empty_plan(),
            audit_seq: 0,
        })
    }

    /// Records the first or latest login of `principal`.
    pub async fn record_login(&self, _principal: &Principal) -> Result<(), StoreError> {
        Ok(())
    }
}

/// Writes a login at most once per interval and principal.
pub struct LoginRecorder {
    _interval: Duration,
}

impl LoginRecorder {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            _interval: interval,
        }
    }

    /// Whether the store was written.
    pub async fn record(
        &self,
        _store: &EditStore,
        _principal: &Principal,
    ) -> Result<bool, StoreError> {
        Ok(false)
    }
}

/// `role` with its list fields sorted and without duplicates.
#[must_use]
pub fn normalize_role(role: &Role) -> Role {
    role.clone()
}

/// `scope` as the store holds it: normalized roles, sorted subjects, no empty binding.
#[must_use]
pub fn normalize_scope(scope: &ScopeSet) -> ScopeSet {
    scope.clone()
}
