//! Roles, role bindings, the admin audit and user records of the edit store (SCOPE-01, SCOPE-07;
//! D-01..D-04).
//!
//! `scope apply` is the only way roles and bindings get into the store (D-03): this is the only
//! module that names `of_role`, `of_role_binding` and `of_admin_audit`, and
//! `tests/scope_store.rs` fails when another source file does. An apply makes the stored scope
//! equal to the scope directory, so a role or binding that left the directory leaves the store
//! (D-02). It runs in one transaction: re-plan, compare the hash that was reviewed, write roles
//! and bindings, add the indexes the row predicates need, append the audit record.
//!
//! Group membership is never stored (D-02). `of_user` keeps the identity of a principal that
//! authenticated and its last login, nothing else (D-04).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use ontoforge_auth::Principal;
use ontoforge_schema::ir::{
    ObjectTypeName, PropertyName, Role, RoleBinding, RoleName, ScopeSet, Subject,
};
use sea_query::{Alias, Expr, ExprTrait, OnConflict, Query};
use serde::Serialize;

use crate::ddl::{self, DdlStep};
use crate::error::StoreError;
use crate::index;
use crate::naming::fnv1a64_hex;
use crate::physical::PhysicalSchema;
use crate::sql_exit::{self, Cell, CellKind};
use crate::store::{EditStore, WriteTx};

const ROLE_TABLE: &str = "of_role";
const BINDING_TABLE: &str = "of_role_binding";
const AUDIT_TABLE: &str = "of_admin_audit";
const USER_TABLE: &str = "of_user";

/// The `action` of the audit record an apply appends.
const APPLY_ACTION: &str = "scope.apply";

/// Who a binding names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubjectKind {
    User,
    Group,
}

impl SubjectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Group => "group",
        }
    }

    fn parse(text: &str) -> Result<Self, StoreError> {
        match text {
            "user" => Ok(Self::User),
            "group" => Ok(Self::Group),
            other => Err(StoreError::Corrupt(format!(
                "a role binding has the subject kind `{other}`"
            ))),
        }
    }
}

/// One row of `of_role_binding`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct BindingRow {
    pub role: String,
    pub kind: SubjectKind,
    pub subject: String,
}

impl BindingRow {
    fn of(role: &RoleName, subject: &Subject) -> Self {
        let (kind, subject) = match subject {
            Subject::User(id) => (SubjectKind::User, id),
            Subject::Group(name) => (SubjectKind::Group, name),
        };
        Self {
            role: role.as_str().to_owned(),
            kind,
            subject: subject.clone(),
        }
    }

    fn subject(&self) -> Subject {
        match self.kind {
            SubjectKind::User => Subject::User(self.subject.clone()),
            SubjectKind::Group => Subject::Group(self.subject.clone()),
        }
    }
}

/// What `scope apply` would change. Every list is sorted, so the plan and its hash do not depend
/// on the order of the YAML documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopePlan {
    pub roles_added: Vec<String>,
    pub roles_changed: Vec<String>,
    pub roles_removed: Vec<String>,
    pub bindings_added: Vec<BindingRow>,
    /// Bindings that leave the store, among them every binding of a removed role (full sync).
    pub bindings_removed: Vec<BindingRow>,
    /// `CREATE INDEX IF NOT EXISTS` statements for the properties row predicates read, for those
    /// indexes that do not exist yet.
    pub index_ddl: Vec<String>,
    /// Hash of the scope in the store now.
    pub before_hash: String,
    /// Hash of the scope in the store after the apply.
    pub after_hash: String,
    /// FNV-1a 64 bit (16 hex digits) over every field above.
    pub hash: String,
}

impl ScopePlan {
    /// Whether applying the plan changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.roles_added.is_empty()
            && self.roles_changed.is_empty()
            && self.roles_removed.is_empty()
            && self.bindings_added.is_empty()
            && self.bindings_removed.is_empty()
            && self.index_ddl.is_empty()
    }
}

