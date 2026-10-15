//! Finding definition files. Minimal for now: plan 01-03 adds `include` globs,
//! recursion and the safety limits.

use camino::Utf8Path;

use super::MANIFEST;

/// Whether `name` has a definition-file extension.
pub fn is_definition_file(name: &str) -> bool {
    matches!(
        Utf8Path::new(name).extension(),
        Some("yaml" | "yml" | "json")
    )
}

/// Whether `name` is a `.json` file (extension compared case-insensitively).
pub fn is_json_file(name: &str) -> bool {
    Utf8Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// Definition files directly under `root`, manifest excluded, sorted by name.
pub fn discover(root: &Utf8Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name != MANIFEST && is_definition_file(&name) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_matched_exactly() {
        assert!(is_definition_file("a.yaml"));
        assert!(is_definition_file("a.yml"));
        assert!(is_definition_file("a.json"));
        assert!(!is_definition_file("a.toml"));
        assert!(!is_definition_file("yaml"));
    }

    #[test]
    fn manifest_is_excluded_and_names_are_sorted() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["b.yaml", "a.json", "ontology.yaml", "notes.md"] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        let root = Utf8Path::from_path(dir.path()).unwrap();
        assert_eq!(discover(root).unwrap(), ["a.json", "b.yaml"]);
    }
}
