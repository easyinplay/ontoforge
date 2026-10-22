use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::ObjectTypeName;
use serde_json::Value;

use crate::ast::OrderBy;
use crate::{QueryError, TokenCodec};

#[derive(Debug, Clone, PartialEq)]
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
    order: Vec<OrderBy>,
}

impl CursorContext {
    pub fn new(principal_id: &str, scope: &PrincipalScope, def_version: i64, object_type: ObjectTypeName, order: Vec<OrderBy>) -> Self {
        Self { principal_id: principal_id.to_owned(), scope: format!("{scope:?}"), def_version, object_type, order }
    }
}

pub struct CursorCodec<'a> {
    _tokens: &'a TokenCodec,
}

impl<'a> CursorCodec<'a> {
    pub fn new(tokens: &'a TokenCodec) -> Self { Self { _tokens: tokens } }
    pub fn encode(&self, _context: &CursorContext, _position: &CursorPosition) -> Result<String, QueryError> { Err(QueryError::InvalidCursor) }
    pub fn decode(&self, _token: &str, _context: &CursorContext) -> Result<CursorPosition, QueryError> { Err(QueryError::InvalidCursor) }
}
