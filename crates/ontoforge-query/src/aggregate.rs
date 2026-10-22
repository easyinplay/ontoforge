use std::collections::{BTreeMap, BTreeSet};

use ontoforge_schema::ir::{BaseType, ObjectType, PropertyDef, PropertyName};
use ontoforge_storage::value::PropertyValue;
use serde::Serialize;
use serde_json::Value;
use ontoforge_engine::Engine;

use crate::ast::{AggregateMeasure, AggregateRequest, PageSpec};
use crate::compile::{CompileLimits, ValidatedQuery};
use crate::execute::WorkBudget;
use crate::{QueryRequest, QueryService};

use crate::QueryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateLimits {
    pub max_groups: usize,
    pub max_buckets: usize,
    pub max_distinct_values: usize,
    pub max_distinct_memory_bytes: usize,
}

impl AggregateLimits {
    #[must_use]
    pub fn phase4_default() -> Self {
        Self {
            max_groups: 1_024,
            max_buckets: 4_096,
            max_distinct_values: 10_000,
            max_distinct_memory_bytes: 16 * 1024 * 1024,
        }
    }

    pub fn try_new(
        max_groups: usize,
        max_buckets: usize,
        max_distinct_values: usize,
        max_distinct_memory_bytes: usize,
    ) -> Result<Self, QueryError> {
        if [max_groups, max_buckets, max_distinct_values, max_distinct_memory_bytes]
            .iter()
            .any(|&value| value == 0 || i64::try_from(value).is_err())
        {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self { max_groups, max_buckets, max_distinct_values, max_distinct_memory_bytes })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateCapabilities {
    pub count: bool,
    pub sum_integer: bool,
    pub sum_double: bool,
    pub min_max: bool,
    pub distinct: bool,
    pub group_by: bool,
    pub case_when: bool,
}

impl AggregateCapabilities {
    #[must_use]
    pub fn all_supported() -> Self {
        Self {
            count: true,
            sum_integer: true,
            sum_double: true,
            min_max: true,
            distinct: true,
            group_by: true,
            case_when: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateBucket {
    pub group: Vec<Value>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateResult {
    pub buckets: Vec<AggregateBucket>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum AggregateError {
    #[error("invalid aggregate query")]
    InvalidQuery,
    #[error("aggregate operation is unsupported")]
    Unsupported,
    #[error("aggregate cost limit exceeded")]
    AggregateCostExceeded,
    #[error("aggregate source unavailable")]
    Source,
    #[error("aggregate edit store unavailable")]
    Store,
    #[error("unsupported numeric value")]
    UnsupportedNumericValue,
}

impl From<crate::QueryError> for AggregateError {
    fn from(value: crate::QueryError) -> Self {
        match value {
            crate::QueryError::Source => Self::Source,
            crate::QueryError::Store => Self::Store,
            crate::QueryError::CostExceeded | crate::QueryError::WorkBudgetExceeded => {
                Self::AggregateCostExceeded
            }
            _ => Self::InvalidQuery,
        }
    }
}

#[derive(Clone)]
enum Total {
    Integer(i128),
    Double(f64),
    Decimal(i128, u32),
}

impl Total {
    fn zero(ty: BaseType, def: &PropertyDef) -> Result<Self, AggregateError> {
        match ty {
            BaseType::Integer => Ok(Self::Integer(0)),
            BaseType::Double => Ok(Self::Double(0.0)),
            BaseType::Decimal => Ok(Self::Decimal(0, def.effective_decimal().1)),
            _ => Err(AggregateError::InvalidQuery),
        }
    }

    fn add(&mut self, value: &Value, def: &PropertyDef) -> Result<(), AggregateError> {
        let value = PropertyValue::from_json(def, value)
            .map_err(|_| AggregateError::UnsupportedNumericValue)?;
        match (self, value) {
            (Self::Integer(sum), PropertyValue::Int(value)) => {
                *sum = sum.checked_add(i128::from(value)).ok_or(AggregateError::UnsupportedNumericValue)?;
            }
            (Self::Double(sum), PropertyValue::Double(value)) if value.is_finite() => {
                *sum += value;
                if !sum.is_finite() {
                    return Err(AggregateError::UnsupportedNumericValue);
                }
            }
            (Self::Decimal(sum, _), PropertyValue::Decimal(value)) => {
                let scaled = value.replace('.', "").parse::<i128>()
                    .map_err(|_| AggregateError::UnsupportedNumericValue)?;
                *sum = sum.checked_add(scaled).ok_or(AggregateError::UnsupportedNumericValue)?;
            }
            _ => return Err(AggregateError::UnsupportedNumericValue),
        }
        Ok(())
    }

    fn result(&self) -> Result<Value, AggregateError> {
        match self {
            Self::Integer(value) => Ok(Value::from(i64::try_from(*value)
                .map_err(|_| AggregateError::UnsupportedNumericValue)?)),
            Self::Double(value) => serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .ok_or(AggregateError::UnsupportedNumericValue),
            Self::Decimal(value, scale) => {
                let negative = *value < 0;
                let digits = value.unsigned_abs().to_string();
                let scale = usize::try_from(*scale).map_err(|_| AggregateError::UnsupportedNumericValue)?;
                let padded = format!("{digits:0>width$}", width = scale + 1);
                let split = padded.len() - scale;
                let body = if scale == 0 { padded } else { format!("{}.{}", &padded[..split], &padded[split..]) };
                Ok(Value::String(if negative { format!("-{body}") } else { body }))
            }
        }
    }

    fn as_f64(&self) -> Result<f64, AggregateError> {
        match self {
            Self::Integer(value) => Ok(*value as f64),
            Self::Double(value) => Ok(*value),
            Self::Decimal(value, scale) => Ok(*value as f64 / 10_f64.powi(i32::try_from(*scale)
                .map_err(|_| AggregateError::UnsupportedNumericValue)?)),
        }
    }
}

enum MeasureState {
    Count(u64),
    Sum { total: Total, count: u64 },
    Avg { total: Total, count: u64 },
    Min(Option<Value>),
    Max(Option<Value>),
    Distinct(BTreeSet<String>),
}

impl MeasureState {
    fn new(measure: &AggregateMeasure, ty: &ObjectType) -> Result<Self, AggregateError> {
        let def = measure.property().and_then(|p| ty.properties.get(p));
        match measure {
            AggregateMeasure::Count { .. } => Ok(Self::Count(0)),
            AggregateMeasure::Sum { .. } => {
                let def = def.ok_or(AggregateError::InvalidQuery)?;
                Ok(Self::Sum { total: Total::zero(def.base_type, def)?, count: 0 })
            }
            AggregateMeasure::Avg { .. } => {
                let def = def.ok_or(AggregateError::InvalidQuery)?;
                Ok(Self::Avg { total: Total::zero(def.base_type, def)?, count: 0 })
            }
            AggregateMeasure::Min { .. } => Ok(Self::Min(None)),
            AggregateMeasure::Max { .. } => Ok(Self::Max(None)),
            AggregateMeasure::Distinct { .. } => Ok(Self::Distinct(BTreeSet::new())),
        }
    }

    fn finish(self) -> Result<Value, AggregateError> {
        match self {
            Self::Count(count) => Ok(Value::from(count)),
            Self::Sum { total, count } => if count == 0 { Ok(Value::Null) } else { total.result() },
            Self::Avg { total, count } => if count == 0 { Ok(Value::Null) } else {
                serde_json::Number::from_f64(total.as_f64()? / count as f64)
                    .map(Value::Number).ok_or(AggregateError::UnsupportedNumericValue)
            },
            Self::Min(value) | Self::Max(value) => Ok(value.unwrap_or(Value::Null)),
            Self::Distinct(values) => Ok(Value::from(u64::try_from(values.len())
                .map_err(|_| AggregateError::AggregateCostExceeded)?)),
        }
    }
}

struct BucketState {
    group: Vec<Value>,
    measures: Vec<MeasureState>,
}

struct AggregateBuilder<'a> {
    request: &'a AggregateRequest,
    ty: &'a ObjectType,
    limits: AggregateLimits,
    buckets: BTreeMap<String, BucketState>,
    bucket_count: usize,
    distinct_count: usize,
    distinct_bytes: usize,
}

impl<'a> AggregateBuilder<'a> {
    fn new(request: &'a AggregateRequest, ty: &'a ObjectType, limits: AggregateLimits) -> Self {
        Self { request, ty, limits, buckets: BTreeMap::new(), bucket_count: 0, distinct_count: 0, distinct_bytes: 0 }
    }

    fn push(&mut self, row: &BTreeMap<PropertyName, Value>) -> Result<(), AggregateError> {
        let group = self.request.group_by.iter().map(|property| {
            row.get(property).cloned().unwrap_or(Value::Null)
        }).collect::<Vec<_>>();
        let key = serde_json::to_string(&group).map_err(|_| AggregateError::InvalidQuery)?;
        if !self.buckets.contains_key(&key) {
            if self.buckets.len().checked_add(1).is_none_or(|n| n > self.limits.max_groups) {
                return Err(AggregateError::AggregateCostExceeded);
            }
            self.bucket_count = self.bucket_count.checked_add(self.request.measures.len())
                .ok_or(AggregateError::AggregateCostExceeded)?;
            if self.bucket_count > self.limits.max_buckets {
                return Err(AggregateError::AggregateCostExceeded);
            }
            self.buckets.insert(key.clone(), BucketState {
                group,
                measures: self.request.measures.iter().map(|m| MeasureState::new(m, self.ty))
                    .collect::<Result<_, _>>()?,
            });
        }
        let bucket = self.buckets.get_mut(&key).ok_or(AggregateError::InvalidQuery)?;
        for (measure, state) in self.request.measures.iter().zip(&mut bucket.measures) {
            let value = measure.property().and_then(|property| row.get(property))
                .filter(|value| !value.is_null());
            let def = measure.property().and_then(|property| self.ty.properties.get(property));
            match state {
                MeasureState::Count(count) if measure.property().is_none() || value.is_some() => {
                    *count = count.checked_add(1).ok_or(AggregateError::AggregateCostExceeded)?;
                }
                MeasureState::Sum { total, count } | MeasureState::Avg { total, count } => {
                    if let Some(value) = value {
                        total.add(value, def.ok_or(AggregateError::InvalidQuery)?)?;
                        *count = count.checked_add(1).ok_or(AggregateError::AggregateCostExceeded)?;
                    }
                }
                MeasureState::Distinct(values) => if let Some(value) = value {
                    let canonical = PropertyValue::from_json(def.ok_or(AggregateError::InvalidQuery)?, value)
                        .map_err(|_| AggregateError::InvalidQuery)?.to_json().to_string();
                    if !values.contains(&canonical) {
                        self.distinct_count = self.distinct_count.checked_add(1)
                            .ok_or(AggregateError::AggregateCostExceeded)?;
                        self.distinct_bytes = self.distinct_bytes.checked_add(canonical.len())
                            .ok_or(AggregateError::AggregateCostExceeded)?;
                        if self.distinct_count > self.limits.max_distinct_values
                            || self.distinct_bytes > self.limits.max_distinct_memory_bytes {
                            return Err(AggregateError::AggregateCostExceeded);
                        }
                        values.insert(canonical);
                    }
                },
                _ => {},
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<AggregateResult, AggregateError> {
        if self.buckets.is_empty() && self.request.group_by.is_empty() {
            self.push(&BTreeMap::new())?;
            let bucket = self.buckets.values_mut().next().ok_or(AggregateError::InvalidQuery)?;
            for (measure, state) in self.request.measures.iter().zip(&mut bucket.measures) {
                if matches!(measure, AggregateMeasure::Count { .. }) { *state = MeasureState::Count(0); }
            }
        }
        let buckets = self.buckets.into_values().map(|bucket| Ok(AggregateBucket {
            group: bucket.group,
            values: bucket.measures.into_iter().map(MeasureState::finish).collect::<Result<_, _>>()?,
        })).collect::<Result<_, AggregateError>>()?;
        Ok(AggregateResult { buckets })
    }
}

fn validate_request(request: &AggregateRequest, ty: &ObjectType, scope: &ontoforge_auth::scope::TypeScope, limits: AggregateLimits) -> Result<(), AggregateError> {
    if request.measures.is_empty() || request.measures.len() > limits.max_buckets
        || request.group_by.len() > 8 || request.group_by.len() > limits.max_groups {
        return Err(AggregateError::InvalidQuery);
    }
    for property in request.group_by.iter().chain(request.measures.iter().filter_map(AggregateMeasure::property)) {
        let def = ty.properties.get(property).ok_or(AggregateError::InvalidQuery)?;
        if !scope.columns.contains_key(property) || matches!(def.base_type, BaseType::Array | BaseType::Geopoint) {
            return Err(AggregateError::InvalidQuery);
        }
    }
    for measure in &request.measures {
        if matches!(measure, AggregateMeasure::Sum { .. } | AggregateMeasure::Avg { .. }) {
            let def = ty.properties.get(measure.property().ok_or(AggregateError::InvalidQuery)?)
                .ok_or(AggregateError::InvalidQuery)?;
            Total::zero(def.base_type, def)?;
        }
    }
    Ok(())
}

impl<E: Engine> QueryService<'_, E> {
    pub async fn execute_aggregate(
        &self,
        request: AggregateRequest,
        _capabilities: AggregateCapabilities,
        limits: AggregateLimits,
    ) -> Result<AggregateResult, AggregateError> {
        let ty = self.active.ontology.object_types.get(request.set.object_type())
            .ok_or(AggregateError::InvalidQuery)?;
        let scope = self.scope.types.get(request.set.object_type())
            .ok_or(AggregateError::InvalidQuery)?;
        validate_request(&request, ty, scope, limits)?;
        let select: Vec<PropertyName> = request.group_by.iter().cloned()
            .chain(request.measures.iter().filter_map(|m| m.property().cloned()))
            .collect::<BTreeSet<_>>().into_iter().collect();
        let mut budget = WorkBudget::new(self.limits);
        let mut after = None;
        let mut builder = AggregateBuilder::new(&request, ty, limits);
        loop {
            let query = ValidatedQuery::new(
                QueryRequest { set: request.set.clone(), select: select.clone() },
                Vec::new(),
                PageSpec { first: 10_000, after },
                &self.active.ontology, scope, CompileLimits::default(),
            )?;
            let page = self.execute_list_internal(&query, &mut budget, None).await?;
            if page.truncated { return Err(AggregateError::AggregateCostExceeded); }
            for row in &page.rows { builder.push(&row.properties)?; }
            if !page.has_more { return builder.finish(); }
            after = Some(page.cursor.ok_or(AggregateError::AggregateCostExceeded)?);
        }
    }
}
