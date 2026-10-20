//! Merging the roles of one principal for one object type (ADR-0018, SCOPE-03).
//!
//! For the roles `R_i` (row predicate) with column sets `C_i` that a principal holds on one
//! object type:
//!
//! - `row = OR(R_i)`, and a role without a row predicate contributes `true`;
//! - `columns[x] = OR(R_i where x in C_i)`;
//! - no role grants the type: no scope is built, and the type is invisible (fail-closed,
//!   SCOPE-05).
//!
//! A row is visible when `row` is True; column `x` of a visible row is visible when
//! `columns[x]` is True. A column therefore never shows through another role's rows: it needs
//! a role that both grants the column and holds for the row.
//!
//! Use by Phase 4: the row predicate joins the user's filter with AND, and the column
//! predicates are used only for masking in the outermost projection, never inside filters or
//! aggregates (spike 3b, rule 1). This module does not rewrite user filters.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::printer::ast::Expr;
use ontoforge_expr::{Attrs, Pred, RowValues, eval};
use ontoforge_schema::ir::PropertyName;

use super::lower::lower;

/// What one role grants on one object type.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleTypeGrant {
    /// The properties the role may read.
    pub properties: BTreeSet<PropertyName>,
    /// The role's row predicate; `None` grants every row.
    pub row: Option<Pred>,
}

/// The merged scope of a principal on one object type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScope {
    pub row: Pred,
    pub columns: BTreeMap<PropertyName, Pred>,
}

/// What a [`TypeScope`] lets a principal see of one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowVisibility {
    pub visible: bool,
    pub columns: BTreeSet<PropertyName>,
}

/// A [`TypeScope`] lowered to the printer AST.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredScope {
    pub row: Expr,
    pub columns: BTreeMap<PropertyName, Expr>,
}

/// Merges the grants of every role of a principal on one object type. An empty slice gives
/// `None`: nothing grants the type, so it is invisible.
#[must_use]
pub fn merge(grants: &[RoleTypeGrant]) -> Option<TypeScope> {
    if grants.is_empty() {
        return None;
    }
    let row_of = |g: &RoleTypeGrant| g.row.clone().unwrap_or(Pred::Const(true));
    let row = any_of(grants.iter().map(row_of));
    let mut by_column: BTreeMap<PropertyName, Vec<Pred>> = BTreeMap::new();
    for grant in grants {
        for property in &grant.properties {
            by_column
                .entry(property.clone())
                .or_default()
                .push(row_of(grant));
        }
    }
    let columns = by_column
        .into_iter()
        .map(|(property, preds)| (property, any_of(preds)))
        .collect();
    Some(TypeScope { row, columns })
}

/// `OR` of `preds`, folded to the left. A `true` absorbs everything and a predicate that
/// is structurally equal to an earlier one is dropped. Not called with an empty iterator;
/// that would be `false`.
fn any_of(preds: impl IntoIterator<Item = Pred>) -> Pred {
    let mut distinct: Vec<Pred> = Vec::new();
    for p in preds {
        if p == Pred::Const(true) {
            return p;
        }
        if !distinct.contains(&p) {
            distinct.push(p);
        }
    }
    distinct
        .into_iter()
        .reduce(|left, right| Pred::Or(Box::new(left), Box::new(right)))
        .unwrap_or(Pred::Const(false))
}

impl TypeScope {
    /// Evaluates the scope against one row (Unknown is invisible).
    pub fn evaluate(&self, row: &impl RowValues, attrs: &Attrs) -> RowVisibility {
        let visible = eval(&self.row, row, attrs).is_true();
        let columns = if visible {
            self.columns
                .iter()
                .filter(|(_, p)| eval(p, row, attrs).is_true())
                .map(|(c, _)| c.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        RowVisibility { visible, columns }
    }

    /// Lowers the row predicate and every column predicate to printer expressions.
    #[must_use]
    pub fn lower(&self, attrs: &Attrs, column: &dyn Fn(&PropertyName) -> String) -> LoweredScope {
        LoweredScope {
            row: lower(&self.row, attrs, column),
            columns: self
                .columns
                .iter()
                .map(|(c, p)| (c.clone(), lower(p, attrs, column)))
                .collect(),
        }
    }
}
