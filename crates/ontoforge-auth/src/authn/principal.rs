//! The authenticated caller: identifier, groups and typed attributes.
//!
//! Identifier format and group handling follow D-04 / D-02; attribute mapping follows D-05.
//! Nothing in an error message here carries a claim value: only attribute names from the
//! configuration and the expected type.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::config::PrincipalAttrConfig;
use ontoforge_expr::rowpred::{AttrValue, Attrs, Scalar};
use ontoforge_schema::ir::AttrType;
use serde_json::{Map, Value};

use super::AuthnError;

/// Canonical principal identifier, `"{iss}|{sub}"` (D-04). `|` cannot occur in a configured
/// issuer, so the first `|` always separates the two parts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrincipalId(String);

impl PrincipalId {
    #[must_use]
    pub fn new(issuer: &str, subject: &str) -> Self {
        Self(format!("{issuer}|{subject}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PrincipalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A caller whose bearer token was verified.
#[derive(Debug, Clone, PartialEq)]
pub struct Principal {
    pub id: PrincipalId,
    pub issuer: String,
    pub subject: String,
    /// Group names from the configured groups claim (D-02). Group membership is never
    /// stored. With more than one trusted issuer each name is `"{iss}|{group}"`.
    pub groups: BTreeSet<String>,
    /// Attributes from the claims named in `[auth.principal_attrs]` (D-05). A claim that is
    /// absent leaves the attribute out, which reads as `Null`.
    pub attrs: Attrs,
}

/// Maps verified claims to a [`Principal`] (D-02 / D-05).
///
/// `namespaced` is true when more than one issuer is trusted.
pub(crate) fn principal_from_claims(
    issuer: &str,
    claims: &Map<String, Value>,
    groups_claim: &str,
    namespaced: bool,
    attrs: &BTreeMap<String, PrincipalAttrConfig>,
) -> Result<Principal, AuthnError> {
    let subject = match claims.get("sub") {
        Some(Value::String(sub)) if !sub.is_empty() => sub.clone(),
        _ => {
            return Err(AuthnError::InvalidClaims(
                "claim `sub` is missing".to_owned(),
            ));
        }
    };
    Ok(Principal {
        id: PrincipalId::new(issuer, &subject),
        issuer: issuer.to_owned(),
        subject,
        groups: groups_from_claims(issuer, claims, groups_claim, namespaced)?,
        attrs: attrs_from_claims(claims, attrs)?,
    })
}

fn groups_from_claims(
    issuer: &str,
    claims: &Map<String, Value>,
    groups_claim: &str,
    namespaced: bool,
) -> Result<BTreeSet<String>, AuthnError> {
    let Some(value) = claims.get(groups_claim) else {
        return Ok(BTreeSet::new());
    };
    let Value::Array(items) = value else {
        return Err(AuthnError::InvalidClaims(format!(
            "claim `{groups_claim}` must be an array of strings"
        )));
    };
    items
        .iter()
        .map(|item| match item {
            Value::String(group) if namespaced => Ok(format!("{issuer}|{group}")),
            Value::String(group) => Ok(group.clone()),
            _ => Err(AuthnError::InvalidClaims(format!(
                "claim `{groups_claim}` must be an array of strings"
            ))),
        })
        .collect()
}

fn attrs_from_claims(
    claims: &Map<String, Value>,
    configured: &BTreeMap<String, PrincipalAttrConfig>,
) -> Result<Attrs, AuthnError> {
    let mut attrs = Attrs::new();
    for (name, attr) in configured {
        let value = match claims.get(&attr.claim) {
            // Not provided: the attribute stays absent and reads as Null (D-05).
            None | Some(Value::Null) => continue,
            Some(value) => attr_value(value, attr.ty).ok_or_else(|| mismatch(name, attr.ty))?,
        };
        // A second line of defence behind the element checks in `attr_value`.
        attrs
            .insert(name.as_str(), value)
            .map_err(|_| mismatch(name, attr.ty))?;
    }
    Ok(attrs)
}

/// The value of `claim` as `ty`, or `None` when its JSON type does not match. A list with a
/// null or differently typed element does not match (ADR-0025 rule 2).
fn attr_value(claim: &Value, ty: AttrType) -> Option<AttrValue> {
    match ty {
        AttrType::String => string(claim).map(AttrValue::Scalar),
        AttrType::Integer => integer(claim).map(AttrValue::Scalar),
        AttrType::Boolean => claim.as_bool().map(|b| AttrValue::Scalar(Scalar::Bool(b))),
        AttrType::StringList => list(claim, string),
        AttrType::IntegerList => list(claim, integer),
    }
}

fn string(value: &Value) -> Option<Scalar> {
    value.as_str().map(|s| Scalar::String(s.to_owned()))
}

fn integer(value: &Value) -> Option<Scalar> {
    // `as_i64` is None for floats and for integers above i64::MAX.
    value.as_i64().map(Scalar::Int)
}

fn list(value: &Value, element: fn(&Value) -> Option<Scalar>) -> Option<AttrValue> {
    value
        .as_array()?
        .iter()
        .map(element)
        .collect::<Option<Vec<_>>>()
        .map(AttrValue::List)
}

fn mismatch(name: &str, ty: AttrType) -> AuthnError {
    let expected = match ty {
        AttrType::String => "a string",
        AttrType::Integer => "an integer",
        AttrType::Boolean => "a boolean",
        AttrType::StringList => "a list of strings",
        AttrType::IntegerList => "a list of integers",
    };
    AuthnError::InvalidClaims(format!("principal attribute `{name}` must be {expected}"))
}
