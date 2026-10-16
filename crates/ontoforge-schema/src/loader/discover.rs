//! Finding definition files: directory walk, manifest `include` globs and the
//! path-safety rules. Disk and in-memory sources share one selection function so
//! both behave the same.

use camino::Utf8Path;
use globset::{GlobBuilder, GlobSetBuilder};

use super::MANIFEST;
use super::sourcemap::{Part, Seg, SourceMap};
use crate::diag::{Diagnostic, Severity, codes};
use crate::ir::Manifest;

/// Used when the manifest has no `include`.
const DEFAULT_INCLUDE: [&str; 3] = ["**/*.yaml", "**/*.yml", "**/*.json"];

/// Directories never searched for definitions (besides those starting with `.`).
const SKIPPED_DIRS: [&str; 2] = ["target", "node_modules"];

/// Directory that holds source bindings, one subdirectory per profile.
pub const BINDINGS_DIR: &str = "bindings";

/// Directory below a profile directory that holds fixture rows.
const FIXTURES_DIR: &str = "fixtures";

/// Lint configuration file; never a definition file.
const LINT_CONFIG: &str = "ontoforge.toml";

/// Whether `name` is a `.json` file (extension compared case-insensitively).
pub fn is_json_file(name: &str) -> bool {
    Utf8Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// Relative paths always use `/`.
pub fn normalize(path: &str) -> String {
    path.replace('\\', "/")
}

fn dir_is_skipped(name: &str) -> bool {
    name.starts_with('.') || SKIPPED_DIRS.contains(&name)
}

/// Whether a normalized relative file path may hold definitions at all. The
/// manifest and lint config names are compared ignoring ASCII case, because a
/// case-insensitive file system opens `Ontology.yaml` as the manifest (WR-06).
fn is_candidate(rel: &str) -> bool {
    let mut parts: Vec<&str> = rel.split('/').collect();
    let Some(file) = parts.pop() else {
        return false;
    };
    !rel.eq_ignore_ascii_case(MANIFEST)
        && !file.eq_ignore_ascii_case(LINT_CONFIG)
        && !parts.iter().any(|d| dir_is_skipped(d))
        && !is_binding_fixture(rel)
}

/// The profile directory of a normalized relative path: `bindings/<x>/...` gives
/// `<x>`. A file directly under `bindings/` has none.
pub fn profile_dir(rel: &str) -> Option<&str> {
    let mut parts = rel.split('/');
    if parts.next()? != BINDINGS_DIR {
        return None;
    }
    let profile = parts.next()?;
    // A file or directory must follow: `bindings/<x>` alone has no profile.
    parts.next()?;
    Some(profile)
}

/// Whether `rel` is a file under `bindings/<profile>/fixtures/`. Those files hold
/// sample rows, not definitions (E5); `fixtures` directories anywhere else are
/// ordinary.
fn is_binding_fixture(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    parts.len() >= 4 && parts[0] == BINDINGS_DIR && parts[2] == FIXTURES_DIR
}

/// Resolve `fixture_file`, written relative to the directory of `binding_file`, to a
/// path relative to the ontology root (`/` separated). Pure string arithmetic: no
/// file system access, so it behaves the same on disk and in memory.
///
/// Fails when the path is absolute, names a drive, passes above the root, or does
/// not end in `.csv` or `.json`.
pub fn fixture_path(binding_file: &str, fixture_file: &str) -> Result<String, String> {
    let fixture = normalize(fixture_file);
    if fixture.is_empty() {
        return Err("the fixture path is empty".to_owned());
    }
    if fixture.starts_with('/') || fixture.as_bytes().get(1) == Some(&b':') {
        return Err(
            "the fixture path must be relative to the binding file, not absolute or on a drive"
                .to_owned(),
        );
    }
    if fixture.chars().any(char::is_control) {
        return Err("the fixture path contains a control character".to_owned());
    }
    if fixture
        .rsplit('/')
        .next()
        .is_none_or(|last| matches!(last, "" | "." | ".."))
    {
        return Err("the fixture path must name a file".to_owned());
    }
    let binding = normalize(binding_file);
    let mut resolved: Vec<&str> = binding.split('/').collect();
    resolved.pop(); // the binding file's own name
    for segment in fixture.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if resolved.pop().is_none() {
                    return Err("the fixture path leaves the ontology directory".to_owned());
                }
            }
            s if s.contains(':') => {
                return Err("the fixture path must not contain `:`".to_owned());
            }
            s => resolved.push(s),
        }
    }
    let path = resolved.join("/");
    let allowed = Utf8Path::new(&path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv") || e.eq_ignore_ascii_case("json"));
    if !allowed {
        return Err("a fixture must be a `.csv` or `.json` file".to_owned());
    }
    Ok(path)
}

