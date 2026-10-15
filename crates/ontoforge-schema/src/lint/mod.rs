//! Lint framework (D-12, D-14, D-15). Rules run on an ontology that passed
//! `validate`, produce [`Finding`]s, and the configured level turns each finding
//! into a [`Diagnostic`] with a source location.
//!
//! `ONT-V` errors never go through this module: they cannot be configured (D-06).

use std::collections::HashMap;

use crate::diag::{Diagnostic, Severity, sort_diagnostics};
use crate::ir::Ontology;
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, Part, Seg};

pub mod config;
mod e08;
mod e09;
mod naming;
mod structure;
pub mod words;

pub use config::{LintConfig, OptionValue, load_config, parse_config};

/// A configurable level. In `ontoforge.toml`: `"off"`, `"info"`, `"warn"`, `"error"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Off,
    Info,
    Warn,
    Error,
}

impl Level {
    /// The accepted spellings, in the order shown in help text.
    pub const NAMES: [&'static str; 4] = ["off", "info", "warn", "error"];

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    /// The diagnostic severity; `Off` has none because the rule does not run.
    pub fn severity(self) -> Option<Severity> {
        match self {
            Self::Off => None,
            Self::Info => Some(Severity::Info),
            Self::Warn => Some(Severity::Warning),
            Self::Error => Some(Severity::Error),
        }
    }
}

/// One problem a rule found, positioned by definition and path (not by file).
#[derive(Debug, Clone)]
pub struct Finding {
    pub def: DefKey,
    pub path: Vec<Seg>,
    pub part: Part,
    pub message: String,
    pub help: Option<String>,
    /// Severity used when `ontoforge.toml` does not set a level for the rule.
    pub severity: Option<Severity>,
}

/// What a rule can see: the validated ontology and its own options.
pub struct LintCx<'a> {
    pub ontology: &'a Ontology,
    options: &'a HashMap<String, OptionValue>,
}

impl LintCx<'_> {
    /// Integer option `key`, or `default` when unset or negative.
    pub fn option_usize(&self, key: &str, default: usize) -> usize {
        match self.options.get(key) {
            Some(OptionValue::Int(n)) => usize::try_from(*n).unwrap_or(default),
            _ => default,
        }
    }

    /// Number option `key` (an integer is accepted), or `default`.
    #[allow(clippy::cast_precision_loss)] // option values are small thresholds
    pub fn option_f64(&self, key: &str, default: f64) -> f64 {
        match self.options.get(key) {
            Some(OptionValue::Float(f)) => *f,
            Some(OptionValue::Int(n)) => *n as f64,
            _ => default,
        }
    }

    /// String-list option `key` (a single string is a list of one), or `default`.
    pub fn option_str_list(&self, key: &str, default: &[&str]) -> Vec<String> {
        match self.options.get(key) {
            Some(OptionValue::List(v)) => v.clone(),
            Some(OptionValue::Str(s)) => vec![s.clone()],
            _ => default.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

/// One lint rule. Implementations are `static` values.
pub trait LintRule: Sync {
    /// `ONT-E08` and so on; also the key in `ontoforge.toml`.
    fn id(&self) -> &'static str;
    fn default_level(&self) -> Level;
    /// Names accepted under `[lint.options."<id>"]`.
    fn option_keys(&self) -> &'static [&'static str] {
        &[]
    }
    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>);
}

/// Every rule id that `ontoforge.toml` may mention (D-12: E08, E09 and W01 to W11).
pub const KNOWN_RULE_IDS: [&str; 13] = [
    "ONT-E08", "ONT-E09", "ONT-W01", "ONT-W02", "ONT-W03", "ONT-W04", "ONT-W05", "ONT-W06",
    "ONT-W07", "ONT-W08", "ONT-W09", "ONT-W10", "ONT-W11",
];

