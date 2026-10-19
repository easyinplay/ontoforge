//! Source level guard for ENGINE-03: SQL text is only written inside `src/printer/`.
//!
//! The compiler already stops other modules from building a `PrintedSql`; this scan catches
//! the remaining ways of smuggling SQL in (for instance a future engine that hands its own
//! string to a driver). It also covers the sources later plans add (Trino client, fixture
//! loading, `--check-pk`).

use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

const KEYWORDS: &str = "SELECT|WITH|INSERT|UPDATE|DELETE|CREATE|DROP|ATTACH|PRAGMA";

/// Returns `file:line: text` for every offending line of `source`.
fn violations(file: &str, source: &str, sql_literal: &Regex, constructor: &Regex) -> Vec<String> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| sql_literal.is_match(line) || constructor.is_match(line))
        .map(|(i, line)| format!("{file}:{}: {}", i + 1, line.trim()))
        .collect()
}

#[test]
fn sql_text_only_from_printer() {
    let sql_literal = Regex::new(&format!(r#""(?:{KEYWORDS})\b"#)).unwrap();
    let constructor = Regex::new(r"PrintedSql::new\(").unwrap();

    // The scanner must be able to see what it is looking for.
    let sample =
        "let a = \"SELECT 1\";\nlet b = PrintedSql::new(x);\nlet c = \"selected\";\n// \"DROP\"\n";
    assert_eq!(
        violations("sample.rs", sample, &sql_literal, &constructor).len(),
        2
    );

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let printer = src.join("printer");
    let mut found = Vec::new();
    let mut scanned = 0;
    for entry in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") || path.starts_with(&printer) {
            continue;
        }
        scanned += 1;
        let text = std::fs::read_to_string(path).unwrap();
        let rel = path.strip_prefix(&src).unwrap().display().to_string();
        found.extend(violations(&rel, &text, &sql_literal, &constructor));
    }
    assert!(
        scanned > 0,
        "no source files scanned under {}",
        src.display()
    );
    assert!(
        found.is_empty(),
        "SQL text outside src/printer/:\n{}",
        found.join("\n")
    );
}
