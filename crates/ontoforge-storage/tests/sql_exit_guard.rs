//! Source level guard for T-03-07: SQL text reaches the database only through `src/sql_exit.rs`.
//!
//! `sqlx` 0.9 only accepts dynamic SQL wrapped in `AssertSqlSafe`. Keeping that wrapper, and every
//! hand written statement, in one file keeps the injection surface reviewable: everything else
//! builds DDL and DML with `sea-query`. The scan also bans SQLite's replacing conflict clause
//! everywhere: it deletes the old row as a side effect, which does not fit an append only edit
//! log (ADR-0016).

use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

const KEYWORDS: &str =
    "SELECT|WITH|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER|ATTACH|PRAGMA|SET|BEGIN|CALL|REPLACE";

struct Rules {
    /// A string literal that starts with an upper case SQL keyword.
    sql_literal: Regex,
    /// The wrapper that lets dynamic text through `sqlx`.
    assert_safe: Regex,
    /// `OR REPLACE` and `REPLACE INTO`.
    replacing: Regex,
}

impl Rules {
    fn new() -> Self {
        Self {
            sql_literal: Regex::new(&format!(r#""(?:{KEYWORDS})\b"#)).unwrap(),
            assert_safe: Regex::new(r"AssertSqlSafe").unwrap(),
            replacing: Regex::new(r"\bOR\s+REPLACE\b|\bREPLACE\s+INTO\b").unwrap(),
        }
    }
}

/// Code lines of `source`: comment lines are not code.
fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(i, line)| (i + 1, line))
}

/// `file:line: text` for every line of `source` that `pattern` matches.
fn hits(file: &str, source: &str, pattern: &Regex) -> Vec<String> {
    code_lines(source)
        .filter(|(_, line)| pattern.is_match(line))
        .map(|(n, line)| format!("{file}:{n}: {}", line.trim()))
        .collect()
}

#[test]
fn sql_text_only_from_sql_exit() {
    let rules = Rules::new();

    // The scanner must be able to see what it is looking for.
    let sample = "let a = \"SELECT 1\";\nlet b = AssertSqlSafe(x);\nlet c = \"selected\";\n\
                  let d = \"INSERT OR REPLACE INTO t\";\n// \"DROP\" AssertSqlSafe\n\
                  let e = \"REPLACE INTO t\";\n";
    assert_eq!(hits("sample.rs", sample, &rules.sql_literal).len(), 3);
    assert_eq!(hits("sample.rs", sample, &rules.assert_safe).len(), 1);
    assert_eq!(hits("sample.rs", sample, &rules.replacing).len(), 2);

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let exit = src.join("sql_exit.rs");
    let mut found = Vec::new();
    let mut scanned = 0;
    let mut saw_exit = false;
    for entry in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        scanned += 1;
        let text = std::fs::read_to_string(path).unwrap();
        let rel = path.strip_prefix(&src).unwrap().display().to_string();
        found.extend(hits(&rel, &text, &rules.replacing));
        if path == exit {
            saw_exit = true;
            continue;
        }
        found.extend(hits(&rel, &text, &rules.sql_literal));
        found.extend(hits(&rel, &text, &rules.assert_safe));
    }
    assert!(
        scanned > 0,
        "no source files scanned under {}",
        src.display()
    );
    assert!(
        saw_exit,
        "src/sql_exit.rs was not found: the guard is stale"
    );
    assert!(
        found.is_empty(),
        "SQL text outside src/sql_exit.rs, or a replacing conflict clause:\n{}",
        found.join("\n")
    );
}