/// All rules, E rules first. The W rules come from `naming` and `structure`.
pub fn registry() -> Vec<&'static dyn LintRule> {
    let mut rules: Vec<&'static dyn LintRule> = vec![&e08::RULE, &e09::RULE];
    rules.extend(naming::rules());
    rules.extend(structure::rules());
    rules
}

/// Run every registered rule under `config`; the result is sorted.
pub fn run(ontology: &Ontology, sources: &SourceMap, config: &LintConfig) -> Vec<Diagnostic> {
    run_rules(&registry(), ontology, sources, config)
}

fn run_rules(
    rules: &[&dyn LintRule],
    ontology: &Ontology,
    sources: &SourceMap,
    config: &LintConfig,
) -> Vec<Diagnostic> {
    let no_options = HashMap::new();
    let mut out = Vec::new();
    for rule in rules {
        let configured = config.level(rule.id());
        if configured.unwrap_or_else(|| rule.default_level()) == Level::Off {
            continue;
        }
        let cx = LintCx {
            ontology,
            options: config.options(rule.id()).unwrap_or(&no_options),
        };
        let mut findings = Vec::new();
        rule.check(&cx, &mut findings);
        for f in findings {
            // A configured level replaces every severity of the rule; otherwise the
            // finding's own severity applies, then the rule's default level.
            let severity = configured
                .and_then(Level::severity)
                .or(f.severity)
                .or_else(|| rule.default_level().severity())
                .unwrap_or(Severity::Warning);
            let at = sources.locate(&f.def, &f.path, f.part);
            let mut d = Diagnostic::new(rule.id(), severity, f.message, &at);
            d.help = f.help;
            out.push(d);
        }
    }
    sort_diagnostics(&mut out);
    out
}

