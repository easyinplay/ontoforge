//! RED stub: replaced by the real renderer in the GREEN commit.

use clap::ValueEnum;
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::loader::SourceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub fn render(
    _diags: &[Diagnostic],
    _sources: &SourceMap,
    _format: Format,
    _color: bool,
) -> String {
    String::new()
}

pub fn exit_code(_diags: &[Diagnostic], _deny_warnings: bool) -> u8 {
    0
}
