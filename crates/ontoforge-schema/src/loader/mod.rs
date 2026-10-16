//! Loading an ontology directory: parse, dispatch, merge, validate.

use std::collections::BTreeMap;

use camino::Utf8Path;

use crate::diag::{Diagnostic, Severity, SourceLocation, codes, has_errors, sort_diagnostics};
use crate::ir::{
    ActionType, ActionTypeName, Definition, LinkType, LinkTypeName, Manifest, ObjectType,
    ObjectTypeName, Ontology,
};

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

/// Largest definition file the loader reads.
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// Most definition files one ontology may select (WR-07).
pub const MAX_FILES: usize = 10_000;

/// Most bytes all selected definition files may hold together (WR-07). Every text is
/// kept in the [`SourceMap`] for rendering, so this bounds the loader's memory.
pub const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

/// Load the ontology under `root`: parse, merge and validate (every `ONT-V`
/// check except CEL, which `ontoforge-expr` owns).
pub fn load_dir(root: &Utf8Path) -> LoadOutput {
    isolated(|| {
        let manifest = read_text(root, MANIFEST);
        let (candidates, diagnostics) = discover::walk(root);
        let listing = Listing {
            candidates,
            diagnostics,
        };
        assemble(manifest, listing, &|name| read_text(root, name))
    })
}

/// Same pipeline over in-memory files; paths use `/` (`\` is accepted) and are
/// the diagnostic `file`s.
pub fn load_from_memory(files: &[(&str, &str)]) -> LoadOutput {
    isolated(|| load_from_memory_inner(files))
}

fn load_from_memory_inner(files: &[(&str, &str)]) -> LoadOutput {
    let by_path: BTreeMap<String, &str> = files
        .iter()
        .map(|(p, t)| (discover::normalize(p), *t))
        .collect();
    let read = |name: &str| match by_path.get(name) {
        Some(t) => check_size(name, t.len() as u64).map(|()| (*t).to_owned()),
        None if name == MANIFEST => Err(missing_manifest()),
        None => Err(Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!("cannot read `{name}`: not found"),
            name,
        )),
    };
    let listing = Listing {
        candidates: by_path.keys().cloned().collect(),
        diagnostics: Vec::new(),
    };
    assemble(read(MANIFEST), listing, &read)
}

/// Stack size of the loader thread. The recursive-descent parsers need far more
/// than a 1 MiB debug-build stack (a Windows main thread) for nested input.
const LOADER_STACK_BYTES: usize = 32 << 20;

/// Run the whole load on a thread with a large stack, so callers never depend on
/// their own stack size. A panic becomes a diagnostic instead of unwinding into
/// the caller.
fn isolated<'a>(work: impl FnOnce() -> LoadOutput + Send + 'a) -> LoadOutput {
    let outcome = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(LOADER_STACK_BYTES)
            .spawn_scoped(scope, work)
            .ok()
            .and_then(|handle| handle.join().ok())
    });
    outcome.unwrap_or_else(|| LoadOutput {
        ontology: None,
        sources: SourceMap::default(),
        diagnostics: vec![Diagnostic::without_location(
            codes::V01,
            Severity::Error,
            "internal error while loading",
            MANIFEST,
        )],
    })
}

/// Candidate definition files plus problems found while listing them.
struct Listing {
    candidates: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

fn check_size(name: &str, len: u64) -> Result<(), Diagnostic> {
    if len > MAX_FILE_BYTES {
        return Err(Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!("{}; skipped", too_large(Some(len))),
            name,
        ));
    }
    Ok(())
}

/// "file is N bytes, over the limit", without the size when it is not known.
pub(crate) fn too_large(len: Option<u64>) -> String {
    match len {
        Some(len) => format!("file is {len} bytes, over the {MAX_FILE_BYTES} byte limit"),
        None => format!("file is over the {MAX_FILE_BYTES} byte limit"),
    }
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

/// Why [`read_limited`] returned no bytes.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadFailure {
    Io(std::io::ErrorKind),
    /// A symbolic link (when links are not followed), directory, device or pipe.
    NotRegular,
    /// Over [`MAX_FILE_BYTES`]; the size when the file system reported it.
    TooLarge(Option<u64>),
}

