//! Scoped L0 reads with a single Rust source/edit overlay (ADR-0020).

use std::collections::BTreeMap;
use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{Engine, QueryContext, TableRef};
use ontoforge_expr::Scalar;
use ontoforge_schema::ir::{Backing, ObjectTypeName, PropertyName};
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::read_audit::{ReadAuditGuard, ReadAuditMetadata, ReadAuditStatus};
use ontoforge_storage::state::EditRow;
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use ontoforge_storage::value::PropertyValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod aggregate;
pub mod ast;
pub mod compile;
pub mod cursor;
pub mod execute;
pub mod l2;
pub mod link;
pub mod overlay;
pub mod set;
pub use ast::ObjectSet;

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QueryRequest {
    pub set: ObjectSet,
    pub select: Vec<PropertyName>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRow {
    pub object_type: ObjectTypeName,
    pub pk: Value,
    pub properties: BTreeMap<PropertyName, Value>,
    #[serde(rename = "_version")]
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryPage {
    pub rows: Vec<ObjectRow>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPage {
    pub rows: Vec<ObjectRow>,
    pub has_more: bool,
    pub truncated: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum QueryError {
    #[error("object not found")]
    NotFound,
    #[error("object key is ambiguous")]
    AmbiguousObject,
    #[error("invalid query")]
    InvalidQuery,
    #[error("query cost limit exceeded")]
    CostExceeded,
    #[error("query is too complex")]
    QueryTooComplex,
    #[error("source unavailable")]
    Source,
    #[error("unsupported numeric value")]
    UnsupportedNumericValue,
    #[error("edit store unavailable")]
    Store,
    #[error("read audit unavailable")]
    AuditUnavailable,
    #[error("invalid version token")]
    InvalidToken,
    #[error("query work budget exceeded")]
    WorkBudgetExceeded,
    #[error("invalid cursor")]
    InvalidCursor,
}

impl QueryError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::AmbiguousObject => "AMBIGUOUS_OBJECT",
            Self::InvalidQuery => "INVALID_QUERY",
            Self::CostExceeded => "COST_EXCEEDED",
            Self::QueryTooComplex => "QUERY_TOO_COMPLEX",
            Self::Source => "SOURCE_UNAVAILABLE",
            Self::UnsupportedNumericValue => "UNSUPPORTED_NUMERIC_VALUE",
            Self::Store => "STORE_UNAVAILABLE",
            Self::AuditUnavailable => "AUDIT_UNAVAILABLE",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::WorkBudgetExceeded => "WORK_BUDGET_EXCEEDED",
            Self::InvalidCursor => "INVALID_CURSOR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub max_backend_calls: usize,
    pub max_raw_candidates: usize,
    pub max_backfill_keys: usize,
    pub max_decoded_bytes: usize,
}

impl ExecutionLimits {
    #[must_use]
    pub fn phase4_default() -> Self {
        Self {
            max_backend_calls: 32,
            max_raw_candidates: 10_000,
            max_backfill_keys: 2_048,
            max_decoded_bytes: 64 * 1024 * 1024,
        }
    }

    pub fn try_new(
        max_backend_calls: usize,
        max_raw_candidates: usize,
        max_backfill_keys: usize,
        max_decoded_bytes: usize,
    ) -> Result<Self, QueryError> {
        if [
            max_backend_calls,
            max_raw_candidates,
            max_backfill_keys,
            max_decoded_bytes,
        ]
        .iter()
        .any(|&value| value == 0 || i64::try_from(value).is_err())
        {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self {
            max_backend_calls,
            max_raw_candidates,
            max_backfill_keys,
            max_decoded_bytes,
        })
    }
}

pub struct TokenCodec {
    key: Vec<u8>,
}

impl TokenCodec {
    pub fn new(key: Vec<u8>) -> Result<Self, QueryError> {
        if key.is_empty() {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self { key })
    }

    pub fn sign_version(&self, fields: &[&[u8]]) -> Result<String, QueryError> {
        use jsonwebtoken::{Algorithm, EncodingKey};
        let message = version_message(fields)?;
        jsonwebtoken::crypto::sign(
            &message,
            &EncodingKey::from_secret(&self.key),
            Algorithm::HS256,
        )
        .map_err(|_| QueryError::InvalidToken)
    }

    pub fn verify_version(&self, token: &str, fields: &[&[u8]]) -> Result<(), QueryError> {
        use jsonwebtoken::{Algorithm, DecodingKey};
        let message = version_message(fields)?;
        match jsonwebtoken::crypto::verify(
            token,
            &message,
            &DecodingKey::from_secret(&self.key),
            Algorithm::HS256,
        ) {
            Ok(true) => Ok(()),
            Ok(false) | Err(_) => Err(QueryError::InvalidToken),
        }
    }
}

impl QueryPage {
    #[must_use]
    pub fn bounded(rows: Vec<ObjectRow>, max_rows: usize, max_bytes: usize) -> Self {
        let mut kept = Vec::new();
        let mut bytes = 0usize;
        for row in rows {
            let row_bytes = serde_json::to_vec(&row).map_or(usize::MAX, |encoded| encoded.len());
            let next = bytes.checked_add(row_bytes);
            if kept.len() >= max_rows || next.is_none_or(|n| n > max_bytes) {
                return Self {
                    rows: kept,
                    truncated: true,
                };
            }
            bytes = next.expect("checked above");
            kept.push(row);
        }
        Self {
            rows: kept,
            truncated: false,
        }
    }
}

fn version_message(fields: &[&[u8]]) -> Result<Vec<u8>, QueryError> {
    let mut message = Vec::new();
    for field in fields {
        let len = u64::try_from(field.len()).map_err(|_| QueryError::CostExceeded)?;
        message.extend_from_slice(&len.to_be_bytes());
        message.extend_from_slice(field);
    }
    Ok(message)
}

pub struct QueryService<'a, E: Engine> {
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub active: Arc<ActiveDefinition>,
    pub engine: &'a E,
    pub store: &'a EditStore,
    pub limits: ExecutionLimits,
    pub tokens: &'a TokenCodec,
}

impl<E: Engine> QueryService<'_, E> {
    pub async fn execute(&self, request: QueryRequest) -> Result<QueryPage, QueryError> {
        let object_type = request.set.object_type();
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: "query.pk".to_owned(),
                object_type: Some(object_type.as_str().to_owned()),
                condition_api_names: self
                    .active
                    .ontology
                    .object_types
                    .get(object_type)
                    .map(|ty| vec![ty.primary_key.as_str().to_owned()])
                    .unwrap_or_default(),
                effective_roles: self
                    .scope
                    .roles
                    .iter()
                    .map(|role| role.as_str().to_owned())
                    .collect(),
                def_version: self.active.def_version,
            })
            .map_err(|_| QueryError::AuditUnavailable)?;
        let result = self.execute_pk(request, &mut audit).await;
        let (status, count) = match &result {
            Ok(page) => (ReadAuditStatus::Success, page.rows.len()),
            Err(QueryError::Source | QueryError::Store) => (ReadAuditStatus::SourceError, 0),
            Err(_) => (ReadAuditStatus::Denied, 0),
        };
        audit
            .finish(status, count)
            .map_err(|_| QueryError::AuditUnavailable)?;
        result
    }

    async fn execute_pk(
        &self,
        request: QueryRequest,
        audit: &mut ReadAuditGuard,
    ) -> Result<QueryPage, QueryError> {
        let ObjectSet::Pk { object_type, pk } = request.set else {
            return Err(QueryError::InvalidQuery);
        };
        let ty = self
            .active
            .ontology
            .object_types
            .get(&object_type)
            .ok_or(QueryError::NotFound)?;
        let grant = self
            .scope
            .types
            .get(&object_type)
            .ok_or(QueryError::NotFound)?;
        let pk_def = ty
            .properties
            .get(&ty.primary_key)
            .ok_or(QueryError::Store)?;
        let canonical_pk =
            PropertyValue::from_json(pk_def, &pk).map_err(|_| QueryError::NotFound)?;
        let mut calls = 0usize;
        charge(&mut calls, 1, self.limits.max_backend_calls)?;
        let edit = self
            .store
            .read_object_edit(&self.active, &object_type, &canonical_pk)
            .await
            .map_err(|_| QueryError::Store)?;
        if edit.as_ref().is_some_and(|e| e.row.deleted) {
            return Err(QueryError::NotFound);
        }

        let all_patched = edit.as_ref().is_some_and(|e| {
            !e.row.released
                && ty
                    .properties
                    .keys()
                    .all(|p| p == &ty.primary_key || e.row.set.contains_key(p))
        });
        let source = if ty.backing == Backing::Source && !all_patched {
            charge(&mut calls, 1, self.limits.max_backend_calls)?;
            self.read_source(&object_type, &canonical_pk, audit).await?
        } else {
            None
        };
        if ty.backing == Backing::Source && !all_patched && source.is_none() {
            return Err(QueryError::NotFound);
        }
        let merged = overlay_pk(
            ty,
            &canonical_pk,
            source.as_ref(),
            edit.as_ref().map(|e| &e.row),
        );
        if source.is_none()
            && edit
                .as_ref()
                .is_none_or(|e| e.row.released || e.row.version == 0)
        {
            return Err(QueryError::NotFound);
        }
        let row = merged
            .iter()
            .map(|(p, v)| (p.clone(), scope_scalar(v)))
            .collect::<BTreeMap<_, _>>();
        let visibility = grant.evaluate(&row, &self.principal.attrs);
        if !visibility.visible
            || request
                .select
                .iter()
                .all(|p| !visibility.columns.contains(p))
        {
            return Err(QueryError::NotFound);
        }
        let properties = request
            .select
            .into_iter()
            .filter(|p| visibility.columns.contains(p))
            .filter_map(|p| merged.get(&p).map(|v| (p, v.clone())))
            .collect();
        let version =
            self.version_for(&object_type, &canonical_pk, source.as_ref(), edit.as_ref())?;
        let row = ObjectRow {
            object_type,
            pk: canonical_pk.to_json(),
            properties,
            version,
        };
        Ok(QueryPage::bounded(
            vec![row],
            self.limits.max_raw_candidates,
            self.limits.max_decoded_bytes,
        ))
    }

    fn version_for(
        &self,
        object_type: &ObjectTypeName,
        pk: &PropertyValue,
        source: Option<&BTreeMap<PropertyName, Value>>,
        edit: Option<&ObjectEdit>,
    ) -> Result<String, QueryError> {
        cursor::VersionTokenCodec::new(self.tokens)
            .sign(&cursor::VersionClaims {
                principal: self.principal,
                scope: self.scope,
                object_type,
                pk,
                def_version: self.active.def_version,
                source,
                edit: edit.map(|value| &value.row),
            })
            .map_err(|_| QueryError::InvalidToken)
    }

    async fn read_source(
        &self,
        type_name: &ObjectTypeName,
        pk: &PropertyValue,
        audit: &mut ReadAuditGuard,
    ) -> Result<Option<BTreeMap<PropertyName, Value>>, QueryError> {
        read_pk_source_with_audit(
            &self.active,
            self.engine,
            type_name,
            pk,
            self.limits.max_raw_candidates.min(3),
            self.limits.max_decoded_bytes,
            Some(audit),
        )
        .await
    }
}