/// Run one registered rule on in-memory files; panics if the rule is not
/// registered, the files do not load cleanly, or `config_toml` is invalid.
#[cfg(test)]
pub(crate) fn lint_rule_on(
    rule_id: &str,
    files: &[(&str, &str)],
    config_toml: Option<&str>,
) -> Vec<Diagnostic> {
    let out = crate::load_from_memory(files);
    assert!(
        !crate::diag::has_errors(&out.diagnostics),
        "fixture does not validate: {:#?}",
        out.diagnostics
    );
    let ontology = out.ontology.expect("clean load has an ontology");
    let rules = registry();
    let rule = *rules
        .iter()
        .find(|r| r.id() == rule_id)
        .unwrap_or_else(|| panic!("rule {rule_id} is not registered"));
    let config = match config_toml {
        Some(text) => {
            let (config, diags) = parse_config(text, "ontoforge.toml", &rules);
            assert!(diags.is_empty(), "bad test config: {diags:#?}");
            config
        }
        None => LintConfig::default(),
    };
    run_rules(&[rule], &ontology, &out.sources, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::sourcemap::DefKind;

    /// Reports one finding per object type, with a fixed own severity.
    struct Probe {
        default: Level,
        own: Option<Severity>,
    }

    impl LintRule for Probe {
        fn id(&self) -> &'static str {
            "ONT-W01"
        }
        fn default_level(&self) -> Level {
            self.default
        }
        fn option_keys(&self) -> &'static [&'static str] {
            &["limit"]
        }
        fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
            for name in cx.ontology.object_types.keys() {
                out.push(Finding {
                    def: DefKey {
                        kind: DefKind::ObjectType,
                        api_name: name.to_string(),
                    },
                    path: vec![Seg::Key("apiName".into())],
                    part: Part::Value,
                    message: format!("probe saw {name}"),
                    help: Some("probe help".into()),
                    severity: self.own,
                });
            }
        }
    }

    const MANIFEST: &str = "apiName: demo\n";
    const OBJECTS: &str = "kind: ObjectType\napiName: Flight\nprimaryKey: id\nproperties:\n  id: { type: string, required: true }\n";

    fn loaded() -> crate::LoadOutput {
        let out = crate::load_from_memory(&[("ontology.yaml", MANIFEST), ("o.yaml", OBJECTS)]);
        assert!(out.ontology.is_some(), "{:#?}", out.diagnostics);
        out
    }

    fn run_probe(probe: &Probe, toml: &str) -> Vec<Diagnostic> {
        let out = loaded();
        let rule: &dyn LintRule = probe;
        let (config, diags) = parse_config(toml, "ontoforge.toml", &[rule]);
        assert!(diags.is_empty(), "{diags:#?}");
        run_rules(&[rule], &out.ontology.unwrap(), &out.sources, &config)
    }

    fn probe(default: Level, own: Option<Severity>) -> Probe {
        Probe { default, own }
    }

    #[test]
    fn level_resolution() {
        // No config: the finding's own severity wins, then the default level.
        let d = run_probe(&probe(Level::Warn, Some(Severity::Error)), "");
        assert_eq!(d.len(), 1);
        assert_eq!(
            (d[0].code.as_str(), d[0].severity),
            ("ONT-W01", Severity::Error)
        );
        let d = run_probe(&probe(Level::Warn, None), "");
        assert_eq!(d[0].severity, Severity::Warning);
        let d = run_probe(&probe(Level::Error, None), "");
        assert_eq!(d[0].severity, Severity::Error);

        // A configured level replaces both, for every finding of the rule.
        let own = Some(Severity::Error);
        let d = run_probe(&probe(Level::Warn, own), "[lint]\n\"ONT-W01\" = \"info\"\n");
        assert_eq!(d[0].severity, Severity::Info);
        let d = run_probe(&probe(Level::Warn, own), "[lint]\n\"ONT-W01\" = \"warn\"\n");
        assert_eq!(d[0].severity, Severity::Warning);

        // Off skips the rule, whether configured or the default.
        let d = run_probe(&probe(Level::Warn, own), "[lint]\n\"ONT-W01\" = \"off\"\n");
        assert!(d.is_empty());
        let d = run_probe(&probe(Level::Off, own), "");
        assert!(d.is_empty());
    }

    #[test]
    fn findings_are_located_in_the_source() {
        let d = run_probe(&probe(Level::Warn, None), "");
        assert_eq!(
            (d[0].file.as_str(), d[0].line, d[0].column),
            ("o.yaml", 2, 10)
        );
        assert_eq!(d[0].help.as_deref(), Some("probe help"));
        let span = d[0].span;
        assert_eq!(&OBJECTS[span.offset..span.offset + span.length], "Flight");
    }

    #[test]
    fn rule_options_reach_the_rule() {
        let out = loaded();
        let ontology = out.ontology.as_ref().unwrap();
        let probe = probe(Level::Warn, None);
        let rule: &dyn LintRule = &probe;
        let text = "[lint.options.\"ONT-W01\"]\nlimit = 7\n";
        let (config, diags) = parse_config(text, "ontoforge.toml", &[rule]);
        assert!(diags.is_empty(), "{diags:#?}");
        let cx = LintCx {
            ontology,
            options: config.options("ONT-W01").unwrap(),
        };
        assert_eq!(cx.option_usize("limit", 1), 7);
        assert_eq!(cx.option_usize("other", 1), 1);
        assert!((cx.option_f64("limit", 0.5) - 7.0).abs() < f64::EPSILON);
        assert_eq!(cx.option_str_list("names", &["a", "b"]), ["a", "b"]);
    }

    #[test]
    #[should_panic(expected = "is not registered")]
    fn lint_rule_on_panics_for_an_unregistered_rule() {
        lint_rule_on(
            "ONT-W99",
            &[("ontology.yaml", MANIFEST), ("o.yaml", OBJECTS)],
            None,
        );
    }

    #[test]
    fn registry_ids_are_known_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in registry() {
            assert!(
                KNOWN_RULE_IDS.contains(&rule.id()),
                "{} is unknown",
                rule.id()
            );
            assert!(seen.insert(rule.id()), "{} registered twice", rule.id());
        }
        let mut ids = KNOWN_RULE_IDS.to_vec();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), KNOWN_RULE_IDS.len());
    }
}
