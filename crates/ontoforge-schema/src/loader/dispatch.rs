//! `kind` dispatch and deserialization-error mapping.
//!
//! A document `{ kind: X, .. }` is rewritten to the externally tagged `{ X: { .. } }`
//! before `serde_path_to_error` deserializes it; internally tagged enums lose the
//! error path. Errors are then turned into located `ONT-V` diagnostics.

use serde_json::Value;
use serde_path_to_error::{Path, Segment};

use super::parse::{Kind, Node};
use super::sourcemap::{FileId, Part, Seg, SourceMap, walk};
use crate::diag::{Diagnostic, SourceLocation, codes, did_you_mean, parse_custom_de_error};
use crate::ir::{Definition, Manifest};

/// Kinds a definition document may declare.
const KINDS: [&str; 4] = ["ObjectType", "LinkType", "ActionType", "SourceBinding"];
/// Kinds whose deserialization exists in this build.
const IMPLEMENTED_KINDS: [&str; 4] = ["ObjectType", "LinkType", "ActionType", "SourceBinding"];
/// Rule kinds an action may use; `function` is recognised only to reject it.
const RULE_KINDS: [&str; 5] = [
    "createObject",
    "modifyObject",
    "deleteObject",
    "addLink",
    "removeLink",
];
/// Kinds reserved for later versions.
const RESERVED_KINDS: [&str; 3] = ["Interface", "ValueType", "SharedProperty"];

/// What the dispatcher needs to turn a node path into a source location.
pub struct Ctx<'a> {
    pub file: FileId,
    pub sources: &'a SourceMap,
}

impl Ctx<'_> {
    fn at(&self, root: &Node, path: &[Seg], part: Part) -> SourceLocation {
        self.sources.locate_in(self.file, root, path, part)
    }
}

