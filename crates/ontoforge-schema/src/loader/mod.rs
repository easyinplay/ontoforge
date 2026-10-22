//! Loading an ontology directory: parse, dispatch, merge, validate.

use std::collections::{BTreeMap, HashMap};

use camino::Utf8Path;

use crate::diag::{
    Diagnostic, Severity, SourceLocation, codes, has_errors, parse_custom_de_error,
    sort_diagnostics,
};
use crate::ir::{
    ActionType, ActionTypeName, BindingTarget, Definition, LinkType, LinkTypeName, Manifest,
    ObjectType, ObjectTypeName, Ontology, ProfileBindings, ProfileName, SourceBinding,
    binding_def_name,
};

mod discover;
mod dispatch;
mod fixture;
mod parse;
mod scope;
pub mod sourcemap;

pub use discover::{fixture_path, profile_dir};
use dispatch::{Ctx, dispatch, parse_manifest};
pub use fixture::fixture_header;
pub use scope::{ScopeLoadOutput, load_scope_dir, load_scope_from_memory};
pub use sourcemap::SourceMap;
use sourcemap::{DefKey, DefKind, FileId, FixtureColumns, Part, Seg};

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
    isolated(internal_error, || {
        let manifest = read_text(root, MANIFEST);
        let (candidates, diagnostics) = discover::walk(root);
        let listing = Listing {
            candidates,
            diagnostics,
        };
        let probe = |rel: &str| probe_disk(root, rel);
        assemble(manifest, listing, &|name| read_text(root, name), &probe)
    })
}

/// Same pipeline over in-memory files; paths use `/` (`\` is accepted) and are
/// the diagnostic `file`s.
pub fn load_from_memory(files: &[(&str, &str)]) -> LoadOutput {
    isolated(internal_error, || load_from_memory_inner(files))
}

/// In-memory files by normalized path.
fn memory_files<'a>(files: &[(&str, &'a str)]) -> BTreeMap<String, &'a str> {
    files
        .iter()
        .map(|(p, t)| (discover::normalize(p), *t))
        .collect()
}

/// Reads one in-memory file, applying the same size limit as the disk reader.
fn read_memory(by_path: &BTreeMap<String, &str>, name: &str) -> Text {
    match by_path.get(name) {
        Some(t) => check_size(name, t.len() as u64).map(|()| (*t).to_owned()),
        None if name == MANIFEST => Err(missing_manifest()),
        None => Err(Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!("cannot read `{name}`: not found"),
            name,
        )),
    }
}

fn load_from_memory_inner(files: &[(&str, &str)]) -> LoadOutput {
    let by_path = memory_files(files);
    let read = |name: &str| read_memory(&by_path, name);
    let listing = Listing {
        candidates: by_path.keys().cloned().collect(),
        diagnostics: Vec::new(),
    };
    let probe = |rel: &str| match by_path.get(rel) {
        Some(t) => {
            let len = t.len() as u64;
            if len > MAX_FILE_BYTES {
                Err(too_large(Some(len)))
            } else {
                Ok(len)
            }
        }
        None => Err("not found".to_owned()),
    };
    assemble(read(MANIFEST), listing, &read, &probe)
}

/// Stack size of the loader thread. The recursive-descent parsers need far more
/// than a 1 MiB debug-build stack (a Windows main thread) for nested input.
const LOADER_STACK_BYTES: usize = 32 << 20;

/// Run the whole load on a thread with a large stack, so callers never depend on
/// their own stack size. A panic becomes the `failed` output instead of unwinding
/// into the caller.
fn isolated<'a, T: Send>(failed: fn() -> T, work: impl FnOnce() -> T + Send + 'a) -> T {
    let outcome = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(LOADER_STACK_BYTES)
            .spawn_scoped(scope, work)
            .ok()
            .and_then(|handle| handle.join().ok())
    });
    outcome.unwrap_or_else(failed)
}

