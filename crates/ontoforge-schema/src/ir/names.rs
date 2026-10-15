//! Validated apiName newtypes (D-04).
//!
//! The check runs inside `Deserialize`, so a bad name surfaces as a located
//! `ONT-V07` through the normal deserialization error path.

use std::borrow::{Borrow, Cow};
use std::fmt;

use heck::{ToLowerCamelCase, ToUpperCamelCase};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer};

use crate::diag::{codes, custom_de_error};

/// Words no apiName may equal, compared case-insensitively.
pub const RESERVED_API_NAMES: [&str; 9] = [
    "ontology",
    "object",
    "property",
    "link",
    "relation",
    "rid",
    "primaryKey",
    "typeId",
    "ontologyObject",
];

const MAX_LEN: usize = 100;

#[derive(Clone, Copy)]
enum Initial {
    Upper,
    Lower,
}

fn well_formed(raw: &str, initial: Initial) -> bool {
    let first_ok = raw.chars().next().is_some_and(|c| match initial {
        Initial::Upper => c.is_ascii_uppercase(),
        Initial::Lower => c.is_ascii_lowercase(),
    });
    first_ok && raw.len() <= MAX_LEN && raw.chars().all(|c| c.is_ascii_alphanumeric())
}

fn reserved(raw: &str) -> Option<&'static str> {
    RESERVED_API_NAMES
        .iter()
        .copied()
        .find(|r| r.eq_ignore_ascii_case(raw))
}

/// Rewrite `raw` into the nearest legal spelling, or `None` when none exists.
fn canonical(raw: &str, initial: Initial) -> Option<String> {
    let out = match initial {
        Initial::Upper => raw.to_upper_camel_case(),
        Initial::Lower => raw.to_lower_camel_case(),
    };
    (well_formed(&out, initial) && reserved(&out).is_none() && out != raw).then_some(out)
}

fn check(raw: &str, what: &str, initial: Initial, pattern: &str) -> Result<(), String> {
    if !well_formed(raw, initial) {
        let help = canonical(raw, initial).map(|c| format!("write it as `{c}`"));
        return Err(custom_de_error(
            codes::V07,
            &format!("invalid {what} apiName `{raw}`: must match {pattern}"),
            help.as_deref(),
        ));
    }
    if let Some(word) = reserved(raw) {
        return Err(custom_de_error(
            codes::V07,
            &format!("`{raw}` is a reserved word (`{word}`) and cannot be used as {what} apiName"),
            Some("choose a different name"),
        ));
    }
    Ok(())
}

macro_rules! api_name {
    ($(#[$meta:meta])* $name:ident, $initial:expr, $what:literal, $pattern:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;

            fn try_from(value: String) -> Result<Self, String> {
                check(&value, $what, $initial, $pattern)?;
                Ok(Self(value))
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($name))
            }

            fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
                json_schema!({ "type": "string", "pattern": $pattern })
            }
        }
    };
}

api_name!(
    /// Ontology apiName, `camelCase`.
    OntologyName, Initial::Lower, "ontology", "^[a-z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Object type apiName, `PascalCase`.
    ObjectTypeName, Initial::Upper, "object type", "^[A-Z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Property apiName, `camelCase`.
    PropertyName, Initial::Lower, "property", "^[a-z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Link type apiName, `camelCase`.
    LinkTypeName, Initial::Lower, "link type", "^[a-z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Link end apiName (accessor on that end's object type), `camelCase`.
    LinkEndName, Initial::Lower, "link end", "^[a-z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Action type apiName, `camelCase`.
    ActionTypeName, Initial::Lower, "action type", "^[a-z][A-Za-z0-9]{0,99}$"
);
api_name!(
    /// Action parameter apiName, `camelCase`.
    ParameterName, Initial::Lower, "parameter", "^[a-z][A-Za-z0-9]{0,99}$"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::parse_custom_de_error;

    fn err(r: Result<ObjectTypeName, String>) -> crate::diag::CustomDeError {
        parse_custom_de_error(&r.unwrap_err()).unwrap()
    }

    #[test]
    fn valid_names_are_accepted() {
        assert!(ObjectTypeName::try_from("Flight2".to_owned()).is_ok());
        assert!(PropertyName::try_from("tailNumber".to_owned()).is_ok());
    }

    #[test]
    fn lowercase_object_type_gets_a_canonical_suggestion() {
        let e = err(ObjectTypeName::try_from("flight".to_owned()));
        assert_eq!(e.code, "ONT-V07");
        assert_eq!(e.help.as_deref(), Some("write it as `Flight`"));
    }

    #[test]
    fn separators_are_folded_into_camel_case() {
        let e = PropertyName::try_from("tail_number".to_owned()).unwrap_err();
        let e = parse_custom_de_error(&e).unwrap();
        assert_eq!(e.help.as_deref(), Some("write it as `tailNumber`"));
    }

    fn help_of<T: TryFrom<String, Error = String>>(raw: &str) -> Option<String> {
        let e = T::try_from(raw.to_owned()).err().expect("name is invalid");
        parse_custom_de_error(&e).unwrap().help
    }

    #[test]
    fn acronym_prefix_is_lowered_as_a_word() {
        assert_eq!(
            help_of::<PropertyName>("HTTPServer").as_deref(),
            Some("write it as `httpServer`")
        );
        assert_eq!(
            help_of::<LinkEndName>("HTTPServer").as_deref(),
            Some("write it as `httpServer`")
        );
    }

    #[test]
    fn non_ascii_names_get_no_misleading_suggestion() {
        assert_eq!(help_of::<PropertyName>("café_name"), None);
        assert_eq!(help_of::<ObjectTypeName>("Café"), None);
    }

    #[test]
    fn unfixable_names_get_no_help() {
        assert!(
            err(ObjectTypeName::try_from("1abc".to_owned()))
                .help
                .is_none()
        );
        assert!(err(ObjectTypeName::try_from(String::new())).help.is_none());
    }

    #[test]
    fn reserved_words_match_case_insensitively() {
        for raw in ["Object", "Link", "Property", "Ontology", "Rid", "TypeId"] {
            let e = err(ObjectTypeName::try_from(raw.to_owned()));
            assert!(e.message.contains("reserved"), "{raw}: {}", e.message);
        }
        assert!(PropertyName::try_from("primaryKey".to_owned()).is_err());
    }

    #[test]
    fn length_is_capped_at_one_hundred() {
        assert!(ObjectTypeName::try_from(format!("A{}", "b".repeat(99))).is_ok());
        assert!(ObjectTypeName::try_from(format!("A{}", "b".repeat(100))).is_err());
    }

    #[test]
    fn schema_carries_the_pattern() {
        let mut g = SchemaGenerator::default();
        let s = ObjectTypeName::json_schema(&mut g);
        assert_eq!(
            s.get("pattern").and_then(|p| p.as_str()),
            Some("^[A-Z][A-Za-z0-9]{0,99}$")
        );
    }
}
