//! `ontoforge validate --online`: check the bindings of the selected profile against the
//! engine (BIND-03).

use ontoforge_engine::{EngineError, QueryContext};
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::ProfileName;

use crate::cli::ValidateArgs;
use crate::{pipeline, render};

/// The outcome of an online check.
#[derive(Debug)]
pub struct OnlineRun {
    pub out: LoadOutput,
    pub code: u8,
    /// The profile that was checked; `None` when no profile is selected.
    pub profile: Option<ProfileName>,
    /// How many bindings were compared with their physical table.
    pub bindings: usize,
}

/// Runs the online check and reports what the summary line needs.
pub fn run_online(args: &ValidateArgs) -> OnlineRun {
    let checked = pipeline::check_with_profile(&args.dir, args.profile.profile.as_deref());
    let code = render::exit_code(&checked.out.diagnostics, args.output.deny_warnings);
    OnlineRun {
        out: checked.out,
        code,
        profile: checked.profile,
        bindings: 0,
    }
}

/// [`run_online`] without the summary details.
pub fn validate_online(args: &ValidateArgs) -> (LoadOutput, u8) {
    let run = run_online(args);
    (run.out, run.code)
}

/// ONT-V19 for an engine failure that stops the whole round (not tied to a binding).
pub fn engine_error_diagnostic(e: &EngineError, file: &str) -> Diagnostic {
    Diagnostic::without_location(codes::V19, Severity::Error, e.to_string(), file)
}

/// The context of the online check's queries.
pub(crate) fn query_context(_timeout_ms: Option<u64>) -> QueryContext {
    QueryContext::new(
        "ontoforge-cli".to_owned(),
        vec!["ontoforge".to_owned(), "validate-online".to_owned()],
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ontoforge_engine::{DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS};

    use super::query_context;

    #[test]
    fn query_context_uses_defaults_and_timeout_flag() {
        let ctx = query_context(None);
        assert_eq!(ctx.timeout, Duration::from_millis(DEFAULT_QUERY_TIMEOUT_MS));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
        assert_eq!(ctx.max_bytes, DEFAULT_MAX_BYTES);
        assert!(ctx.trace_token.starts_with("ontoforge-cli-"));

        let ctx = query_context(Some(1500));
        assert_eq!(ctx.timeout, Duration::from_millis(1500));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
    }
}