fn internal_error_diagnostic(file: &str) -> Diagnostic {
    Diagnostic::without_location(
        codes::V01,
        Severity::Error,
        "internal error while loading",
        file,
    )
}

fn internal_error() -> LoadOutput {
    LoadOutput {
        ontology: None,
        sources: SourceMap::default(),
        diagnostics: vec![internal_error_diagnostic(MANIFEST)],
    }
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
    /// Over the caller's limit; the size when the file system reported it.
    TooLarge(Option<u64>),
}

/// Read a regular file of at most `max_bytes` (CR-03). With `follow_links` false a
/// symbolic link as the last component is refused; a link on a directory on the way is
/// not, so repository content goes through [`read_beneath`], which refuses those too
/// (CR-S01). The read itself is bounded, so a file that grows or lies about its size
/// still cannot exhaust memory.
///
/// Repository content (definitions, binding files, fixtures) is read with
/// [`read_beneath`] and [`MAX_FILE_BYTES`]. Paths the deployer supplies
/// (server configuration, secret files, CA files) may use `true`, because Kubernetes
/// mounts secrets and config maps as symbolic links.
pub fn read_limited(
    path: &std::path::Path,
    follow_links: bool,
    max_bytes: u64,
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
    if meta.len() > max_bytes {
        return Err(ReadFailure::TooLarge(Some(meta.len())));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(max_bytes.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|e| ReadFailure::Io(e.kind()))?;
    if bytes.len() as u64 > max_bytes {
        return Err(ReadFailure::TooLarge(None));
    }
    Ok(bytes)
}

/// Refuse `rel` (`/` separated, relative to `root`) when a symbolic link or junction sits
/// anywhere on the way from `root` to it, or when the resolved path is not below the resolved
/// `root` (CR-S01). `lstat` on the full path only looks at the last component, so a checked-in
/// directory link (a junction on Windows) would otherwise lead a read out of the ontology.
/// A component that does not exist is left to the caller's own open, which reports it.
pub fn check_beneath(root: &Utf8Path, rel: &str) -> Result<(), ReadFailure> {
    let mut at = root.as_std_path().to_path_buf();
    for segment in rel.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if segment == ".." {
            return Err(ReadFailure::NotRegular);
        }
        at.push(segment);
        match std::fs::symlink_metadata(&at) {
            // On Windows this is true for symbolic links and junctions alike.
            Ok(meta) if meta.file_type().is_symlink() => return Err(ReadFailure::NotRegular),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(ReadFailure::Io(e.kind())),
        }
    }
    // Defense in depth: whatever the file system did, the real path stays under the root.
    let resolved = std::fs::canonicalize(&at).map_err(|e| ReadFailure::Io(e.kind()))?;
    let base = std::fs::canonicalize(root.as_std_path()).map_err(|e| ReadFailure::Io(e.kind()))?;
    if resolved.starts_with(&base) {
        Ok(())
    } else {
        Err(ReadFailure::NotRegular)
    }
}

/// [`read_limited`] of repository content at `rel` below `root`, links refused on the whole
/// way there ([`check_beneath`]). The schema loader and the engine's fixture loader both read
/// fixtures through this.
pub fn read_beneath(root: &Utf8Path, rel: &str, max_bytes: u64) -> Result<Vec<u8>, ReadFailure> {
    check_beneath(root, rel)?;
    read_limited(root.join(rel).as_std_path(), false, max_bytes)
}