/// V14 for a root file that is the manifest in other letter case: it is opened as
/// the manifest on Windows and macOS but not on Linux, so it must be renamed.
fn miscased_manifest(rel: &str) -> Option<Diagnostic> {
    (rel != MANIFEST && rel.eq_ignore_ascii_case(MANIFEST)).then(|| {
        Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!("the manifest must be named `{MANIFEST}`, not `{rel}`"),
            rel,
        )
        .with_help(format!("rename `{rel}` to `{MANIFEST}`"))
    })
}

/// Regular files under `root` as normalized relative paths. Symbolic links are
/// neither followed nor listed. A file or directory whose name is not UTF-8 is
/// reported and skipped (WR-12).
pub fn walk(root: &Utf8Path) -> (Vec<String>, Vec<Diagnostic>) {
    let mut walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !e.file_type().is_dir()
                || e.file_name().to_str().is_none_or(|n| !dir_is_skipped(n))
        });
    let mut files = Vec::new();
    let mut diags = Vec::new();
    while let Some(entry) = walker.next() {
        match entry {
            Ok(e) if e.depth() > 0 && (e.file_type().is_file() || e.file_type().is_dir()) => {
                let rel_path = e
                    .path()
                    .strip_prefix(root.as_std_path())
                    .unwrap_or(e.path());
                let Some(rel) = Utf8Path::from_path(rel_path).map(|p| normalize(p.as_str())) else {
                    let shown = normalize(&rel_path.to_string_lossy());
                    diags.push(Diagnostic::without_location(
                        codes::V14,
                        Severity::Error,
                        format!("the name of `{shown}` is not valid UTF-8; skipped"),
                        shown,
                    ));
                    if e.file_type().is_dir() {
                        walker.skip_current_dir();
                    }
                    continue;
                };
                if e.file_type().is_file() {
                    diags.extend(miscased_manifest(&rel));
                    files.push(rel);
                }
            }
            Ok(_) => {}
            Err(e) => diags.push(Diagnostic::without_location(
                codes::V14,
                Severity::Error,
                format!("cannot list the ontology directory: {e}"),
                MANIFEST,
            )),
        }
    }
    (files, diags)
}

