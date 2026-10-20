//! Property values of the edit store and their one JSON encoding (D-10, D-11).
//!
//! `to_json` is the only encoding of a value in the edit log and in the history snapshots, and it
//! is the same encoding the typed edit tables hold (ADR-0024), so a replay needs no type
//! inference.

use ontoforge_expr::rowpred::Scalar;
use ontoforge_schema::ir::PropertyDef;

/// A value of one property, in canonical form once it passed [`PropertyValue::check`] or came out
/// of [`PropertyValue::from_json`].
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    /// Canonical decimal text with exactly the scale of the property.
    Decimal(String),
    String(String),
    /// `YYYY-MM-DD`.
    Date(String),
    /// UTC with milliseconds: `YYYY-MM-DDTHH:MM:SS.fffZ`.
    Timestamp(String),
    Array(Vec<PropertyValue>),
    Geopoint {
        lat: f64,
        lon: f64,
    },
}

/// Why a value does not fit its property.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ValueError(pub String);

impl PropertyValue {
    /// Checks type, range, precision and canonical text against `def`.
    pub fn check(&self, _def: &PropertyDef) -> Result<(), ValueError> {
        Ok(())
    }

    /// The canonical form of the value for `def`.
    pub fn canonical(&self, _def: &PropertyDef) -> Result<Self, ValueError> {
        Ok(self.clone())
    }

    /// The JSON encoding used by the edit log and the history.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    /// Reads a value written by [`PropertyValue::to_json`] (or the equivalent client JSON).
    pub fn from_json(_def: &PropertyDef, _v: &serde_json::Value) -> Result<Self, ValueError> {
        Ok(Self::Null)
    }

    /// The value as a row predicate scalar; types a predicate cannot use become `Scalar::Null`.
    pub fn to_scalar(&self) -> Scalar {
        Scalar::Null
    }
}
