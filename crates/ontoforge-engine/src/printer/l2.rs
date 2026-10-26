//! The only path from untrusted L2 SQL to the private `PrintedSql` constructor.

use std::collections::BTreeSet;
use std::ops::ControlFlow;
use std::thread;

use sqlparser::ast::{
    DataType, Distinct, Expr, Fetch, GroupByExpr, LimitClause, ObjectName, OffsetRows, Query,
    Select, SelectFlavor, SetExpr, SetQuantifier, Statement, TableFactor, Visit, VisitMut, Visitor,
    VisitorMut,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use thiserror::Error;

use super::{Dialect, PrintedSql, print_explain_distributed_sql};

const MAX_SQL_BYTES: usize = 32 * 1024;
const PARSER_STACK_BYTES: usize = 16 * 1024 * 1024;

/// Grants and deployment exclusions established by the caller's scope snapshot.
#[derive(Clone)]
pub struct L2Policy {
    pub raw_allowed: bool,
    pub catalogs: BTreeSet<String>,
    pub schemas: BTreeSet<String>,
    pub edit_catalog: String,
    pub max_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum L2Error {
    #[error("L2 access denied")]
    Denied,
    #[error("unsupported or invalid L2 SQL")]
    Unsupported,
}

/// Validates one SQL statement, returning only its normalized, bounded Display text.
pub fn validate_l2(raw: &str, policy: &L2Policy) -> Result<PrintedSql, L2Error> {
    if !policy.raw_allowed {
        return Err(L2Error::Denied);
    }
    if raw.is_empty() || raw.len() > MAX_SQL_BYTES || policy.max_rows == 0 {
        return Err(L2Error::Unsupported);
    }
    let raw = raw.to_owned();
    let policy = policy.clone();
    thread::Builder::new()
        .stack_size(PARSER_STACK_BYTES)
        .spawn(move || validate_on_parser_stack(&raw, &policy))
        .map_err(|_| L2Error::Unsupported)?
        .join()
        .map_err(|_| L2Error::Unsupported)?
}

/// Only a validated SQL value can be wrapped for a distributed EXPLAIN.
pub fn explain_l2(sql: &PrintedSql) -> PrintedSql {
    print_explain_distributed_sql(sql, Dialect::Trino).expect("Trino EXPLAIN is printable")
}

fn validate_on_parser_stack(raw: &str, policy: &L2Policy) -> Result<PrintedSql, L2Error> {
    let mut statements =
        Parser::parse_sql(&GenericDialect {}, raw).map_err(|_| L2Error::Unsupported)?;
    if statements.len() != 1 {
        return Err(L2Error::Unsupported);
    }
    let mut statement = statements.pop().expect("checked single statement");
    let Statement::Query(query) = &mut statement else {
        return Err(L2Error::Unsupported);
    };
    if !matches!(
        query.body.as_ref(),
        SetExpr::Select(_) | SetExpr::SetOperation { .. }
    ) {
        return Err(L2Error::Unsupported);
    }
    let mut validator = Validator {
        policy,
        ctes: Vec::new(),
    };
    if let ControlFlow::Break(error) = statement.visit(&mut validator) {
        return Err(error);
    }
    rewrite_limits(&mut statement, policy.max_rows)?;
    let display = statement.to_string();
    let reparsed =
        Parser::parse_sql(&GenericDialect {}, &display).map_err(|_| L2Error::Unsupported)?;
    if reparsed.as_slice() != [statement] {
        return Err(L2Error::Unsupported);
    }
    Ok(PrintedSql::new(display))
}

struct Validator<'a> {
    policy: &'a L2Policy,
    ctes: Vec<BTreeSet<String>>,
}

fn validate_set_expr(body: &SetExpr) -> Result<(), L2Error> {
    match body {
        SetExpr::Select(_) | SetExpr::Query(_) => Ok(()),
        SetExpr::SetOperation {
            left,
            right,
            set_quantifier: SetQuantifier::All | SetQuantifier::Distinct | SetQuantifier::None,
            ..
        } => {
            validate_set_expr(left)?;
            validate_set_expr(right)
        }
        SetExpr::Values(values) if !values.explicit_row && !values.value_keyword => Ok(()),
        _ => Err(L2Error::Unsupported),
    }
}

impl Visitor for Validator<'_> {
    type Break = L2Error;

    fn pre_visit_statement(&mut self, statement: &Statement) -> ControlFlow<Self::Break> {
        if matches!(statement, Statement::Query(_)) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(L2Error::Unsupported)
        }
    }

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        if !query.locks.is_empty()
            || query.for_clause.is_some()
            || query.settings.is_some()
            || query.format_clause.is_some()
            || !query.pipe_operators.is_empty()
            || !valid_limit(query)
        {
            return ControlFlow::Break(L2Error::Unsupported);
        }
        if let Err(error) = validate_set_expr(&query.body) {
            return ControlFlow::Break(error);
        }
        let mut names = BTreeSet::new();
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                if cte.from.is_some()
                    || cte.materialized.is_some()
                    || !names.insert(cte.alias.name.value.to_lowercase())
                {
                    return ControlFlow::Break(L2Error::Unsupported);
                }
            }
        }
        self.ctes.push(names);
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &Query) -> ControlFlow<Self::Break> {
        self.ctes.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_select(&mut self, select: &Select) -> ControlFlow<Self::Break> {
        let supported = select.optimizer_hints.is_empty()
            && select.select_modifiers.is_none()
            && select.top.is_none()
            && select.exclude.is_none()
            && select.into.is_none()
            && select.lateral_views.is_empty()
            && select.prewhere.is_none()
            && select.connect_by.is_empty()
            && select.cluster_by.is_empty()
            && select.distribute_by.is_empty()
            && select.sort_by.is_empty()
            && select.qualify.is_none()
            && select.value_table_mode.is_none()
            && select.flavor == SelectFlavor::Standard
            && !matches!(select.group_by, GroupByExpr::All(_))
            && !matches!(select.distinct, Some(Distinct::On(_)));
        if supported {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(L2Error::Unsupported)
        }
    }

    fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<Self::Break> {
        let supported = match factor {
            TableFactor::Table {
                args,
                with_hints,
                version,
                with_ordinality,
                partitions,
                json_path,
                sample,
                index_hints,
                ..
            } => {
                args.is_none()
                    && with_hints.is_empty()
                    && version.is_none()
                    && !with_ordinality
                    && partitions.is_empty()
                    && json_path.is_none()
                    && sample.is_none()
                    && index_hints.is_empty()
            }
            TableFactor::Derived { sample, .. } => sample.is_none(),
            TableFactor::NestedJoin { .. } => true,
            _ => false,
        };
        if supported {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(L2Error::Unsupported)
        }
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        let Some(parts) = relation
            .0
            .iter()
            .map(|part| part.as_ident().map(|i| i.value.to_lowercase()))
            .collect::<Option<Vec<_>>>()
        else {
            return ControlFlow::Break(L2Error::Denied);
        };
        if parts.len() == 1
            && self
                .ctes
                .iter()
                .rev()
                .any(|names| names.contains(&parts[0]))
        {
            return ControlFlow::Continue(());
        }
        if parts.len() != 3 {
            return ControlFlow::Break(L2Error::Denied);
        }
        let (catalog, schema, table) = (&parts[0], &parts[1], &parts[2]);
        let allowed = catalog != &self.policy.edit_catalog.to_lowercase()
            && catalog != "system"
            && schema != "system"
            && !table.contains('$')
            && (self
                .policy
                .catalogs
                .iter()
                .any(|c| c.eq_ignore_ascii_case(catalog))
                || self
                    .policy
                    .schemas
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&format!("{catalog}.{schema}"))));
        if allowed {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(L2Error::Denied)
        }
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        let supported = match expr {
            Expr::Function(function) => {
                function.name.0.len() == 1 && function.name.0[0].as_ident().is_some()
            }
            Expr::Cast {
                data_type, format, ..
            } => format.is_none() && !matches!(data_type, DataType::Custom(..)),
            Expr::Identifier(_)
            | Expr::CompoundIdentifier(_)
            | Expr::CompoundFieldAccess { .. }
            | Expr::IsFalse(_)
            | Expr::IsNotFalse(_)
            | Expr::IsTrue(_)
            | Expr::IsNotTrue(_)
            | Expr::IsNull(_)
            | Expr::IsNotNull(_)
            | Expr::IsDistinctFrom(_, _)
            | Expr::IsNotDistinctFrom(_, _)
            | Expr::InList { .. }
            | Expr::InSubquery { .. }
            | Expr::Between { .. }
            | Expr::BinaryOp { .. }
            | Expr::Like { .. }
            | Expr::UnaryOp { .. }
            | Expr::Nested(_)
            | Expr::Value(_)
            | Expr::TypedString(_)
            | Expr::Case { .. }
            | Expr::Exists { .. }
            | Expr::Subquery(_)
            | Expr::Tuple(_)
            | Expr::Array(_)
            | Expr::Interval(_)
            | Expr::Wildcard(_)
            | Expr::QualifiedWildcard(_, _) => true,
            _ => false,
        };
        if supported {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(L2Error::Unsupported)
        }
    }
}

