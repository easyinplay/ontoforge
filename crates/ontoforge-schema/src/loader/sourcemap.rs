//! Source text and span lookup, kept apart from the IR.
//!
//! The IR carries no positions; validation and lint ask the [`SourceMap`] where a
//! definition's field lives, by definition key and path.

use std::collections::HashMap;

use serde_saphyr::Location;

use super::parse::{Kind, Node};
use crate::diag::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DefKind {
    ObjectType,
    LinkType,
    ActionType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefKey {
    pub kind: DefKind,
    pub api_name: String,
}

/// One step of a path through the original document (the `kind` field included,
/// no rewrite wrapper).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Index(usize),
}

/// Whether a location should point at a mapping key or at its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Key,
    Value,
}

#[derive(Debug)]
struct SourceFile {
    path: String,
    text: String,
}

#[derive(Debug)]
struct Entry {
    file: FileId,
    node: Node,
}

#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
    defs: HashMap<DefKey, Entry>,
    manifest: Option<Entry>,
}

impl SourceMap {
    /// Register a file's text (also used for files that hold no definitions).
    pub fn add_file(&mut self, path: &str, text: String) -> FileId {
        self.files.push(SourceFile {
            path: path.to_owned(),
            text,
        });
        FileId(self.files.len() - 1)
    }

    pub fn text(&self, path: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.text.as_str())
    }

    pub(crate) fn register_def(&mut self, key: DefKey, file: FileId, node: Node) {
        self.defs.insert(key, Entry { file, node });
    }

    pub(crate) fn set_manifest(&mut self, file: FileId, node: Node) {
        self.manifest = Some(Entry { file, node });
    }

    /// Where `path` of the definition `def` lives. A path that does not exist
    /// resolves to its nearest existing ancestor.
    pub fn locate(&self, def: &DefKey, path: &[Seg], part: Part) -> SourceLocation {
        match self.defs.get(def) {
            Some(e) => self.locate_in(e.file, &e.node, path, part),
            None => Self::unknown(),
        }
    }

    pub fn locate_manifest(&self, path: &[Seg], part: Part) -> SourceLocation {
        match &self.manifest {
            Some(e) => self.locate_in(e.file, &e.node, path, part),
            None => Self::unknown(),
        }
    }

    /// Resolve `path` inside a node that may not be registered yet.
    pub(crate) fn locate_in(
        &self,
        file: FileId,
        root: &Node,
        path: &[Seg],
        part: Part,
    ) -> SourceLocation {
        self.location(file, resolve(root, path, part))
    }

    pub(crate) fn location(&self, file: FileId, loc: Location) -> SourceLocation {
        let span = loc.span();
        let to_usize = |v: Option<u64>| v.and_then(|v| usize::try_from(v).ok());
        self.raw_location(
            file,
            loc.line(),
            loc.column(),
            to_usize(span.byte_offset()).unwrap_or(0),
            to_usize(span.byte_len()).unwrap_or(1),
        )
    }

    /// Build a location, clamping the span to the file text.
    pub(crate) fn raw_location(
        &self,
        file: FileId,
        line: u64,
        column: u64,
        offset: usize,
        length: usize,
    ) -> SourceLocation {
        let f = &self.files[file.0];
        let offset = offset.min(f.text.len());
        let length = length.clamp(1, f.text.len().saturating_sub(offset).max(1));
        SourceLocation {
            file: f.path.clone(),
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(column).unwrap_or(u32::MAX),
            offset,
            length,
        }
    }

    /// A position-less location in `file`, for problems with a whole file.
    pub(crate) fn whole_file(&self, file: FileId) -> SourceLocation {
        SourceLocation {
            file: self.files[file.0].path.clone(),
            line: 0,
            column: 0,
            offset: 0,
            length: 0,
        }
    }

    fn unknown() -> SourceLocation {
        SourceLocation {
            file: String::new(),
            line: 0,
            column: 0,
            offset: 0,
            length: 0,
        }
    }
}

