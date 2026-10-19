//! The column names a fixture file declares.
//!
//! The only implementation of fixture header semantics: the engine loads fixtures
//! with it and the offline lint rule compares the same names against the binding.

use camino::Utf8Path;
use indexmap::IndexSet;

/// The column names of a fixture, in file order.
///
/// A leading UTF-8 byte order mark is ignored in either format. A `.csv` file's header
/// is its first record. A `.json` file is a top-level array of objects whose header is
/// the union of all keys, in order of first appearance. A column without a name, or two
/// names that are equal in lower case, are errors: the engine could not load such a
/// fixture, so the lint must not look clean either (WR-S01). The error text names a
/// structural problem only and never repeats data from the file.
pub fn fixture_header(file: &str, text: &str) -> Result<Vec<String>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let extension = Utf8Path::new(file).extension().map(str::to_ascii_lowercase);
    let header = match extension.as_deref() {
        Some("csv") => csv_header(text),
        Some("json") => json_header(text),
        _ => Err("a fixture must be a `.csv` or `.json` file".to_owned()),
    }?;
    let mut seen = std::collections::HashSet::new();
    for name in &header {
        if name.is_empty() {
            return Err("the fixture has a column without a name".to_owned());
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(format!(
                "the fixture has two columns named `{name}` (names are compared in lower case)"
            ));
        }
    }
    Ok(header)
}

fn csv_header(text: &str) -> Result<Vec<String>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    // `headers()` reads the first record only; the rest of the file is not parsed.
    let headers = reader
        .headers()
        .map_err(|_| "the first line is not valid CSV".to_owned())?;
    if headers.is_empty() {
        return Err("the CSV fixture has no header line".to_owned());
    }
    Ok(headers.iter().map(str::to_owned).collect())
}

fn json_header(text: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        format!(
            "the JSON fixture is not valid JSON (line {}, column {})",
            e.line(),
            e.column()
        )
    })?;
    let serde_json::Value::Array(rows) = value else {
        return Err("expected an array of objects at the top level".to_owned());
    };
    let mut keys: IndexSet<String> = IndexSet::new();
    for (i, row) in rows.into_iter().enumerate() {
        let serde_json::Value::Object(row) = row else {
            return Err(format!("element {i} of the array is not an object"));
        };
        keys.extend(row.into_iter().map(|(key, _)| key));
    }
    Ok(keys.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_header_rules() {
        // A byte order mark is ignored, quoted fields keep their commas.
        assert_eq!(
            fixture_header("rows.csv", "\u{FEFF}a,\"b,c\",d\n1,2,3\n").unwrap(),
            ["a", "b,c", "d"]
        );
        // A header without data rows is a header.
        assert_eq!(fixture_header("rows.CSV", "x,y\n").unwrap(), ["x", "y"]);
        assert_eq!(fixture_header("rows.csv", "x,y").unwrap(), ["x", "y"]);
        // Only the first record is parsed: a ragged body is not this function's concern.
        assert_eq!(
            fixture_header("rows.csv", "x,y\n1,2,3,4\n").unwrap(),
            ["x", "y"]
        );
        assert!(fixture_header("rows.csv", "").is_err());
        assert!(fixture_header("rows.csv", "\u{FEFF}").is_err());

        // JSON: union of keys in order of first appearance.
        assert_eq!(
            fixture_header("rows.json", r#"[{"a":1},{"b":2,"a":3}]"#).unwrap(),
            ["a", "b"]
        );
        assert!(fixture_header("rows.json", "[]").unwrap().is_empty());
        assert!(fixture_header("rows.json", r#"{"a":1}"#).is_err());
        assert!(fixture_header("rows.json", r#"[{"a":1}, 2]"#).is_err());
        assert!(fixture_header("rows.json", "[{").is_err());

        // Anything else is not a fixture.
        assert!(fixture_header("rows.txt", "a,b").is_err());
        assert!(fixture_header("rows", "a,b").is_err());
    }

    /// WR-S01: the header rules the engine loads with live here, so the lint agrees with it.
    #[test]
    fn fixture_header_matches_the_engine_rules() {
        // A byte order mark is ignored in JSON too (Notepad writes one).
        assert_eq!(
            fixture_header("rows.json", "\u{FEFF}[{\"a\":1}]").unwrap(),
            ["a"]
        );
        // A column without a name, and names equal in lower case, are refused.
        for (file, text) in [
            ("rows.csv", "id,,name\n"),
            ("rows.csv", "id,ID\n"),
            ("rows.json", r#"[{"id":1,"":2}]"#),
            ("rows.json", r#"[{"Id":1},{"iD":2}]"#),
        ] {
            assert!(fixture_header(file, text).is_err(), "{file}: {text:?}");
        }
    }

    #[test]
    fn fixture_header_errors_do_not_echo_data() {
        let secret = "s3cr3t-value";
        for (file, text) in [
            ("rows.json", format!(r#"[{{"a":"{secret}"}}, "{secret}"]"#)),
            ("rows.json", format!(r#"{{"a":"{secret}"}}"#)),
            ("rows.json", format!(r#"[{{"a":"{secret}"#)),
        ] {
            let err = fixture_header(file, &text).expect_err("malformed");
            assert!(!err.contains(secret), "{err}");
        }
    }
}
