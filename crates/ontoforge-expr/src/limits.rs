//! Lexical precheck run before any expression reaches the CEL compiler.
//!
//! The scan mirrors the lexer rules of `cel` 0.15 (`src/parser/gen/CEL.g4`) closely
//! enough that bracket nesting is never undercounted: string literals (single, double
//! and triple quoted, with `r` / `b` / `br` prefixes), `//` comments and numbers are
//! skipped the way the real lexer skips them. Anything the real lexer would reject
//! (unterminated string, invalid escape) is rejected here too, so no input whose
//! tokenization we could get wrong ever reaches the compiler.

use crate::analyze::ExprError;

/// Maximum expression length, counted in `char`s.
pub const MAX_EXPR_CHARS: usize = 4096;
/// Maximum bracket nesting depth (`(`, `[`, `{`).
pub const MAX_EXPR_NESTING: usize = 32;

/// Checks length, bracket nesting and string literal termination.
pub fn precheck(src: &str) -> Result<(), ExprError> {
    let chars = src.chars().count();
    if chars > MAX_EXPR_CHARS {
        return Err(ExprError::TooLong { chars });
    }
    let mut scan = Scanner::new(src);
    let mut depth = 0usize;
    while let Some(c) = scan.peek(0) {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if depth > MAX_EXPR_NESTING {
                    return Err(ExprError::TooDeep { depth });
                }
                scan.bump();
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                scan.bump();
            }
            '/' if scan.peek(1) == Some('/') => {
                while scan.peek(0).is_some_and(|c| c != '\n') {
                    scan.bump();
                }
            }
            '\'' | '"' => scan.string(false)?,
            c if c.is_ascii_alphabetic() || c == '_' => scan.identifier_or_prefixed_string()?,
            c if c.is_ascii_digit() => scan.number(),
            _ => scan.bump(),
        }
    }
    Ok(())
}

struct Scanner {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    fn new(src: &str) -> Self {
        Self {
            chars: src.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.index + ahead).copied()
    }

    fn bump(&mut self) {
        if let Some(c) = self.peek(0) {
            self.index += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn bump_n(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
        }
    }

    fn syntax(line: usize, column: usize, message: &str) -> ExprError {
        ExprError::Syntax {
            line,
            column,
            message: message.to_owned(),
        }
    }

    /// Consumes a whole identifier; when a quote follows directly, consumes the string
    /// literal as well (raw when the identifier is `r` or `br`, any case).
    fn identifier_or_prefixed_string(&mut self) -> Result<(), ExprError> {
        let start = self.index;
        while self
            .peek(0)
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.bump();
        }
        if !matches!(self.peek(0), Some('\'' | '"')) {
            return Ok(());
        }
        let ident: String = self.chars[start..self.index].iter().collect();
        // `b'..'` is an ordinary bytes literal and any other identifier is followed by
        // an ordinary string, so only `r` and `br` switch escapes off.
        self.string(ident.eq_ignore_ascii_case("r") || ident.eq_ignore_ascii_case("br"))
    }

    /// Skips a numeric literal so a following `r'...'` is seen at a token boundary.
    fn number(&mut self) {
        let is_hex = |c: Option<char>| c.is_some_and(|c| c.is_ascii_hexdigit());
        let is_digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
        if self.peek(0) == Some('0') && self.peek(1) == Some('x') && is_hex(self.peek(2)) {
            self.bump_n(2);
            while is_hex(self.peek(0)) {
                self.bump();
            }
            return;
        }
        while is_digit(self.peek(0)) {
            self.bump();
        }
        if self.peek(0) == Some('.') && is_digit(self.peek(1)) {
            self.bump();
            while is_digit(self.peek(0)) {
                self.bump();
            }
        }
        if matches!(self.peek(0), Some('e' | 'E')) {
            let sign = usize::from(matches!(self.peek(1), Some('+' | '-')));
            if is_digit(self.peek(1 + sign)) {
                self.bump_n(1 + sign);
                while is_digit(self.peek(0)) {
                    self.bump();
                }
            }
        }
    }

    /// Consumes one string literal starting at the opening quote.
    fn string(&mut self, raw: bool) -> Result<(), ExprError> {
        let (line, column) = (self.line, self.column);
        let unterminated = || Self::syntax(line, column, "unterminated string literal");
        let Some(quote) = self.peek(0) else {
            return Ok(());
        };
        let triple = self.peek(1) == Some(quote) && self.peek(2) == Some(quote);
        self.bump_n(if triple { 3 } else { 1 });
        loop {
            let Some(c) = self.peek(0) else {
                return Err(unterminated());
            };
            if triple {
                if c == quote && self.peek(1) == Some(quote) && self.peek(2) == Some(quote) {
                    self.bump_n(3);
                    return Ok(());
                }
            } else if c == '\n' || c == '\r' {
                return Err(unterminated());
            } else if c == quote {
                self.bump();
                return Ok(());
            }
            if c == '\\' && !raw {
                self.escape()?;
            } else {
                self.bump();
            }
        }
    }

    /// Consumes one escape sequence (cursor on the backslash); same set as `ESC_SEQ`.
    fn escape(&mut self) -> Result<(), ExprError> {
        let (line, column) = (self.line, self.column);
        let hex = |scan: &Self, from: usize, n: usize| {
            (from..from + n).all(|i| scan.peek(i).is_some_and(|c| c.is_ascii_hexdigit()))
        };
        let octal = |scan: &Self, i: usize| scan.peek(i).is_some_and(|c| ('0'..='7').contains(&c));
        let len = match self.peek(1) {
            None => {
                return Err(Self::syntax(line, column, "unterminated string literal"));
            }
            Some('a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '"' | '\'' | '\\' | '?' | '`') => 2,
            Some('0'..='3') if octal(self, 2) && octal(self, 3) => 4,
            Some('x' | 'X') if hex(self, 2, 2) => 4,
            Some('u') if hex(self, 2, 4) => 6,
            Some('U') if hex(self, 2, 8) => 10,
            Some(_) => return Err(Self::syntax(line, column, "invalid escape sequence")),
        };
        self.bump_n(len);
        Ok(())
    }
}
