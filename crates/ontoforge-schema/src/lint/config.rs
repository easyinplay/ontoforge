//! `ontoforge.toml` (D-14, LINT-04): rule levels and rule options.
//!
//! ```toml
//! [lint]
//! "ONT-W01" = "off"       # off | info | warn | error
//! "ONT-E09" = "warn"
//!
//! [lint.options."ONT-W06"]
//! max_properties = 60
//! ```
//!
//! Rule ids contain a hyphen, so they must be quoted keys. Only the lint rules
//! (`ONT-E*`, `ONT-W*`) can be configured; `ONT-V*` structural errors cannot (D-06).
//! Anything the loader does not recognize is `ONT-V15`, so a typo never silently
//! disables nothing.

use std::collections::HashMap;

use camino::Utf8Path;

use super::{Level, LintRule};
use crate::diag::Diagnostic;
use crate::loader::SourceMap;

/// File name of the configuration at the ontology root.
pub const CONFIG_FILE: &str = "ontoforge.toml";

/// A rule option value: an integer, a number, a string, or a list of strings.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<String>),
}

/// Parsed `ontoforge.toml`. The default configuration changes nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LintConfig {
    levels: HashMap<String, Level>,
    options: HashMap<String, HashMap<String, OptionValue>>,
}

impl LintConfig {
    /// The level configured for `rule_id`, if any.
    pub fn level(&self, rule_id: &str) -> Option<Level> {
        self.levels.get(rule_id).copied()
    }

    /// The options configured for `rule_id`, if any.
    pub fn options(&self, rule_id: &str) -> Option<&HashMap<String, OptionValue>> {
        self.options.get(rule_id)
    }
}

/// Parse and check `text`. `file` is the name used in diagnostics; `rules` tell
/// which options each rule declares.
pub fn parse_config(
    text: &str,
    file: &str,
    rules: &[&dyn LintRule],
) -> (LintConfig, Vec<Diagnostic>) {
    let _ = (text, file, rules);
    (LintConfig::default(), Vec::new())
}

