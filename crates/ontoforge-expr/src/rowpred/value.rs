//! Runtime values of principal attributes (D-05).

use std::collections::BTreeMap;
use std::fmt;

/// One non-list value.
///
/// `Null` is the SQL NULL of three-valued evaluation. It is only meaningful as
/// a row value or as a result; an attribute that is absent is [`AttrValue::Null`].
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
}

/// The value of one principal attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Missing or not provided by the identity provider.
    Null,
    Scalar(Scalar),
    /// A list. It never holds [`Scalar::Null`] (`x in list` stays two-valued).
    List(Vec<Scalar>),
}

/// An attribute value that cannot be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValueError {
    /// A list element is NULL.
    NullInList,
    /// A scalar wraps NULL; use [`AttrValue::Null`] to express absence.
    NullScalar,
}

impl fmt::Display for AttrValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullInList => f.write_str("a list attribute must not contain NULL"),
            Self::NullScalar => {
                f.write_str("a scalar attribute must not wrap NULL; use AttrValue::Null")
            }
        }
    }
}

impl std::error::Error for AttrValueError {}

static NULL: AttrValue = AttrValue::Null;

/// The attributes of one principal, by declared name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attrs(BTreeMap<String, AttrValue>);

impl Attrs {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` under `name`, replacing an earlier value.
    ///
    /// # Errors
    ///
    /// [`AttrValueError::NullInList`] when a list holds [`Scalar::Null`], and
    /// [`AttrValueError::NullScalar`] when a scalar wraps it.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: AttrValue,
    ) -> Result<(), AttrValueError> {
        match &value {
            AttrValue::List(items) if items.contains(&Scalar::Null) => {
                return Err(AttrValueError::NullInList);
            }
            AttrValue::Scalar(Scalar::Null) => return Err(AttrValueError::NullScalar),
            _ => {}
        }
        self.0.insert(name.into(), value);
        Ok(())
    }

    /// The value of `name`; [`AttrValue::Null`] when undeclared or absent (D-05).
    #[must_use]
    pub fn get(&self, name: &str) -> &AttrValue {
        self.0.get(name).unwrap_or(&NULL)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &AttrValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrs_missing_is_null() {
        let attrs = Attrs::new();
        assert_eq!(attrs.get("tenantId"), &AttrValue::Null);
    }

    #[test]
    fn attrs_list_rejects_null_element() {
        let mut attrs = Attrs::new();
        let bad = AttrValue::List(vec![Scalar::String("eu".into()), Scalar::Null]);
        assert!(attrs.insert("regions", bad).is_err());
        assert_eq!(attrs.get("regions"), &AttrValue::Null);
    }
}