/// A scope that was applied, and the sequence number of its audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedScope {
    pub plan: ScopePlan,
    pub audit_seq: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeApplyError {
    /// The plan made inside the transaction is not the plan that was reviewed.
    #[error(
        "the scope plan changed since it was reviewed (reviewed hash {expected}, now {actual})"
    )]
    PlanChanged { expected: String, actual: String },
    #[error("no definition is active yet; run `ontoforge reload` first")]
    NoActiveDefinition,
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The properties row predicates read, per object type (`CompiledScope::row_filter_properties`).
type RowFilterProperties = BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>;

/// `role` with its list fields sorted and without duplicates: the properties of every grant,
/// `actions`, `sqlLevels`, `toolsets` and the L2 catalogs and schemas. The order of those lists
/// carries no meaning, so it must not reach the stored form or the hash (SCOPE-02).
#[must_use]
pub fn normalize_role(role: &Role) -> Role {
    fn sorted<T: Ord>(list: &mut Vec<T>) {
        list.sort();
        list.dedup();
    }
    let mut role = role.clone();
    for grant in role.object_types.values_mut() {
        sorted(&mut grant.properties);
    }
    sorted(&mut role.actions);
    sorted(&mut role.sql_levels);
    sorted(&mut role.toolsets);
    sorted(&mut role.l2.catalogs);
    sorted(&mut role.l2.schemas);
    role
}

/// `scope` as the store holds it: normalized roles, subjects sorted by kind and name without
/// duplicates, and no binding without a subject.
#[must_use]
pub fn normalize_scope(scope: &ScopeSet) -> ScopeSet {
    let roles = scope
        .roles
        .iter()
        .map(|(name, role)| (name.clone(), normalize_role(role)))
        .collect();
    scope_from_parts(roles, binding_rows(scope))
}

/// The roles and bindings of a scope set, bindings as sorted rows.
fn binding_rows(scope: &ScopeSet) -> BTreeSet<BindingRow> {
    scope
        .bindings
        .values()
        .flat_map(|binding| {
            binding
                .subjects
                .iter()
                .map(|subject| BindingRow::of(&binding.role, subject))
        })
        .collect()
}

fn scope_from_parts(roles: BTreeMap<RoleName, Role>, rows: BTreeSet<BindingRow>) -> ScopeSet {
    let mut bindings: BTreeMap<RoleName, RoleBinding> = BTreeMap::new();
    for row in rows {
        let Ok(role) = RoleName::try_from(row.role.clone()) else {
            continue;
        };
        let subject = row.subject();
        bindings
            .entry(role.clone())
            .or_insert_with(|| RoleBinding {
                role,
                subjects: Vec::new(),
            })
            .subjects
            .push(subject);
    }
    ScopeSet { roles, bindings }
}

/// A role as written to `of_role`.
#[derive(Debug, Clone)]
struct RoleRecord {
    ir_json: String,
    hash: String,
}

/// The roles and bindings of a store, or of the scope that is to replace them.
#[derive(Debug, Default)]
struct ScopeContent {
    roles: BTreeMap<String, RoleRecord>,
    bindings: BTreeSet<BindingRow>,
}

impl ScopeContent {
    fn of(scope: &ScopeSet) -> Result<Self, StoreError> {
        let mut roles = BTreeMap::new();
        for (name, role) in &scope.roles {
            let ir_json = to_json(&normalize_role(role))?;
            let hash = fnv1a64_hex(ir_json.as_bytes());
            roles.insert(name.as_str().to_owned(), RoleRecord { ir_json, hash });
        }
        Ok(Self {
            roles,
            bindings: binding_rows(scope),
        })
    }

