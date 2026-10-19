//! ONT-W06: God Object (D-13, static part only).
//!
//! An object type is reported when it has more than `max_properties` properties
//! (default 60), or when it has a `string` discriminator property (last word `type`
//! or `subtype`, such as `accountType`) and more than half of `max_properties`
//! properties: a type that switches on a discriminator usually hides several types.
//!
//! The null-rate branch of the original rule needs stored data, so it is Deferred
//! until the storage phase.

use super::super::words::split_camel;
use super::super::{Finding, Level, LintCx, LintRule, OptionKind};
use super::{off_hint, type_finding};
use crate::ir::BaseType;

const DEFAULT_MAX_PROPERTIES: usize = 60;

pub(super) struct W06;

pub(super) static RULE: W06 = W06;

impl LintRule for W06 {
    fn id(&self) -> &'static str {
        "ONT-W06"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn option_keys(&self) -> &'static [(&'static str, OptionKind)] {
        &[("max_properties", OptionKind::Count)]
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        let max = cx.option_usize("max_properties", DEFAULT_MAX_PROPERTIES);
        for object in cx.ontology.object_types.values() {
            let count = object.properties.len();
            if count > max {
                out.push(type_finding(
                    object,
                    format!(
                        "object type `{}` has {count} properties, more than the limit of {max}",
                        object.api_name
                    ),
                    format!(
                        "split it into several object types; {}",
                        off_hint(self.id())
                    ),
                ));
            } else if count > max / 2 {
                let discriminator = object.properties.iter().find(|(name, def)| {
                    def.base_type == BaseType::String
                        && split_camel(name.as_str())
                            .last()
                            .is_some_and(|w| w == "type" || w == "subtype")
                });
                if let Some((name, _)) = discriminator {
                    out.push(type_finding(
                        object,
                        format!(
                            "object type `{}` has {count} properties and the discriminator `{name}`",
                            object.api_name
                        ),
                        format!(
                            "one object type per `{name}` value, or an interface for the shared part, is easier to maintain; {}",
                            off_hint(self.id())
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run, run_with};

    /// An object type with `total` properties (primary key included); `extra` are
    /// part of that total.
    fn wide(name: &str, total: usize, extra: &[(&str, &str)]) -> String {
        let names: Vec<String> = (0..total - 1 - extra.len())
            .map(|i| format!("p{i}"))
            .collect();
        let mut props: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "integer")).collect();
        props.extend_from_slice(extra);
        object(name, &format!("{}Id", name.to_lowercase()), &props)
    }

    #[test]
    fn triggers() {
        let text = join(&[
            wide("Big", 61, &[]),
            wide("Account", 31, &[("accountType", "string")]),
            wide("Item", 31, &[("itemSubtype", "string")]),
        ]);
        let found = run("ONT-W06", &text);
        assert_eq!(flagged(&text, &found), ["Big", "Account", "Item"]);
        assert!(found[0].message.contains("61"), "{}", found[0].message);
        assert!(found[1].message.contains("`accountType`"));
        assert!(found[2].message.contains("`itemSubtype`"));
        assert!(found[0].help.as_deref().unwrap().contains("ONT-W06"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            wide("Plain", 20, &[("type", "string")]),
            wide("Skew", 31, &[("accountType", "integer")]),
            wide("Edge", 60, &[]),
        ]);
        let found = run("ONT-W06", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn max_properties_option_applies() {
        let text = wide("Small", 6, &[]);
        assert!(run("ONT-W06", &text).is_empty());
        let config = "[lint.options.\"ONT-W06\"]\nmax_properties = 5\n";
        let found = run_with("ONT-W06", &text, Some(config));
        assert_eq!(flagged(&text, &found), ["Small"]);
        assert!(found[0].message.contains('5'), "{}", found[0].message);
    }

    #[test]
    fn unknown_option_key_is_rejected() {
        let rules = crate::lint::registry();
        let (_, diags) = crate::lint::parse_config(
            "[lint.options.\"ONT-W06\"]\nmax_props = 5\n",
            "ontoforge.toml",
            &rules,
        );
        assert_eq!(diags.len(), 1, "{diags:#?}");
        assert_eq!(diags[0].code, "ONT-V15");
    }
}
