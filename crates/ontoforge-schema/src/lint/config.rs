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
//! Anything unrecognized is `ONT-V15`, so a typo never silently disables nothing.
//!
//! Positions come from `toml::Spanned`: the file is read into a [`Raw`] tree whose
//! every key and value carries its byte span, then checked by hand.

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

use camino::Utf8Path;
use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use toml::Spanned;

use super::{KNOWN_RULE_IDS, Level, LintRule, registry};
use crate::diag::{Diagnostic, Severity, SourceLocation, codes, did_you_mean};
use crate::loader::{MAX_FILE_BYTES, SourceMap};

/// File name of the configuration at the ontology root.
pub const CONFIG_FILE: &str = "ontoforge.toml";

/// Key `toml` uses to hand a datetime to a generic visitor.
const DATETIME_KEY: &str = "$__toml_private_datetime";

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

/// Any TOML value, every nested key and value with its span.
enum Raw {
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Spanned<Raw>>),
    Table(Vec<(Spanned<String>, Spanned<Raw>)>),
    /// A value no option or level can be: a boolean, a datetime or an out-of-range integer.
    Other,
}

impl<'de> Deserialize<'de> for Raw {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(RawVisitor)
    }
}

struct RawVisitor;

impl<'de> Visitor<'de> for RawVisitor {
    type Value = Raw;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a TOML value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<Raw, E> {
        Ok(Raw::Other)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Raw, E> {
        Ok(Raw::Int(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Raw, E> {
        Ok(i64::try_from(v).map_or(Raw::Other, Raw::Int))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Raw, E> {
        Ok(Raw::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Raw, E> {
        Ok(Raw::Str(v.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Raw, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<Spanned<Raw>>()? {
            items.push(item);
        }
        Ok(Raw::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Raw, A::Error> {
        let mut entries = Vec::new();
        while let Some(key) = map.next_key::<Spanned<String>>()? {
            entries.push((key, map.next_value::<Spanned<Raw>>()?));
        }
        let is_datetime = entries.len() == 1 && entries[0].0.get_ref() == DATETIME_KEY;
        Ok(if is_datetime {
            Raw::Other
        } else {
            Raw::Table(entries)
        })
    }
}

/// Parse and check `text`. `file` is the name used in diagnostics; `rules` tell
/// which options each rule declares.
pub fn parse_config(
    text: &str,
    file: &str,
    rules: &[&dyn LintRule],
) -> (LintConfig, Vec<Diagnostic>) {
    let mut parser = Parser {
        text,
        file,
        rules,
        config: LintConfig::default(),
        diags: Vec::new(),
    };
    match toml::from_str::<Raw>(text) {
        Ok(Raw::Table(sections)) => {
            for (key, value) in sections {
                if key.get_ref() == "lint" {
                    parser.lint(value);
                } else {
                    let name = key.get_ref();
                    let help = did_you_mean(name, ["lint"]).map_or_else(
                        || "the only section is `[lint]`".to_owned(),
                        |s| format!("did you mean `{s}`?"),
                    );
                    parser.problem(key.span(), format!("unknown section `{name}`"), Some(help));
                }
            }
        }
        Ok(_) => {}
        Err(e) => parser.problem(e.span().unwrap_or(0..0), e.message(), None),
    }
    (parser.config, parser.diags)
}

struct Parser<'a> {
    text: &'a str,
    file: &'a str,
    rules: &'a [&'a dyn LintRule],
    config: LintConfig,
    diags: Vec<Diagnostic>,
}

/// The known id for a number written without its zero padding (`ONT-W1` is
/// `ONT-W01`). Edit distance cannot pick it: `ONT-W10` is as close.
fn zero_padded(id: &str) -> Option<&'static str> {
    let rest = id.to_ascii_uppercase();
    let rest = rest.strip_prefix("ONT-")?;
    let mut chars = rest.chars();
    let letter = chars.next()?;
    let number: u32 = chars.as_str().parse().ok()?;
    let padded = format!("ONT-{letter}{number:02}");
    KNOWN_RULE_IDS.iter().find(|k| **k == padded).copied()
}

fn levels_help() -> String {
    let all: Vec<String> = Level::NAMES.iter().map(|l| format!("\"{l}\"")).collect();
    format!("use one of: {}", all.join(", "))
}

impl Parser<'_> {
    fn at(&self, span: &Range<usize>) -> SourceLocation {
        let start = self.text.floor_char_boundary(span.start);
        let before = &self.text[..start];
        let line = before.matches('\n').count() + 1;
        let column = before.chars().rev().take_while(|c| *c != '\n').count() + 1;
        let room = (self.text.len() - start).max(1);
        SourceLocation {
            file: self.file.to_owned(),
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(column).unwrap_or(u32::MAX),
            offset: start,
            length: span.end.saturating_sub(start).clamp(1, room),
        }
    }

    fn problem(&mut self, span: Range<usize>, message: impl Into<String>, help: Option<String>) {
        let mut d = Diagnostic::error(codes::V15, message, &self.at(&span));
        d.help = help;
        self.diags.push(d);
    }

    /// Report `key` when it is not a configurable rule id; true if it was reported.
    fn id_problem(&mut self, key: &Spanned<String>, extra: &[&str]) -> bool {
        let id = key.get_ref();
        if KNOWN_RULE_IDS.contains(&id.as_str()) {
            return false;
        }
        if id.starts_with("ONT-V") {
            self.problem(
                key.span(),
                format!("`{id}` is a structural validation error and cannot be configured"),
                Some(
                    "only lint rules (ONT-E*, ONT-W*) can be set; ONT-V errors are always errors"
                        .to_owned(),
                ),
            );
            return true;
        }
        let candidates = KNOWN_RULE_IDS.iter().chain(extra).copied();
        let help = zero_padded(id)
            .or_else(|| did_you_mean(id, candidates))
            .map_or_else(
                || format!("known rules: {}", KNOWN_RULE_IDS.join(", ")),
                |s| format!("did you mean `{s}`?"),
            );
        self.problem(key.span(), format!("unknown lint rule `{id}`"), Some(help));
        true
    }

    fn lint(&mut self, value: Spanned<Raw>) {
        let span = value.span();
        let Raw::Table(entries) = value.into_inner() else {
            self.problem(
                span,
                "`lint` must be a table",
                Some("write `[lint]` followed by lines like `\"ONT-W01\" = \"off\"`".to_owned()),
            );
            return;
        };
        for (key, value) in entries {
            if key.get_ref() == "options" {
                self.options(value);
            } else if !self.id_problem(&key, &["options"]) {
                self.level(key.get_ref(), &value);
            }
        }
    }

    fn level(&mut self, id: &str, value: &Spanned<Raw>) {
        match value.get_ref() {
            Raw::Str(s) => match Level::parse(s) {
                Some(level) => {
                    self.config.levels.insert(id.to_owned(), level);
                }
                None => self.problem(
                    value.span(),
                    format!("invalid level `{s}` for {id}"),
                    Some(levels_help()),
                ),
            },
            _ => self.problem(
                value.span(),
                format!("the level of {id} must be a string"),
                Some(levels_help()),
            ),
        }
    }

    fn options(&mut self, value: Spanned<Raw>) {
        let span = value.span();
        let Raw::Table(per_rule) = value.into_inner() else {
            self.problem(
                span,
                "`options` must be a table keyed by rule id",
                Some("write `[lint.options.\"ONT-W06\"]`".to_owned()),
            );
            return;
        };
        for (key, table) in per_rule {
            if self.id_problem(&key, &[]) {
                continue;
            }
            let id = key.get_ref();
            let table_span = table.span();
            let Raw::Table(options) = table.into_inner() else {
                self.problem(
                    table_span,
                    format!("the options of {id} must be a table"),
                    Some(format!("write `[lint.options.\"{id}\"]`")),
                );
                continue;
            };
            let rules = self.rules;
            let declared = rules
                .iter()
                .find(|r| r.id() == id)
                .map_or(&[][..], |r| r.option_keys());
            for (name_key, value) in options {
                let name = name_key.get_ref();
                if !declared.contains(&name.as_str()) {
                    let help = if declared.is_empty() {
                        format!("{id} takes no options")
                    } else if let Some(s) = did_you_mean(name, declared.iter().copied()) {
                        format!("did you mean `{s}`?")
                    } else {
                        format!("available options: {}", declared.join(", "))
                    };
                    self.problem(
                        name_key.span(),
                        format!("rule {id} has no option `{name}`"),
                        Some(help),
                    );
                    continue;
                }
                match option_value(&value) {
                    Ok(v) => {
                        self.config
                            .options
                            .entry(id.clone())
                            .or_default()
                            .insert(name.clone(), v);
                    }
                    Err(bad) => self.problem(
                        bad,
                        format!("option `{name}` of {id} has an unsupported value"),
                        Some(
                            "use an integer, a number, a string or an array of strings".to_owned(),
                        ),
                    ),
                }
            }
        }
    }
}

/// The value as an option, or the span of the part that cannot be one.
fn option_value(value: &Spanned<Raw>) -> Result<OptionValue, Range<usize>> {
    match value.get_ref() {
        Raw::Int(n) => Ok(OptionValue::Int(*n)),
        Raw::Float(f) => Ok(OptionValue::Float(*f)),
        Raw::Str(s) => Ok(OptionValue::Str(s.clone())),
        Raw::Array(items) => items
            .iter()
            .map(|item| match item.get_ref() {
                Raw::Str(s) => Ok(s.clone()),
                _ => Err(item.span()),
            })
            .collect::<Result<_, _>>()
            .map(OptionValue::List),
        Raw::Table(_) | Raw::Other => Err(value.span()),
    }
}

/// Load the configuration for the ontology at `root`: `override_path` when given
/// (a missing file is an error), else `<root>/ontoforge.toml` (a missing file means
/// defaults). The text is registered in `sources` so diagnostics can be rendered.
///
/// A relative `override_path` is relative to the current directory. Diagnostics
/// name the file as given, with `/` separators.
pub fn load_config(
    root: &Utf8Path,
    override_path: Option<&Utf8Path>,
    sources: &mut SourceMap,
) -> (LintConfig, Vec<Diagnostic>) {
    let (path, name, required) = match override_path {
        Some(p) => (p.to_owned(), p.as_str().replace('\\', "/"), true),
        None => (root.join(CONFIG_FILE), CONFIG_FILE.to_owned(), false),
    };
    let unreadable = |why: String| {
        let message = format!("cannot read lint configuration `{name}`: {why}");
        let d = Diagnostic::without_location(codes::V15, Severity::Error, message, name.clone());
        (LintConfig::default(), vec![d])
    };
    let bytes = match std::fs::metadata(&path) {
        Ok(m) if m.len() > MAX_FILE_BYTES => {
            return unreadable(format!(
                "file is {} bytes, over the {MAX_FILE_BYTES} byte limit",
                m.len()
            ));
        }
        Ok(_) => match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => return unreadable(e.kind().to_string()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {
            return (LintConfig::default(), Vec::new());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return unreadable("not found".to_owned());
        }
        Err(e) => return unreadable(e.kind().to_string()),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return unreadable("file is not valid UTF-8".to_owned());
    };
    let parsed = parse_config(&text, &name, &registry());
    sources.add_file(&name, text);
    parsed
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
