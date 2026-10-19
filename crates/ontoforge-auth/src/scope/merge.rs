//! Merging the roles of one principal for one object type (ADR-0018, SCOPE-03).

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::printer::ast::Expr;
use ontoforge_expr::{Attrs, Pred, RowValues};
use ontoforge_schema::ir::PropertyName;

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

/// Merges the grants of every role of a principal on one object type.
#[must_use]
pub fn merge(_grants: &[RoleTypeGrant]) -> Option<TypeScope> {
    None
}

impl TypeScope {
    /// Evaluates the scope against one row.
    pub fn evaluate(&self, _row: &impl RowValues, _attrs: &Attrs) -> RowVisibility {
        RowVisibility {
            visible: false,
            columns: BTreeSet::new(),
        }
    }

    /// Lowers the row predicate and every column predicate.
    #[must_use]
    pub fn lower(&self, _attrs: &Attrs, _column: &dyn Fn(&PropertyName) -> String) -> LoweredScope {
        LoweredScope {
            row: Expr::Literal(ontoforge_engine::printer::ast::Literal::Null),
            columns: BTreeMap::new(),
        }
    }
}
