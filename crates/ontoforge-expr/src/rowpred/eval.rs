//! Single-row evaluation of a [`Pred`] with SQL three-valued (Kleene) logic (D-06).

use std::collections::BTreeMap;

use ontoforge_schema::ir::PropertyName;

use super::ir::Pred;
use super::value::{Attrs, Scalar};

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

#[allow(clippy::only_used_in_recursion)] // RED stub; leaves with the Cmp implementation
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
        // Filled in by the full subset (plan 03-05 Task 3).
        _ => Tri::Unknown,
    }
}
