//! Dialect specific printing.
//!
//! Differences that matter (Trino vs. the SQLite stand-in):
//!
//! * table references: `"cat"."sch"."tbl"` vs. `"cat__sch"."tbl"` (the ATTACH name),
//! * typed literals: Trino gets `DECIMAL '..'`, `DOUBLE '..'`, `DATE '..'`, `TIMESTAMP '..'`;
//!   SQLite gets plain strings / numbers; the stand-in's decimal and timestamp columns carry
//!   collations that compare such strings by value (`sqlite_ddl::DECIMAL_COLLATION`,
//!   `sqlite_ddl::TIMESTAMP_COLLATION`), so `'1.5'` equals a stored `1.50` as in Trino,
//! * string matching: Trino `LIKE '<pattern>' ESCAPE '\'`, SQLite `GLOB '<pattern>'`. `LIKE` is
//!   case insensitive for ASCII in SQLite, `GLOB` is case sensitive like Trino's `LIKE`, so the
//!   stand-in is never wider than production (ADR-0025),
//! * `ORDER BY` always spells out `NULLS FIRST` / `NULLS LAST` because the defaults differ.

use chrono::{NaiveDate, NaiveDateTime};

use super::ast::{AggregateFunc, CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use super::escape::{glob_pattern, like_pattern, quote_ident, quote_string};
use super::{Dialect, PrintedSql};
use crate::TableRef;
use crate::error::PrintError;
use crate::value::split_decimal;

pub(super) fn print_select(select: &Select, dialect: Dialect) -> Result<PrintedSql, PrintError> {
    if select.projection.is_empty() {
        return Err(PrintError::EmptyProjection);
    }
    let items = select
        .projection
        .iter()
        .map(|item| select_item(item, dialect))
        .collect::<Result<Vec<_>, _>>()?;
    let mut sql = String::from("SELECT ");
    sql.push_str(&items.join(", "));
    if let Some(table) = &select.from {
        sql.push_str(" FROM ");
        sql.push_str(&table_ref(table, dialect)?);
    }
    if let Some(filter) = &select.filter {
        sql.push_str(" WHERE ");
        sql.push_str(&expr(filter, dialect)?);
    }
    if !select.group_by.is_empty() {
        let keys = select
            .group_by
            .iter()
            .map(|e| expr(e, dialect))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(" GROUP BY ");
        sql.push_str(&keys.join(", "));
    }
    if let Some(having) = &select.having {
        sql.push_str(" HAVING ");
        sql.push_str(&expr(having, dialect)?);
    }
    if !select.order_by.is_empty() {
        let items = select
            .order_by
            .iter()
            .map(|o| order_item(o, dialect))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(" ORDER BY ");
        sql.push_str(&items.join(", "));
    }
    if let Some(limit) = select.limit {
        sql.push_str(" LIMIT ");
        sql.push_str(&limit.to_string());
    }
    Ok(PrintedSql::new(sql))
}

fn select_item(item: &SelectItem, dialect: Dialect) -> Result<String, PrintError> {
    let mut out = expr(&item.expr, dialect)?;
    if let Some(alias) = &item.alias {
        out.push_str(" AS ");
        out.push_str(&quote_ident(alias)?);
    }
    Ok(out)
}

fn order_item(item: &OrderItem, dialect: Dialect) -> Result<String, PrintError> {
    Ok(format!(
        "{} {} {}",
        expr(&item.expr, dialect)?,
        if item.descending { "DESC" } else { "ASC" },
        if item.nulls_first {
            "NULLS FIRST"
        } else {
            "NULLS LAST"
        }
    ))
}

pub(super) fn table_ref(table: &TableRef, dialect: Dialect) -> Result<String, PrintError> {
    match dialect {
        Dialect::Trino => Ok(format!(
            "{}.{}.{}",
            quote_ident(&table.catalog)?,
            quote_ident(&table.schema)?,
            quote_ident(&table.table)?
        )),
        Dialect::Sqlite => Ok(format!(
            "{}.{}",
            quote_ident(&table.sqlite_schema_name())?,
            quote_ident(&table.table)?
        )),
    }
}

fn expr(e: &Expr, dialect: Dialect) -> Result<String, PrintError> {
    match e {
        Expr::Column(name) => quote_ident(name),
        Expr::Literal(l) => literal(l, dialect),
        Expr::CaseWhen { .. } => Err(PrintError::UnsupportedDialect),
        Expr::CountStar => Ok("count(*)".to_owned()),
        Expr::Aggregate { func, arg } => Ok(format!(
            "{}({})",
            aggregate_name(*func),
            expr(arg, dialect)?
        )),
        Expr::Compare { left, op, right } => Ok(format!(
            "{} {} {}",
            operand(left, dialect)?,
            compare_op(*op),
            operand(right, dialect)?
        )),
        Expr::InList {
            expr: inner,
            list,
            negated,
        } => {
            if list.is_empty() {
                return Err(PrintError::EmptyInList);
            }
            let items = list
                .iter()
                .map(|l| literal(l, dialect))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!(
                "{} {}IN ({})",
                operand(inner, dialect)?,
                if *negated { "NOT " } else { "" },
                items.join(", ")
            ))
        }
        Expr::IsNull {
            expr: inner,
            negated,
        } => Ok(format!(
            "{} IS {}NULL",
            operand(inner, dialect)?,
            if *negated { "NOT " } else { "" }
        )),
        Expr::StringMatch {
            expr: inner,
            needle,
            kind,
        } => match dialect {
            Dialect::Trino => Ok(format!(
                "{} LIKE {} ESCAPE {}",
                operand(inner, dialect)?,
                quote_string(&like_pattern(needle, *kind)?)?,
                quote_string("\\")?
            )),
            Dialect::Sqlite => Ok(format!(
                "{} GLOB {}",
                operand(inner, dialect)?,
                quote_string(&glob_pattern(needle, *kind)?)?
            )),
        },
        Expr::And(parts) => bool_list(parts, " AND ", dialect),
        Expr::Or(parts) => bool_list(parts, " OR ", dialect),
        Expr::Not(inner) => Ok(format!("NOT ({})", expr(inner, dialect)?)),
    }
}

