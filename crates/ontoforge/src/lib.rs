//! `ontoforge` command line library: argument model and orchestration.

use std::io::Write;

pub mod cli;
pub mod lint_cmd;
pub mod pipeline;
pub mod render;

use camino::Utf8Path;
use cli::{Cli, Command, LintArgs, OutputArgs, SchemaCommand, ValidateArgs};
use ontoforge_schema::diag::{Severity, codes};
use ontoforge_schema::{LoadOutput, schema_export};
use render::{ColorChoice, Format};

/// Run the CLI and return the process exit code.
pub fn run(cli: Cli) -> u8 {
    match cli.command {
        Command::Validate(args) => validate(&args),
        Command::Lint(args) => lint(&args),
        Command::Schema(args) => match args.command {
            SchemaCommand::Export { out_dir } => export_schemas(&out_dir),
        },
    }
}

/// Write each exported schema into `out_dir`, naming every written file on stderr.
fn export_schemas(out_dir: &Utf8Path) -> u8 {
    let result = std::fs::create_dir_all(out_dir).and_then(|()| {
        for (name, text) in schema_export::rendered_schemas() {
            std::fs::write(out_dir.join(name), text)?;
            eprintln!("{}/{name}", out_dir.as_str().trim_end_matches(['/', '\\']));
        }
        Ok(())
    });
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: cannot write schemas to {out_dir}: {e}");
            1
        }
    }
}

/// Print the diagnostics to stdout and return the exit code.
fn report(out: &LoadOutput, args: &OutputArgs) -> u8 {
    let color = match args.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        // NO_COLOR, CLICOLOR, CLICOLOR_FORCE, TERM and the terminal check, per anstream.
        ColorChoice::Auto => {
            anstream::AutoStream::choice(&std::io::stdout()) != anstream::ColorChoice::Never
        }
    };
    let text = render::render(&out.diagnostics, &out.sources, args.format, color);
    let _ = std::io::stdout().write_all(text.as_bytes());
    render::exit_code(&out.diagnostics, args.deny_warnings)
}

fn validate(args: &ValidateArgs) -> u8 {
    let out = pipeline::check(&args.dir);
    let code = report(&out, &args.output);
    if args.output.format == Format::Human {
        let summary = match &out.ontology {
            Some(ontology) => format!(
                "ok: ontology `{}` with {} object type(s)",
                ontology.api_name,
                ontology.object_types.len()
            ),
            None => format!("{} problem(s) found", out.diagnostics.len()),
        };
        eprintln!("{summary}");
    }
    code
}

fn lint(args: &LintArgs) -> u8 {
    let out = lint_cmd::lint(&args.dir, args.config.as_deref(), None);
    let code = report(&out, &args.output);
    if args.output.format == Format::Human {
        eprintln!("{}", lint_summary(&out));
    }
    code
}

/// The stderr line of a human-format lint run.
fn lint_summary(out: &LoadOutput) -> String {
    let count = |severity| {
        out.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    };
    let is_error = |code: &str| {
        out.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error && d.code == code)
    };
    if out.diagnostics.iter().any(|d| {
        d.severity == Severity::Error && d.code.starts_with("ONT-V") && d.code != codes::V15
    }) {
        return format!(
            "validation failed, lint skipped: {}",
            plural(count(Severity::Error), "error", "errors")
        );
    }
    if is_error(codes::V15) {
        return "lint configuration is invalid, lint skipped".to_owned();
    }
    if out.diagnostics.is_empty() {
        let name = out.ontology.as_ref().map_or("", |o| o.api_name.as_str());
        return format!("ok: ontology `{name}` has no lint findings");
    }
    format!(
        "{}, {}, {}",
        plural(count(Severity::Error), "error", "errors"),
        plural(count(Severity::Warning), "warning", "warnings"),
        plural(count(Severity::Info), "info", "infos"),
    )
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
