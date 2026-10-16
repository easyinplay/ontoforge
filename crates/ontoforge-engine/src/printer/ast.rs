//! Typed AST of the SQL subset ontoforge generates.
//!
//! Nodes carry raw values. Escaping happens only while printing, so there is no "already
//! escaped string" type that could be abused to smuggle SQL text past the printer.
//!
//! Phase 4 note: a `LIKE` node must route its pattern wildcard escaping through the same
//! `escape` exit as every other literal.

use crate::TableRef;

/// A single-table `SELECT`.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub projection: Vec<SelectItem>,
    pub from: Option<TableRef>,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderItem>,
    pub limit: Option<u64>,
}

/// One projection item with an optional alias.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Column(String),
    Literal(Literal),
    CountStar,
    Aggregate {
        func: AggregateFunc,
        arg: Box<Expr>,
    },
    Compare {
        left: Box<Expr>,
        op: CompareOp,
        right: Box<Expr>,
    },
    InList {
        expr: Box<Expr>,
        list: Vec<Literal>,
        negated: bool,
    },
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Boolean(bool),
    Long(i64),
    Double(f64),
    /// Decimal text `-?digits(.digits)?`.
    Decimal(String),
    String(String),
    /// `YYYY-MM-DD`.
    Date(String),
    /// `YYYY-MM-DD HH:MM:SS(.fff)?`.
    Timestamp(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// One `ORDER BY` item. NULL ordering is always spelled out when printing.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderItem {
    pub expr: Expr,
    pub descending: bool,
    pub nulls_first: bool,
}
