//! The only exits for values and identifiers into SQL text.
//!
//! Every string literal and every identifier in generated SQL goes through [`quote_string`]
//! or [`quote_ident`]. Neither Trino nor SQLite treats a backslash specially inside a
//! string literal, so doubling the quote character is the complete escaping rule.

use super::ast::MatchKind;
use crate::error::PrintError;

/// Quotes `s` as a SQL string literal. Rejects NUL: SQLite's C API truncates SQL text there.
pub(super) fn quote_string(s: &str) -> Result<String, PrintError> {
    if s.contains('\0') {
        return Err(PrintError::NulInString);
    }
    Ok(format!("'{}'", s.replace('\'', "''")))
}

/// Quotes `s` as a delimited identifier. Rejects the empty string and NUL.
pub(super) fn quote_ident(s: &str) -> Result<String, PrintError> {
    if s.is_empty() {
        return Err(PrintError::EmptyIdentifier);
    }
    if s.contains('\0') {
        return Err(PrintError::NulInIdentifier);
    }
    Ok(format!("\"{}\"", s.replace('"', "\"\"")))
}

/// The `LIKE` pattern (escape character `\`) matching strings that start with, end with or
/// contain `needle`, with `\`, `%` and `_` in `needle` taken literally.
pub(super) fn like_pattern(needle: &str, kind: MatchKind) -> Result<String, PrintError> {
    let body = escape_chars(needle, &['\\', '%', '_'], |c, out| {
        out.push('\\');
        out.push(c);
    })?;
    Ok(wrap(&body, kind, '%'))
}

/// The SQLite `GLOB` pattern matching strings that start with, end with or contain `needle`,
/// with `[`, `*` and `?` in `needle` taken literally. GLOB has no escape character, so each
/// of them becomes a one-character class.
pub(super) fn glob_pattern(needle: &str, kind: MatchKind) -> Result<String, PrintError> {
    let body = escape_chars(needle, &['[', '*', '?'], |c, out| {
        out.push('[');
        out.push(c);
        out.push(']');
    })?;
    Ok(wrap(&body, kind, '*'))
}

/// Copies `needle`, passing every character of `special` to `emit`. Rejects NUL like
/// [`quote_string`], which the pattern goes through next.
fn escape_chars(
    needle: &str,
    special: &[char],
    emit: impl Fn(char, &mut String),
) -> Result<String, PrintError> {
    if needle.contains('\0') {
        return Err(PrintError::NulInString);
    }
    let mut out = String::with_capacity(needle.len() + 2);
    for c in needle.chars() {
        if special.contains(&c) {
            emit(c, &mut out);
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Adds the pattern's "anything" character where `kind` leaves the text open.
fn wrap(body: &str, kind: MatchKind, any: char) -> String {
    match kind {
        MatchKind::StartsWith => format!("{body}{any}"),
        MatchKind::EndsWith => format!("{any}{body}"),
        MatchKind::Contains => format!("{any}{body}{any}"),
    }
}
