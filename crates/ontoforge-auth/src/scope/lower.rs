//! `Pred` to printer AST.

use ontoforge_engine::printer::ast::{Expr, Literal};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_schema::ir::PropertyName;

/// Lowers `pred` to a printer expression.
pub fn lower(_pred: &Pred, _attrs: &Attrs, _column: &dyn Fn(&PropertyName) -> String) -> Expr {
    Expr::Literal(Literal::Null)
}
