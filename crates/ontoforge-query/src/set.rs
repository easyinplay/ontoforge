use std::collections::BTreeSet;

use ontoforge_engine::Engine;

use crate::ast::{ObjectSet, PageSpec};
use crate::compile::ValidatedQuery;
use crate::execute::WorkBudget;
use crate::{ListPage, QueryError, QueryRequest, QueryService};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperator {
    Union,
    Intersect,
    Subtract,
}

pub fn evaluate_set_operation(
    operator: SetOperator,
    operands: &[BTreeSet<String>],
) -> Result<BTreeSet<String>, QueryError> {
    let result = match operator {
        SetOperator::Union => operands
            .iter()
            .flat_map(|set| set.iter().cloned())
            .collect(),
        SetOperator::Intersect => operands
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        SetOperator::Subtract => operands
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| !set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
    };
    Ok(result)
}

impl<E: Engine> QueryService<'_, E> {
    pub(crate) async fn execute_set_list(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<ListPage, QueryError> {
        let keys = self.authorized_keys(&query.request.set, budget).await?;
        let final_query = ValidatedQuery {
            request: QueryRequest {
                set: ObjectSet::Type {
                    object_type: query.request.set.object_type().clone(),
                },
                select: query.request.select.clone(),
            },
            order_by: query.order_by.clone(),
            page: query.page.clone(),
        };
        self.execute_list_internal(&final_query, budget, Some(&keys))
            .await
    }

    async fn authorized_keys(
        &self,
        set: &ObjectSet,
        budget: &mut WorkBudget,
    ) -> Result<BTreeSet<String>, QueryError> {
        let (operator, children): (SetOperator, Vec<&ObjectSet>) = match set {
            ObjectSet::Union { sets, .. } => (SetOperator::Union, sets.iter().collect()),
            ObjectSet::Intersect { sets, .. } => (SetOperator::Intersect, sets.iter().collect()),
            ObjectSet::Subtract { left, right, .. } => (SetOperator::Subtract, vec![left, right]),
            _ => return self.leaf_authorized_keys(set, budget).await,
        };
        let mut operands = Vec::with_capacity(children.len());
        for child in children {
            operands.push(Box::pin(self.authorized_keys(child, budget)).await?);
        }
        evaluate_set_operation(operator, &operands)
    }

    async fn leaf_authorized_keys(
        &self,
        set: &ObjectSet,
        budget: &mut WorkBudget,
    ) -> Result<BTreeSet<String>, QueryError> {
        let mut keys = BTreeSet::new();
        let mut after = None;
        loop {
            let query = ValidatedQuery {
                request: QueryRequest {
                    set: set.clone(),
                    select: Vec::new(),
                },
                order_by: Vec::new(),
                page: PageSpec {
                    first: u64::try_from(self.limits.max_raw_candidates.min(10_000))
                        .map_err(|_| QueryError::InvalidQuery)?,
                    after,
                },
            };
            let page = if matches!(set, ObjectSet::SearchAround { .. }) {
                Box::pin(self.execute_search_around(&query, budget)).await?
            } else {
                self.execute_list_internal(&query, budget, None).await?
            };
            if page.truncated {
                return Err(QueryError::WorkBudgetExceeded);
            }
            keys.extend(page.rows.into_iter().map(|row| row.pk.to_string()));
            if !page.has_more {
                return Ok(keys);
            }
            after = Some(page.cursor.ok_or(QueryError::WorkBudgetExceeded)?);
        }
    }
}