/// Load the configuration for the ontology at `root`: `override_path` when given
/// (a missing file is an error), else `<root>/ontoforge.toml` (a missing file means
/// defaults). The text is registered in `sources` so diagnostics can be rendered.
pub fn load_config(
    root: &Utf8Path,
    override_path: Option<&Utf8Path>,
    sources: &mut SourceMap,
) -> (LintConfig, Vec<Diagnostic>) {
    let _ = (root, override_path, sources);
    (LintConfig::default(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Severity, codes};
    use crate::lint::{Finding, LintCx};

    struct Fake {
        id: &'static str,
        keys: &'static [&'static str],
    }

    impl LintRule for Fake {
        fn id(&self) -> &'static str {
            self.id
        }
        fn default_level(&self) -> Level {
            Level::Warn
        }
        fn option_keys(&self) -> &'static [&'static str] {
            self.keys
        }
        fn check(&self, _: &LintCx<'_>, _: &mut Vec<Finding>) {}
    }

    static E08: Fake = Fake {
        id: "ONT-E08",
        keys: &[],
    };
    static E09: Fake = Fake {
        id: "ONT-E09",
        keys: &[],
    };
    static W01: Fake = Fake {
        id: "ONT-W01",
        keys: &[],
    };
    static W06: Fake = Fake {
        id: "ONT-W06",
        keys: &["max_properties", "ratio", "words"],
    };

    fn rules() -> Vec<&'static dyn LintRule> {
        vec![&E08, &E09, &W01, &W06]
    }

    fn parse(text: &str) -> (LintConfig, Vec<Diagnostic>) {
        parse_config(text, "ontoforge.toml", &rules())
    }

    /// The single V15 diagnostic of `text`, with the text it points at.
    fn one_problem(text: &str) -> (Diagnostic, String) {
        let (_, diags) = parse(text);
        assert_eq!(diags.len(), 1, "{diags:#?}");
        let d = diags.into_iter().next().unwrap();
        assert_eq!(d.code, codes::V15);
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.file, "ontoforge.toml");
        let at = text[d.span.offset..d.span.offset + d.span.length].to_owned();
        (d, at)
    }

    #[test]
    fn config_levels_parse() {
        let (config, diags) = parse("[lint]\n\"ONT-W01\" = \"off\"\n\"ONT-E09\" = \"warn\"\n");
        assert!(diags.is_empty(), "{diags:#?}");
        assert_eq!(config.level("ONT-W01"), Some(Level::Off));
        assert_eq!(config.level("ONT-E09"), Some(Level::Warn));
        assert_eq!(config.level("ONT-E08"), None);
    }

    #[test]
    fn config_options_parse() {
        let text = "[lint.options.\"ONT-W06\"]\nmax_properties = 60\nratio = 0.5\nwords = [\"a\", \"b\"]\n";
        let (config, diags) = parse(text);
        assert!(diags.is_empty(), "{diags:#?}");
        let options = config.options("ONT-W06").unwrap();
        assert_eq!(options["max_properties"], OptionValue::Int(60));
        assert_eq!(options["ratio"], OptionValue::Float(0.5));
        assert_eq!(
            options["words"],
            OptionValue::List(vec!["a".into(), "b".into()])
        );
        assert!(config.options("ONT-W01").is_none());
    }

    #[test]
    fn empty_and_lint_only_files_are_fine() {
        assert!(parse("").1.is_empty());
        assert!(parse("[lint]\n").1.is_empty());
    }

    #[test]
    fn unknown_lint_rule_id_is_error() {
        let text = "[lint]\n\"ONT-W1\" = \"off\"\n";
        let (d, at) = one_problem(text);
        assert_eq!((d.line, d.column), (2, 1));
        assert_eq!(at, "\"ONT-W1\"");
        assert!(d.message.contains("ONT-W1"), "{}", d.message);
        assert!(d.help.as_deref().unwrap().contains("ONT-W01"));
    }

    #[test]
    fn validate_codes_cannot_be_configured() {
        let (d, at) = one_problem("[lint]\n\"ONT-V04\" = \"off\"\n");
        assert_eq!(at, "\"ONT-V04\"");
        assert!(d.message.contains("structural"), "{}", d.message);
        assert!(d.message.contains("cannot be configured"), "{}", d.message);
    }

    #[test]
    fn invalid_level_is_error() {
        let (d, at) = one_problem("[lint]\n\"ONT-W01\" = \"silent\"\n");
        assert_eq!(at, "\"silent\"");
        assert_eq!((d.line, d.column), (2, 13));
        let help = d.help.unwrap();
        for level in ["off", "info", "warn", "error"] {
            assert!(help.contains(level), "{help}");
        }
        let (_, at) = one_problem("[lint]\n\"ONT-W01\" = 3\n");
        assert_eq!(at, "3");
    }

    #[test]
    fn unknown_section_is_error() {
        let (d, at) = one_problem("[lnt]\n\"ONT-W01\" = \"off\"\n");
        assert_eq!(at, "lnt");
        assert_eq!((d.line, d.column), (1, 2));
        assert!(d.help.unwrap().contains("lint"));
    }

    #[test]
    fn unknown_key_in_lint_section_is_error() {
        let (_, at) = one_problem("[lint]\nwarnings = \"off\"\n");
        assert_eq!(at, "warnings");
    }

    #[test]
    fn undeclared_option_is_error() {
        let (d, at) = one_problem("[lint.options.\"ONT-E08\"]\nx = 1\n");
        assert_eq!(at, "x");
        assert!(d.message.contains("ONT-E08"), "{}", d.message);
        let (d, at) =
            one_problem("[lint.options.\"ONT-W06\"]\nmax_properties = 60\nmax_propertys = 3\n");
        assert_eq!(at, "max_propertys");
        assert!(d.help.unwrap().contains("max_properties"));
    }

    #[test]
    fn option_for_unknown_rule_or_with_bad_value_is_error() {
        let (_, at) = one_problem("[lint.options.\"ONT-W66\"]\nx = 1\n");
        assert_eq!(at, "\"ONT-W66\"");
        let (_, at) = one_problem("[lint.options.\"ONT-W06\"]\nratio = true\n");
        assert_eq!(at, "true");
        let (_, at) = one_problem("[lint.options.\"ONT-W06\"]\nwords = [1]\n");
        assert_eq!(at, "1");
        let (_, at) = one_problem("[lint.options]\n\"ONT-W06\" = 3\n");
        assert_eq!(at, "3");
    }

    #[test]
    fn lint_must_be_a_table() {
        let (_, at) = one_problem("lint = 1\n");
        assert_eq!(at, "1");
    }

    #[test]
    fn toml_syntax_error_is_v15_at_its_span() {
        let (d, _) = one_problem("[lint\n");
        assert_eq!(d.line, 1);
    }

    #[test]
    fn every_problem_is_reported() {
        let (_, diags) = parse("[lint]\n\"ONT-W1\" = \"off\"\n\"ONT-W01\" = \"silent\"\n");
        assert_eq!(diags.len(), 2, "{diags:#?}");
    }

    #[test]
    fn missing_config_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let mut sources = SourceMap::default();
        let (config, diags) = load_config(root, None, &mut sources);
        assert!(diags.is_empty());
        assert_eq!(config, LintConfig::default());
    }

    #[test]
    fn root_file_is_read_and_registered() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(
            root.join("ontoforge.toml"),
            "[lint]\n\"ONT-W01\" = \"silent\"\n",
        )
        .unwrap();
        let mut sources = SourceMap::default();
        let (_, diags) = load_config(root, None, &mut sources);
        assert_eq!(diags.len(), 1, "{diags:#?}");
        assert_eq!(diags[0].file, "ontoforge.toml");
        assert_eq!(diags[0].line, 2);
        assert!(sources.text("ontoforge.toml").unwrap().contains("silent"));
    }

    #[test]
    fn override_path_replaces_the_root_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(
            root.join("ontoforge.toml"),
            "[lint]\n\"ONT-W01\" = \"silent\"\n",
        )
        .unwrap();
        let other = root.join("other.toml");
        std::fs::write(&other, "[lint]\n\"ONT-E09\" = \"info\"\n").unwrap();
        let mut sources = SourceMap::default();
        let (config, diags) = load_config(root, Some(&other), &mut sources);
        assert!(diags.is_empty(), "{diags:#?}");
        assert_eq!(config.level("ONT-E09"), Some(Level::Info));
    }

    #[test]
    fn missing_override_path_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let mut sources = SourceMap::default();
        let (_, diags) = load_config(
            root,
            Some(Utf8Path::new("nope\\missing.toml")),
            &mut sources,
        );
        assert_eq!(diags.len(), 1, "{diags:#?}");
        assert_eq!(diags[0].code, codes::V15);
        assert_eq!(diags[0].file, "nope/missing.toml");
    }
}