fn literal_nonnegative(expr: &Expr) -> Option<u64> {
    let Expr::Value(value) = expr else {
        return None;
    };
    let sqlparser::ast::Value::Number(number, _) = &value.value else {
        return None;
    };
    number.parse().ok()
}

fn valid_limit(query: &Query) -> bool {
    let limit_ok = match &query.limit_clause {
        None => true,
        Some(LimitClause::LimitOffset {
            limit,
            offset,
            limit_by,
        }) => {
            limit_by.is_empty()
                && limit
                    .as_ref()
                    .is_none_or(|e| literal_nonnegative(e).is_some())
                && offset
                    .as_ref()
                    .is_none_or(|o| literal_nonnegative(&o.value).is_some())
        }
        _ => false,
    };
    limit_ok
        && query.fetch.as_ref().is_none_or(|f| {
            !f.percent
                && !f.with_ties
                && f.quantity
                    .as_ref()
                    .is_some_and(|e| literal_nonnegative(e).is_some())
        })
        && !(query.fetch.is_some()
            && query
                .limit_clause
                .as_ref()
                .is_some_and(|l| matches!(l, LimitClause::LimitOffset { limit: Some(_), .. })))
}

fn rewrite_limits(statement: &mut Statement, max_rows: u64) -> Result<(), L2Error> {
    struct Rewrite;
    impl VisitorMut for Rewrite {
        type Break = L2Error;
        fn post_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
            if let Some(LimitClause::LimitOffset {
                limit,
                offset: Some(offset),
                limit_by,
            }) = &mut query.limit_clause
            {
                if !limit_by.is_empty() {
                    return ControlFlow::Break(L2Error::Unsupported);
                }
                offset.rows = OffsetRows::Rows;
                if let Some(quantity) = limit.take() {
                    if query.fetch.is_some() {
                        return ControlFlow::Break(L2Error::Unsupported);
                    }
                    query.fetch = Some(Fetch {
                        with_ties: false,
                        percent: false,
                        quantity: Some(quantity),
                    });
                }
            }
            ControlFlow::Continue(())
        }
    }
    if let ControlFlow::Break(error) = statement.visit(&mut Rewrite) {
        return Err(error);
    }
    let Statement::Query(query) = statement else {
        return Err(L2Error::Unsupported);
    };
    let cap = max_rows.checked_add(1).ok_or(L2Error::Unsupported)?;
    let requested = match &query.limit_clause {
        Some(LimitClause::LimitOffset {
            limit: Some(expr), ..
        }) => literal_nonnegative(expr),
        _ => query
            .fetch
            .as_ref()
            .and_then(|f| f.quantity.as_ref())
            .and_then(literal_nonnegative),
    };
    if requested.is_none_or(|n| n > cap) {
        let cap_expr = Expr::Value(sqlparser::ast::ValueWithSpan::from(
            sqlparser::ast::Value::Number(cap.to_string(), false),
        ));
        if let Some(fetch) = &mut query.fetch {
            fetch.quantity = Some(cap_expr);
        } else if matches!(
            &query.limit_clause,
            Some(LimitClause::LimitOffset {
                offset: Some(_),
                ..
            })
        ) {
            query.fetch = Some(Fetch {
                with_ties: false,
                percent: false,
                quantity: Some(cap_expr),
            });
        } else {
            query.limit_clause = Some(LimitClause::LimitOffset {
                limit: Some(cap_expr),
                offset: None,
                limit_by: Vec::new(),
            });
        }
    }
    Ok(())
}
