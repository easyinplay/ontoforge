use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use ontoforge_api::dto as wire;
use ontoforge_engine::Engine;
use ontoforge_query::aggregate::{AggregateCapabilities, AggregateLimits};
use ontoforge_query::ast;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use serde_json::{Value, json};

use crate::{ServerHost, context, error};

const MAX_BODY: usize = 1024 * 1024;

pub async fn ontology<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
    request: Request,
) -> Response {
    let headers = request.headers().clone();
    let invalid = request.uri().query().is_some_and(|query| !query.is_empty())
        || to_bytes(request.into_body(), 1)
            .await
            .map_or(true, |body| !body.is_empty());
    let (ctx, request_id) = match context::context_from_headers(&host, headers, "ontology").await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if invalid {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "ontology",
            &request_id,
        );
    }
    let mut object_types = Vec::new();
    for (name, ty) in &ctx.active.ontology.object_types {
        if ty.hidden || !ctx.scope.types.contains_key(name) {
            continue;
        }
        let scope = &ctx.scope.types[name];
        let properties = ty
            .properties
            .iter()
            .filter(|(property, _)| scope.columns.contains_key(*property))
            .map(|(property, def)| wire::PropertyMetadata {
                api_name: property.to_string(),
                r#type: format!("{:?}", def.base_type),
                required: def.required,
                display_name: def.display_name.clone(),
                description: def.description.clone(),
            })
            .collect();
        object_types.push(wire::ObjectMetadata {
            api_name: name.to_string(),
            display_name: ty.display_name.clone(),
            description: ty.description.clone(),
            aliases: ty.aliases.clone(),
            primary_key: ty.primary_key.to_string(),
            title_key: ty.effective_title_key().to_string(),
            properties,
        });
    }
    let links = ctx
        .active
        .ontology
        .link_types
        .values()
        .filter(|link| {
            ctx.scope.types.contains_key(&link.from.object_type)
                && ctx.scope.types.contains_key(&link.to.object_type)
        })
        .map(|link| wire::LinkMetadata {
            api_name: link.api_name.to_string(),
            display_name: link.display_name.clone(),
            description: link.description.clone(),
            from: link.from.object_type.to_string(),
            to: link.to.object_type.to_string(),
        })
        .collect();
    let actions = ctx
        .active
        .ontology
        .action_types
        .values()
        .filter(|action| ctx.scope.actions.contains(&action.api_name))
        .map(|action| wire::ActionMetadata {
            api_name: action.api_name.to_string(),
            display_name: action.display_name.clone(),
            description: action.description.clone(),
        })
        .collect();
    error::response(
        StatusCode::OK,
        &request_id,
        json!(wire::Metadata {
            api_name: ctx.active.ontology.api_name.to_string(),
            display_name: ctx.active.ontology.display_name.clone(),
            description: ctx.active.ontology.description.clone(),
            object_types,
            links,
            actions,
            def_version: ctx.active.def_version
        }),
    )
}

pub async fn object<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
    Path((api_name, pk_text)): Path<(String, String)>,
    request: Request,
) -> Response {
    let headers = request.headers().clone();
    let invalid = request.uri().query().is_some_and(|query| !query.is_empty())
        || to_bytes(request.into_body(), 1)
            .await
            .map_or(true, |body| !body.is_empty());
    let (ctx, request_id) = match context::context_from_headers(&host, headers, &api_name).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if invalid {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            &api_name,
            &request_id,
        );
    }
    let Some(ty) = ctx.active.ontology.object_types.get(api_name.as_str()) else {
        return error::wire(StatusCode::NOT_FOUND, "NOT_FOUND", &api_name, &request_id);
    };
    let pk = match ty.properties.get(&ty.primary_key).map(|p| p.base_type) {
        Some(
            ontoforge_schema::ir::BaseType::String
            | ontoforge_schema::ir::BaseType::Date
            | ontoforge_schema::ir::BaseType::Timestamp,
        ) => Value::String(pk_text),
        _ => serde_json::from_str(&pk_text).unwrap_or(Value::String(pk_text)),
    };
    let object_type = match ObjectTypeName::try_from(api_name.to_owned()) {
        Ok(value) => value,
        Err(_) => return error::wire(StatusCode::NOT_FOUND, "NOT_FOUND", &api_name, &request_id),
    };
    let execution = QueryService {
        principal: &ctx.principal,
        scope: &ctx.scope,
        active: Arc::clone(&ctx.active),
        engine: &*host.engine,
        store: &host.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &host.tokens,
    }
    .execute_audited(QueryRequest {
        set: ObjectSet::Pk { object_type, pk },
        select: ty.properties.keys().cloned().collect(),
    })
    .await;
    let id = execution.request_id.as_deref().unwrap_or(&request_id);
    match execution.result {
        Ok(page) => page.rows.into_iter().next().map_or_else(
            || error::wire(StatusCode::NOT_FOUND, "NOT_FOUND", &api_name, id),
            |row| error::response(StatusCode::OK, id, json!(row)),
        ),
        Err(e) => error::wire(error::query_status(&e), e.code(), &api_name, id),
    }
}

