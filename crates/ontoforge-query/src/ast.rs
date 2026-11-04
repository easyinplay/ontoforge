use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ObjectSet {
    Pk {
        object_type: ObjectTypeName,
        pk: Value,
    },
    Type {
        object_type: ObjectTypeName,
    },
    Where {
        object_type: ObjectTypeName,
        predicate: WhereClause,
    },
    Search {
        object_type: ObjectTypeName,
        property: PropertyName,
        query: String,
        mode: TermMode,
    },
    Union {
        object_type: ObjectTypeName,
        sets: Vec<ObjectSet>,
    },
    Intersect {
        object_type: ObjectTypeName,
        sets: Vec<ObjectSet>,
    },
    Subtract {
        object_type: ObjectTypeName,
        left: Box<ObjectSet>,
        right: Box<ObjectSet>,
    },
    SearchAround {
        object_type: ObjectTypeName,
        base: Box<ObjectSet>,
        link: LinkTypeName,
        direction: LinkDirection,
    },
}

impl ObjectSet {
    pub fn object_type(&self) -> &ObjectTypeName {
        match self {
            Self::Pk { object_type, .. }
            | Self::Type { object_type }
            | Self::Where { object_type, .. }
            | Self::Search { object_type, .. }
            | Self::Union { object_type, .. }
            | Self::Intersect { object_type, .. }
            | Self::Subtract { object_type, .. }
            | Self::SearchAround { object_type, .. } => object_type,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkDirection {
    FromTo,
    ToFrom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TermMode {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct QueryLiteral(pub Value);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum WhereClause {
    Eq {
        property: PropertyName,
        value: QueryLiteral,
    },
    Gt {
        property: PropertyName,
        value: QueryLiteral,
    },
    Gte {
        property: PropertyName,
        value: QueryLiteral,
    },
    Lt {
        property: PropertyName,
        value: QueryLiteral,
    },
    Lte {
        property: PropertyName,
        value: QueryLiteral,
    },
    IsNull {
        property: PropertyName,
    },
    In {
        property: PropertyName,
        values: Vec<QueryLiteral>,
    },
    StartsWith {
        property: PropertyName,
        value: String,
    },
    ContainsAnyTerm {
        property: PropertyName,
        value: String,
    },
    ContainsAllTerms {
        property: PropertyName,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OrderBy {
    pub property: PropertyName,
    pub direction: Direction,
    pub nulls: Nulls,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Nulls {
    First,
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageSpec {
    pub first: u64,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AggregateRequest {
    pub set: ObjectSet,
    pub group_by: Vec<PropertyName>,
    pub measures: Vec<AggregateMeasure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum AggregateMeasure {
    Count { property: Option<PropertyName> },
    Sum { property: PropertyName },
    Avg { property: PropertyName },
    Min { property: PropertyName },
    Max { property: PropertyName },
    Distinct { property: PropertyName },
}

impl AggregateMeasure {
    pub fn property(&self) -> Option<&PropertyName> {
        match self {
            Self::Count { property } => property.as_ref(),
            Self::Sum { property }
            | Self::Avg { property }
            | Self::Min { property }
            | Self::Max { property }
            | Self::Distinct { property } => Some(property),
        }
    }
}