    /// The hash that identifies this content: every role by name and content hash, every binding.
    fn hash(&self) -> Result<String, StoreError> {
        let roles: Vec<(&str, &str)> = self
            .roles
            .iter()
            .map(|(name, record)| (name.as_str(), record.hash.as_str()))
            .collect();
        let bindings: Vec<&BindingRow> = self.bindings.iter().collect();
        Ok(fnv1a64_hex(to_json(&(roles, bindings))?.as_bytes()))
    }
}

fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value)
        .map_err(|e| StoreError::Corrupt(format!("cannot serialize a scope document: {e}")))
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn text(cell: &Cell) -> Result<String, StoreError> {
    match cell {
        Cell::Text(text) => Ok(text.clone()),
        other => Err(StoreError::Corrupt(format!(
            "expected a text column, found {other:?}"
        ))),
    }
}

/// The roles and bindings in the store, read in the transaction `tx`.
async fn read_stored(tx: &mut WriteTx<'_>) -> Result<ScopeContent, StoreError> {
    let backend = tx.backend();
    let roles = Query::select()
        .column(Alias::new("api_name"))
        .expr(sql_exit::column_text(backend, "ir_json"))
        .column(Alias::new("content_hash"))
        .from(Alias::new(ROLE_TABLE))
        .to_owned();
    let built = sql_exit::build(&roles, tx);
    let rows = sql_exit::fetch_all_cells(tx, built, &[CellKind::Text; 3]).await?;
    let mut content = ScopeContent::default();
    for row in rows {
        let [name, ir_json, hash] = row.as_slice() else {
            return Err(StoreError::Corrupt("a role row has the wrong shape".into()));
        };
        content.roles.insert(
            text(name)?,
            RoleRecord {
                ir_json: text(ir_json)?,
                hash: text(hash)?,
            },
        );
    }

    let bindings = Query::select()
        .column(Alias::new("role"))
        .column(Alias::new("subject_kind"))
        .column(Alias::new("subject"))
        .from(Alias::new(BINDING_TABLE))
        .to_owned();
    let built = sql_exit::build(&bindings, tx);
    let rows = sql_exit::fetch_all_cells(tx, built, &[CellKind::Text; 3]).await?;
    for row in rows {
        let [role, kind, subject] = row.as_slice() else {
            return Err(StoreError::Corrupt(
                "a role binding row has the wrong shape".into(),
            ));
        };
        content.bindings.insert(BindingRow {
            role: text(role)?,
            kind: SubjectKind::parse(&text(kind)?)?,
            subject: text(subject)?,
        });
    }
    Ok(content)
}

/// The indexes the row predicates need that do not exist yet, as statements.
async fn missing_index_ddl(
    tx: &mut WriteTx<'_>,
    physical: Option<&PhysicalSchema>,
    props: &RowFilterProperties,
) -> Result<Vec<String>, StoreError> {
    let Some(physical) = physical else {
        return Ok(Vec::new());
    };
    let steps = index::role_index_steps(physical, props);
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    let existing = sql_exit::existing_index_names(tx).await?;
    let backend = tx.backend();
    let schema = tx.schema_edit().map(str::to_owned);
    Ok(steps
        .iter()
        .filter(|step| match step {
            DdlStep::CreateIndex { index, .. } => !existing.contains(&index.name),
            _ => true,
        })
        .flat_map(|step| ddl::render(step, backend, schema.as_deref()))
        .collect())
}

