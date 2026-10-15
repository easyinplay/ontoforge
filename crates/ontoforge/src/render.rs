//! Diagnostic output: miette for people, a JSON array for tools.

use clap::ValueEnum;
use miette::{
    GraphicalReportHandler, GraphicalTheme, LabeledSpan, MietteDiagnostic, NamedSource, Report,
    Severity as MietteSeverity,
};
use ontoforge_schema::diag::{Diagnostic, Severity};
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

/// Render all diagnostics. `color` only matters for [`Format::Human`].
pub fn render(diags: &[Diagnostic], sources: &SourceMap, format: Format, color: bool) -> String {
    match format {
        Format::Json => {
            let mut s = serde_json::to_string_pretty(diags).unwrap_or_else(|_| "[]".to_owned());
            s.push('\n');
            s
        }
        Format::Human => render_human(diags, sources, color),
    }
}

fn render_human(diags: &[Diagnostic], sources: &SourceMap, color: bool) -> String {
    let theme = if color {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme).with_width(100);
    let mut out = String::new();
    for d in diags {
        let report = to_report(d, sources);
        // Writing into a String cannot fail.
        let _ = handler.render_report(&mut out, report.as_ref());
        out.push('\n');
    }
    out
}

fn to_report(d: &Diagnostic, sources: &SourceMap) -> Report {
    let severity = match d.severity {
        Severity::Error => MietteSeverity::Error,
        Severity::Warning => MietteSeverity::Warning,
        Severity::Info => MietteSeverity::Advice,
    };
    let mut diag = MietteDiagnostic::new(sanitize(&d.message))
        .with_code(sanitize(&d.code))
        .with_severity(severity);
    if let Some(help) = &d.help {
        diag = diag.with_help(sanitize(help));
    }
    let text = if d.line == 0 {
        None
    } else {
        sources.text(&d.file)
    };
    let Some(text) = text else {
        return Report::new(diag);
    };
    diag = diag.with_label(LabeledSpan::at(
        d.span.offset..d.span.offset + d.span.length,
        "here",
    ));
    // File names come from the (possibly untrusted) checkout and miette prints them
    // unescaped in the source header (CR-04).
    Report::new(diag).with_source_code(NamedSource::new(sanitize(&d.file), sanitize(text)))
}

/// Replace terminal control characters so a definition file cannot drive the
/// reader's terminal. Byte lengths are preserved so span offsets stay valid: C0
/// controls (except tab, newline, carriage return) and DEL become one `?`; C1
/// controls (two bytes in UTF-8) become two.
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\t' | '\n' | '\r' => out.push(c),
            '\u{0}'..='\u{1f}' | '\u{7f}' => out.push('?'),
            '\u{80}'..='\u{9f}' => out.push_str("??"),
            _ => out.push(c),
        }
    }
    out
}

/// 1 when there is an error, or a warning under `--deny-warnings`; otherwise 0.
pub fn exit_code(diags: &[Diagnostic], deny_warnings: bool) -> u8 {
    let failed = diags.iter().any(|d| match d.severity {
        Severity::Error => true,
        Severity::Warning => deny_warnings,
        Severity::Info => false,
    });
    u8::from(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_preserves_byte_length_and_strips_escapes() {
        let input = "a\u{1b}[31mb\u{9b}c\u{7f}\td\n";
        let out = sanitize(input);
        assert_eq!(out.len(), input.len());
        assert!(!out.contains('\u{1b}') && !out.contains('\u{9b}') && !out.contains('\u{7f}'));
        assert!(out.contains('\t') && out.ends_with('\n'));
    }

    /// CR-04: the file name reaches the source header, so it is sanitized too.
    #[test]
    fn file_name_cannot_drive_the_terminal() {
        let name = "\u{1b}]52;c;aGk=\u{7}.yaml";
        let mut sources = SourceMap::default();
        sources.add_file(name, "a: 1\n".to_owned());
        let at = ontoforge_schema::diag::SourceLocation {
            file: name.into(),
            line: 1,
            column: 1,
            offset: 0,
            length: 1,
        };
        let out = render(
            &[Diagnostic::error("ONT-V01", "x", &at)],
            &sources,
            Format::Human,
            false,
        );
        assert!(out.contains("?]52;c;aGk=?.yaml"), "{out}");
        assert!(!out.contains('\u{1b}') && !out.contains('\u{7}'), "{out:?}");
    }

    #[test]
    fn multibyte_text_is_untouched() {
        assert_eq!(sanitize("航班 Flight"), "航班 Flight");
    }

    #[test]
    fn exit_codes() {
        let at = ontoforge_schema::diag::SourceLocation {
            file: "a".into(),
            line: 1,
            column: 1,
            offset: 0,
            length: 1,
        };
        let err = Diagnostic::error("ONT-V01", "x", &at);
        let warn = Diagnostic::new("ONT-W01", Severity::Warning, "x", &at);
        assert_eq!(exit_code(&[], false), 0);
        assert_eq!(exit_code(std::slice::from_ref(&err), false), 1);
        assert_eq!(exit_code(std::slice::from_ref(&warn), false), 0);
        assert_eq!(exit_code(std::slice::from_ref(&warn), true), 1);
    }
}
