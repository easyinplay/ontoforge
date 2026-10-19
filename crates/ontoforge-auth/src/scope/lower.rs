//! `Pred` to printer AST, with the principal's attributes substituted as literals.
//!
//! Every rule here mirrors `ontoforge_expr::eval` (ADR-0025), so that the printed SQL and
//! the Rust evaluator give the same truth value for the same row and principal:
//!
//! - a missing principal attribute becomes `NULL`, so every comparison with it is Unknown;
//! - `x in []` becomes the constant `FALSE`, because the printer rejects an empty `IN ()`
//!   and Trino does not accept one; the evaluator also gives False for a NULL `x`;
//! - `x in principal.attrs.<list>` with a missing list is Unknown for every `x`, which is
//!   `<operand> = NULL` in SQL.

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal};
use ontoforge_expr::{AttrValue, Attrs, CmpOp, ListSource, Operand, Pred, Scalar};
use ontoforge_schema::ir::PropertyName;

/// Lowers `pred` to a printer expression. `column` maps a property to its SQL column name.
pub fn lower(pred: &Pred, attrs: &Attrs, column: &dyn Fn(&PropertyName) -> String) -> Expr {
    match pred {
        Pred::Const(b) => Expr::Literal(Literal::Boolean(*b)),
        Pred::Cmp { left, op, right } => Expr::Compare {
            left: Box::new(operand(left, attrs, column)),
            op: compare_op(*op),
            right: Box::new(operand(right, attrs, column)),
        },
        Pred::IsNull {
            operand: o,
            negated,
        } => Expr::IsNull {
            expr: Box::new(operand(o, attrs, column)),
            negated: *negated,
        },
        Pred::InList { operand: o, list } => in_list(o, list, attrs, column),
        // Replaced by the real `StringMatch` lowering together with the printer node.
        Pred::StrMatch { column: c, .. } => unknown(Expr::Column(column(c))),
        Pred::And(..) => {
            let mut parts = Vec::new();
            collect(pred, true, attrs, column, &mut parts);
            Expr::And(parts)
        }
        Pred::Or(..) => {
            let mut parts = Vec::new();
            collect(pred, false, attrs, column, &mut parts);
            Expr::Or(parts)
        }
        Pred::Not(inner) => Expr::Not(Box::new(lower(inner, attrs, column))),
    }
}

/// Flattens a chain of the same connective into one n-ary list. AND and OR are associative
/// in SQL's three-valued logic, so the flattening does not change the result.
fn collect(
    pred: &Pred,
    and: bool,
    attrs: &Attrs,
    column: &dyn Fn(&PropertyName) -> String,
    out: &mut Vec<Expr>,
) {
    match pred {
        Pred::And(a, b) if and => {
            collect(a, and, attrs, column, out);
            collect(b, and, attrs, column, out);
        }
        Pred::Or(a, b) if !and => {
            collect(a, and, attrs, column, out);
            collect(b, and, attrs, column, out);
        }
        other => out.push(lower(other, attrs, column)),
    }
}

fn operand(o: &Operand, attrs: &Attrs, column: &dyn Fn(&PropertyName) -> String) -> Expr {
    match o {
        Operand::Column(c) => Expr::Column(column(c)),
        Operand::Lit(s) => Expr::Literal(literal(s)),
        // A missing attribute, or a list where a scalar is expected, is NULL (D-05).
        Operand::Attr(name) => Expr::Literal(match attrs.get(name) {
            AttrValue::Scalar(s) => literal(s),
            AttrValue::Null | AttrValue::List(_) => Literal::Null,
        }),
    }
}

fn in_list(
    o: &Operand,
    list: &ListSource,
    attrs: &Attrs,
    column: &dyn Fn(&PropertyName) -> String,
) -> Expr {
    let items = match list {
        ListSource::Literal(items) => Some(items.as_slice()),
        ListSource::Attr(name) => match attrs.get(name) {
            AttrValue::List(items) => Some(items.as_slice()),
            AttrValue::Null | AttrValue::Scalar(_) => None,
        },
    };
    match items {
        None => unknown(operand(o, attrs, column)),
        Some([]) => Expr::Literal(Literal::Boolean(false)),
        Some(items) => Expr::InList {
            expr: Box::new(operand(o, attrs, column)),
            list: items.iter().map(literal).collect(),
            negated: false,
        },
    }
}

/// An expression that is Unknown whatever `e` is: `e = NULL`.
fn unknown(e: Expr) -> Expr {
    Expr::Compare {
        left: Box::new(e),
        op: CompareOp::Eq,
        right: Box::new(Expr::Literal(Literal::Null)),
    }
}

fn literal(s: &Scalar) -> Literal {
    match s {
        Scalar::Null => Literal::Null,
        Scalar::Bool(b) => Literal::Boolean(*b),
        Scalar::Int(i) => Literal::Long(*i),
        Scalar::Double(d) => Literal::Double(*d),
        Scalar::String(s) => Literal::String(s.clone()),
    }
}

fn compare_op(op: CmpOp) -> CompareOp {
    match op {
        CmpOp::Eq => CompareOp::Eq,
        CmpOp::Ne => CompareOp::NotEq,
        CmpOp::Lt => CompareOp::Lt,
        CmpOp::Le => CompareOp::LtEq,
        CmpOp::Gt => CompareOp::Gt,
        CmpOp::Ge => CompareOp::GtEq,
    }
}
