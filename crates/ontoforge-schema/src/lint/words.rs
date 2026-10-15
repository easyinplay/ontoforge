//! Name tokenizer shared by the naming rules.

/// Split a camelCase or PascalCase name into lowercase words. Boundaries are
/// lower to upper (`releaseDate`), letter to digit and back (`code01`), and the end
/// of an acronym (`HTTPServer` is `http`, `server`).
pub fn split_camel(name: &str) -> Vec<String> {
    let _ = name;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::split_camel;

    #[test]
    fn split_camel_cases() {
        assert_eq!(split_camel("ReleaseDate"), ["release", "date"]);
        assert_eq!(split_camel("code01"), ["code", "01"]);
        assert_eq!(split_camel("HTTPServer"), ["http", "server"]);
        assert_eq!(split_camel("FlightV2"), ["flight", "v", "2"]);
        assert_eq!(split_camel("isCurrent"), ["is", "current"]);
        assert_eq!(split_camel("status"), ["status"]);
        assert!(split_camel("").is_empty());
    }
}