/// Size of the fixture at `rel` (relative to `root`) from its metadata alone: it must be a
/// regular file inside the ontology directory, with no symbolic link or junction on the way,
/// within [`MAX_FILE_BYTES`] (E23, CR-S01).
fn probe_disk(root: &Utf8Path, rel: &str) -> Result<u64, String> {
    const NOT_REGULAR: &str = "not a regular file inside the ontology directory (symbolic links and junctions are not followed)";
    match check_beneath(root, rel) {
        Ok(()) => {}
        Err(ReadFailure::Io(std::io::ErrorKind::NotFound)) => return Err("not found".to_owned()),
        Err(ReadFailure::Io(kind)) => return Err(format!("cannot be read: {kind}")),
        Err(ReadFailure::NotRegular | ReadFailure::TooLarge(_)) => {
            return Err(NOT_REGULAR.to_owned());
        }
    }
    let meta = std::fs::symlink_metadata(root.join(rel).as_std_path()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "not found".to_owned()
        } else {
            format!("cannot be read: {}", e.kind())
        }
    })?;
    if !meta.file_type().is_file() {
        return Err(NOT_REGULAR.to_owned());
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(too_large(Some(meta.len())));
    }
    Ok(meta.len())
}

fn read_text(root: &Utf8Path, name: &str) -> Text {
    let v14 =
        |message: String| Diagnostic::without_location(codes::V14, Severity::Error, message, name);
    match read_beneath(root, name, MAX_FILE_BYTES) {
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

/// What [`assemble`] asks about a fixture file: its size, or why it cannot be used. Only
/// metadata is looked at; the content is never read here.
type Probe<'a> = &'a dyn Fn(&str) -> Result<u64, String>;

fn assemble(
    manifest: Text,
    listing: Listing,
    read: &dyn Fn(&str) -> Text,
    probe: Probe,
) -> LoadOutput {
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
        default_profile: m.default_profile,
        bindings: defs.bindings,
    });
    if let Some(o) = &ontology {
        diags.extend(crate::validate::validate(o, &sources));
        diags.extend(crate::validate::check_fixture_files(o, &sources, probe));
    }
    sort_diagnostics(&mut diags);
    if has_errors(&diags) {
        ontology = None;
    }
    if let Some(o) = &ontology {
        capture_fixture_headers(o, &mut sources, read, probe, total);
    }
    LoadOutput {
        ontology,
        sources,
        diagnostics: diags,
    }
}

/// Record the header of every fixture the bindings name, for the offline lint rule that
/// compares it with the bound columns (ONT-W17).
///
/// Fixture bytes count against [`MAX_TOTAL_BYTES`] together with the definition files
/// (`total` is what those already used, E23), and a fixture several bindings share is
/// read once. A fixture that is not read, or whose header cannot be parsed, is recorded
/// with the reason instead of being left out, so the rule can say so (E20). Only the
/// header is kept: the text is dropped after it is parsed.
fn capture_fixture_headers(
    ontology: &Ontology,
    sources: &mut SourceMap,
    read: &dyn Fn(&str) -> Text,
    probe: Probe,
    mut total: u64,
) {
    let mut by_path: HashMap<String, FixtureColumns> = HashMap::new();
    for (profile, bound) in &ontology.bindings {
        for binding in bound.objects.values().chain(bound.links.values()) {
            let Some(fixture) = &binding.fixture else {
                continue;
            };
            let def_name = binding_def_name(profile, binding.target_name());
            let key = DefKey {
                kind: DefKind::SourceBinding,
                api_name: def_name.clone(),
            };
            let binding_file = sources.locate(&key, &[], Part::Value).file;
            let columns = match fixture_path(&binding_file, &fixture.file) {
                Ok(resolved) => by_path
                    .entry(resolved.clone())
                    .or_insert_with(|| read_fixture_header(&resolved, read, probe, &mut total))
                    .clone(),
                Err(reason) => Err(reason),
            };
            sources.set_fixture_columns(def_name, columns);
        }
    }
}

