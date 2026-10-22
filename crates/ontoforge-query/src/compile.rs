use std::collections::BTreeMap;

use ontoforge_auth::scope::{RowVisibility, TypeScope};
use ontoforge_engine::printer::ast::Select;
use ontoforge_expr::Tri;
use ontoforge_schema::ir::{Ontology, PropertyName, SourceBinding};
use serde_json::Value;

use crate::ast::{OrderBy, PageSpec, WhereClause};
use crate::{QueryError, QueryRequest};

#[derive(Debug, Clone, Copy)]
pub struct CompileLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_in_values: usize,
    pub max_selected: usize,
    pub max_order: usize,
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self { max_depth: 64, max_nodes: 4096, max_in_values: 256, max_selected: 256, max_order: 8 }
    }
}

#[derive(Debug)]
pub struct ValidatedQuery {
    pub request: QueryRequest,
    pub order_by: Vec<OrderBy>,
    pub page: PageSpec,
}

impl ValidatedQuery {
    pub fn new(
        _request: QueryRequest,
        _order_by: Vec<OrderBy>,
        _page: PageSpec,
        _ontology: &Ontology,
        _scope: &TypeScope,
        _limits: CompileLimits,
    ) -> Result<Self, QueryError> {
        Err(QueryError::InvalidQuery)
    }
}

pub struct SourceSelect {
    pub select: Select,
    pub required_columns: Vec<PropertyName>,
    pub final_recheck: bool,
    pub logical_order: Vec<OrderBy>,
}

pub fn compile_source_select(
    _query: &ValidatedQuery,
    _ontology: &Ontology,
    _binding: &SourceBinding,
    _scope: &TypeScope,
    _attrs: &ontoforge_expr::Attrs,
) -> Result<SourceSelect, QueryError> {
    Err(QueryError::InvalidQuery)
}

pub fn evaluate_where(
    _predicate: &WhereClause,
    _row: &BTreeMap<PropertyName, Value>,
    _visibility: &RowVisibility,
) -> Tri {
    Tri::Unknown
}