fn ticked(items: &[&str]) -> String {
    items
        .iter()
        .map(|i| format!("`{i}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse one definition document into its IR.
pub fn dispatch(doc: &Node, cx: &Ctx) -> Result<Definition, Diagnostic> {
    let root = |p: &[Seg]| cx.at(doc, p, Part::Value);
    if !matches!(doc.kind, Kind::Map(_)) {
        return Err(Diagnostic::error(
            codes::V02,
            "a definition document must be a mapping",
            &root(&[]),
        ));
    }
    let Some((_, kind_node)) = doc.get("kind") else {
        return Err(
            Diagnostic::error(codes::V02, "document has no `kind` field", &root(&[]))
                .with_help(format!("add `kind: <{}>`", KINDS.join(" | "))),
        );
    };
    let kind_at = || root(&[Seg::Key("kind".into())]);
    let Kind::Str(kind) = &kind_node.kind else {
        return Err(Diagnostic::error(
            codes::V02,
            "`kind` must be a string",
            &kind_at(),
        ));
    };
    if RESERVED_KINDS.contains(&kind.as_str()) {
        return Err(Diagnostic::error(
            codes::V03,
            format!("kind `{kind}` is reserved and not supported in v1"),
            &kind_at(),
        ));
    }
    if !IMPLEMENTED_KINDS.contains(&kind.as_str()) {
        let d = if KINDS.contains(&kind.as_str()) {
            Diagnostic::error(
                codes::V03,
                format!("kind `{kind}` is not implemented yet"),
                &kind_at(),
            )
        } else {
            let help = did_you_mean(kind, KINDS).map_or_else(
                || format!("expected one of: {}", ticked(&KINDS)),
                |s| format!("did you mean `{s}`?"),
            );
            Diagnostic::error(codes::V03, format!("unknown kind `{kind}`"), &kind_at())
                .with_help(help)
        };
        return Err(d);
    }

    let Value::Object(mut body) = doc.to_value() else {
        return Err(Diagnostic::error(
            codes::V02,
            "a definition document must be a mapping",
            &root(&[]),
        ));
    };
    body.shift_remove("kind");
    if kind == "ActionType" {
        rewrite_rules(&mut body, cx, doc)?;
    }
    let mut wrapped = serde_json::Map::new();
    wrapped.insert(kind.clone(), Value::Object(body));
    serde_path_to_error::deserialize::<_, Definition>(Value::Object(wrapped))
        .map_err(|e| map_error(&e, doc, cx))
}

/// Rewrite each `rules[i]` from `{ kind: x, .. }` to `{ x: { .. } }`, the same
/// transformation the document root gets, so rule errors keep their paths.
fn rewrite_rules(
    body: &mut serde_json::Map<String, Value>,
    cx: &Ctx,
    doc: &Node,
) -> Result<(), Diagnostic> {
    let Some(Value::Array(rules)) = body.get_mut("rules") else {
        return Ok(());
    };
    for (i, rule) in rules.iter_mut().enumerate() {
        let rule_path = [Seg::Key("rules".into()), Seg::Index(i)];
        let at_rule = |field: Option<&str>| {
            let mut path = rule_path.to_vec();
            path.extend(field.map(|f| Seg::Key(f.into())));
            cx.at(doc, &path, Part::Value)
        };
        let Value::Object(fields) = rule else {
            return Err(Diagnostic::error(
                codes::V02,
                "a rule must be a mapping",
                &at_rule(None),
            ));
        };
        let Some(kind) = fields.shift_remove("kind") else {
            return Err(
                Diagnostic::error(codes::V02, "rule has no `kind` field", &at_rule(None))
                    .with_help(format!("add `kind: <{}>`", RULE_KINDS.join(" | "))),
            );
        };
        let Value::String(kind) = kind else {
            return Err(Diagnostic::error(
                codes::V02,
                "`kind` must be a string",
                &at_rule(Some("kind")),
            ));
        };
        if kind == "function" {
            return Err(Diagnostic::error(
                codes::V03,
                "function-backed rules are not supported in v1",
                &at_rule(Some("kind")),
            )
            .with_help(format!("use one of: {}", ticked(&RULE_KINDS))));
        }
        let inner = std::mem::take(fields);
        let mut wrapped = serde_json::Map::new();
        wrapped.insert(kind, Value::Object(inner));
        *rule = Value::Object(wrapped);
    }
    Ok(())
}

/// Parse the manifest document (no `kind`).
pub fn parse_manifest(doc: &Node, cx: &Ctx) -> Result<Manifest, Diagnostic> {
    if !matches!(doc.kind, Kind::Map(_)) {
        let at = cx.at(doc, &[], Part::Value);
        return Err(Diagnostic::error(
            codes::V02,
            "the manifest must be a mapping",
            &at,
        ));
    }
    serde_path_to_error::deserialize::<_, Manifest>(doc.to_value())
        .map_err(|e| map_error(&e, doc, cx))
}

/// Backtick-quoted fragments of a serde message, in order.
fn ticks(msg: &str) -> Vec<&str> {
    msg.split('`').skip(1).step_by(2).collect()
}

/// Convert a `serde_path_to_error` path into original-document segments. An enum
/// segment that names no key of the original node is the rewrite wrapper and is
/// dropped.
fn to_segs(path: &Path, root: &Node) -> Vec<Seg> {
    let mut cur = root;
    let mut out = Vec::new();
    for seg in path {
        let next = match seg {
            Segment::Map { key } => Seg::Key(key.clone()),
            Segment::Seq { index } => Seg::Index(*index),
            Segment::Enum { variant } if cur.get(variant).is_some() => Seg::Key(variant.clone()),
            Segment::Enum { .. } => continue,
            Segment::Unknown => break,
        };
        cur = walk(cur, std::slice::from_ref(&next)).0;
        out.push(next);
    }
    out
}

fn map_error(
    e: &serde_path_to_error::Error<serde_json::Error>,
    root: &Node,
    cx: &Ctx,
) -> Diagnostic {
    let msg = e.inner().to_string();
    let mut path = to_segs(e.path(), root);
    let parts = ticks(&msg);

    if let Some(c) = parse_custom_de_error(&msg) {
        let d = Diagnostic::error(&c.code, c.message, &cx.at(root, &path, Part::Value));
        return match c.help {
            Some(h) => d.with_help(h),
            None => d,
        };
    }

    if msg.starts_with("unknown field ") {
        let name = parts.first().copied().unwrap_or_default();
        let expected = parts.get(1..).unwrap_or_default();
        if path.last() != Some(&Seg::Key(name.to_owned())) {
            path.push(Seg::Key(name.to_owned()));
        }
        let help = match did_you_mean(name, expected.iter().copied()) {
            Some(s) => format!("did you mean `{s}`?"),
            None if expected.is_empty() => "this object takes no fields".to_owned(),
            None => format!("expected one of: {}", ticked(expected)),
        };
        return Diagnostic::error(
            codes::V04,
            format!("unknown field `{name}`"),
            &cx.at(root, &path, Part::Key),
        )
        .with_help(help);
    }

    if msg.starts_with("missing field ") {
        let name = parts.first().copied().unwrap_or_default();
        if path.is_empty() && root.get("kind").is_some() {
            path.push(Seg::Key("kind".into()));
        }
        return Diagnostic::error(
            codes::V05,
            format!("missing required field `{name}`"),
            &cx.at(root, &path, Part::Value),
        );
    }

    if msg.starts_with("unknown variant ") {
        let got = parts.first().copied().unwrap_or_default();
        let expected = parts.get(1..).unwrap_or_default();
        let help = did_you_mean(got, expected.iter().copied()).map_or_else(
            || format!("expected one of: {}", ticked(expected)),
            |s| format!("did you mean `{s}`?"),
        );
        if walk(root, &path).0.get("kind").is_some() {
            path.push(Seg::Key("kind".into()));
            return Diagnostic::error(
                codes::V03,
                format!("unknown kind `{got}`"),
                &cx.at(root, &path, Part::Value),
            )
            .with_help(help);
        }
        return Diagnostic::error(
            codes::V06,
            format!("unknown value `{got}`"),
            &cx.at(root, &path, Part::Value),
        )
        .with_help(help);
    }

    let d = Diagnostic::error(codes::V06, msg.clone(), &cx.at(root, &path, Part::Value));
    if msg.contains("expected a string") {
        d.with_help("quote the value if it is meant to be a string")
    } else {
        d
    }
}
