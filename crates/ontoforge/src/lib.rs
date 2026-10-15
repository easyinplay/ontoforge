//! `ontoforge` command line library: argument model and orchestration.

use std::io::{IsTerminal, Write};

pub mod cli;
pub mod pipeline;
pub mod render;

use cli::{Cli, Command, ValidateArgs};
use render::{ColorChoice, Format};

/// Run the CLI and return the process exit code.
pub fn run(cli: Cli) -> u8 {
    match cli.command {
        Command::Validate(args) => validate(&args),
    }
}

fn validate(args: &ValidateArgs) -> u8 {
    let out = pipeline::check(&args.dir);
    let out_args = &args.output;
    let color = match out_args.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
        }
    };
    let text = render::render(&out.diagnostics, &out.sources, out_args.format, color);
    let _ = std::io::stdout().write_all(text.as_bytes());
    let code = render::exit_code(&out.diagnostics, out_args.deny_warnings);
    if out_args.format == Format::Human {
        let summary = match &out.ontology {
            Some(_) => "ok".to_owned(),
            None => format!("{} problem(s) found", out.diagnostics.len()),
        };
        eprintln!("{summary}");
    }
    code
}
