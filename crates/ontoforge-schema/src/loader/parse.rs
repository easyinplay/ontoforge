//! YAML/JSON text to a span-carrying [`Node`] tree.
//!
//! Every definition file goes through this one module, so replacing the YAML
//! parser only touches this file. Input is always a `&str`: byte offsets are only
//! available for string input.

use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_saphyr::{Budget, DuplicateKeyPolicy, Location, MergeKeyPolicy, Options, Spanned};

/// Maximum structural nesting depth accepted in definition files.
const MAX_DEPTH: usize = 32;

#[derive(Debug)]
pub struct Node {
    pub loc: Location,
    pub kind: Kind,
}

#[derive(Debug)]
pub enum Kind {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Node>),
    Map(Vec<(Spanned<String>, Node)>),
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = Spanned::<Kind>::deserialize(d)?;
        Ok(Node {
            loc: s.referenced,
            kind: s.value,
        })
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Kind;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a YAML or JSON value")
            }
            fn visit_unit<E>(self) -> Result<Kind, E> {
                Ok(Kind::Null)
            }
            fn visit_bool<E>(self, v: bool) -> Result<Kind, E> {
                Ok(Kind::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Kind, E> {
                Ok(Kind::Int(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Kind, E> {
                i64::try_from(v).map(Kind::Int).map_err(E::custom)
            }
            fn visit_f64<E>(self, v: f64) -> Result<Kind, E> {
                Ok(Kind::Float(v))
            }
            fn visit_str<E>(self, v: &str) -> Result<Kind, E> {
                Ok(Kind::Str(v.to_owned()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Kind, A::Error> {
                let mut items = Vec::new();
                while let Some(n) = a.next_element()? {
                    items.push(n);
                }
                Ok(Kind::Seq(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Kind, A::Error> {
                let mut entries = Vec::new();
                while let Some(k) = a.next_key()? {
                    entries.push((k, a.next_value()?));
                }
                Ok(Kind::Map(entries))
            }
        }
        d.deserialize_any(V)
    }
}

impl Node {
    /// Convert to a `serde_json::Value` (map order preserved).
    pub fn to_value(&self) -> serde_json::Value {
        use serde_json::Value as J;
        match &self.kind {
            Kind::Null => J::Null,
            Kind::Bool(b) => J::Bool(*b),
            Kind::Int(i) => J::from(*i),
            Kind::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
            Kind::Str(s) => J::String(s.clone()),
            Kind::Seq(items) => J::Array(items.iter().map(Node::to_value).collect()),
            Kind::Map(entries) => J::Object(
                entries
                    .iter()
                    .map(|(k, v)| (k.value.clone(), v.to_value()))
                    .collect(),
            ),
        }
    }

    /// The entry named `key` when this node is a mapping.
    pub fn get(&self, key: &str) -> Option<(&Spanned<String>, &Node)> {
        match &self.kind {
            Kind::Map(entries) => entries
                .iter()
                .find(|(k, _)| k.value == key)
                .map(|(k, v)| (k, v)),
            _ => None,
        }
    }
}

/// A syntax-level failure with the numbers needed to build a location later.
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: u64,
    pub column: u64,
    pub offset: usize,
    pub length: usize,
}

fn options() -> Options {
    let mut o = Options::default();
    o.strict_booleans = true;
    o.duplicate_keys = DuplicateKeyPolicy::Error;
    o.merge_keys = MergeKeyPolicy::Error;
    o.reject_unsupported_tags = true;
    o.with_snippet = false;
    let mut budget = Budget::default();
    budget.max_depth = MAX_DEPTH;
    o.budget = Some(budget);
    o
}

/// Parse all documents of a file. A `.json` file is first checked with a strict JSON
/// parser (the YAML parser accepts trailing commas and comments), and a top-level
/// array is flattened into one document per element.
pub fn parse_documents(text: &str, json: bool) -> Result<Vec<Node>, ParseError> {
    if json {
        precheck_json(text)?;
    }
    let mut docs: Vec<Node> =
        serde_saphyr::from_multiple_with_options(text, options()).map_err(|e| {
            let loc = e.location();
            let span = loc.map(|l| l.span());
            ParseError {
                message: clean_message(&e.to_string()),
                line: loc.map_or(0, |l| l.line()),
                column: loc.map_or(0, |l| l.column()),
                offset: span
                    .and_then(|s| s.byte_offset())
                    .and_then(|o| usize::try_from(o).ok())
                    .unwrap_or(0),
                length: span
                    .and_then(|s| s.byte_len())
                    .and_then(|o| usize::try_from(o).ok())
                    .unwrap_or(1),
            }
        })?;
    if json
        && docs.len() == 1
        && matches!(docs[0].kind, Kind::Seq(_))
        && let Kind::Seq(items) = docs.remove(0).kind
    {
        docs = items;
    }
    Ok(docs)
}

fn precheck_json(text: &str) -> Result<(), ParseError> {
    let body = text.trim_start_matches('\u{feff}');
    serde_json::from_str::<de::IgnoredAny>(body)
        .map(|_| ())
        .map_err(|e| {
            let bom = text.len() - body.len();
            ParseError {
                message: e.to_string(),
                line: u64::try_from(e.line()).unwrap_or(0),
                column: u64::try_from(e.column()).unwrap_or(0),
                offset: bom + offset_of(body, e.line(), e.column()),
                length: 1,
            }
        })
}

/// Byte offset of a 1-based line and byte column.
fn offset_of(text: &str, line: usize, column: usize) -> usize {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum();
    (line_start + column.saturating_sub(1)).min(text.len())
}

/// Drop the trailing ` at line L, column C` the parser appends; the location is
/// reported separately.
fn clean_message(raw: &str) -> String {
    let first_line = raw.lines().next().unwrap_or(raw);
    match first_line.rfind(" at line ") {
        Some(i) => first_line[..i].to_owned(),
        None => first_line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_document_yaml_yields_one_node_each() {
        let docs = parse_documents("a: 1\n---\nb: 2\n", false).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn json_array_is_flattened() {
        let docs = parse_documents("[{\"a\":1},{\"b\":2}]", true).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn json_trailing_comma_reports_line_and_offset() {
        let e = parse_documents("{\"a\":1,\n}\n", true).unwrap_err();
        assert_eq!(e.line, 2);
        assert_eq!(e.offset, 8, "start of line 2");
    }

    #[test]
    fn yaml_1_1_booleans_stay_strings() {
        let docs = parse_documents("a: yes\n", false).unwrap();
        assert_eq!(docs[0].to_value()["a"], "yes");
    }

    #[test]
    fn duplicate_keys_are_an_error_without_the_position_suffix() {
        let e = parse_documents("a: 1\na: 2\n", false).unwrap_err();
        assert!(!e.message.contains(" at line "), "{}", e.message);
        assert_eq!(e.line, 2);
    }

    #[test]
    fn nesting_beyond_the_budget_is_an_error() {
        let mut text = String::new();
        for depth in 0..100 {
            text.push_str(&"  ".repeat(depth));
            text.push_str("a:\n");
        }
        assert!(parse_documents(&text, false).is_err());
    }
}
