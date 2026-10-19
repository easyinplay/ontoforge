//! CEL row predicates: parsed with `cel` into an own `Pred` IR and evaluated with
//! Kleene three-valued logic (D-06, ADR-0025). This module never uses the `cel`
//! runtime to evaluate: its `null == 'a'` is false and `null != 'a'` is true, the
//! opposite of D-06.

pub mod eval;
pub mod ir;
pub mod parse;
pub mod value;

pub use eval::{RowValues, Tri, eval};
pub use ir::{CmpOp, ListSource, MatchKind, Operand, Pred};
pub use parse::{PredContext, PredIssue, compile_row_filter};
pub use value::{AttrValue, AttrValueError, Attrs, Scalar};