/// Walk `path` down a node tree. On a missing step the walk stops at the nearest
/// existing ancestor. Also returns the key location of the last step taken.
pub(crate) fn walk<'a>(root: &'a Node, path: &[Seg]) -> (&'a Node, Option<Location>) {
    let mut cur = root;
    let mut key_loc: Option<Location> = None;
    for seg in path {
        match (seg, &cur.kind) {
            (Seg::Key(k), Kind::Map(entries)) => {
                match entries.iter().find(|(ek, _)| &ek.value == k) {
                    Some((ek, v)) => {
                        key_loc = Some(ek.referenced);
                        cur = v;
                    }
                    None => break,
                }
            }
            (Seg::Index(i), Kind::Seq(items)) => match items.get(*i) {
                Some(v) => {
                    key_loc = None;
                    cur = v;
                }
                None => break,
            },
            _ => break,
        }
    }
    (cur, key_loc)
}

pub(crate) fn resolve(root: &Node, path: &[Seg], part: Part) -> Location {
    let (node, key_loc) = walk(root, path);
    match part {
        Part::Key => key_loc.unwrap_or(node.loc),
        Part::Value => node.loc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse::parse_documents;

    fn map_with(text: &str) -> (SourceMap, DefKey) {
        let mut sm = SourceMap::default();
        let id = sm.add_file("a.yaml", text.to_owned());
        let doc = parse_documents(text, false).unwrap().remove(0);
        let key = DefKey {
            kind: DefKind::ObjectType,
            api_name: "Flight".into(),
        };
        sm.register_def(key.clone(), id, doc);
        (sm, key)
    }

    const TEXT: &str = "kind: ObjectType\napiName: Flight\nproperties:\n  id: { type: string }\n  tags:\n    - one\n";

    #[test]
    fn key_and_value_parts_differ() {
        let (sm, k) = map_with(TEXT);
        let path = [Seg::Key("apiName".into())];
        let key = sm.locate(&k, &path, Part::Key);
        let value = sm.locate(&k, &path, Part::Value);
        assert_eq!((key.file.as_str(), key.line, key.column), ("a.yaml", 2, 1));
        assert_eq!((value.line, value.column), (2, 10));
        assert_eq!(&TEXT[value.offset..value.offset + value.length], "Flight");
        assert_eq!(&TEXT[key.offset..key.offset + key.length], "apiName");
    }

    #[test]
    fn nested_keys_and_sequence_indexes_resolve() {
        let (sm, k) = map_with(TEXT);
        let prop = [Seg::Key("properties".into()), Seg::Key("id".into())];
        assert_eq!(sm.locate(&k, &prop, Part::Key).line, 4);
        let item = [
            Seg::Key("properties".into()),
            Seg::Key("tags".into()),
            Seg::Index(0),
        ];
        assert_eq!(sm.locate(&k, &item, Part::Value).line, 6);
    }

    #[test]
    fn missing_path_falls_back_to_the_nearest_ancestor() {
        let (sm, k) = map_with(TEXT);
        let path = [Seg::Key("properties".into()), Seg::Key("nope".into())];
        assert_eq!(sm.locate(&k, &path, Part::Value).line, 4);
        assert_eq!(sm.locate(&k, &path, Part::Key).line, 3);
    }

    #[test]
    fn multibyte_values_report_byte_offsets_and_char_columns() {
        let text = "apiName: 航班\nx: 1\n";
        let (sm, k) = map_with(text);
        let v = sm.locate(&k, &[Seg::Key("apiName".into())], Part::Value);
        assert_eq!(v.column, 10);
        assert_eq!(&text[v.offset..v.offset + v.length], "航班");
    }

    #[test]
    fn text_is_available_by_path() {
        let (sm, _) = map_with(TEXT);
        assert_eq!(sm.text("a.yaml"), Some(TEXT));
        assert_eq!(sm.text("b.yaml"), None);
    }
}