/// Prints `e` so it can sit next to an operator without changing precedence.
fn operand(e: &Expr, dialect: Dialect) -> Result<String, PrintError> {
    let text = expr(e, dialect)?;
    if matches!(
        e,
        Expr::Compare { .. }
            | Expr::InList { .. }
            | Expr::IsNull { .. }
            | Expr::StringMatch { .. }
            | Expr::And(_)
            | Expr::Or(_)
            | Expr::Not(_)
    ) {
        Ok(format!("({text})"))
    } else {
        Ok(text)
    }
}

fn bool_list(parts: &[Expr], sep: &str, dialect: Dialect) -> Result<String, PrintError> {
    if parts.is_empty() {
        return Err(PrintError::EmptyBoolList);
    }
    let printed = parts
        .iter()
        .map(|p| expr(p, dialect).map(|t| format!("({t})")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(printed.join(sep))
}

fn aggregate_name(f: AggregateFunc) -> &'static str {
    match f {
        AggregateFunc::Count => "count",
        AggregateFunc::Sum => "sum",
        AggregateFunc::Avg => "avg",
        AggregateFunc::Min => "min",
        AggregateFunc::Max => "max",
    }
}

fn compare_op(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "=",
        CompareOp::NotEq => "<>",
        CompareOp::Lt => "<",
        CompareOp::LtEq => "<=",
        CompareOp::Gt => ">",
        CompareOp::GtEq => ">=",
    }
}

fn literal(l: &Literal, dialect: Dialect) -> Result<String, PrintError> {
    let trino = dialect == Dialect::Trino;
    match l {
        Literal::Null => Ok("NULL".to_owned()),
        Literal::Boolean(b) => Ok(match (trino, b) {
            (true, true) => "TRUE",
            (true, false) => "FALSE",
            (false, true) => "1",
            (false, false) => "0",
        }
        .to_owned()),
        // `-9223372036854775808` is not a valid literal in Trino (the magnitude overflows
        // before the sign applies), so the minimum is spelled as an expression.
        Literal::Long(n) if *n == i64::MIN => Ok("(-9223372036854775807 - 1)".to_owned()),
        Literal::Long(n) => Ok(n.to_string()),
        Literal::Double(x) => {
            if !x.is_finite() {
                return Err(PrintError::NonFiniteDouble);
            }
            // `{:?}` prints the shortest text that round-trips.
            let text = format!("{x:?}");
            if trino {
                Ok(format!("DOUBLE {}", quote_string(&text)?))
            } else {
                Ok(text)
            }
        }
        Literal::Decimal(t) => {
            if split_decimal(t).is_none() {
                return Err(PrintError::InvalidDecimal(t.clone()));
            }
            typed(trino, "DECIMAL", t)
        }
        Literal::String(s) => quote_string(s),
        Literal::Date(t) => {
            if !is_date(t) {
                return Err(PrintError::InvalidDate(t.clone()));
            }
            typed(trino, "DATE", t)
        }
        Literal::Timestamp(t) => {
            if !is_timestamp(t) {
                return Err(PrintError::InvalidTimestamp(t.clone()));
            }
            typed(trino, "TIMESTAMP", t)
        }
    }
}

fn typed(trino: bool, prefix: &str, text: &str) -> Result<String, PrintError> {
    let quoted = quote_string(text)?;
    if trino {
        Ok(format!("{prefix} {quoted}"))
    } else {
        Ok(quoted)
    }
}

/// Whether `b` has the shape of `pattern`, where `d` stands for an ASCII digit and every
/// other pattern byte must match literally.
fn has_shape(b: &[u8], pattern: &[u8]) -> bool {
    b.len() == pattern.len()
        && b.iter().zip(pattern).all(|(c, p)| {
            if *p == b'd' {
                c.is_ascii_digit()
            } else {
                c == p
            }
        })
}

fn is_date(s: &str) -> bool {
    has_shape(s.as_bytes(), b"dddd-dd-dd") && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// `YYYY-MM-DD HH:MM:SS` with an optional fraction of 1 to 9 digits.
fn is_timestamp(s: &str) -> bool {
    let Some(head) = s.get(..19) else {
        return false;
    };
    let tail = &s[19..];
    let fraction_ok = tail.is_empty()
        || (tail.len() <= 10
            && tail.starts_with('.')
            && tail.len() > 1
            && tail[1..].bytes().all(|c| c.is_ascii_digit()));
    // Seconds 60 would be accepted by chrono as a leap second; Trino rejects them.
    fraction_ok
        && has_shape(head.as_bytes(), b"dddd-dd-dd dd:dd:dd")
        && &head[17..19] < "60"
        && NaiveDateTime::parse_from_str(head, "%Y-%m-%d %H:%M:%S").is_ok()
}