/// The header of the fixture at `path`, adding the bytes read to `total`.
fn read_fixture_header(
    path: &str,
    read: &dyn Fn(&str) -> Text,
    probe: Probe,
    total: &mut u64,
) -> FixtureColumns {
    let size = probe(path)?;
    if total.saturating_add(size) > MAX_TOTAL_BYTES {
        return Err(format!(
            "fixture skipped: the ontology byte budget of {MAX_TOTAL_BYTES} bytes is exhausted"
        ));
    }
    let text = read(path).map_err(|d| d.message)?;
    *total += text.len() as u64;
    fixture_header(path, &text)
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
    bindings: BTreeMap<ProfileName, ProfileBindings>,
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
    let path = sources.path(file).to_owned();
    let def = match def {
        Definition::SourceBinding(binding) => {
            merge_binding(sources, file, &path, doc, binding, defs, diags);
            return;
        }
        other => other,
    };
    if path.starts_with(&format!("{}/", discover::BINDINGS_DIR)) {
        let label = match &def {
            Definition::ObjectType(_) => "ObjectType",
            Definition::LinkType(_) => "LinkType",
            Definition::ActionType(_) => "ActionType",
            Definition::SourceBinding(_) => "SourceBinding",
        };
        let at = sources.locate_in(file, &doc, &[Seg::Key("kind".into())], Part::Value);
        diags.push(
            Diagnostic::error(
                codes::V16,
                format!(
                    "a {label} document cannot be placed under `bindings/`, which holds only SourceBinding documents"
                ),
                &at,
            )
            .with_help("move this file out of `bindings/`"),
        );
        return;
    }
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
        // Handled above.
        Definition::SourceBinding(_) => return,
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
        Definition::SourceBinding(_) => {}
    }
}

