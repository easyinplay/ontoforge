use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::ObjectTypeName;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::overlay::NormalizedOrder;
use crate::{QueryError, TokenCodec};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CursorPosition {
    pub sort_values: Vec<Value>,
    pub pk: Value,
}

#[derive(Debug, Clone)]
pub struct CursorContext {
    principal_id: String,
    scope: String,
    def_version: i64,
    object_type: ObjectTypeName,
    order: Vec<NormalizedOrder>,
}

impl CursorContext {
    pub fn new(
        principal_id: &str,
        scope: &PrincipalScope,
        def_version: i64,
        object_type: ObjectTypeName,
        order: Vec<NormalizedOrder>,
    ) -> Self {
        Self {
            principal_id: principal_id.to_owned(),
            scope: format!("{scope:?}"),
            def_version,
            object_type,
            order,
        }
    }
}

pub struct CursorCodec<'a> {
    tokens: &'a TokenCodec,
}

impl<'a> CursorCodec<'a> {
    pub fn new(tokens: &'a TokenCodec) -> Self {
        Self { tokens }
    }

    pub fn encode(
        &self,
        context: &CursorContext,
        position: &CursorPosition,
    ) -> Result<String, QueryError> {
        if position.sort_values.len() != context.order.len() || position.pk.is_null() {
            return Err(QueryError::InvalidCursor);
        }
        let payload = Payload {
            version: 1,
            principal_id: context.principal_id.clone(),
            scope_fingerprint: self.scope_fingerprint(context)?,
            def_version: context.def_version,
            object_type: context.object_type.clone(),
            order: context.order.clone(),
            position: position.clone(),
        };
        let bytes = serde_json::to_vec(&payload).map_err(|_| QueryError::InvalidCursor)?;
        let mac = self
            .tokens
            .sign_version(&[b"cursor-v1", &bytes])
            .map_err(|_| QueryError::InvalidCursor)?;
        Ok(format!("{}.{}", URL_SAFE_NO_PAD.encode(bytes), mac))
    }

    pub fn decode(
        &self,
        token: &str,
        context: &CursorContext,
    ) -> Result<CursorPosition, QueryError> {
        if token.len() > 16_384 {
            return Err(QueryError::InvalidCursor);
        }
        let (encoded, mac) = token.split_once('.').ok_or(QueryError::InvalidCursor)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| QueryError::InvalidCursor)?;
        self.tokens
            .verify_version(mac, &[b"cursor-v1", &bytes])
            .map_err(|_| QueryError::InvalidCursor)?;
        let payload: Payload =
            serde_json::from_slice(&bytes).map_err(|_| QueryError::InvalidCursor)?;
        let scope_fingerprint = self.scope_fingerprint(context)?;
        let checks = [
            payload.version == 1,
            payload.principal_id == context.principal_id,
            payload.scope_fingerprint == scope_fingerprint,
            payload.def_version == context.def_version,
            payload.object_type == context.object_type,
            payload.order == context.order,
            payload.position.sort_values.len() == context.order.len(),
            !payload.position.pk.is_null(),
        ];
        if !checks.into_iter().all(|check| check) {
            return Err(QueryError::InvalidCursor);
        }
        Ok(payload.position)
    }

    fn scope_fingerprint(&self, context: &CursorContext) -> Result<String, QueryError> {
        self.tokens
            .sign_version(&[b"scope-v1", context.scope.as_bytes()])
            .map_err(|_| QueryError::InvalidCursor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Payload {
    version: u8,
    principal_id: String,
    scope_fingerprint: String,
    def_version: i64,
    object_type: ObjectTypeName,
    order: Vec<NormalizedOrder>,
    position: CursorPosition,
}
