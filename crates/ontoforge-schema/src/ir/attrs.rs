//! Declared type of a principal attribute (D-05).

use serde::{Deserialize, Serialize};

/// The declared type of a principal attribute, set in `ontoforge-server.toml`
/// under `[auth.principal_attrs]`.
///
/// Row predicates (`rowFilter`) are statically type checked against these
/// declarations, so a predicate cannot read `principal.attrs.x` as the wrong
/// type at request time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttrType {
    String,
    Integer,
    Boolean,
    StringList,
    IntegerList,
}

impl AttrType {
    /// Whether values of this type are lists.
    #[must_use]
    pub fn is_list(self) -> bool {
        matches!(self, Self::StringList | Self::IntegerList)
    }
}

#[cfg(test)]
mod tests {
    use super::AttrType;

    #[test]
    fn is_list_only_for_list_types() {
        assert!(AttrType::StringList.is_list());
        assert!(AttrType::IntegerList.is_list());
        for t in [AttrType::String, AttrType::Integer, AttrType::Boolean] {
            assert!(!t.is_list());
        }
    }
}
