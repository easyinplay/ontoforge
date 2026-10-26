use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::Engine;
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{QueryContext, Value as EngineValue};
use ontoforge_schema::ir::{Backing, SourceBinding};
use ontoforge_schema::ir::{BaseType, ObjectType, PropertyDef, PropertyName};
use ontoforge_storage::query_read::{MAX_EDIT_READ, ObjectEdit};
use ontoforge_storage::read_audit::ReadAuditStatus;
use ontoforge_storage::value::PropertyValue;
use serde::Serialize;
use serde_json::Value;

use crate::ast::{
    AggregateMeasure, AggregateRequest, Direction, Nulls, ObjectSet, OrderBy, PageSpec,
};
use crate::compile::{CompileLimits, ValidatedQuery, compile_aggregate_partial};
use crate::execute::{WorkBudget, charge_edit_page, raw_backfill_select};
use crate::overlay::{eligible_row, overlay_object};
use crate::{QueryRequest, QueryService, engine_json};

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
        if [
            max_groups,
            max_buckets,
            max_distinct_values,
            max_distinct_memory_bytes,
        ]
        .iter()
        .any(|&value| value == 0 || i64::try_from(value).is_err())
        {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self {
            max_groups,
            max_buckets,
            max_distinct_values,
            max_distinct_memory_bytes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "connector pushdown features are independent switches"
)]
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
    #[error("read audit unavailable")]
    AuditUnavailable,
    #[error("unsupported numeric value")]
    UnsupportedNumericValue,
}

