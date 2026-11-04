use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ontoforge_auth::{Principal, scope::PrincipalScope};
use ontoforge_expr::{AttrValue, Attrs, Scalar};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_storage::{state::EditRow, value::PropertyValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::overlay::NormalizedOrder;
use crate::{QueryError, TokenCodec};

/// Inputs to the object version signed by both Query and Action.
pub struct VersionClaims<'a> {
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub object_type: &'a ObjectTypeName,
    pub pk: &'a PropertyValue,
    pub def_version: i64,
    pub source: Option<&'a BTreeMap<PropertyName, Value>>,
    pub edit: Option<&'a EditRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionTokenError {
    UnsupportedFormat,
    Invalid,
}

/// A format-versioned object token. Legacy untagged signatures are never accepted.
pub struct VersionTokenCodec<'a> {
    tokens: &'a TokenCodec,
}

impl<'a> VersionTokenCodec<'a> {
    pub fn new(tokens: &'a TokenCodec) -> Self {
        Self { tokens }
    }

    pub fn sign(&self, claims: &VersionClaims<'_>) -> Result<String, VersionTokenError> {
        let message = claims.message()?;
        let signature = self
            .tokens
            .sign_version(&[b"object-version-v1", &message])
            .map_err(|_| VersionTokenError::Invalid)?;
        Ok(format!("v1.{signature}"))
    }

    pub fn verify(&self, token: &str, claims: &VersionClaims<'_>) -> Result<(), VersionTokenError> {
        let signature = token
            .strip_prefix("v1.")
            .ok_or(VersionTokenError::UnsupportedFormat)?;
        if signature.is_empty() || signature.contains('.') {
            return Err(VersionTokenError::Invalid);
        }
        let message = claims.message()?;
        self.tokens
            .verify_version(signature, &[b"object-version-v1", &message])
            .map_err(|_| VersionTokenError::Invalid)
    }
}

impl VersionClaims<'_> {
    fn message(&self) -> Result<Vec<u8>, VersionTokenError> {
        let ty = self
            .scope
            .types
            .get(self.object_type)
            .ok_or(VersionTokenError::Invalid)?;
        let attrs: BTreeMap<_, _> = self
            .principal
            .attrs
            .iter()
            .map(|(name, value)| Ok((name.to_owned(), attr_value(value)?)))
            .collect::<Result<_, VersionTokenError>>()?;
        let columns: BTreeMap<_, _> = ty
            .columns
            .iter()
            .map(|(name, pred)| (name.as_str(), pred.to_string()))
            .collect();
        let edit = self.edit.map(|row| {
            json!({
                "version": row.version,
                "deleted": row.deleted,
                "released": row.released,
                "set": row.set.iter().map(|(name, value)| (name.as_str(), value.to_json()))
                    .collect::<BTreeMap<_, _>>(),
            })
        });
        let claims = json!({
            "principalId": self.principal.id.as_str(),
            "attributes": attrs,
            "scope": { "row": ty.row.to_string(), "columns": columns },
            "objectType": self.object_type.as_str(),
            "pk": self.pk.to_json(),
            "defVersion": self.def_version,
            "source": self.source,
            "edit": edit,
        });
        serde_json::to_vec(&canonical_json(&claims)).map_err(|_| VersionTokenError::Invalid)
    }
}

fn attr_value(value: &AttrValue) -> Result<Value, VersionTokenError> {
    match value {
        AttrValue::Null => Ok(Value::Null),
        AttrValue::Scalar(value) => scalar(value),
        AttrValue::List(items) => items
            .iter()
            .map(scalar)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
    }
}

fn scalar(value: &Scalar) -> Result<Value, VersionTokenError> {
    match value {
        Scalar::Null => Ok(Value::Null),
        Scalar::Bool(value) => Ok(Value::Bool(*value)),
        Scalar::Int(value) => Ok(json!(value)),
        Scalar::Double(value) => serde_json::Number::from_f64(*value)
            .map(Value::Number)
            .ok_or(VersionTokenError::Invalid),
        Scalar::String(value) => Ok(Value::String(value.clone())),
    }
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut fields: Vec<_> = map.iter().collect();
            fields.sort_unstable_by_key(|(name, _)| *name);
            Value::Object(
                fields
                    .into_iter()
                    .map(|(name, value)| (name.clone(), canonical_json(value)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        _ => value.clone(),
    }
}

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
        attrs: &Attrs,
        def_version: i64,
        object_type: ObjectTypeName,
        order: Vec<NormalizedOrder>,
    ) -> Self {
        Self {
            principal_id: principal_id.to_owned(),
            scope: format!("{scope:?}|{attrs:?}"),
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
