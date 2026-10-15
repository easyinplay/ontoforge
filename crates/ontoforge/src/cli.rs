//! Command line definition.

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};

use crate::render::{ColorChoice, Format};

#[derive(Debug, Parser)]
#[command(
    name = "ontoforge",
    version,
    about = "Validate and lint ontology-as-code"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check an ontology directory for structural errors.
    Validate(ValidateArgs),
    /// Validate, then run the lint rules.
    Lint(LintArgs),
}

#[derive(Debug, Args)]
pub struct LintArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    /// Lint configuration file; replaces `<DIR>/ontoforge.toml`.
    #[arg(long)]
    pub config: Option<Utf8PathBuf>,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    pub format: Format,
    /// When to use colors in human output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
    /// Exit with code 1 when warnings are present.
    #[arg(long)]
    pub deny_warnings: bool,
}