/// Read a regular file of at most [`MAX_FILE_BYTES`] (CR-03). With `follow_links`
/// false a symbolic link is refused, so a checked-in link cannot point the loader at
/// a device or at a file outside the ontology. The read itself is bounded too, so a
/// file that grows or lies about its size still cannot exhaust memory.
pub fn read_limited(
    path: &std::path::Path,
    follow_links: bool,
    _max_bytes: u64,
) -> Result<Vec<u8>, ReadFailure> {
    use std::io::Read;

    let meta = if follow_links {
        std::fs::metadata(path)
    } else {
        std::fs::symlink_metadata(path)
    }
    .map_err(|e| ReadFailure::Io(e.kind()))?;
    if !meta.file_type().is_file() {
        return Err(ReadFailure::NotRegular);
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(ReadFailure::TooLarge(Some(meta.len())));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| ReadFailure::Io(e.kind()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ReadFailure::TooLarge(None));
    }
    Ok(bytes)
}

fn read_text(root: &Utf8Path, name: &str) -> Text {
    let v14 =
        |message: String| Diagnostic::without_location(codes::V14, Severity::Error, message, name);
    match read_limited(root.join(name).as_std_path(), false, MAX_FILE_BYTES) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|_| {
            Diagnostic::without_location(
                codes::V01,
                Severity::Error,
                "file is not valid UTF-8",
                name,
            )
        }),
        Err(ReadFailure::TooLarge(len)) => Err(v14(format!("{}; skipped", too_large(len)))),
        Err(ReadFailure::NotRegular) => Err(v14(format!(
            "`{name}` is not a regular file (symbolic links are not followed); skipped"
        ))),
        Err(ReadFailure::Io(std::io::ErrorKind::NotFound)) if name == MANIFEST => {
            Err(missing_manifest())
        }
        Err(ReadFailure::Io(kind)) => Err(v14(format!("cannot read `{name}`: {kind}"))),
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

