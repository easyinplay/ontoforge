use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ObjectSet {
    Pk {
        object_type: String,
        pk: Value,
    },
    Type {
        object_type: String,
    },
    Where {
        object_type: String,
        predicate: WhereClause,
    },
    Search {
        object_type: String,
        property: String,
        query: String,
        mode: TermMode,
    },
    Union {
        object_type: String,
        #[schema(no_recursion)]
        sets: Vec<ObjectSet>,
    },
    Intersect {
        object_type: String,
        #[schema(no_recursion)]
        sets: Vec<ObjectSet>,
    },
    Subtract {
        object_type: String,
        #[schema(no_recursion)]
        left: Box<ObjectSet>,
        #[schema(no_recursion)]
        right: Box<ObjectSet>,
    },
    SearchAround {
        object_type: String,
        #[schema(no_recursion)]
        base: Box<ObjectSet>,
        link: String,
        direction: LinkDirection,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LinkDirection {
    FromTo,
    ToFrom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum TermMode {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum WhereClause {
    Eq {
        property: String,
        value: Value,
    },
    Gt {
        property: String,
        value: Value,
    },
    Gte {
        property: String,
        value: Value,
    },
    Lt {
        property: String,
        value: Value,
    },
    Lte {
        property: String,
        value: Value,
    },
    IsNull {
        property: String,
    },
    In {
        property: String,
        values: Vec<Value>,
    },
    StartsWith {
        property: String,
        value: String,
    },
    ContainsAnyTerm {
        property: String,
        value: String,
    },
    ContainsAllTerms {
        property: String,
        value: String,
    },
    And {
        #[schema(no_recursion)]
        clauses: Vec<WhereClause>,
    },
    Or {
        #[schema(no_recursion)]
        clauses: Vec<WhereClause>,
    },
    Not {
        #[schema(no_recursion)]
        clause: Box<WhereClause>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderBy {
    pub property: String,
    pub direction: Direction,
    pub nulls: Nulls,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Nulls {
    First,
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageSpec {
    pub first: u64,
    #[serde(default)]
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryRequest {
    pub set: ObjectSet,
    #[serde(default)]
    pub select: Vec<String>,
    #[serde(default)]
    pub order_by: Vec<OrderBy>,
    pub page: PageSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregateRequest {
    pub set: ObjectSet,
    #[serde(default)]
    pub group_by: Vec<String>,
    pub measures: Vec<AggregateMeasure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum AggregateMeasure {
    Count {
        #[serde(default)]
        property: Option<String>,
    },
    Sum {
        property: String,
    },
    Avg {
        property: String,
    },
    Min {
        property: String,
    },
    Max {
        property: String,
    },
    Distinct {
        property: String,
    },
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub object_types: Vec<ObjectMetadata>,
    pub links: Vec<LinkMetadata>,
    pub actions: Vec<ActionMetadata>,
    pub def_version: i64,
    pub engine_status: EngineAvailability,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EngineAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMetadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub aliases: Vec<String>,
    pub primary_key: String,
    pub title_key: String,
    pub properties: Vec<PropertyMetadata>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PropertyMetadata {
    pub api_name: String,
    pub r#type: String,
    pub required: bool,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkMetadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionMetadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub code: String,
    #[serde(rename = "apiName")]
    pub api_name: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionRequest {
    pub command_id: Option<String>,
    pub items: Vec<ActionItem>,
    #[serde(default)]
    pub validate_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionItem {
    pub params: std::collections::BTreeMap<String, Value>,
    #[serde(default)]
    pub expected_versions: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRow {
    pub object_type: String,
    pub pk: Value,
    pub properties: std::collections::BTreeMap<String, Value>,
    #[serde(rename = "_version")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPage {
    pub rows: Vec<ObjectRow>,
    pub has_more: bool,
    pub truncated: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggregateBucket {
    pub group: Vec<Value>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggregateResult {
    pub buckets: Vec<AggregateBucket>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub seq: i64,
    pub event_id: String,
    pub actor: String,
    pub action: String,
    pub committed_at: String,
    pub op: String,
    pub version: i64,
    pub values: std::collections::BTreeMap<String, Value>,
    pub unset: Vec<String>,
    pub masked_changes: Vec<String>,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub truncated: bool,
    pub next_before_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemReceipt {
    pub item_ordinal: u32,
    pub object_type: String,
    pub pk: Value,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionReceipt {
    pub command_id: String,
    pub event_id: String,
    pub log_seq: i64,
    pub committed_at: String,
    pub chosen_current_time: String,
    pub items: Vec<ItemReceipt>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ValidationResponse {
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub engine_status: EngineAvailability,
}
