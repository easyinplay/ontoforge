//! Diagnostic model shared by the loader, validation, lint and the CLI (D-10).

use serde::Serialize;

/// Diagnostic severity. Serialized as `error` / `warning` / `info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Byte span inside the source text, consumed by miette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub offset: usize,
    pub length: usize,
}

/// Converts a 0-based UTF-8 byte offset into a 1-based `(line, column)` pair, with the
/// column counted in chars. An offset inside a multi-byte char is moved back to the
/// start of that char; one past the end is clamped to the end of `text`.
#[must_use]
pub fn offset_to_line_col(text: &str, byte_offset: usize) -> (u32, u32) {
    let start = text.floor_char_boundary(byte_offset);
    let before = &text[..start];
    let line = before.matches('\n').count() + 1;
    let column = before.chars().rev().take_while(|c| *c != '\n').count() + 1;
    (
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(column).unwrap_or(u32::MAX),
    )
}

/// A resolved position inside a source file.
///
/// `file` is relative to the ontology root and always uses `/` separators.
/// `line` and `column` are 1-based; `0` means "no position".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub offset: usize,
    pub length: usize,
}

/// One finding. Wire format is camelCase JSON (D-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub span: Span,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(
        code: &str,
        severity: Severity,
        message: impl Into<String>,
        at: &SourceLocation,
    ) -> Self {
        Self {
            code: code.to_owned(),
            severity,
            message: message.into(),
            file: at.file.clone(),
            line: at.line,
            column: at.column,
            span: Span {
                offset: at.offset,
                length: at.length,
            },
            help: None,
        }
    }

    pub fn error(code: &str, message: impl Into<String>, at: &SourceLocation) -> Self {
        Self::new(code, Severity::Error, message, at)
    }

    pub fn without_location(
        code: &str,
        severity: Severity,
        message: impl Into<String>,
        file: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_owned(),
            severity,
            message: message.into(),
            file: file.into(),
            line: 0,
            column: 0,
            span: Span {
                offset: 0,
                length: 0,
            },
            help: None,
        }
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Deterministic order: `(file, line, column, code, message)`.
pub fn sort_diagnostics(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.code, &a.message)
            .cmp(&(&b.file, b.line, b.column, &b.code, &b.message))
    });
}

pub fn has_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.severity == Severity::Error)
}

/// Separator between message and help inside a `de::Error::custom` string.
const HELP_MARK: char = '\u{1f}';

/// Encode a code, message and optional help into one string so that it survives
/// `serde::de::Error::custom`. [`parse_custom_de_error`] reverses it.
pub fn custom_de_error(code: &str, message: &str, help: Option<&str>) -> String {
    match help {
        Some(h) => format!("[{code}] {message}{HELP_MARK}{h}"),
        None => format!("[{code}] {message}"),
    }
}

/// A decoded [`custom_de_error`] string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomDeError {
    pub code: String,
    pub message: String,
    pub help: Option<String>,
}

/// Decode a string produced by [`custom_de_error`]; `None` for any other error text.
pub fn parse_custom_de_error(text: &str) -> Option<CustomDeError> {
    let rest = text.strip_prefix("[ONT-")?;
    let (code_tail, body) = rest.split_once("] ")?;
    let (message, help) = match body.split_once(HELP_MARK) {
        Some((m, h)) => (m, Some(h.to_owned())),
        None => (body, None),
    };
    Some(CustomDeError {
        code: format!("ONT-{code_tail}"),
        message: message.to_owned(),
        help,
    })
}

/// The closest candidate to `input` (case-insensitive Jaro-Winkler >= 0.8).
pub fn did_you_mean<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let needle = input.to_lowercase();
    candidates
        .into_iter()
        .map(|c| (strsim::jaro_winkler(&needle, &c.to_lowercase()), c))
        .filter(|(score, _)| *score >= 0.8)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
}