/// The canonical bounded source lookup shared by Query and Action before its write transaction.
pub async fn read_pk_source<E: Engine>(
    active: &ActiveDefinition,
    engine: &E,
    type_name: &ObjectTypeName,
    pk: &PropertyValue,
    max_rows: usize,
    max_decoded_bytes: usize,
) -> Result<Option<BTreeMap<PropertyName, Value>>, QueryError> {
    read_pk_source_with_audit(
        active,
        engine,
        type_name,
        pk,
        max_rows,
        max_decoded_bytes,
        None,
    )
    .await
}

async fn read_pk_source_with_audit<E: Engine>(
    active: &ActiveDefinition,
    engine: &E,
    type_name: &ObjectTypeName,
    pk: &PropertyValue,
    max_rows: usize,
    max_decoded_bytes: usize,
    audit: Option<&mut ReadAuditGuard>,
) -> Result<Option<BTreeMap<PropertyName, Value>>, QueryError> {
    let ontology = &active.ontology;
    let profile = ontology
        .default_profile
        .as_ref()
        .ok_or(QueryError::Source)?;
    let binding = ontology
        .profile_bindings(profile)
        .and_then(|p| p.objects.get(type_name))
        .ok_or(QueryError::Source)?;
    let ty = ontology
        .object_types
        .get(type_name)
        .ok_or(QueryError::NotFound)?;
    let columns = ty.properties.keys().cloned().collect::<Vec<_>>();
    let pk_column = binding.column_for(ty.primary_key.as_str());
    let select = Select {
        projection: columns
            .iter()
            .map(|p| SelectItem {
                expr: Expr::Column(binding.column_for(p.as_str()).to_owned()),
                alias: Some(p.as_str().to_owned()),
            })
            .collect(),
        from: Some(TableRef {
            catalog: binding.source.catalog.clone(),
            schema: binding.source.schema.clone(),
            table: binding.source.table.clone(),
        }),
        filter: Some(Expr::Compare {
            left: Box::new(Expr::Column(pk_column.to_owned())),
            op: CompareOp::Eq,
            right: Box::new(Expr::Literal(pk_literal(pk)?)),
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: Some(2),
    };
    let sql = print_select(&select, engine.dialect()).map_err(|_| QueryError::Source)?;
    if let Some(audit) = audit {
        audit.observe_sql(sql.as_str());
    }
    let mut ctx = QueryContext::new("l0-pk".to_owned(), Vec::new());
    ctx.max_rows = max_rows;
    ctx.max_bytes = max_decoded_bytes;
    let output = engine
        .execute(&sql, &ctx)
        .await
        .map_err(|_| QueryError::Source)?;
    if output.truncated {
        return Err(QueryError::CostExceeded);
    }
    if output.rows.len() > 1 {
        return Err(QueryError::AmbiguousObject);
    }
    let Some(values) = output.rows.into_iter().next() else {
        return Ok(None);
    };
    if values.len() != columns.len() {
        return Err(QueryError::Source);
    }
    let mut row = BTreeMap::new();
    for (property, value) in columns.into_iter().zip(values) {
        row.insert(property, engine_json(value)?);
    }
    Ok(Some(row))
}

/// Applies an edit overlay to a sampled source row with the same semantics as an L0 pk read.
pub fn overlay_pk(
    ty: &ontoforge_schema::ir::ObjectType,
    pk: &PropertyValue,
    source: Option<&BTreeMap<PropertyName, Value>>,
    edit: Option<&EditRow>,
) -> BTreeMap<PropertyName, Value> {
    let mut merged = source.cloned().unwrap_or_default();
    merged.insert(ty.primary_key.clone(), pk.to_json());
    if let Some(edit) = edit
        && !edit.released
    {
        for (property, value) in &edit.set {
            merged.insert(property.clone(), value.to_json());
        }
    }
    merged
}

fn charge(current: &mut usize, amount: usize, cap: usize) -> Result<(), QueryError> {
    *current = current
        .checked_add(amount)
        .ok_or(QueryError::CostExceeded)?;
    if *current > cap {
        return Err(QueryError::CostExceeded);
    }
    Ok(())
}

fn pk_literal(value: &PropertyValue) -> Result<Literal, QueryError> {
    match value {
        PropertyValue::String(s) => Ok(Literal::String(s.clone())),
        PropertyValue::Int(i) => Ok(Literal::Long(*i)),
        _ => Err(QueryError::NotFound),
    }
}

fn engine_json(value: ontoforge_engine::Value) -> Result<Value, QueryError> {
    use ontoforge_engine::Value as V;
    match value {
        V::Null => Ok(Value::Null),
        V::Boolean(v) => Ok(Value::Bool(v)),
        V::Long(v) => Ok(Value::from(v)),
        V::Double(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or(QueryError::UnsupportedNumericValue),
        V::Decimal(v) | V::String(v) | V::Date(v) | V::Timestamp(v) | V::TimestampTz(v) => {
            Ok(Value::String(v))
        }
        V::Array(values) => values
            .into_iter()
            .map(engine_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        V::Unsupported(_) => Err(QueryError::Source),
    }
}

pub(crate) fn scope_scalar(value: &Value) -> Scalar {
    match value {
        Value::Bool(v) => Scalar::Bool(*v),
        Value::Number(v) => v.as_i64().map_or_else(
            || v.as_f64().map_or(Scalar::Null, Scalar::Double),
            Scalar::Int,
        ),
        Value::String(v) => Scalar::String(v.clone()),
        _ => Scalar::Null,
    }
}
