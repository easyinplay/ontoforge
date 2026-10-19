//! Single-row evaluation of a [`Pred`] with SQL three-valued (Kleene) logic (D-06).

use std::collections::BTreeMap;

use ontoforge_schema::ir::PropertyName;

use std::cmp::Ordering;

use super::ir::{CmpOp, Operand, Pred};
use super::value::{AttrValue, Attrs, Scalar};

/// A truth value that can be unknown. Only [`Tri::True`] makes a row visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tri {
    True,
    False,
    Unknown,
}

impl From<bool> for Tri {
    fn from(b: bool) -> Self {
        if b { Self::True } else { Self::False }
    }
}

impl Tri {
    /// Kleene conjunction: false wins over unknown.
    #[must_use]
    pub fn and(self, other: Tri) -> Tri {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    /// Kleene disjunction: true wins over unknown.
    #[must_use]
    pub fn or(self, other: Tri) -> Tri {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    #[must_use]
    #[allow(clippy::should_implement_trait)] // the plan 03-05 interface names it `not`
    pub fn not(self) -> Tri {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Unknown is invisible (D-06).
    #[must_use]
    pub fn is_true(self) -> bool {
        self == Self::True
    }
}

/// The property values of one row. A property that is absent is [`Scalar::Null`].
pub trait RowValues {
    fn value(&self, property: &PropertyName) -> Scalar;
}

impl RowValues for BTreeMap<PropertyName, Scalar> {
    fn value(&self, property: &PropertyName) -> Scalar {
        self.get(property).cloned().unwrap_or(Scalar::Null)
    }
}

/// Evaluates `pred` against one row and the caller's principal attributes.
pub fn eval(pred: &Pred, row: &impl RowValues, attrs: &Attrs) -> Tri {
    match pred {
        Pred::Const(b) => Tri::from(*b),
        Pred::And(a, b) => match eval(a, row, attrs) {
            Tri::False => Tri::False,
            left => left.and(eval(b, row, attrs)),
        },
        Pred::Or(a, b) => match eval(a, row, attrs) {
            Tri::True => Tri::True,
            left => left.or(eval(b, row, attrs)),
        },
        Pred::Not(p) => eval(p, row, attrs).not(),
        Pred::Cmp { left, op, right } => {
            compare(&scalar(left, row, attrs), *op, &scalar(right, row, attrs))
        }
        Pred::IsNull { operand, negated } => {
            let is_null = scalar(operand, row, attrs) == Scalar::Null;
            Tri::from(is_null != *negated)
        }
        // Filled in by the full subset (plan 03-05 Task 3).
        Pred::InList { .. } | Pred::StrMatch { .. } => Tri::Unknown,
    }
}

/// The value of an operand. A principal attribute that is missing, or a list where a
/// scalar is expected, is Null (D-05).
fn scalar(operand: &Operand, row: &impl RowValues, attrs: &Attrs) -> Scalar {
    match operand {
        Operand::Column(c) => row.value(c),
        Operand::Lit(s) => s.clone(),
        Operand::Attr(name) => match attrs.get(name) {
            AttrValue::Scalar(s) => s.clone(),
            AttrValue::Null | AttrValue::List(_) => Scalar::Null,
        },
    }
}

/// Order of two non-null values of the same family; `None` when either side is Null,
/// the families differ, or a double is NaN.
fn ordering(a: &Scalar, b: &Scalar) -> Option<Ordering> {
    match (a, b) {
        (Scalar::Bool(a), Scalar::Bool(b)) => Some(a.cmp(b)),
        (Scalar::Int(a), Scalar::Int(b)) => Some(a.cmp(b)),
        (Scalar::Double(a), Scalar::Double(b)) => a.partial_cmp(b),
        // Byte order of UTF-8 equals code point order: SQLite BINARY, PG `COLLATE "C"`.
        (Scalar::String(a), Scalar::String(b)) => Some(a.as_bytes().cmp(b.as_bytes())),
        _ => None,
    }
}

fn compare(a: &Scalar, op: CmpOp, b: &Scalar) -> Tri {
    let Some(ord) = ordering(a, b) else {
        return Tri::Unknown;
    };
    Tri::from(match op {
        CmpOp::Eq => ord == Ordering::Equal,
        CmpOp::Ne => ord != Ordering::Equal,
        CmpOp::Lt => ord == Ordering::Less,
        CmpOp::Le => ord != Ordering::Greater,
        CmpOp::Gt => ord == Ordering::Greater,
        CmpOp::Ge => ord != Ordering::Less,
    })
}
