//! The row predicate IR: what a `rowFilter` looks like after parsing (D-07).
//!
//! `Pred` is the only form a predicate takes after [`compile_row_filter`]
//! (`super::compile_row_filter`). The Rust evaluator and the SQL lowering (plan 03-11)
//! both start from it, never from the `cel` AST or the `cel` runtime.

use std::collections::BTreeSet;
use std::fmt;

use ontoforge_schema::ir::PropertyName;

use super::value::Scalar;

/// A row predicate.
#[derive(Debug, Clone, PartialEq)]
pub enum Pred {
    Const(bool),
    Cmp {
        left: Operand,
        op: CmpOp,
        right: Operand,
    },
    /// `operand == null` (`negated: false`) or `operand != null` (`negated: true`).
    /// Always two-valued.
    IsNull {
        operand: Operand,
        negated: bool,
    },
    InList {
        operand: Operand,
        list: ListSource,
    },
    StrMatch {
        column: PropertyName,
        needle: Operand,
        kind: MatchKind,
    },
    And(Box<Pred>, Box<Pred>),
    Or(Box<Pred>, Box<Pred>),
    Not(Box<Pred>),
}

/// One side of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// `object.<property>`.
    Column(PropertyName),
    Lit(Scalar),
    /// `principal.attrs.<name>`, resolved at evaluation (or SQL compilation) time.
    Attr(String),
}

/// Right-hand side of `in`.
#[derive(Debug, Clone, PartialEq)]
pub enum ListSource {
    /// A list literal. Never holds [`Scalar::Null`].
    Literal(Vec<Scalar>),
    /// A list-typed `principal.attrs.<name>`.
    Attr(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    fn symbol(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    StartsWith,
    EndsWith,
    Contains,
}

impl MatchKind {
    fn method(self) -> &'static str {
        match self {
            Self::StartsWith => "startsWith",
            Self::EndsWith => "endsWith",
            Self::Contains => "contains",
        }
    }
}

impl Pred {
    /// Every `object.<property>` the predicate reads.
    #[must_use]
    pub fn referenced_properties(&self) -> BTreeSet<PropertyName> {
        let mut out = BTreeSet::new();
        self.collect(
            &mut |c| {
                out.insert(c.clone());
            },
            &mut |_| {},
        );
        out
    }

    /// Every `principal.attrs.<name>` the predicate reads.
    #[must_use]
    pub fn referenced_attrs(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.collect(&mut |_| {}, &mut |a| {
            out.insert(a.to_owned());
        });
        out
    }

    fn collect(&self, column: &mut dyn FnMut(&PropertyName), attr: &mut dyn FnMut(&str)) {
        fn operand(o: &Operand, column: &mut dyn FnMut(&PropertyName), attr: &mut dyn FnMut(&str)) {
            match o {
                Operand::Column(c) => column(c),
                Operand::Attr(a) => attr(a),
                Operand::Lit(_) => {}
            }
        }
        match self {
            Self::Const(_) => {}
            Self::Cmp { left, right, .. } => {
                operand(left, column, attr);
                operand(right, column, attr);
            }
            Self::IsNull { operand: o, .. } => operand(o, column, attr),
            Self::InList { operand: o, list } => {
                operand(o, column, attr);
                if let ListSource::Attr(a) = list {
                    attr(a);
                }
            }
            Self::StrMatch {
                column: c, needle, ..
            } => {
                column(c);
                operand(needle, column, attr);
            }
            Self::And(a, b) | Self::Or(a, b) => {
                a.collect(column, attr);
                b.collect(column, attr);
            }
            Self::Not(p) => p.collect(column, attr),
        }
    }
}

/// Canonical CEL text of a scalar literal.
fn write_scalar(f: &mut fmt::Formatter<'_>, s: &Scalar) -> fmt::Result {
    match s {
        Scalar::Null => f.write_str("null"),
        Scalar::Bool(b) => write!(f, "{b}"),
        Scalar::Int(i) => write!(f, "{i}"),
        // `{:?}` always keeps a `.` or an exponent, so the text stays a double.
        Scalar::Double(d) => write!(f, "{d:?}"),
        Scalar::String(s) => {
            f.write_str("'")?;
            for c in s.chars() {
                match c {
                    '\\' => f.write_str("\\\\")?,
                    '\'' => f.write_str("\\'")?,
                    '\n' => f.write_str("\\n")?,
                    '\r' => f.write_str("\\r")?,
                    '\t' => f.write_str("\\t")?,
                    c if c.is_control() => write!(f, "\\u{:04x}", u32::from(c))?,
                    c => write!(f, "{c}")?,
                }
            }
            f.write_str("'")
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Column(c) => write!(f, "object.{c}"),
            Self::Attr(a) => write!(f, "principal.attrs.{a}"),
            Self::Lit(s) => write_scalar(f, s),
        }
    }
}

impl fmt::Display for ListSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attr(a) => write!(f, "principal.attrs.{a}"),
            Self::Literal(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write_scalar(f, item)?;
                }
                f.write_str("]")
            }
        }
    }
}

/// Canonical CEL text that [`compile_row_filter`](super::compile_row_filter) parses
/// back to an equal `Pred`: every binary form is parenthesized and `Not` keeps a
/// parenthesis between two `!` (the CEL grammar folds a bare run of them).
impl fmt::Display for Pred {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(b) => write!(f, "{b}"),
            Self::Cmp { left, op, right } => write!(f, "({left} {} {right})", op.symbol()),
            Self::IsNull { operand, negated } => {
                write!(f, "({operand} {} null)", if *negated { "!=" } else { "==" })
            }
            Self::InList { operand, list } => write!(f, "({operand} in {list})"),
            Self::StrMatch {
                column,
                needle,
                kind,
            } => write!(f, "object.{column}.{}({needle})", kind.method()),
            Self::And(a, b) => write!(f, "({a} && {b})"),
            Self::Or(a, b) => write!(f, "({a} || {b})"),
            Self::Not(p) => match **p {
                Self::Not(_) => write!(f, "!({p})"),
                _ => write!(f, "!{p}"),
            },
        }
    }
}