/// The definition files chosen by the manifest, in a deterministic order.
#[derive(Debug, Default)]
pub struct Selection {
    pub files: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Apply the manifest's `include` to `candidates` (normalized relative paths).
///
/// `manifest` is `None` when it could not be loaded; the default patterns apply
/// then and an empty result is not reported (the manifest error is enough).
pub fn select(
    candidates: Vec<String>,
    manifest: Option<&Manifest>,
    sources: &SourceMap,
    report_empty: bool,
) -> Selection {
    let include = manifest.and_then(|m| m.include.as_deref());
    let patterns: Vec<&str> = include.map_or_else(
        || DEFAULT_INCLUDE.to_vec(),
        |p| p.iter().map(String::as_str).collect(),
    );

    let mut diagnostics = Vec::new();
    let mut set = GlobSetBuilder::new();
    for (i, pattern) in patterns.iter().enumerate() {
        let at =
            || sources.locate_manifest(&[Seg::Key("include".into()), Seg::Index(i)], Part::Value);
        if escapes_root(pattern) {
            diagnostics.push(
                Diagnostic::error(
                    codes::V14,
                    format!("include pattern `{pattern}` escapes the ontology root"),
                    &at(),
                )
                .with_help("use a path relative to the ontology directory, without `..`"),
            );
            continue;
        }
        match GlobBuilder::new(pattern).literal_separator(true).build() {
            Ok(g) => {
                set.add(g);
            }
            Err(e) => diagnostics.push(Diagnostic::error(
                codes::V14,
                format!("invalid include pattern `{pattern}`: {}", e.kind()),
                &at(),
            )),
        }
    }
    if !diagnostics.is_empty() {
        return Selection {
            files: Vec::new(),
            diagnostics,
        };
    }
    let Ok(set) = set.build() else {
        return Selection::default();
    };

    let mut files: Vec<String> = candidates
        .into_iter()
        .filter(|p| is_candidate(p) && set.is_match(p))
        .collect();
    files.sort();
    files.dedup();

    if files.is_empty() && report_empty && manifest.is_some() {
        let key = if include.is_some() {
            "include"
        } else {
            "apiName"
        };
        let at = sources.locate_manifest(&[Seg::Key(key.into())], Part::Value);
        diagnostics.push(
            Diagnostic::error(
                codes::V14,
                "no definition files matched the include patterns",
                &at,
            )
            .with_help(format!(
                "add definition files, or adjust `include` in `{MANIFEST}` (default: {})",
                DEFAULT_INCLUDE.join(", ")
            )),
        );
    }
    Selection { files, diagnostics }
}

fn escapes_root(pattern: &str) -> bool {
    pattern.starts_with(['/', '\\'])
        || pattern.as_bytes().get(1) == Some(&b':')
        || pattern.split(['/', '\\']).any(|seg| seg == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_extension_is_case_insensitive() {
        assert!(is_json_file("a.JSON"));
        assert!(!is_json_file("a.yaml"));
    }

    #[test]
    fn candidates_skip_manifest_lint_config_and_hidden_dirs() {
        assert!(is_candidate("a.yaml"));
        assert!(is_candidate("sub/a.yaml"));
        assert!(!is_candidate("ontology.yaml"));
        assert!(is_candidate("sub/ontology.yaml"));
        assert!(!is_candidate("ontoforge.toml"));
        assert!(!is_candidate("sub/ontoforge.toml"));
        assert!(!is_candidate(".github/ci.yml"));
        assert!(!is_candidate("a/target/x.yaml"));
        assert!(!is_candidate("node_modules/p/package.json"));
    }

    #[test]
    fn fixture_path_rules() {
        let binding = "bindings/dev/flight.yaml";
        assert_eq!(
            fixture_path(binding, "fixtures/flights.csv").as_deref(),
            Ok("bindings/dev/fixtures/flights.csv")
        );
        assert_eq!(fixture_path(binding, "../../x.csv").as_deref(), Ok("x.csv"));
        assert_eq!(
            fixture_path(binding, "./a/../rows.JSON").as_deref(),
            Ok("bindings/dev/rows.JSON")
        );
        for bad in [
            "../../../x.csv",
            "/abs.csv",
            "\\abs.csv",
            "C:/x.csv",
            "fixtures/x.txt",
            "fixtures/x",
            "",
            ".",
            "fixtures/a:b.csv",
            "fixtures/\u{7}.csv",
        ] {
            assert!(fixture_path(binding, bad).is_err(), "{bad:?}");
        }
        assert!(fixture_path("a.yaml", "../x.csv").is_err());
    }

    #[test]
    fn profile_dir_is_the_second_segment_under_bindings() {
        assert_eq!(profile_dir("bindings/dev/flight.yaml"), Some("dev"));
        assert_eq!(profile_dir("bindings/dev/sub/x.yaml"), Some("dev"));
        assert_eq!(profile_dir("bindings/Prod/x.yaml"), Some("Prod"));
        assert_eq!(profile_dir("bindings/flight.yaml"), None);
        assert_eq!(profile_dir("defs/bindings/dev/x.yaml"), None);
        assert_eq!(profile_dir("objects.yaml"), None);
    }

    #[test]
    fn only_binding_fixture_directories_are_excluded() {
        assert!(!is_candidate("bindings/dev/fixtures/rows.json"));
        assert!(!is_candidate("bindings/dev/fixtures/deep/rows.json"));
        assert!(is_candidate("bindings/dev/flight.yaml"));
        assert!(is_candidate("fixtures/objects.yaml"));
        assert!(is_candidate("defs/fixtures/x.yaml"));
        assert!(is_candidate("bindings/fixtures/x.yaml"));
    }

    #[test]
    fn walk_lists_nested_files_and_prunes_dirs() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["b.yaml", "x/a.json", ".git/c.yaml", "target/d.yaml"] {
            let p = dir.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        }
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let (mut files, diags) = walk(root);
        files.sort();
        assert!(diags.is_empty());
        assert_eq!(files, ["b.yaml", "x/a.json"]);
    }
}