fn assemble(manifest: Text, listing: Listing, read: &dyn Fn(&str) -> Text) -> LoadOutput {
    let mut sources = SourceMap::default();
    let mut diags = Vec::new();
    let manifest = load_manifest(&mut sources, manifest, &mut diags);

    let report_empty = listing.diagnostics.is_empty();
    diags.extend(listing.diagnostics);
    let selection = discover::select(
        listing.candidates,
        manifest.as_ref(),
        &sources,
        report_empty,
    );
    diags.extend(selection.diagnostics);

    let mut defs = Defs::default();
    if selection.files.len() > MAX_FILES {
        diags.push(
            Diagnostic::without_location(
                codes::V14,
                Severity::Error,
                format!(
                    "the include patterns select {} files, more than the {MAX_FILES} an ontology may have; only the first {MAX_FILES} were read",
                    selection.files.len()
                ),
                MANIFEST,
            )
            .with_help("narrow `include` in the manifest"),
        );
    }
    let mut total: u64 = 0;
    for path in selection.files.into_iter().take(MAX_FILES) {
        let text = match read(&path) {
            Ok(t) => t,
            Err(d) => {
                diags.push(d);
                continue;
            }
        };
        total += text.len() as u64;
        if total > MAX_TOTAL_BYTES {
            diags.push(Diagnostic::without_location(
                codes::V14,
                Severity::Error,
                format!(
                    "definition files exceed the {MAX_TOTAL_BYTES} byte budget of an ontology; this file and the ones after it were not read"
                ),
                path,
            ));
            break;
        }
        let parsed = parse::parse_documents(&text, discover::is_json_file(&path));
        let file = sources.add_file(&path, text);
        match parsed {
            Err(e) => diags.push(syntax_diagnostic(&sources, file, &e)),
            Ok(docs) => {
                for doc in docs {
                    merge(&mut sources, file, doc, &mut defs, &mut diags);
                }
            }
        }
    }

    let mut ontology = manifest.filter(|_| !has_errors(&diags)).map(|m| Ontology {
        api_name: m.api_name,
        display_name: m.display_name,
        description: m.description,
        object_types: defs.objects,
        link_types: defs.links,
        action_types: defs.actions,
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

/// Definitions collected from all files, keyed by apiName.
#[derive(Default)]
struct Defs {
    objects: BTreeMap<ObjectTypeName, ObjectType>,
    links: BTreeMap<LinkTypeName, LinkType>,
    actions: BTreeMap<ActionTypeName, ActionType>,
}

fn merge(
    sources: &mut SourceMap,
    file: FileId,
    doc: parse::Node,
    defs: &mut Defs,
    diags: &mut Vec<Diagnostic>,
) {
    let def = match dispatch(&doc, &Ctx { file, sources }) {
        Ok(d) => d,
        Err(d) => {
            diags.push(d);
            return;
        }
    };
    let (kind, label, api_name, duplicate) = match &def {
        Definition::ObjectType(o) => (
            DefKind::ObjectType,
            "ObjectType",
            o.api_name.to_string(),
            defs.objects.contains_key(o.api_name.as_str()),
        ),
        Definition::LinkType(l) => (
            DefKind::LinkType,
            "LinkType",
            l.api_name.to_string(),
            defs.links.contains_key(l.api_name.as_str()),
        ),
        Definition::ActionType(a) => (
            DefKind::ActionType,
            "ActionType",
            a.api_name.to_string(),
            defs.actions.contains_key(a.api_name.as_str()),
        ),
    };
    let key = DefKey { kind, api_name };
    let name_path = [Seg::Key("apiName".into())];
    if duplicate {
        let first: SourceLocation = sources.locate(&key, &name_path, Part::Value);
        let at = sources.locate_in(file, &doc, &name_path, Part::Value);
        diags.push(Diagnostic::error(
            codes::V08,
            format!(
                "duplicate {label} `{}`; first defined at {}:{}:{}",
                key.api_name, first.file, first.line, first.column
            ),
            &at,
        ));
        return;
    }
    sources.register_def(key, file, doc);
    match def {
        Definition::ObjectType(o) => {
            defs.objects.insert(o.api_name.clone(), o);
        }
        Definition::LinkType(l) => {
            defs.links.insert(l.api_name.clone(), l);
        }
        Definition::ActionType(a) => {
            defs.actions.insert(a.api_name.clone(), a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReadFailure, read_limited};

    #[test]
    fn read_limited_custom_limit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_limit = dir.path().join("at-limit.txt");
        std::fs::write(&at_limit, vec![b'a'; 65_536]).expect("write");
        let over = dir.path().join("over.txt");
        std::fs::write(&over, vec![b'a'; 65_537]).expect("write");

        assert_eq!(
            read_limited(&at_limit, false, 65_536).map(|bytes| bytes.len()),
            Ok(65_536)
        );
        assert_eq!(
            read_limited(&over, false, 65_536).map(|bytes| bytes.len()),
            Err(ReadFailure::TooLarge(Some(65_537)))
        );
    }

    #[test]
    fn read_limited_missing_and_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            read_limited(&dir.path().join("nope"), false, 16),
            Err(ReadFailure::Io(std::io::ErrorKind::NotFound))
        );
        assert_eq!(
            read_limited(dir.path(), false, 16),
            Err(ReadFailure::NotRegular)
        );
    }

    #[cfg(unix)]
    #[test]
    fn read_limited_follows_links_only_when_asked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target.txt");
        std::fs::write(&target, b"hello").expect("write");
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        assert_eq!(read_limited(&link, false, 16), Err(ReadFailure::NotRegular));
        assert_eq!(read_limited(&link, true, 16), Ok(b"hello".to_vec()));
    }
}
