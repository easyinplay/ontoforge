//! Compile-only analysis of CEL expressions on a dedicated large-stack thread.

use std::fmt;

use crate::limits::{MAX_EXPR_CHARS, MAX_EXPR_NESTING, precheck};

/// Stack size of the analysis thread. The ANTLR-generated CEL parser recurses per
/// nesting level and overflows a 1 MiB Windows stack at around ten levels in debug
/// builds; the thread, not the caller, therefore owns every recursion.
const ANALYZE_STACK_BYTES: usize = 32 << 20;

/// Why an expression could not be analyzed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprError {
    TooLong {
        chars: usize,
    },
    TooDeep {
        depth: usize,
    },
    /// The CEL parser's own recursion limit was reached (long unparenthesized
    /// chains such as nested `? :`), which the nesting precheck does not count.
    ParserLimit,
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
            Self::TooLong { chars } => write!(
                f,
                "expression has {chars} characters, the limit is {MAX_EXPR_CHARS}"
            ),
            Self::TooDeep { depth } => write!(
                f,
                "expression nesting reaches {depth} levels, the limit is {MAX_EXPR_NESTING}"
            ),
            Self::Syntax {
                line,
                column,
                message,
            } => write!(f, "syntax error at {line}:{column}: {message}"),
            Self::ParserLimit => f.write_str(
                "expression nests too deeply for the CEL parser; split it into several conditions",
            ),
            Self::Internal => f.write_str("internal error while analyzing expression"),
        }
    }
}

impl std::error::Error for ExprError {}

/// Compiles `src` (never evaluates it) and runs `f` on the compiled program.
///
/// Length and nesting are prechecked first. Compilation and `f` both run on one
/// scoped 32 MiB thread, so the program never leaves it and `f` may recurse over the
/// AST safely. A panic in either becomes [`ExprError::Internal`].
pub fn analyze<R, F>(src: &str, f: F) -> Result<R, ExprError>
where
    F: FnOnce(&cel::Program) -> R + Send,
    R: Send,
{
    precheck(src)?;
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(ANALYZE_STACK_BYTES)
            .spawn_scoped(scope, || compile_and_run(src, f))
            .map_err(|_| ExprError::Internal)?
            .join()
            .map_err(|_| ExprError::Internal)?
    })
}

fn compile_and_run<R>(src: &str, f: impl FnOnce(&cel::Program) -> R) -> Result<R, ExprError> {
    match cel::Program::compile(src) {
        Ok(program) => Ok(f(&program)),
        // `cel` 0.15 reports its recursion limit as a debug-formatted
        // `OtherError(ParseError { .. })`, sometimes after follow-on errors such as
        // "Incomplete `ExprContext`" (WR-08).
        Err(errors)
            if errors
                .errors
                .iter()
                .any(|e| e.msg.contains("Recursion limit of")) =>
        {
            Err(ExprError::ParserLimit)
        }
        Err(errors) => Err(errors.errors.first().map_or_else(
            || ExprError::Syntax {
                line: 1,
                column: 1,
                message: "syntax error".to_owned(),
            },
            |e| ExprError::Syntax {
                line: usize::try_from(e.pos.0).unwrap_or(1),
                column: usize::try_from(e.pos.1).unwrap_or(1),
                message: e.msg.clone(),
            },
        )),
    }
}