/// The plan that takes the store from what it holds to `desired`, made in the transaction `tx`.
async fn plan_in(
    tx: &mut WriteTx<'_>,
    desired: &ScopeContent,
    props: &RowFilterProperties,
    physical: Option<&PhysicalSchema>,
) -> Result<ScopePlan, StoreError> {
    let stored = read_stored(tx).await?;
    let names = |pick: &dyn Fn(&str, &RoleRecord) -> bool| -> Vec<String> {
        desired
            .roles
            .iter()
            .filter(|(name, record)| pick(name, record))
            .map(|(name, _)| name.clone())
            .collect()
    };
    let mut plan = ScopePlan {
        roles_added: names(&|name, _| !stored.roles.contains_key(name)),
        roles_changed: names(&|name, record| {
            stored
                .roles
                .get(name)
                .is_some_and(|old| old.hash != record.hash)
        }),
        roles_removed: stored
            .roles
            .keys()
            .filter(|name| !desired.roles.contains_key(*name))
            .cloned()
            .collect(),
        bindings_added: desired
            .bindings
            .difference(&stored.bindings)
            .cloned()
            .collect(),
        bindings_removed: stored
            .bindings
            .difference(&desired.bindings)
            .cloned()
            .collect(),
        index_ddl: missing_index_ddl(tx, physical, props).await?,
        before_hash: stored.hash()?,
        after_hash: desired.hash()?,
        hash: String::new(),
    };
    plan.hash = fnv1a64_hex(to_json(&plan)?.as_bytes());
    Ok(plan)
}

/// The roles and bindings in the store as a scope set, read in the transaction `tx`. `reload`
/// reads them under the definition lock, so a role cannot arrive between its role check and its
/// switch.
pub(crate) async fn load_scope_in(tx: &mut WriteTx<'_>) -> Result<ScopeSet, StoreError> {
    let stored = read_stored(tx).await?;
    let mut roles = BTreeMap::new();
    for (name, record) in stored.roles {
        let role: Role = serde_json::from_str(&record.ir_json).map_err(|e| {
            StoreError::Corrupt(format!("the stored role `{name}` cannot be read: {e}"))
        })?;
        roles.insert(role.api_name.clone(), role);
    }
    Ok(scope_from_parts(roles, stored.bindings))
}

impl EditStore {
    /// The roles and bindings in the store as a scope set.
    pub async fn load_scope(&self) -> Result<ScopeSet, StoreError> {
        let mut tx = self.begin_write().await?;
        let scope = load_scope_in(&mut tx).await;
        sql_exit::rollback(tx).await;
        scope
    }

    /// What applying `desired` would change. `row_filter_properties` are the properties its row
    /// predicates read (`CompiledScope::row_filter_properties`); the plan lists the indexes they
    /// still need. Nothing is written.
    pub async fn scope_plan(
        &self,
        desired: &ScopeSet,
        row_filter_properties: &RowFilterProperties,
    ) -> Result<ScopePlan, StoreError> {
        let desired = ScopeContent::of(desired)?;
        let active = self.active_definition().await;
        let mut tx = self.begin_write().await?;
        let plan = plan_in(
            &mut tx,
            &desired,
            row_filter_properties,
            active.as_ref().map(|a| &a.physical),
        )
        .await;
        sql_exit::rollback(tx).await;
        plan
    }

    /// Makes the store hold exactly `desired`, if its plan still has the hash `expected_hash`.
    ///
    /// The plan is made again inside the write transaction, under the lock `reload` and
    /// `migrate` take, so what is written is what was reviewed. `actor` goes to the audit record.
    pub async fn scope_apply(
        &self,
        desired: &ScopeSet,
        row_filter_properties: &RowFilterProperties,
        expected_hash: &str,
        actor: &str,
    ) -> Result<AppliedScope, ScopeApplyError> {
        let desired = ScopeContent::of(desired)?;
        let mut tx = self.begin_write().await?;
        let result = apply_in(
            &mut tx,
            self.settings.lock_timeout,
            &desired,
            row_filter_properties,
            expected_hash,
            actor,
        )
        .await;
        match result {
            Ok(applied) => {
                sql_exit::commit(tx).await?;
                Ok(applied)
            }
            Err(error) => {
                sql_exit::rollback(tx).await;
                Err(error)
            }
        }
    }

    /// Records that `principal` authenticated: the first call adds its `of_user` row, later calls
    /// move `last_login_at`. Only the identifier, issuer, subject and the two times are stored,
    /// never a group, a claim or a permission (D-04).
    pub async fn record_login(&self, principal: &Principal) -> Result<(), StoreError> {
        let mut tx = self.begin_write().await?;
        let result = login_in(&mut tx, principal).await;
        match result {
            Ok(()) => sql_exit::commit(tx).await,
            Err(error) => {
                sql_exit::rollback(tx).await;
                Err(error)
            }
        }
    }
}

