//! Loading an ontology directory: parse, dispatch, merge, validate.

use std::collections::BTreeMap;

use camino::Utf8Path;

use crate::diag::{Diagnostic, Severity, SourceLocation, codes, has_errors, sort_diagnostics};
use crate::ir::{Definition, Manifest, ObjectType, ObjectTypeName, Ontology};

mod discover;
mod dispatch;
mod parse;
pub mod sourcemap;

use dispatch::{Ctx, dispatch, parse_manifest};
pub use sourcemap::SourceMap;
use sourcemap::{DefKey, DefKind, FileId, Part, Seg};

/// File name of the manifest at the ontology root.
pub const MANIFEST: &str = "ontology.yaml";

#[derive(Debug)]
pub struct LoadOutput {
    /// `Some` only when no error was found.
    pub ontology: Option<Ontology>,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

type Text = Result<String, Diagnostic>;

/// Load the ontology under `root`: parse, merge and validate (every `ONT-V`
/// check except CEL, which `ontoforge-expr` owns).
pub fn load_dir(root: &Utf8Path) -> LoadOutput {
    let manifest = read_text(root, MANIFEST);
    let files = match discover::discover(root) {
        Ok(names) => names
            .into_iter()
            .map(|n| {
                let text = read_text(root, &n);
                (n, text)
            })
            .collect(),
        Err(e) => {
            let mut out = assemble(manifest, Vec::new());
            out.diagnostics.push(Diagnostic::without_location(
                codes::V14,
                Severity::Error,
                format!("cannot list the ontology directory: {}", e.kind()),
                MANIFEST,
            ));
            return out;
        }
    };
    assemble(manifest, files)
}

/// Same pipeline over in-memory files; paths use `/` and are the diagnostic `file`s.
pub fn load_from_memory(files: &[(&str, &str)]) -> LoadOutput {
    let manifest = files
        .iter()
        .find(|(p, _)| *p == MANIFEST)
        .map_or_else(|| Err(missing_manifest()), |(_, t)| Ok((*t).to_owned()));
    let mut rest: Vec<(String, Text)> = files
        .iter()
        .filter(|(p, _)| *p != MANIFEST && discover::is_definition_file(p))
        .map(|(p, t)| ((*p).to_owned(), Ok((*t).to_owned())))
        .collect();
    rest.sort_by(|a, b| a.0.cmp(&b.0));
    assemble(manifest, rest)
}

fn missing_manifest() -> Diagnostic {
    Diagnostic::without_location(
        codes::V14,
        Severity::Error,
        format!("manifest `{MANIFEST}` not found in the ontology directory"),
        MANIFEST,
    )
    .with_help(format!(
        "create `{MANIFEST}` with at least `apiName: <camelCaseName>`"
    ))
}

fn read_text(root: &Utf8Path, name: &str) -> Text {
    match std::fs::read(root.join(name)) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|_| {
            Diagnostic::without_location(
                codes::V01,
                Severity::Error,
                "file is not valid UTF-8",
                name,
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && name == MANIFEST => {
            Err(missing_manifest())
        }
        Err(e) => Err(Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!("cannot read `{name}`: {}", e.kind()),
            name,
        )),
    }
}

fn syntax_diagnostic(sources: &SourceMap, file: FileId, e: &parse::ParseError) -> Diagnostic {
    let at = if e.line == 0 {
        sources.whole_file(file)
    } else {
        sources.raw_location(file, e.line, e.column, e.offset, e.length)
    };
    Diagnostic::error(codes::V01, e.message.clone(), &at)
}

fn assemble(manifest: Text, files: Vec<(String, Text)>) -> LoadOutput {
    let mut sources = SourceMap::default();
    let mut diags = Vec::new();
    let manifest = load_manifest(&mut sources, manifest, &mut diags);

    let mut object_types: BTreeMap<ObjectTypeName, ObjectType> = BTreeMap::new();
    for (path, text) in files {
        let text = match text {
            Ok(t) => t,
            Err(d) => {
                diags.push(d);
                continue;
            }
        };
        let parsed = parse::parse_documents(&text, discover::is_json_file(&path));
        let file = sources.add_file(&path, text);
        match parsed {
            Err(e) => diags.push(syntax_diagnostic(&sources, file, &e)),
            Ok(docs) => {
                for doc in docs {
                    merge(&mut sources, file, doc, &mut object_types, &mut diags);
                }
            }
        }
    }

    let mut ontology = manifest.filter(|_| !has_errors(&diags)).map(|m| Ontology {
        api_name: m.api_name,
        display_name: m.display_name,
        description: m.description,
        object_types,
    });
    if let Some(o) = &ontology {
        diags.extend(crate::validate::validate(o, &sources));
    }
    sort_diagnostics(&mut diags);
    if has_errors(&diags) {
        ontology = None;
    }
    LoadOutput {
        ontology,
        sources,
        diagnostics: diags,
    }
}

fn load_manifest(
    sources: &mut SourceMap,
    text: Text,
    diags: &mut Vec<Diagnostic>,
) -> Option<Manifest> {
    let text = match text {
        Ok(t) => t,
        Err(d) => {
            diags.push(d);
            return None;
        }
    };
    let parsed = parse::parse_documents(&text, false);
    let file = sources.add_file(MANIFEST, text);
    let mut docs = match parsed {
        Ok(d) => d,
        Err(e) => {
            diags.push(syntax_diagnostic(sources, file, &e));
            return None;
        }
    };
    if docs.is_empty() {
        diags.push(Diagnostic::error(
            codes::V02,
            "the manifest is empty",
            &sources.whole_file(file),
        ));
        return None;
    }
    if docs.len() > 1 {
        let at = sources.locate_in(file, &docs[1], &[], Part::Value);
        diags.push(Diagnostic::error(
            codes::V02,
            "the manifest must be a single document",
            &at,
        ));
        return None;
    }
    let doc = docs.remove(0);
    let result = parse_manifest(&doc, &Ctx { file, sources });
    sources.set_manifest(file, doc);
    match result {
        Ok(m) => Some(m),
        Err(d) => {
            diags.push(d);
            None
        }
    }
}

fn merge(
    sources: &mut SourceMap,
    file: FileId,
    doc: parse::Node,
    object_types: &mut BTreeMap<ObjectTypeName, ObjectType>,
    diags: &mut Vec<Diagnostic>,
) {
    let def = dispatch(&doc, &Ctx { file, sources });
    let Definition::ObjectType(object) = match def {
        Ok(d) => d,
        Err(d) => {
            diags.push(d);
            return;
        }
    };
    let key = DefKey {
        kind: DefKind::ObjectType,
        api_name: object.api_name.to_string(),
    };
    let name_path = [Seg::Key("apiName".into())];
    if object_types.contains_key(object.api_name.as_str()) {
        let first: SourceLocation = sources.locate(&key, &name_path, Part::Value);
        let at = sources.locate_in(file, &doc, &name_path, Part::Value);
        diags.push(Diagnostic::error(
            codes::V08,
            format!(
                "duplicate ObjectType `{}`; first defined at {}:{}:{}",
                object.api_name, first.file, first.line, first.column
            ),
            &at,
        ));
        return;
    }
    sources.register_def(key, file, doc);
    object_types.insert(object.api_name.clone(), object);
}
