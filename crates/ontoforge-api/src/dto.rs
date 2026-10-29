use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        sets: Vec<ObjectSet>,
    },
    Intersect {
        object_type: String,
        sets: Vec<ObjectSet>,
    },
    Subtract {
        object_type: String,
        left: Box<ObjectSet>,
        right: Box<ObjectSet>,
    },
    SearchAround {
        object_type: String,
        base: Box<ObjectSet>,
        link: String,
        direction: LinkDirection,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkDirection {
    FromTo,
    ToFrom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TermMode {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        clauses: Vec<WhereClause>,
    },
    Or {
        clauses: Vec<WhereClause>,
    },
    Not {
        clause: Box<WhereClause>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrderBy {
    pub property: String,
    pub direction: Direction,
    pub nulls: Nulls,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Nulls {
    First,
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageSpec {
    pub first: u64,
    #[serde(default)]
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryRequest {
    pub set: ObjectSet,
    #[serde(default)]
    pub select: Vec<String>,
    #[serde(default)]
    pub order_by: Vec<OrderBy>,
    pub page: PageSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregateRequest {
    pub set: ObjectSet,
    #[serde(default)]
    pub group_by: Vec<String>,
    pub measures: Vec<AggregateMeasure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyMetadata {
    pub api_name: String,
    pub r#type: String,
    pub required: bool,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkMetadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionMetadata {
    pub api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub code: String,
    #[serde(rename = "apiName")]
    pub api_name: String,
    pub request_id: String,
}