async fn apply_in(
    tx: &mut WriteTx<'_>,
    lock_timeout: Duration,
    desired: &ScopeContent,
    props: &RowFilterProperties,
    expected_hash: &str,
    actor: &str,
) -> Result<AppliedScope, ScopeApplyError> {
    // The timeout first: it bounds the wait for the advisory lock as well.
    let millis = u64::try_from(lock_timeout.as_millis()).unwrap_or(u64::MAX);
    sql_exit::set_local_lock_timeout(tx, millis).await?;
    sql_exit::advisory_xact_lock(tx).await?;
    let active = sql_exit::read_active_definition_in(tx)
        .await?
        .ok_or(ScopeApplyError::NoActiveDefinition)?;

    let plan = plan_in(tx, desired, props, Some(&active.physical)).await?;
    if plan.hash != expected_hash {
        return Err(ScopeApplyError::PlanChanged {
            expected: expected_hash.to_owned(),
            actual: plan.hash,
        });
    }

    let at = now();
    // Bindings before roles when something leaves (a binding refers to its role), roles before
    // bindings when something arrives.
    for row in &plan.bindings_removed {
        delete_binding(tx, row).await?;
    }
    for name in &plan.roles_removed {
        delete_role(tx, name).await?;
    }
    for name in &plan.roles_added {
        insert_role(tx, name, &desired.roles[name], &at).await?;
    }
    for name in &plan.roles_changed {
        update_role(tx, name, &desired.roles[name], &at).await?;
    }
    for row in &plan.bindings_added {
        insert_binding(tx, row).await?;
    }
    for statement in &plan.index_ddl {
        sql_exit::execute_ddl(tx, statement).await?;
    }
    let audit_seq = insert_audit(tx, &plan, actor, &at).await?;
    Ok(AppliedScope { plan, audit_seq })
}