pub async fn query<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
    headers: axum::http::HeaderMap,
    body: Body,
) -> Response {
    let (ctx, request_id, body) = match body_context(&host, headers, body, "query").await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let input: wire::QueryRequest = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return error::json_rejection("query", &request_id),
    };
    let Ok(set) = convert_set(input.set) else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "query",
            &request_id,
        );
    };
    let Ok(select) = input
        .select
        .into_iter()
        .map(PropertyName::try_from)
        .collect::<Result<Vec<_>, _>>()
    else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "query",
            &request_id,
        );
    };
    let Ok(order_by) = input
        .order_by
        .into_iter()
        .map(convert_order)
        .collect::<Result<Vec<_>, _>>()
    else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "query",
            &request_id,
        );
    };
    let object_type = set.object_type().clone();
    let Some(scope) = ctx.scope.types.get(&object_type) else {
        return error::wire(StatusCode::NOT_FOUND, "NOT_FOUND", "query", &request_id);
    };
    let request_ast = QueryRequest { set, select };
    let page = ast::PageSpec {
        first: input.page.first,
        after: input.page.after,
    };
    let validated = match ValidatedQuery::new(
        request_ast,
        order_by,
        page,
        &ctx.active.ontology,
        scope,
        CompileLimits::default(),
    ) {
        Ok(value) => value,
        Err(e) => return error::wire(error::query_status(&e), e.code(), "query", &request_id),
    };
    let result = QueryService {
        principal: &ctx.principal,
        scope: &ctx.scope,
        active: Arc::clone(&ctx.active),
        engine: &*host.engine,
        store: &host.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &host.tokens,
    }
    .execute_list(&validated)
    .await;
    match result {
        Ok(value) => error::response(StatusCode::OK, &request_id, json!(value)),
        Err(e) => error::wire(error::query_status(&e), e.code(), "query", &request_id),
    }
}

pub async fn aggregate<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
    headers: axum::http::HeaderMap,
    body: Body,
) -> Response {
    let (ctx, request_id, body) = match body_context(&host, headers, body, "aggregate").await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let input: wire::AggregateRequest = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return error::json_rejection("aggregate", &request_id),
    };
    let Ok(set) = convert_set(input.set) else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "aggregate",
            &request_id,
        );
    };
    let Ok(group_by) = input
        .group_by
        .into_iter()
        .map(PropertyName::try_from)
        .collect::<Result<Vec<_>, _>>()
    else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "aggregate",
            &request_id,
        );
    };
    let Ok(measures) = input
        .measures
        .into_iter()
        .map(convert_measure)
        .collect::<Result<Vec<_>, _>>()
    else {
        return error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "aggregate",
            &request_id,
        );
    };
    let result = QueryService {
        principal: &ctx.principal,
        scope: &ctx.scope,
        active: Arc::clone(&ctx.active),
        engine: &*host.engine,
        store: &host.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &host.tokens,
    }
    .execute_aggregate(
        ast::AggregateRequest {
            set,
            group_by,
            measures,
        },
        AggregateCapabilities::all_supported(),
        AggregateLimits::phase4_default(),
    )
    .await;
    match result {
        Ok(value) => error::response(StatusCode::OK, &request_id, json!(value)),
        Err(e) => error::wire(
            error::aggregate_status(&e),
            error::aggregate_code(&e),
            "aggregate",
            &request_id,
        ),
    }
}

async fn body_context<E: Engine + Send + Sync + 'static>(
    host: &ServerHost<E>,
    headers: axum::http::HeaderMap,
    body: Body,
    api_name: &str,
) -> Result<(crate::context::RequestContext, String, Vec<u8>), Response> {
    let (ctx, request_id) = context::context_from_headers(host, headers, api_name).await?;
    let body = to_bytes(body, MAX_BODY)
        .await
        .map_err(|_| {
            error::wire(
                StatusCode::PAYLOAD_TOO_LARGE,
                "BODY_TOO_LARGE",
                api_name,
                &request_id,
            )
        })?
        .to_vec();
    Ok((ctx, request_id, body))
}

