//! The column names a fixture file declares.
//!
//! The only implementation of fixture header semantics: the engine loads fixtures
//! with it and the offline lint rule compares the same names against the binding.

/// The column names of a fixture, in file order.
///
/// A `.csv` file's header is its first record (a leading UTF-8 byte order mark is
/// ignored). A `.json` file is a top-level array of objects whose header is the union
/// of all keys, in order of first appearance. The error text names a structural
/// problem only and never repeats data from the file.
pub fn fixture_header(file: &str, text: &str) -> Result<Vec<String>, String> {
    let _ = (file, text);
    Err("not implemented".to_owned())
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