/// Validate diagnostic codes. Codes are never reused once published.
///
/// V21..V27 (Phase 3) are V-level, not lint E-level: an E rule can be turned off or
/// downgraded in `ontoforge.toml`, but scope, row-predicate and destructive-change
/// failures must never be silenced (SCOPE-06, D-07, STORE-02).
pub mod codes {
    /// Syntax error, duplicate key, budget exceeded, or non-UTF-8 input.
    pub const V01: &str = "ONT-V01";
    /// Document shape: not a mapping, missing `kind`, or `kind` is not a string.
    pub const V02: &str = "ONT-V02";
    /// Unknown or unsupported `kind` (including rule kinds).
    pub const V03: &str = "ONT-V03";
    /// Unknown field.
    pub const V04: &str = "ONT-V04";
    /// Missing required field.
    pub const V05: &str = "ONT-V05";
    /// Wrong type or value.
    pub const V06: &str = "ONT-V06";
    /// apiName format violation or reserved word.
    pub const V07: &str = "ONT-V07";
    /// Duplicate definition, property, parameter, or accessor.
    pub const V08: &str = "ONT-V08";
    /// Reference to something that does not exist.
    pub const V09: &str = "ONT-V09";
    /// Primary key or title key problem.
    pub const V10: &str = "ONT-V10";
    /// Link type consistency.
    pub const V11: &str = "ONT-V11";
    /// Action type structure.
    pub const V12: &str = "ONT-V12";
    /// CEL expression.
    pub const V13: &str = "ONT-V13";
    /// Manifest and source files.
    pub const V14: &str = "ONT-V14";
    /// Lint configuration.
    pub const V15: &str = "ONT-V15";
    /// Source binding definition, profile selection and fixture references.
    pub const V16: &str = "ONT-V16";
    /// Online binding check: the physical table does not match the binding.
    pub const V17: &str = "ONT-V17";
    /// Duplicate or NULL primary keys found by `--check-pk`.
    pub const V18: &str = "ONT-V18";
    /// The online check could not complete because of the engine (unavailable,
    /// authentication, timeout, protocol or query error).
    pub const V19: &str = "ONT-V19";
    /// Server configuration (`ontoforge-server.toml`).
    pub const V20: &str = "ONT-V20";
    /// Scope document structure and references: a `Role` / `RoleBinding` refers to an
    /// unknown object type, property, action or role; a property allow-list omits
    /// the primary key; empty or duplicate subjects.
    pub const V21: &str = "ONT-V21";
    /// Row predicate (`rowFilter`): outside the allowed CEL subset, type error,
    /// reference other than same-row properties and declared `principal.attrs`, or
    /// a case-insensitive collation column (D-07, D-08).
    pub const V22: &str = "ONT-V22";
    /// Criteria column coverage: a role granted an action cannot see a property
    /// its submission criteria read (SCOPE-06).
    pub const V23: &str = "ONT-V23";
    /// Edit store physical name: two definitions map to the same physical table,
    /// column or index name.
    pub const V24: &str = "ONT-V24";
    /// Destructive definition change found by reload; apply it with
    /// `ontoforge migrate`.
    pub const V25: &str = "ONT-V25";
    /// The edit store or the Trino visibility probe could not complete:
    /// connection, lock timeout, probe timeout, stale definition.
    pub const V26: &str = "ONT-V26";
    /// Import data: unreadable file, unknown column, value that does not convert
    /// to the property type, or an existing object.
    pub const V27: &str = "ONT-V27";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase3_codes() {
        let all = [
            codes::V21,
            codes::V22,
            codes::V23,
            codes::V24,
            codes::V25,
            codes::V26,
            codes::V27,
        ];
        for (offset, code) in all.iter().enumerate() {
            assert_eq!(*code, format!("ONT-V{}", 21 + offset));
        }
    }

    #[test]
    fn custom_error_round_trips() {
        let s = custom_de_error(codes::V07, "bad name", Some("use Flight"));
        let parsed = parse_custom_de_error(&s).unwrap();
        assert_eq!(parsed.code, "ONT-V07");
        assert_eq!(parsed.message, "bad name");
        assert_eq!(parsed.help.as_deref(), Some("use Flight"));
        assert!(parse_custom_de_error("unknown field `x`").is_none());
    }

    #[test]
    fn did_you_mean_picks_closest_and_ignores_far_names() {
        let names = ["apiName", "primaryKey", "properties"];
        assert_eq!(
            did_you_mean("primaryKy", names.iter().copied()),
            Some("primaryKey")
        );
        assert_eq!(did_you_mean("zzzz", names.iter().copied()), None);
    }

    #[test]
    fn offset_to_line_col_takes_byte_offsets_and_counts_char_columns() {
        assert_eq!(offset_to_line_col("ab\ncd", 0), (1, 1));
        assert_eq!(offset_to_line_col("ab\ncd", 4), (2, 2));
        // `é` is two bytes: byte offset 2 is the char after it, in column 2.
        assert_eq!(offset_to_line_col("éa", 2), (1, 2));
        // Mid-char and past-the-end offsets are clamped instead of panicking.
        assert_eq!(offset_to_line_col("éa", 1), (1, 1));
        assert_eq!(offset_to_line_col("ab", 99), (1, 3));
    }

    #[test]
    fn sort_is_by_file_line_column_code() {
        let at = |file: &str, line| SourceLocation {
            file: file.into(),
            line,
            column: 1,
            offset: 0,
            length: 1,
        };
        let mut d = vec![
            Diagnostic::error(codes::V05, "b", &at("b.yaml", 1)),
            Diagnostic::error(codes::V04, "a", &at("a.yaml", 9)),
            Diagnostic::error(codes::V04, "a", &at("a.yaml", 2)),
        ];
        sort_diagnostics(&mut d);
        let order: Vec<_> = d.iter().map(|x| (x.file.as_str(), x.line)).collect();
        assert_eq!(order, [("a.yaml", 2), ("a.yaml", 9), ("b.yaml", 1)]);
    }
}
