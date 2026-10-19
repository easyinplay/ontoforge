//! CEL row predicates: parsed into an own `Pred` IR and evaluated with Kleene
//! three-valued logic. This module does not use the `cel` runtime to evaluate
//! (plan 03-05 adds parsing and evaluation; this plan only provides the runtime
//! value types shared with authentication).

pub mod value;

pub use value::{AttrValue, AttrValueError, Attrs, Scalar};