impl From<crate::QueryError> for AggregateError {
    fn from(value: crate::QueryError) -> Self {
        match value {
            crate::QueryError::Source => Self::Source,
            crate::QueryError::UnsupportedNumericValue => Self::UnsupportedNumericValue,
            crate::QueryError::Store => Self::Store,
            crate::QueryError::AuditUnavailable => Self::AuditUnavailable,
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
        self.adjust(value, def, true)
    }

    fn subtract(&mut self, value: &Value, def: &PropertyDef) -> Result<(), AggregateError> {
        self.adjust(value, def, false)
    }

    fn adjust(
        &mut self,
        value: &Value,
        def: &PropertyDef,
        add: bool,
    ) -> Result<(), AggregateError> {
        let value = PropertyValue::from_json(def, value)
            .map_err(|_| AggregateError::UnsupportedNumericValue)?;
        match (self, value) {
            (Self::Integer(sum), PropertyValue::Int(value)) => {
                *sum = if add {
                    sum.checked_add(i128::from(value))
                } else {
                    sum.checked_sub(i128::from(value))
                }
                .ok_or(AggregateError::UnsupportedNumericValue)?;
            }
            (Self::Double(sum), PropertyValue::Double(value)) if value.is_finite() => {
                if add {
                    *sum += value;
                } else {
                    *sum -= value;
                }
                if !sum.is_finite() {
                    return Err(AggregateError::UnsupportedNumericValue);
                }
            }
            (Self::Decimal(sum, _), PropertyValue::Decimal(value)) => {
                let scaled = value
                    .replace('.', "")
                    .parse::<i128>()
                    .map_err(|_| AggregateError::UnsupportedNumericValue)?;
                *sum = if add {
                    sum.checked_add(scaled)
                } else {
                    sum.checked_sub(scaled)
                }
                .ok_or(AggregateError::UnsupportedNumericValue)?;
            }
            _ => return Err(AggregateError::UnsupportedNumericValue),
        }
        Ok(())
    }

    fn result(&self) -> Result<Value, AggregateError> {
        match self {
            Self::Integer(value) => Ok(Value::from(
                i64::try_from(*value).map_err(|_| AggregateError::UnsupportedNumericValue)?,
            )),
            Self::Double(value) => serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .ok_or(AggregateError::UnsupportedNumericValue),
            Self::Decimal(value, scale) => {
                let negative = *value < 0;
                let digits = value.unsigned_abs().to_string();
                let scale =
                    usize::try_from(*scale).map_err(|_| AggregateError::UnsupportedNumericValue)?;
                let padded = format!("{digits:0>width$}", width = scale + 1);
                let split = padded.len() - scale;
                let body = if scale == 0 {
                    padded
                } else {
                    format!("{}.{}", &padded[..split], &padded[split..])
                };
                Ok(Value::String(if negative {
                    format!("-{body}")
                } else {
                    body
                }))
            }
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the avg result contract is a finite JSON double"
    )]
    fn as_f64(&self) -> Result<f64, AggregateError> {
        match self {
            Self::Integer(value) => Ok(*value as f64),
            Self::Double(value) => Ok(*value),
            Self::Decimal(value, scale) => Ok(*value as f64
                / 10_f64.powi(
                    i32::try_from(*scale).map_err(|_| AggregateError::UnsupportedNumericValue)?,
                )),
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

fn compare_values(
    left: &Value,
    right: &Value,
    def: &PropertyDef,
) -> Result<Ordering, AggregateError> {
    let left = PropertyValue::from_json(def, left).map_err(|_| AggregateError::Source)?;
    let right = PropertyValue::from_json(def, right).map_err(|_| AggregateError::Source)?;
    match (left, right) {
        (PropertyValue::Bool(a), PropertyValue::Bool(b)) => Ok(a.cmp(&b)),
        (PropertyValue::Int(a), PropertyValue::Int(b)) => Ok(a.cmp(&b)),
        (PropertyValue::Double(a), PropertyValue::Double(b)) if a.is_finite() && b.is_finite() => {
            Ok(a.total_cmp(&b))
        }
        (PropertyValue::Decimal(a), PropertyValue::Decimal(b)) => {
            let a = a
                .replace('.', "")
                .parse::<i128>()
                .map_err(|_| AggregateError::Source)?;
            let b = b
                .replace('.', "")
                .parse::<i128>()
                .map_err(|_| AggregateError::Source)?;
            Ok(a.cmp(&b))
        }
        (PropertyValue::String(a), PropertyValue::String(b))
        | (PropertyValue::Date(a), PropertyValue::Date(b))
        | (PropertyValue::Timestamp(a), PropertyValue::Timestamp(b)) => Ok(a.cmp(&b)),
        _ => Err(AggregateError::UnsupportedNumericValue),
    }
}

impl MeasureState {
    fn new(measure: &AggregateMeasure, ty: &ObjectType) -> Result<Self, AggregateError> {
        let def = measure.property().and_then(|p| ty.properties.get(p));
        match measure {
            AggregateMeasure::Count { .. } => Ok(Self::Count(0)),
            AggregateMeasure::Sum { .. } => {
                let def = def.ok_or(AggregateError::InvalidQuery)?;
                Ok(Self::Sum {
                    total: Total::zero(def.base_type, def)?,
                    count: 0,
                })
            }
            AggregateMeasure::Avg { .. } => {
                let def = def.ok_or(AggregateError::InvalidQuery)?;
                Ok(Self::Avg {
                    total: Total::zero(def.base_type, def)?,
                    count: 0,
                })
            }
            AggregateMeasure::Min { .. } => Ok(Self::Min(None)),
            AggregateMeasure::Max { .. } => Ok(Self::Max(None)),
            AggregateMeasure::Distinct { .. } => Ok(Self::Distinct(BTreeSet::new())),
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the avg result contract is a finite JSON double"
    )]
    fn finish(self) -> Result<Value, AggregateError> {
        match self {
            Self::Count(count) => Ok(Value::from(count)),
            Self::Sum { total, count } => {
                if count == 0 {
                    Ok(Value::Null)
                } else {
                    total.result()
                }
            }
            Self::Avg { total, count } => {
                if count == 0 {
                    Ok(Value::Null)
                } else {
                    serde_json::Number::from_f64(total.as_f64()? / count as f64)
                        .map(Value::Number)
                        .ok_or(AggregateError::UnsupportedNumericValue)
                }
            }
            Self::Min(value) | Self::Max(value) => Ok(value.unwrap_or(Value::Null)),
            Self::Distinct(values) => Ok(Value::from(
                u64::try_from(values.len()).map_err(|_| AggregateError::AggregateCostExceeded)?,
            )),
        }
    }
}

struct BucketState {
    group: Vec<Value>,
    rows: u64,
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
        Self {
            request,
            ty,
            limits,
            buckets: BTreeMap::new(),
            bucket_count: 0,
            distinct_count: 0,
            distinct_bytes: 0,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one visible row updates each aggregate measure exactly once"
    )]
    fn push(&mut self, row: &BTreeMap<PropertyName, Value>) -> Result<(), AggregateError> {
        let group = self
            .request
            .group_by
            .iter()
            .map(|property| row.get(property).cloned().unwrap_or(Value::Null))
            .collect::<Vec<_>>();
        let key = serde_json::to_string(&group).map_err(|_| AggregateError::InvalidQuery)?;
        if !self.buckets.contains_key(&key) {
            if self
                .buckets
                .len()
                .checked_add(1)
                .is_none_or(|n| n > self.limits.max_groups)
            {
                return Err(AggregateError::AggregateCostExceeded);
            }
            self.bucket_count = self
                .bucket_count
                .checked_add(self.request.measures.len())
                .ok_or(AggregateError::AggregateCostExceeded)?;
            if self.bucket_count > self.limits.max_buckets {
                return Err(AggregateError::AggregateCostExceeded);
            }
            self.buckets.insert(
                key.clone(),
                BucketState {
                    group,
                    rows: 0,
                    measures: self
                        .request
                        .measures
                        .iter()
                        .map(|m| MeasureState::new(m, self.ty))
                        .collect::<Result<_, _>>()?,
                },
            );
        }
        let bucket = self
            .buckets
            .get_mut(&key)
            .ok_or(AggregateError::InvalidQuery)?;
        bucket.rows = bucket
            .rows
            .checked_add(1)
            .ok_or(AggregateError::AggregateCostExceeded)?;
        for (measure, state) in self.request.measures.iter().zip(&mut bucket.measures) {
            let value = measure
                .property()
                .and_then(|property| row.get(property))
                .filter(|value| !value.is_null());
            let def = measure
                .property()
                .and_then(|property| self.ty.properties.get(property));
            match state {
                MeasureState::Count(count) if measure.property().is_none() || value.is_some() => {
                    *count = count
                        .checked_add(1)
                        .ok_or(AggregateError::AggregateCostExceeded)?;
                }
                MeasureState::Sum { total, count } | MeasureState::Avg { total, count } => {
                    if let Some(value) = value {
                        total.add(value, def.ok_or(AggregateError::InvalidQuery)?)?;
                        *count = count
                            .checked_add(1)
                            .ok_or(AggregateError::AggregateCostExceeded)?;
                    }
                }
                MeasureState::Distinct(values) => {
                    if let Some(value) = value {
                        let typed = PropertyValue::from_json(
                            def.ok_or(AggregateError::InvalidQuery)?,
                            value,
                        )
                        .map_err(|_| AggregateError::UnsupportedNumericValue)?;
                        let canonical = match typed {
                            PropertyValue::Double(0.0) => PropertyValue::Double(0.0),
                            other => other,
                        }
                        .to_json()
                        .to_string();
                        if !values.contains(&canonical) {
                            self.distinct_count = self
                                .distinct_count
                                .checked_add(1)
                                .ok_or(AggregateError::AggregateCostExceeded)?;
                            self.distinct_bytes = self
                                .distinct_bytes
                                .checked_add(canonical.len())
                                .ok_or(AggregateError::AggregateCostExceeded)?;
                            if self.distinct_count > self.limits.max_distinct_values
                                || self.distinct_bytes > self.limits.max_distinct_memory_bytes
                            {
                                return Err(AggregateError::AggregateCostExceeded);
                            }
                            values.insert(canonical);
                        }
                    }
                }
                MeasureState::Min(current) | MeasureState::Max(current) => {
                    if let Some(value) = value {
                        let def = def.ok_or(AggregateError::InvalidQuery)?;
                        let replace = match current.as_ref() {
                            None => true,
                            Some(old) => {
                                let order = compare_values(value, old, def)?;
                                if matches!(measure, AggregateMeasure::Min { .. }) {
                                    order == Ordering::Less
                                } else {
                                    order == Ordering::Greater
                                }
                            }
                        };
                        if replace {
                            *current = Some(value.clone());
                        }
                    }
                }
                MeasureState::Count(_) => {}
            }
        }
        Ok(())
    }

    fn seed_partial(&mut self, values: Vec<Value>) -> Result<(), AggregateError> {
        let group_len = self.request.group_by.len();
        let mut iter = values.into_iter();
        let group = iter.by_ref().take(group_len).collect::<Vec<_>>();
        if group.len() != group_len {
            return Err(AggregateError::Source);
        }
        let rows = iter
            .next()
            .and_then(|v| v.as_u64())
            .ok_or(AggregateError::Source)?;
        let key = serde_json::to_string(&group).map_err(|_| AggregateError::Source)?;
        if self.buckets.contains_key(&key) {
            return Err(AggregateError::Source);
        }
        if self
            .buckets
            .len()
            .checked_add(1)
            .is_none_or(|n| n > self.limits.max_groups)
        {
            return Err(AggregateError::AggregateCostExceeded);
        }
        self.bucket_count = self
            .bucket_count
            .checked_add(self.request.measures.len())
            .ok_or(AggregateError::AggregateCostExceeded)?;
        if self.bucket_count > self.limits.max_buckets {
            return Err(AggregateError::AggregateCostExceeded);
        }
        let measures = self
            .request
            .measures
            .iter()
            .map(|measure| {
                let def = measure
                    .property()
                    .and_then(|property| self.ty.properties.get(property));
                let value = iter.next().ok_or(AggregateError::Source)?;
                match measure {
                    AggregateMeasure::Count { .. } => Ok(MeasureState::Count(
                        value.as_u64().ok_or(AggregateError::Source)?,
                    )),
                    AggregateMeasure::Sum { .. } | AggregateMeasure::Avg { .. } => {
                        let def = def.ok_or(AggregateError::Source)?;
                        let mut total = Total::zero(def.base_type, def)?;
                        if !value.is_null() {
                            total.add(&value, def)?;
                        }
                        let count = iter
                            .next()
                            .and_then(|v| v.as_u64())
                            .ok_or(AggregateError::Source)?;
                        if matches!(measure, AggregateMeasure::Avg { .. }) {
                            Ok(MeasureState::Avg { total, count })
                        } else {
                            Ok(MeasureState::Sum { total, count })
                        }
                    }
                    _ => Err(AggregateError::Source),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if iter.next().is_some() {
            return Err(AggregateError::Source);
        }
        self.buckets.insert(
            key,
            BucketState {
                group,
                rows,
                measures,
            },
        );
        Ok(())
    }

    fn subtract(&mut self, row: &BTreeMap<PropertyName, Value>) -> Result<(), AggregateError> {
        let group = self
            .request
            .group_by
            .iter()
            .map(|property| row.get(property).cloned().unwrap_or(Value::Null))
            .collect::<Vec<_>>();
        let key = serde_json::to_string(&group).map_err(|_| AggregateError::Source)?;
        let bucket = self.buckets.get_mut(&key).ok_or(AggregateError::Source)?;
        bucket.rows = bucket.rows.checked_sub(1).ok_or(AggregateError::Source)?;
        for (measure, state) in self.request.measures.iter().zip(&mut bucket.measures) {
            let value = measure
                .property()
                .and_then(|property| row.get(property))
                .filter(|value| !value.is_null());
            match state {
                MeasureState::Count(count) if measure.property().is_none() || value.is_some() => {
                    *count = count.checked_sub(1).ok_or(AggregateError::Source)?;
                }
                MeasureState::Sum { total, count } | MeasureState::Avg { total, count } => {
                    if let Some(value) = value {
                        let def = self
                            .ty
                            .properties
                            .get(measure.property().ok_or(AggregateError::Source)?)
                            .ok_or(AggregateError::Source)?;
                        total.subtract(value, def)?;
                        *count = count.checked_sub(1).ok_or(AggregateError::Source)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<AggregateResult, AggregateError> {
        if self.buckets.is_empty() && self.request.group_by.is_empty() {
            self.buckets.insert(
                "[]".to_owned(),
                BucketState {
                    group: Vec::new(),
                    rows: 0,
                    measures: self
                        .request
                        .measures
                        .iter()
                        .map(|m| MeasureState::new(m, self.ty))
                        .collect::<Result<_, _>>()?,
                },
            );
        }
        let buckets = self
            .buckets
            .into_values()
            .filter(|bucket| bucket.rows > 0 || self.request.group_by.is_empty())
            .map(|bucket| {
                Ok(AggregateBucket {
                    group: bucket.group,
                    values: bucket
                        .measures
                        .into_iter()
                        .map(MeasureState::finish)
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, AggregateError>>()?;
        Ok(AggregateResult { buckets })
    }
}

fn validate_request(
    request: &AggregateRequest,
    ty: &ObjectType,
    scope: &ontoforge_auth::scope::TypeScope,
    limits: AggregateLimits,
) -> Result<(), AggregateError> {
    if request.measures.is_empty()
        || request.measures.len() > limits.max_buckets
        || request.group_by.len() > 8
        || request.group_by.len() > limits.max_groups
    {
        return Err(AggregateError::InvalidQuery);
    }
    for property in request.group_by.iter().chain(
        request
            .measures
            .iter()
            .filter_map(AggregateMeasure::property),
    ) {
        let def = ty
            .properties
            .get(property)
            .ok_or(AggregateError::InvalidQuery)?;
        if !scope.columns.contains_key(property)
            || matches!(def.base_type, BaseType::Array | BaseType::Geopoint)
        {
            return Err(AggregateError::InvalidQuery);
        }
    }
    for measure in &request.measures {
        if matches!(
            measure,
            AggregateMeasure::Sum { .. } | AggregateMeasure::Avg { .. }
        ) {
            let def = ty
                .properties
                .get(measure.property().ok_or(AggregateError::InvalidQuery)?)
                .ok_or(AggregateError::InvalidQuery)?;
            Total::zero(def.base_type, def)?;
        }
    }
    Ok(())
}

impl<E: Engine> QueryService<'_, E> {
    async fn execute_aggregate_page(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<crate::ListPage, AggregateError> {
        let page = match &query.request.set {
            ObjectSet::Union { .. } | ObjectSet::Intersect { .. } | ObjectSet::Subtract { .. } => {
                self.execute_set_list(query, budget).await?
            }
            ObjectSet::SearchAround { .. } => self.execute_search_around(query, budget).await?,
            _ => self.execute_list_internal(query, budget, None).await?,
        };
        Ok(page)
    }

    async fn execute_minmax_topn(
        &self,
        request: &AggregateRequest,
        scope: &ontoforge_auth::scope::TypeScope,
        budget: &mut WorkBudget,
    ) -> Result<AggregateResult, AggregateError> {
        let mut values = Vec::with_capacity(request.measures.len());
        for measure in &request.measures {
            let (property, direction) = match measure {
                AggregateMeasure::Min { property } => (property, Direction::Asc),
                AggregateMeasure::Max { property } => (property, Direction::Desc),
                _ => return Err(AggregateError::InvalidQuery),
            };
            let query = ValidatedQuery::new(
                QueryRequest {
                    set: request.set.clone(),
                    select: vec![property.clone()],
                },
                vec![OrderBy {
                    property: property.clone(),
                    direction,
                    nulls: Nulls::Last,
                }],
                PageSpec {
                    first: 1,
                    after: None,
                },
                &self.active.ontology,
                scope,
                CompileLimits::default(),
            )?;
            let page = self.execute_aggregate_page(&query, budget).await?;
            if page.truncated {
                return Err(AggregateError::AggregateCostExceeded);
            }
            values.push(
                page.rows
                    .first()
                    .and_then(|row| row.properties.get(property))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        Ok(AggregateResult {
            buckets: vec![AggregateBucket {
                group: Vec::new(),
                values,
            }],
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_aggregate_partial(
        &self,
        request: &AggregateRequest,
        select: &ontoforge_engine::printer::ast::Select,
        ty: &ObjectType,
        scope: &ontoforge_auth::scope::TypeScope,
        binding: &SourceBinding,
        limits: AggregateLimits,
        budget: &mut WorkBudget,
    ) -> Result<AggregateResult, AggregateError> {
        budget.charge_backend_calls(1)?;
        let sql =
            print_select(select, self.engine.dialect()).map_err(|_| AggregateError::Source)?;
        budget.observe_sql(sql.as_str());
        let mut ctx = QueryContext::new("l0-aggregate".to_owned(), Vec::new());
        ctx.max_rows = limits
            .max_groups
            .checked_add(1)
            .ok_or(AggregateError::AggregateCostExceeded)?;
        let output = self
            .engine
            .execute(&sql, &ctx)
            .await
            .map_err(|_| AggregateError::Source)?;
        if output.truncated || output.rows.len() > limits.max_groups {
            return Err(AggregateError::AggregateCostExceeded);
        }
        let mut builder = AggregateBuilder::new(request, ty, limits);
        for row in output.rows {
            budget.charge_raw_candidates(1)?;
            let values = row
                .into_iter()
                .map(|value| match value {
                    EngineValue::Double(v) if !v.is_finite() => {
                        Err(AggregateError::UnsupportedNumericValue)
                    }
                    other => engine_json(other).map_err(|_| AggregateError::Source),
                })
                .collect::<Result<Vec<_>, _>>()?;
            budget.charge_decoded_bytes(
                serde_json::to_vec(&values)
                    .map_err(|_| AggregateError::Source)?
                    .len(),
            )?;
            builder.seed_partial(values)?;
        }
        let mut edits = Vec::<ObjectEdit>::new();
        let mut cursor = None;
        loop {
            budget.charge_backend_calls(1)?;
            let page = self
                .store
                .scan_object_edits(
                    &self.active,
                    request.set.object_type(),
                    cursor.as_ref(),
                    MAX_EDIT_READ,
                )
                .await
                .map_err(|_| AggregateError::Store)?;
            charge_edit_page(&page, budget)?;
            edits.extend(page.edits);
            if !page.has_more {
                break;
            }
            cursor = Some(page.next.ok_or(AggregateError::Store)?);
        }
        let mut old_rows = BTreeMap::new();
        for batch in edits.chunks(MAX_EDIT_READ) {
            let keys = batch.iter().map(|edit| edit.pk.clone()).collect::<Vec<_>>();
            budget.charge_backfill_keys(keys.len())?;
            let backfill = raw_backfill_select(ty, binding, &keys)?;
            let columns = ty.properties.keys().cloned().collect::<Vec<_>>();
            let rows = self
                .read_source_select(&backfill, &columns, ty, keys.len() + 1, budget)
                .await?;
            for (pk, row) in rows {
                if old_rows.insert(pk.to_json().to_string(), row).is_some() {
                    return Err(AggregateError::Source);
                }
            }
        }
        for edit in &edits {
            let source = old_rows.get(&edit.pk.to_json().to_string());
            if let Some(source) = source
                && let Some(old) =
                    eligible_row(source, &request.set, ty, scope, &self.principal.attrs)
            {
                builder.subtract(&old)?;
            }
            if let Some(merged) = overlay_object(source, Some(edit), &ty.primary_key, &edit.pk)?
                && let Some(new) =
                    eligible_row(&merged, &request.set, ty, scope, &self.principal.attrs)
            {
                builder.push(&new)?;
            }
        }
        builder.finish()
    }

    #[allow(
        clippy::too_many_lines,
        reason = "execution keeps one request budget across all strategies"
    )]
    pub async fn execute_aggregate(
        &self,
        request: AggregateRequest,
        capabilities: AggregateCapabilities,
        limits: AggregateLimits,
    ) -> Result<AggregateResult, AggregateError> {
        let audit = self
            .store
            .read_audit()
            .reserve(self.audit_metadata("query.aggregate", &request.set))
            .map_err(|_| AggregateError::AuditUnavailable)?;
        let mut budget = WorkBudget::with_audit(self.limits, audit);
        let result = self
            .execute_aggregate_inner(request, capabilities, limits, &mut budget)
            .await;
        let (status, count) = match &result {
            Ok(value) => (ReadAuditStatus::Success, value.buckets.len()),
            Err(AggregateError::Source | AggregateError::Store) => {
                (ReadAuditStatus::SourceError, 0)
            }
            Err(_) => (ReadAuditStatus::Denied, 0),
        };
        budget.finish_audit(status, count)?;
        result
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keeps the bounded aggregate execution paths together"
    )]
    async fn execute_aggregate_inner(
        &self,
        request: AggregateRequest,
        capabilities: AggregateCapabilities,
        limits: AggregateLimits,
        budget: &mut WorkBudget,
    ) -> Result<AggregateResult, AggregateError> {
        AggregateLimits::try_new(
            limits.max_groups,
            limits.max_buckets,
            limits.max_distinct_values,
            limits.max_distinct_memory_bytes,
        )?;
        let ty = self
            .active
            .ontology
            .object_types
            .get(request.set.object_type())
            .ok_or(AggregateError::InvalidQuery)?;
        let scope = self
            .scope
            .types
            .get(request.set.object_type())
            .ok_or(AggregateError::InvalidQuery)?;
        validate_request(&request, ty, scope, limits)?;
        let select: Vec<PropertyName> = request
            .group_by
            .iter()
            .cloned()
            .chain(
                request
                    .measures
                    .iter()
                    .filter_map(|m| m.property().cloned()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if request.group_by.is_empty()
            && capabilities.min_max
            && request.measures.iter().all(|measure| {
                matches!(
                    measure,
                    AggregateMeasure::Min { .. } | AggregateMeasure::Max { .. }
                )
            })
        {
            return self.execute_minmax_topn(&request, scope, budget).await;
        }
        if ty.backing == Backing::Source {
            let profile = self
                .active
                .ontology
                .default_profile
                .as_ref()
                .ok_or(AggregateError::Source)?;
            let binding = self
                .active
                .ontology
                .profile_bindings(profile)
                .and_then(|bindings| bindings.objects.get(request.set.object_type()))
                .ok_or(AggregateError::Source)?;
            if let Some(partial) = compile_aggregate_partial(
                &request,
                &self.active.ontology,
                binding,
                scope,
                &self.principal.attrs,
                capabilities,
            )? {
                return self
                    .execute_aggregate_partial(
                        &request, &partial, ty, scope, binding, limits, budget,
                    )
                    .await;
            }
        }
        let mut after = None;
        let mut builder = AggregateBuilder::new(&request, ty, limits);
        loop {
            let query = ValidatedQuery::new(
                QueryRequest {
                    set: request.set.clone(),
                    select: select.clone(),
                },
                Vec::new(),
                PageSpec {
                    first: 10_000,
                    after,
                },
                &self.active.ontology,
                scope,
                CompileLimits::default(),
            )?;
            let page = self.execute_aggregate_page(&query, budget).await?;
            if page.truncated {
                return Err(AggregateError::AggregateCostExceeded);
            }
            for row in &page.rows {
                builder.push(&row.properties)?;
            }
            if !page.has_more {
                return builder.finish();
            }
            after = Some(page.cursor.ok_or(AggregateError::AggregateCostExceeded)?);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8Path;
    use ontoforge_schema::ir::ObjectTypeName;
    use ontoforge_schema::load_dir;
    use proptest::prelude::*;
    use serde_json::json;

    fn fixture_type() -> (ObjectTypeName, ObjectType) {
        let root = Utf8Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation"
        ));
        let ontology = load_dir(root).ontology.unwrap();
        let flight = ObjectTypeName::try_from("Flight".to_owned()).unwrap();
        let ty = ontology.object_types[&flight].clone();
        (flight, ty)
    }

    fn property(raw: &str) -> PropertyName {
        PropertyName::try_from(raw.to_owned()).unwrap()
    }

    fn request(
        flight: ObjectTypeName,
        group_by: Vec<PropertyName>,
        measures: Vec<AggregateMeasure>,
    ) -> AggregateRequest {
        AggregateRequest {
            set: crate::ast::ObjectSet::Type {
                object_type: flight,
            },
            group_by,
            measures,
        }
    }

    #[test]
    fn group_distinct_treats_negative_and_positive_zero_as_one_value() {
        let (flight, mut ty) = fixture_type();
        let delay = property("delayMinutes");
        ty.properties.get_mut(&delay).unwrap().base_type = BaseType::Double;
        let request = request(
            flight,
            vec![],
            vec![AggregateMeasure::Distinct {
                property: delay.clone(),
            }],
        );
        let mut builder = AggregateBuilder::new(&request, &ty, AggregateLimits::phase4_default());
        builder
            .push(&BTreeMap::from([(delay.clone(), json!(-0.0))]))
            .unwrap();
        builder
            .push(&BTreeMap::from([(delay, json!(0.0))]))
            .unwrap();
        assert_eq!(builder.finish().unwrap().buckets[0].values, vec![json!(1)]);
    }

    #[test]
    fn group_distinct_group_and_bucket_bounds_are_inclusive() {
        let (flight, ty) = fixture_type();
        let status = property("status");
        let count = AggregateMeasure::Count { property: None };
        let group_request = request(flight.clone(), vec![status.clone()], vec![count.clone()]);
        let mut groups =
            AggregateBuilder::new(&group_request, &ty, AggregateLimits::phase4_default());
        for index in 0..1_024 {
            groups
                .push(&BTreeMap::from([(
                    status.clone(),
                    json!(format!("g{index}")),
                )]))
                .unwrap();
        }
        assert_eq!(groups.buckets.len(), 1_024);
        assert!(matches!(
            groups.push(&BTreeMap::from([(status.clone(), json!("overflow"))])),
            Err(AggregateError::AggregateCostExceeded)
        ));

        let bucket_limits =
            AggregateLimits::try_new(4_097, 4_096, 10_000, 16 * 1024 * 1024).unwrap();
        let bucket_request = request(flight, vec![status.clone()], vec![count]);
        let mut buckets = AggregateBuilder::new(&bucket_request, &ty, bucket_limits);
        for index in 0..4_096 {
            buckets
                .push(&BTreeMap::from([(
                    status.clone(),
                    json!(format!("b{index}")),
                )]))
                .unwrap();
        }
        assert_eq!(buckets.bucket_count, 4_096);
        assert!(matches!(
            buckets.push(&BTreeMap::from([(status, json!("overflow"))])),
            Err(AggregateError::AggregateCostExceeded)
        ));
    }

    #[test]
    fn group_distinct_value_and_memory_bounds_are_inclusive_and_shared() {
        let (flight, ty) = fixture_type();
        let delay = property("delayMinutes");
        let status = property("status");
        let value_request = request(
            flight.clone(),
            vec![],
            vec![AggregateMeasure::Distinct {
                property: delay.clone(),
            }],
        );
        let mut values =
            AggregateBuilder::new(&value_request, &ty, AggregateLimits::phase4_default());
        for index in 0..10_000 {
            values
                .push(&BTreeMap::from([(delay.clone(), json!(index))]))
                .unwrap();
        }
        assert_eq!(values.distinct_count, 10_000);
        assert!(matches!(
            values.push(&BTreeMap::from([(delay.clone(), json!(10_000))])),
            Err(AggregateError::AggregateCostExceeded)
        ));

        let memory_request = request(
            flight.clone(),
            vec![],
            vec![AggregateMeasure::Distinct {
                property: status.clone(),
            }],
        );
        let mut memory =
            AggregateBuilder::new(&memory_request, &ty, AggregateLimits::phase4_default());
        let max_bytes = 16 * 1024 * 1024;
        memory
            .push(&BTreeMap::from([(
                status.clone(),
                json!("a".repeat(max_bytes - 2)),
            )]))
            .unwrap();
        assert_eq!(memory.distinct_bytes, max_bytes);
        let mut over_memory =
            AggregateBuilder::new(&memory_request, &ty, AggregateLimits::phase4_default());
        assert!(matches!(
            over_memory.push(&BTreeMap::from([(
                status.clone(),
                json!("a".repeat(max_bytes - 1))
            )])),
            Err(AggregateError::AggregateCostExceeded)
        ));

        let shared = request(
            flight,
            vec![status.clone()],
            vec![
                AggregateMeasure::Distinct {
                    property: delay.clone(),
                },
                AggregateMeasure::Distinct {
                    property: delay.clone(),
                },
            ],
        );
        let limits = AggregateLimits::try_new(2, 4, 2, 16 * 1024 * 1024).unwrap();
        let mut builder = AggregateBuilder::new(&shared, &ty, limits);
        builder
            .push(&BTreeMap::from([
                (status.clone(), json!("a")),
                (delay.clone(), json!(1)),
            ]))
            .unwrap();
        assert_eq!(builder.distinct_count, 2);
        assert!(matches!(
            builder.push(&BTreeMap::from([(status, json!("b")), (delay, json!(2))])),
            Err(AggregateError::AggregateCostExceeded)
        ));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        #[allow(clippy::cast_precision_loss, reason = "random oracle values and row counts are bounded below 40,000")]
        fn count_sum_avg_random_old_new_correction_matches_reference(
            rows in proptest::collection::vec((
                proptest::option::of(-100i16..100),
                proptest::option::of(-100i16..100),
                any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>(),
            ), 0..40),
        ) {
            let (flight, ty) = fixture_type();
            let status = property("status");
            let delay = property("delayMinutes");
            let request = request(
                flight,
                vec![status.clone()],
                vec![
                    AggregateMeasure::Count { property: None },
                    AggregateMeasure::Sum { property: delay.clone() },
                    AggregateMeasure::Avg { property: delay.clone() },
                ],
            );
            let mut builder = AggregateBuilder::new(&request, &ty, AggregateLimits::phase4_default());
            let visible_row = |group: bool, value: Option<i16>| BTreeMap::from([
                (status.clone(), json!(if group { "a" } else { "b" })),
                (delay.clone(), value.map_or(Value::Null, |n| json!(n))),
            ]);
            for source_group in [false, true] {
                let source_rows = rows.iter().filter(|(_, _, group, _, _, _)| *group == source_group)
                    .collect::<Vec<_>>();
                if source_rows.is_empty() {
                    continue;
                }
                let non_null = source_rows.iter().filter_map(|(value, ..)| *value).collect::<Vec<_>>();
                let sum: i64 = non_null.iter().map(|value| i64::from(*value)).sum();
                let count = non_null.len();
                builder.seed_partial(vec![
                    json!(if source_group { "a" } else { "b" }),
                    json!(source_rows.len()),
                    json!(source_rows.len()),
                    if count == 0 { Value::Null } else { json!(sum) },
                    json!(count),
                    if count == 0 { Value::Null } else { json!(sum) },
                    json!(count),
                ]).unwrap();
            }
            let mut oracle = BTreeMap::<String, (Vec<Value>, u64, i64, u64)>::new();
            for (old, new, old_group, new_group, edited, alive) in rows {
                let old_row = visible_row(old_group, old);
                if edited {
                    builder.subtract(&old_row).unwrap();
                }
                if !edited || alive {
                    let (group, value) = if edited { (new_group, new) } else { (old_group, old) };
                    let row = visible_row(group, value);
                    if edited {
                        builder.push(&row).unwrap();
                    }
                    let group = vec![json!(if group { "a" } else { "b" })];
                    let entry = oracle.entry(serde_json::to_string(&group).unwrap())
                        .or_insert_with(|| (group, 0, 0, 0));
                    entry.1 += 1;
                    if let Some(value) = value {
                        entry.2 += i64::from(value);
                        entry.3 += 1;
                    }
                }
            }
            let expected = AggregateResult {
                buckets: oracle.into_values().map(|(group, rows, sum, count)| AggregateBucket {
                    group,
                    values: vec![
                        json!(rows),
                        if count == 0 { Value::Null } else { json!(sum) },
                        if count == 0 { Value::Null } else { json!(sum as f64 / count as f64) },
                    ],
                }).collect(),
            };
            prop_assert_eq!(builder.finish().unwrap(), expected);
        }
    }
}