fn convert_set(input: wire::ObjectSet) -> Result<ObjectSet, ()> {
    Ok(match input {
        wire::ObjectSet::Pk { object_type, pk } => ObjectSet::Pk {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            pk,
        },
        wire::ObjectSet::Type { object_type } => ObjectSet::Type {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
        },
        wire::ObjectSet::Where {
            object_type,
            predicate,
        } => ObjectSet::Where {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            predicate: convert_where(predicate)?,
        },
        wire::ObjectSet::Search {
            object_type,
            property,
            query,
            mode,
        } => ObjectSet::Search {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            property: PropertyName::try_from(property).map_err(|_| ())?,
            query,
            mode: match mode {
                wire::TermMode::Any => ast::TermMode::Any,
                wire::TermMode::All => ast::TermMode::All,
            },
        },
        wire::ObjectSet::Union { object_type, sets } => ObjectSet::Union {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            sets: sets
                .into_iter()
                .map(convert_set)
                .collect::<Result<_, _>>()?,
        },
        wire::ObjectSet::Intersect { object_type, sets } => ObjectSet::Intersect {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            sets: sets
                .into_iter()
                .map(convert_set)
                .collect::<Result<_, _>>()?,
        },
        wire::ObjectSet::Subtract {
            object_type,
            left,
            right,
        } => ObjectSet::Subtract {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            left: Box::new(convert_set(*left)?),
            right: Box::new(convert_set(*right)?),
        },
        wire::ObjectSet::SearchAround {
            object_type,
            base,
            link,
            direction,
        } => ObjectSet::SearchAround {
            object_type: ObjectTypeName::try_from(object_type).map_err(|_| ())?,
            base: Box::new(convert_set(*base)?),
            link: ontoforge_schema::ir::LinkTypeName::try_from(link).map_err(|_| ())?,
            direction: match direction {
                wire::LinkDirection::FromTo => ast::LinkDirection::FromTo,
                wire::LinkDirection::ToFrom => ast::LinkDirection::ToFrom,
            },
        },
    })
}

fn convert_where(input: wire::WhereClause) -> Result<ast::WhereClause, ()> {
    use ast::WhereClause as A;
    use wire::WhereClause as W;
    let prop = |value: String| PropertyName::try_from(value).map_err(|_| ());
    Ok(match input {
        W::Eq { property, value } => A::Eq {
            property: prop(property)?,
            value: ast::QueryLiteral(value),
        },
        W::Gt { property, value } => A::Gt {
            property: prop(property)?,
            value: ast::QueryLiteral(value),
        },
        W::Gte { property, value } => A::Gte {
            property: prop(property)?,
            value: ast::QueryLiteral(value),
        },
        W::Lt { property, value } => A::Lt {
            property: prop(property)?,
            value: ast::QueryLiteral(value),
        },
        W::Lte { property, value } => A::Lte {
            property: prop(property)?,
            value: ast::QueryLiteral(value),
        },
        W::IsNull { property } => A::IsNull {
            property: prop(property)?,
        },
        W::In { property, values } => A::In {
            property: prop(property)?,
            values: values.into_iter().map(ast::QueryLiteral).collect(),
        },
        W::StartsWith { property, value } => A::StartsWith {
            property: prop(property)?,
            value,
        },
        W::ContainsAnyTerm { property, value } => A::ContainsAnyTerm {
            property: prop(property)?,
            value,
        },
        W::ContainsAllTerms { property, value } => A::ContainsAllTerms {
            property: prop(property)?,
            value,
        },
        W::And { clauses } => A::And {
            clauses: clauses
                .into_iter()
                .map(convert_where)
                .collect::<Result<_, _>>()?,
        },
        W::Or { clauses } => A::Or {
            clauses: clauses
                .into_iter()
                .map(convert_where)
                .collect::<Result<_, _>>()?,
        },
        W::Not { clause } => A::Not {
            clause: Box::new(convert_where(*clause)?),
        },
    })
}

fn convert_order(input: wire::OrderBy) -> Result<ast::OrderBy, ()> {
    Ok(ast::OrderBy {
        property: PropertyName::try_from(input.property).map_err(|_| ())?,
        direction: match input.direction {
            wire::Direction::Asc => ast::Direction::Asc,
            wire::Direction::Desc => ast::Direction::Desc,
        },
        nulls: match input.nulls {
            wire::Nulls::First => ast::Nulls::First,
            wire::Nulls::Last => ast::Nulls::Last,
        },
    })
}
fn convert_measure(input: wire::AggregateMeasure) -> Result<ast::AggregateMeasure, ()> {
    Ok(match input {
        wire::AggregateMeasure::Count { property } => ast::AggregateMeasure::Count {
            property: property
                .map(|p| PropertyName::try_from(p))
                .transpose()
                .map_err(|_| ())?,
        },
        wire::AggregateMeasure::Sum { property } => ast::AggregateMeasure::Sum {
            property: PropertyName::try_from(property).map_err(|_| ())?,
        },
        wire::AggregateMeasure::Avg { property } => ast::AggregateMeasure::Avg {
            property: PropertyName::try_from(property).map_err(|_| ())?,
        },
        wire::AggregateMeasure::Min { property } => ast::AggregateMeasure::Min {
            property: PropertyName::try_from(property).map_err(|_| ())?,
        },
        wire::AggregateMeasure::Max { property } => ast::AggregateMeasure::Max {
            property: PropertyName::try_from(property).map_err(|_| ())?,
        },
        wire::AggregateMeasure::Distinct { property } => ast::AggregateMeasure::Distinct {
            property: PropertyName::try_from(property).map_err(|_| ())?,
        },
    })
}
