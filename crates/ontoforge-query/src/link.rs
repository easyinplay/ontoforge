use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::Engine;
use ontoforge_schema::ir::{Cardinality, ObjectTypeName, PropertyName};
use ontoforge_storage::query_read::LinkEdit;
use ontoforge_storage::value::PropertyValue;

use crate::ast::{LinkDirection, ObjectSet, PageSpec, QueryLiteral, WhereClause};
use crate::compile::{CompileLimits, ValidatedQuery};
use crate::execute::WorkBudget;
use crate::{ListPage, ObjectRow, QueryError, QueryRequest, QueryService};

pub struct SearchAroundPlan {
    pub target_type: ObjectTypeName,
    pub fk: Option<PropertyName>,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkEdge {
    pub from: PropertyValue,
    pub to: PropertyValue,
}

pub fn merge_edges(source: Vec<LinkEdge>, edits: &[LinkEdit]) -> Result<Vec<LinkEdge>, QueryError> {
    let mut live = BTreeMap::new();
    for edge in source {
        live.insert(edge_key(&edge), edge);
    }
    let mut latest = BTreeMap::new();
    for edit in edits {
        if edit.row.version <= 0 {
            return Err(QueryError::Store);
        }
        let key = (
            edit.from.to_json().to_string(),
            edit.to.to_json().to_string(),
        );
        if let Some(previous) = latest.get(&key) {
            let previous: &LinkEdit = previous;
            if previous.row.version == edit.row.version && previous != edit {
                return Err(QueryError::Store);
            }
            if previous.row.version >= edit.row.version {
                continue;
            }
        }
        latest.insert(key, edit.clone());
    }
    for edit in latest.into_values() {
        let edge = LinkEdge {
            from: edit.from.clone(),
            to: edit.to.clone(),
        };
        if edit.row.deleted {
            live.remove(&edge_key(&edge));
        } else {
            live.insert(edge_key(&edge), edge);
        }
    }
    Ok(live.into_values().collect())
}

fn edge_key(edge: &LinkEdge) -> (String, String) {
    (
        edge.from.to_json().to_string(),
        edge.to.to_json().to_string(),
    )
}

impl<E: Engine> QueryService<'_, E> {
    pub(crate) async fn execute_search_around(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<ListPage, QueryError> {
        let ObjectSet::SearchAround { object_type, base, link, direction } = &query.request.set else {
            return Err(QueryError::InvalidQuery);
        };
        let definition = self.active.ontology.link_types.get(link).ok_or(QueryError::InvalidQuery)?;
        let (base_type, target_type) = match direction {
            LinkDirection::FromTo => (&definition.from.object_type, &definition.to.object_type),
            LinkDirection::ToFrom => (&definition.to.object_type, &definition.from.object_type),
        };
        if base.object_type() != base_type || object_type != target_type {
            return Err(QueryError::InvalidQuery);
        }
        let base_scope = self.scope.types.get(base_type).ok_or(QueryError::NotFound)?;
        let target_scope = self.scope.types.get(target_type).ok_or(QueryError::NotFound)?;
        let fk = definition.to.foreign_key.as_ref();
        let base_select = if *direction == LinkDirection::ToFrom && definition.cardinality != Cardinality::ManyToMany {
            vec![fk.ok_or(QueryError::InvalidQuery)?.clone()]
        } else {
            Vec::new()
        };
        let base_query = ValidatedQuery::new(
            QueryRequest { set: *base.clone(), select: base_select }, Vec::new(),
            PageSpec { first: self.limits.max_raw_candidates.min(10_000) as u64, after: None },
            &self.active.ontology, base_scope, CompileLimits::default(),
        )?;
        let base_rows = self.collect_rows(&base_query, budget).await?;
        let mut keys = BTreeSet::new();
        match definition.cardinality {
            Cardinality::OneToOne | Cardinality::OneToMany => match direction {
                LinkDirection::ToFrom => {
                    let fk = fk.ok_or(QueryError::InvalidQuery)?;
                    for row in base_rows {
                        if let Some(value) = row.properties.get(fk).filter(|value| !value.is_null()) {
                            keys.insert(value.to_string());
                        }
                    }
                }
                LinkDirection::FromTo => {
                    let fk = fk.ok_or(QueryError::InvalidQuery)?;
                    if !target_scope.columns.contains_key(fk) {
                        return Err(QueryError::NotFound);
                    }
                    for row in base_rows {
                        let child = ValidatedQuery::new(
                            QueryRequest {
                                set: ObjectSet::Where {
                                    object_type: target_type.clone(),
                                    predicate: WhereClause::Eq { property: fk.clone(), value: QueryLiteral(row.pk) },
                                },
                                select: Vec::new(),
                            },
                            Vec::new(),
                            PageSpec { first: self.limits.max_raw_candidates.min(10_000) as u64, after: None },
                            &self.active.ontology, target_scope, CompileLimits::default(),
                        )?;
                        keys.extend(self.collect_rows(&child, budget).await?.into_iter().map(|row| row.pk.to_string()));
                    }
                }
            },
            Cardinality::ManyToMany => return Err(QueryError::InvalidQuery),
        }
        let final_query = ValidatedQuery {
            request: QueryRequest { set: ObjectSet::Type { object_type: target_type.clone() }, select: query.request.select.clone() },
            order_by: query.order_by.clone(), page: query.page.clone(),
        };
        self.execute_list_internal(&final_query, budget, Some(&keys)).await
    }

    async fn collect_rows(
        &self,
        query: &ValidatedQuery,
        budget: &mut WorkBudget,
    ) -> Result<Vec<ObjectRow>, QueryError> {
        let mut rows = Vec::new();
        let mut after = None;
        loop {
            let current = ValidatedQuery {
                request: query.request.clone(), order_by: query.order_by.clone(),
                page: PageSpec { first: query.page.first, after },
            };
            let page = self.execute_list_internal(&current, budget, None).await?;
            if page.truncated { return Err(QueryError::WorkBudgetExceeded); }
            rows.extend(page.rows);
            if !page.has_more { return Ok(rows); }
            after = Some(page.cursor.ok_or(QueryError::WorkBudgetExceeded)?);
        }
    }
}