async fn delete_binding(tx: &mut WriteTx<'_>, row: &BindingRow) -> Result<(), StoreError> {
    let delete = Query::delete()
        .from_table(Alias::new(BINDING_TABLE))
        .and_where(Expr::col(Alias::new("role")).eq(row.role.as_str()))
        .and_where(Expr::col(Alias::new("subject_kind")).eq(row.kind.as_str()))
        .and_where(Expr::col(Alias::new("subject")).eq(row.subject.as_str()))
        .to_owned();
    let built = sql_exit::build(&delete, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

async fn delete_role(tx: &mut WriteTx<'_>, name: &str) -> Result<(), StoreError> {
    let delete = Query::delete()
        .from_table(Alias::new(ROLE_TABLE))
        .and_where(Expr::col(Alias::new("api_name")).eq(name))
        .to_owned();
    let built = sql_exit::build(&delete, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

async fn insert_role(
    tx: &mut WriteTx<'_>,
    name: &str,
    record: &RoleRecord,
    at: &str,
) -> Result<(), StoreError> {
    let backend = tx.backend();
    let insert = Query::insert()
        .into_table(Alias::new(ROLE_TABLE))
        .columns([
            Alias::new("api_name"),
            Alias::new("ir_json"),
            Alias::new("content_hash"),
            Alias::new("applied_at"),
        ])
        .values_panic([
            name.into(),
            sql_exit::typed_text(backend, &record.ir_json, "jsonb"),
            record.hash.as_str().into(),
            sql_exit::typed_text(backend, at, "timestamptz"),
        ])
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

async fn update_role(
    tx: &mut WriteTx<'_>,
    name: &str,
    record: &RoleRecord,
    at: &str,
) -> Result<(), StoreError> {
    let backend = tx.backend();
    let update = Query::update()
        .table(Alias::new(ROLE_TABLE))
        .value(
            Alias::new("ir_json"),
            sql_exit::typed_text(backend, &record.ir_json, "jsonb"),
        )
        .value(Alias::new("content_hash"), record.hash.as_str())
        .value(
            Alias::new("applied_at"),
            sql_exit::typed_text(backend, at, "timestamptz"),
        )
        .and_where(Expr::col(Alias::new("api_name")).eq(name))
        .to_owned();
    let built = sql_exit::build(&update, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

async fn insert_binding(tx: &mut WriteTx<'_>, row: &BindingRow) -> Result<(), StoreError> {
    let insert = Query::insert()
        .into_table(Alias::new(BINDING_TABLE))
        .columns([
            Alias::new("role"),
            Alias::new("subject_kind"),
            Alias::new("subject"),
        ])
        .values_panic([
            row.role.as_str().into(),
            row.kind.as_str().into(),
            row.subject.as_str().into(),
        ])
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

/// Appends the audit record and returns its sequence number.
async fn insert_audit(
    tx: &mut WriteTx<'_>,
    plan: &ScopePlan,
    actor: &str,
    at: &str,
) -> Result<i64, StoreError> {
    let backend = tx.backend();
    let insert = Query::insert()
        .into_table(Alias::new(AUDIT_TABLE))
        .columns([
            Alias::new("at"),
            Alias::new("actor"),
            Alias::new("action"),
            Alias::new("plan_json"),
            Alias::new("before_hash"),
            Alias::new("after_hash"),
        ])
        .values_panic([
            sql_exit::typed_text(backend, at, "timestamptz"),
            actor.into(),
            APPLY_ACTION.into(),
            sql_exit::typed_text(backend, &to_json(plan)?, "jsonb"),
            plan.before_hash.as_str().into(),
            plan.after_hash.as_str().into(),
        ])
        .returning_col(Alias::new("seq"))
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::fetch_scalar_i64(tx, built)
        .await?
        .ok_or_else(|| StoreError::Corrupt("the audit record has no sequence number".into()))
}

async fn login_in(tx: &mut WriteTx<'_>, principal: &Principal) -> Result<(), StoreError> {
    let backend = tx.backend();
    let at = now();
    let insert = Query::insert()
        .into_table(Alias::new(USER_TABLE))
        .columns([
            Alias::new("principal_id"),
            Alias::new("issuer"),
            Alias::new("subject"),
            Alias::new("created_at"),
            Alias::new("last_login_at"),
        ])
        .values_panic([
            principal.id.as_str().into(),
            principal.issuer.as_str().into(),
            principal.subject.as_str().into(),
            sql_exit::typed_text(backend, &at, "timestamptz"),
            sql_exit::typed_text(backend, &at, "timestamptz"),
        ])
        .on_conflict(
            OnConflict::column(Alias::new("principal_id"))
                .update_column(Alias::new("last_login_at"))
                .to_owned(),
        )
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

/// Writes a login at most once per interval and principal (D-04): a busy API would otherwise
/// write a row on every request.
pub struct LoginRecorder {
    interval: Duration,
    last: Mutex<HashMap<String, Instant>>,
}

impl LoginRecorder {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Mutex::new(HashMap::new()),
        }
    }

    /// Records the login unless this process wrote one for `principal` within the interval.
    /// Returns whether the store was written.
    pub async fn record(
        &self,
        store: &EditStore,
        principal: &Principal,
    ) -> Result<bool, StoreError> {
        let key = principal.id.as_str();
        {
            let mut last = self
                .last
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if last
                .get(key)
                .is_some_and(|written| written.elapsed() < self.interval)
            {
                return Ok(false);
            }
            last.insert(key.to_owned(), Instant::now());
        }
        if let Err(error) = store.record_login(principal).await {
            // Nothing was written, so the next request may try again.
            self.last
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(key);
            return Err(error);
        }
        Ok(true)
    }
}