/// Place a `SourceBinding` document: it must sit under `bindings/<profile>/` with a
/// valid profile name, and each target may be bound once per profile. The profile is
/// taken from the directory and used only as a map key.
fn merge_binding(
    sources: &mut SourceMap,
    file: FileId,
    path: &str,
    doc: parse::Node,
    binding: SourceBinding,
    defs: &mut Defs,
    diags: &mut Vec<Diagnostic>,
) {
    let kind_at = |sources: &SourceMap| {
        sources.locate_in(file, &doc, &[Seg::Key("kind".into())], Part::Value)
    };
    let Some(dir) = discover::profile_dir(path) else {
        diags.push(
            Diagnostic::error(
                codes::V16,
                "a SourceBinding document must be placed under `bindings/<profile>/`",
                &kind_at(sources),
            )
            .with_help("move this file to `bindings/<profile>/`, for example `bindings/dev/`"),
        );
        return;
    };
    let profile = match ProfileName::try_from(dir.to_owned()) {
        Ok(profile) => profile,
        Err(raw) => {
            let (message, help) =
                parse_custom_de_error(&raw).map_or((raw, None), |c| (c.message, c.help));
            let d = Diagnostic::error(
                codes::V16,
                format!("{message} (the profile is the directory name under `bindings/`)"),
                &kind_at(sources),
            );
            diags.push(match help {
                Some(h) => d.with_help(h),
                None => d,
            });
            return;
        }
    };

    let (target_key, target_name, duplicate) = {
        let bound = defs.bindings.entry(profile.clone()).or_default();
        match &binding.target {
            BindingTarget::ObjectType(n) => (
                "objectType",
                n.as_str().to_owned(),
                bound.objects.contains_key(n.as_str()),
            ),
            BindingTarget::LinkType(n) => (
                "linkType",
                n.as_str().to_owned(),
                bound.links.contains_key(n.as_str()),
            ),
        }
    };
    let key = DefKey {
        kind: DefKind::SourceBinding,
        api_name: binding_def_name(&profile, &target_name),
    };
    let name_path = [Seg::Key("target".into()), Seg::Key(target_key.into())];
    if duplicate {
        let first = sources.locate(&key, &name_path, Part::Value);
        let at = sources.locate_in(file, &doc, &name_path, Part::Value);
        diags.push(Diagnostic::error(
            codes::V08,
            format!(
                "duplicate SourceBinding for `{target_name}` in profile `{profile}`; first defined at {}:{}:{}",
                first.file, first.line, first.column
            ),
            &at,
        ));
        return;
    }
    sources.register_def(key, file, doc);
    let bound = defs.bindings.entry(profile).or_default();
    match &binding.target {
        BindingTarget::ObjectType(n) => {
            bound.objects.insert(n.clone(), binding);
        }
        BindingTarget::LinkType(n) => {
            bound.links.insert(n.clone(), binding);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_FILE_BYTES, ReadFailure, load_from_memory, read_limited};

    const MANIFEST: &str = "apiName: demo\n";

    fn object(name: &str) -> String {
        format!(
            "kind: ObjectType\napiName: {name}\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\nbacking: source\n"
        )
    }

    fn binding(name: &str, fixture: &str) -> String {
        format!(
            "kind: SourceBinding\ntarget: {{ objectType: {name} }}\nsource: {{ catalog: demo, schema: s, table: t }}\nfixture: {{ file: {fixture} }}\n"
        )
    }

    #[test]
    fn fixture_header_is_captured() {
        let objects = format!("{}---\n{}", object("Flight"), object("Pilot"));
        let flight = binding("Flight", "fixtures/flights.csv");
        let pilot = binding("Pilot", "fixtures/pilots.json");
        let broken = "kind: SourceBinding\ntarget: { objectType: Seat }\nsource: { catalog: demo, schema: s, table: t }\nfixture: { file: fixtures/seats.json }\n";
        let seat = object("Seat");
        let files = [
            ("ontology.yaml", MANIFEST),
            ("objects.yaml", objects.as_str()),
            ("seats.yaml", seat.as_str()),
            ("bindings/dev/flight.yaml", flight.as_str()),
            (
                "bindings/dev/fixtures/flights.csv",
                "\u{feff}id,flight_no\nF-1,LH1\n",
            ),
            ("bindings/dev/pilot.yaml", pilot.as_str()),
            (
                "bindings/dev/fixtures/pilots.json",
                r#"[{"id": "P-1"}, {"name": "Ann", "id": "P-2"}]"#,
            ),
            ("bindings/dev/seat.yaml", broken),
            ("bindings/dev/fixtures/seats.json", r#"{"id": "S-1"}"#),
        ];
        let out = load_from_memory(&files);
        assert!(out.ontology.is_some(), "{:#?}", out.diagnostics);
        let columns = |name: &str| out.sources.fixture_columns(name).cloned();
        assert_eq!(
            columns("dev/Flight"),
            Some(Ok(vec!["id".to_owned(), "flight_no".to_owned()]))
        );
        assert_eq!(
            columns("dev/Pilot"),
            Some(Ok(vec!["id".to_owned(), "name".to_owned()]))
        );
        let Some(Err(reason)) = columns("dev/Seat") else {
            panic!("a malformed fixture records why it cannot be read");
        };
        assert!(reason.contains("array of objects"), "{reason}");
        // A binding without a fixture has no entry.
        assert_eq!(columns("dev/Nothing"), None);
    }

    /// E23: fixtures are read after the definition files and share their byte budget.
    #[test]
    fn fixture_bytes_count_toward_budget() {
        let (objects, bindings, fixtures) = many_fixture_bindings(17, 17);
        let big = max_size_fixture();
        let out = load_from_memory(&memory_files(&objects, &bindings, &fixtures, &big));
        assert!(out.ontology.is_some(), "{:#?}", out.diagnostics);
        // 64 MiB hold 15 fixtures of 4 MiB next to the small definition files.
        for (i, letter) in letters(17).enumerate() {
            let columns = out.sources.fixture_columns(&format!("dev/Item{letter}"));
            if i < 15 {
                assert_eq!(columns, Some(&Ok(vec!["id".to_owned()])), "Item{letter}");
            } else {
                let Some(Err(reason)) = columns else {
                    panic!("Item{letter} was read past the budget: {columns:?}");
                };
                assert!(reason.contains("byte budget"), "{reason}");
            }
        }
    }

    /// E23: a fixture that several bindings use is read, and counted, once.
    #[test]
    fn shared_fixture_is_counted_once() {
        let (objects, bindings, fixtures) = many_fixture_bindings(17, 15);
        let big = max_size_fixture();
        let out = load_from_memory(&memory_files(&objects, &bindings, &fixtures, &big));
        assert!(out.ontology.is_some(), "{:#?}", out.diagnostics);
        for letter in letters(17) {
            let columns = out.sources.fixture_columns(&format!("dev/Item{letter}"));
            assert_eq!(columns, Some(&Ok(vec!["id".to_owned()])), "Item{letter}");
        }
    }

    fn letters(n: u8) -> impl Iterator<Item = char> {
        (0..n).map(|i| char::from(b'A' + i))
    }

    /// A CSV with a one-column header that is exactly [`MAX_FILE_BYTES`] long.
    fn max_size_fixture() -> String {
        let big = format!(
            "id\n{}",
            "x".repeat(usize::try_from(MAX_FILE_BYTES).unwrap() - 3)
        );
        assert_eq!(big.len() as u64, MAX_FILE_BYTES);
        big
    }

    /// `bindings` object types `ItemA`.. each with a binding; binding `i` uses
    /// fixture `f{i % distinct}.csv`. Returns the object text, the (path, text) of
    /// every binding, and the fixture paths.
    fn many_fixture_bindings(
        bindings: u8,
        distinct: u8,
    ) -> (String, Vec<(String, String)>, Vec<String>) {
        let objects: Vec<String> = letters(bindings)
            .map(|l| object(&format!("Item{l}")))
            .collect();
        let bound: Vec<(String, String)> = letters(bindings)
            .zip(0..)
            .map(|(l, i)| {
                (
                    format!("bindings/dev/item-{l}.yaml"),
                    binding(
                        &format!("Item{l}"),
                        &format!("fixtures/f{}.csv", i % distinct),
                    ),
                )
            })
            .collect();
        let fixtures = (0..distinct)
            .map(|i| format!("bindings/dev/fixtures/f{i}.csv"))
            .collect();
        (objects.join("---\n"), bound, fixtures)
    }

    fn memory_files<'a>(
        objects: &'a str,
        bindings: &'a [(String, String)],
        fixtures: &'a [String],
        fixture_text: &'a str,
    ) -> Vec<(&'a str, &'a str)> {
        let mut files = vec![("ontology.yaml", MANIFEST), ("objects.yaml", objects)];
        files.extend(bindings.iter().map(|(p, t)| (p.as_str(), t.as_str())));
        files.extend(fixtures.iter().map(|p| (p.as_str(), fixture_text)));
        files
    }

    /// E20: bytes that are not UTF-8 are recorded as unreadable, not skipped.
    #[test]
    fn fixture_that_is_not_utf8_is_recorded_as_unreadable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = camino::Utf8Path::from_path(dir.path()).expect("utf-8 temp dir");
        std::fs::create_dir_all(dir.path().join("bindings/dev/fixtures")).unwrap();
        std::fs::write(dir.path().join("ontology.yaml"), MANIFEST).unwrap();
        std::fs::write(dir.path().join("objects.yaml"), object("Flight")).unwrap();
        std::fs::write(
            dir.path().join("bindings/dev/flight.yaml"),
            binding("Flight", "fixtures/flights.csv"),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("bindings/dev/fixtures/flights.csv"),
            [b'i', b'd', b'\n', 0xff, 0xfe],
        )
        .unwrap();
        let out = super::load_dir(root);
        assert!(out.ontology.is_some(), "{:#?}", out.diagnostics);
        let Some(Err(reason)) = out.sources.fixture_columns("dev/Flight") else {
            panic!("an unreadable fixture records why");
        };
        assert!(reason.contains("UTF-8"), "{reason}");
    }

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
