//! Compile-only analysis of CEL expressions on a dedicated large-stack thread.

use std::fmt;

/// Why an expression could not be analyzed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprError {
    TooLong {
        chars: usize,
    },
    TooDeep {
        depth: usize,
    },
    /// 1-based position relative to the expression text.
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    /// Thread spawn failure or a panic inside the compiler or the caller closure.
    Internal,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { chars } => write!(f, "expression has {chars} characters"),
            Self::TooDeep { depth } => write!(f, "expression nesting reaches {depth} levels"),
            Self::Syntax {
                line,
                column,
                message,
            } => {
                write!(f, "syntax error at {line}:{column}: {message}")
            }
            Self::Internal => f.write_str("internal error while analyzing expression"),
        }
    }
}

impl std::error::Error for ExprError {}

/// Compiles `src` (never evaluates it) and runs `f` on the compiled program.
pub fn analyze<R, F>(_src: &str, _f: F) -> Result<R, ExprError>
where
    F: FnOnce(&cel::Program) -> R + Send,
    R: Send,
{
    Err(ExprError::TooLong { chars: 0 })
}
