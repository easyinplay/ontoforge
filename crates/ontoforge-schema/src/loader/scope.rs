//! Loading a scope directory: roles and role bindings (D-01).
//!
//! The pipeline is the ontology loader's: size and file-count budgets, path safety,
//! the large-stack thread, the span-carrying parser and `kind` dispatch, so a scope
//! error carries the same file, line and column as an ontology error. A scope
//! directory has no manifest. Checks that need the ontology (does the type, property
//! or action exist) are not made here.

use std::collections::HashSet;

use camino::Utf8Path;

use super::dispatch::{Ctx, dispatch_scope};
use super::parse::{self, Node};
use super::sourcemap::{DefKey, DefKind, FileId, Part, Seg};
use super::{
    Listing, MAX_FILES, MAX_TOTAL_BYTES, SourceMap, Text, discover, internal_error_diagnostic,
    isolated, memory_files, read_memory, read_text,
};
use crate::diag::{Diagnostic, Severity, codes, did_you_mean, has_errors, sort_diagnostics};
use crate::ir::{Role, RoleBinding, RoleName, ScopeDefinition, ScopeSet, Subject};

#[derive(Debug)]
pub struct ScopeLoadOutput {
    /// `Some` only when no error was found.
    pub scope: Option<ScopeSet>,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

/// Load every YAML and JSON file below `root` as scope documents.
pub fn load_scope_dir(root: &Utf8Path) -> ScopeLoadOutput {
    isolated(internal_error, || {
        let (candidates, diagnostics) = discover::walk(root);
        let listing = Listing {
            candidates,
            diagnostics,
        };
        assemble_scope(listing, &|name| read_text(root, name))
    })
}

/// Same pipeline over in-memory files; paths use `/` (`\` is accepted) and are the
/// diagnostic `file`s.
pub fn load_scope_from_memory(files: &[(&str, &str)]) -> ScopeLoadOutput {
    isolated(internal_error, || {
        let by_path = memory_files(files);
        let listing = Listing {
            candidates: by_path.keys().cloned().collect(),
            diagnostics: Vec::new(),
        };
        assemble_scope(listing, &|name| read_memory(&by_path, name))
    })
}

fn internal_error() -> ScopeLoadOutput {
    ScopeLoadOutput {
        scope: None,
        sources: SourceMap::default(),
        diagnostics: vec![internal_error_diagnostic(".")],
    }
}

fn assemble_scope(listing: Listing, read: &dyn Fn(&str) -> Text) -> ScopeLoadOutput {
    let mut sources = SourceMap::default();
    let mut diags = listing.diagnostics;
    let files = discover::select_scope(listing.candidates);

    if files.is_empty() && diags.is_empty() {
        diags.push(
            Diagnostic::without_location(
                codes::V14,
                Severity::Error,
                "no scope documents found",
                ".",
            )
            .with_help("add `.yaml`, `.yml` or `.json` files holding `kind: Role` and `kind: RoleBinding` documents"),
        );
    }
    if files.len() > MAX_FILES {
        diags.push(Diagnostic::without_location(
            codes::V14,
            Severity::Error,
            format!(
                "the scope directory holds {} files, more than the {MAX_FILES} it may have; only the first {MAX_FILES} were read",
                files.len()
            ),
            ".",
        ));
    }

    let mut scope = ScopeSet::default();
    let mut total: u64 = 0;
    for path in files.into_iter().take(MAX_FILES) {
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
                    "scope files exceed the {MAX_TOTAL_BYTES} byte budget; this file and the ones after it were not read"
                ),
                path,
            ));
            break;
        }
        let parsed = parse::parse_documents(&text, discover::is_json_file(&path));
        let file = sources.add_file(&path, text);
        match parsed {
            Err(e) => diags.push(super::syntax_diagnostic(&sources, file, &e)),
            Ok(docs) => {
                for doc in docs {
                    merge_scope(&mut sources, file, doc, &mut scope, &mut diags);
                }
            }
        }
    }

    check_bound_roles_exist(&sources, &scope, &mut diags);
    sort_diagnostics(&mut diags);
    let scope = (!has_errors(&diags)).then_some(scope);
    ScopeLoadOutput {
        scope,
        sources,
        diagnostics: diags,
    }
}

fn merge_scope(
    sources: &mut SourceMap,
    file: FileId,
    doc: Node,
    scope: &mut ScopeSet,
    diags: &mut Vec<Diagnostic>,
) {
    let def = match dispatch_scope(&doc, &Ctx { file, sources }) {
        Ok(d) => d,
        Err(d) => {
            diags.push(d);
            return;
        }
    };
    match def {
        ScopeDefinition::Role(role) => merge_role(sources, file, doc, role, scope, diags),
        ScopeDefinition::RoleBinding(binding) => {
            merge_binding(sources, file, doc, binding, scope, diags);
        }
    }
}

fn merge_role(
    sources: &mut SourceMap,
    file: FileId,
    doc: Node,
    role: Role,
    scope: &mut ScopeSet,
    diags: &mut Vec<Diagnostic>,
) {
    let key = DefKey {
        kind: DefKind::Role,
        api_name: role.api_name.to_string(),
    };
    let name_path = [Seg::Key("apiName".into())];
    if scope.roles.contains_key(&role.api_name) {
        let first = sources.locate(&key, &name_path, Part::Value);
        let at = sources.locate_in(file, &doc, &name_path, Part::Value);
        diags.push(Diagnostic::error(
            codes::V08,
            format!(
                "duplicate Role `{}`; first defined at {}:{}:{}",
                key.api_name, first.file, first.line, first.column
            ),
            &at,
        ));
        return;
    }
    sources.register_def(key, file, doc);
    scope.roles.insert(role.api_name.clone(), role);
}

fn merge_binding(
    sources: &mut SourceMap,
    file: FileId,
    doc: Node,
    binding: RoleBinding,
    scope: &mut ScopeSet,
    diags: &mut Vec<Diagnostic>,
) {
    diags.extend(subject_errors(sources, file, &doc, &binding));
    let key = DefKey {
        kind: DefKind::RoleBinding,
        api_name: binding.role.to_string(),
    };
    let role_path = [Seg::Key("role".into())];
    if scope.bindings.contains_key(&binding.role) {
        let first = sources.locate(&key, &role_path, Part::Value);
        let at = sources.locate_in(file, &doc, &role_path, Part::Value);
        diags.push(
            Diagnostic::error(
                codes::V08,
                format!(
                    "a second RoleBinding for role `{}`; first defined at {}:{}:{}",
                    key.api_name, first.file, first.line, first.column
                ),
                &at,
            )
            .with_help("list all subjects of a role in one RoleBinding"),
        );
        return;
    }
    sources.register_def(key, file, doc);
    scope.bindings.insert(binding.role.clone(), binding);
}

/// Structure rules for the subjects of one binding (D-02, D-04): at least one, no
/// repeats, a user is `<iss>|<sub>`, a group has a name.
fn subject_errors(
    sources: &SourceMap,
    file: FileId,
    doc: &Node,
    binding: &RoleBinding,
) -> Vec<Diagnostic> {
    let at = |path: &[Seg]| sources.locate_in(file, doc, path, Part::Value);
    let subjects = || Seg::Key("subjects".into());
    if binding.subjects.is_empty() {
        return vec![
            Diagnostic::error(
                codes::V21,
                format!("RoleBinding for `{}` has no subjects", binding.role),
                &at(&[subjects()]),
            )
            .with_help("list at least one `user: \"<iss>|<sub>\"` or `group: <name>`"),
        ];
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (i, subject) in binding.subjects.iter().enumerate() {
        let key = match subject {
            Subject::User(_) => "user",
            Subject::Group(_) => "group",
        };
        if let Some((message, help)) = subject_problem(subject) {
            let path = [subjects(), Seg::Index(i), Seg::Key(key.into())];
            out.push(Diagnostic::error(codes::V21, message, &at(&path)).with_help(help));
        } else if !seen.insert(subject) {
            let path = [subjects(), Seg::Index(i)];
            out.push(
                Diagnostic::error(
                    codes::V21,
                    format!(
                        "duplicate subject in the RoleBinding for `{}`",
                        binding.role
                    ),
                    &at(&path),
                )
                .with_help("remove the repeated entry"),
            );
        }
    }
    out
}

/// What is wrong with a single subject, as a message and a help text.
fn subject_problem(subject: &Subject) -> Option<(String, &'static str)> {
    match subject {
        Subject::User(user) => {
            if user.chars().any(char::is_control) {
                return Some((
                    "a user subject contains a control character".to_owned(),
                    "a user is a principal id such as `ontoforge-dev|mia`",
                ));
            }
            let mut parts = user.split('|');
            let well_formed = matches!(
                (parts.next(), parts.next(), parts.next()),
                (Some(iss), Some(sub), None) if !iss.is_empty() && !sub.is_empty()
            );
            (!well_formed).then(|| {
                (
                    format!("user `{user}` is not a principal id `<iss>|<sub>`"),
                    "write the issuer and the subject separated by one `|`, for example `ontoforge-dev|mia`",
                )
            })
        }
        Subject::Group(group) => {
            if group.trim().is_empty() {
                Some((
                    "a group subject has an empty name".to_owned(),
                    "name the group as it appears in the token's `groups` claim",
                ))
            } else if group.chars().any(char::is_control) {
                Some((
                    "a group subject contains a control character".to_owned(),
                    "name the group as it appears in the token's `groups` claim",
                ))
            } else {
                None
            }
        }
    }
}

/// A binding must name a role defined somewhere in the directory. Checked after all
/// files are read, because the role may sit in a later file.
fn check_bound_roles_exist(sources: &SourceMap, scope: &ScopeSet, diags: &mut Vec<Diagnostic>) {
    for role in scope.bindings.keys() {
        if scope.roles.contains_key(role) {
            continue;
        }
        let key = DefKey {
            kind: DefKind::RoleBinding,
            api_name: role.to_string(),
        };
        let at = sources.locate(&key, &[Seg::Key("role".into())], Part::Value);
        let help = did_you_mean(role.as_str(), scope.roles.keys().map(RoleName::as_str))
            .map_or_else(
                || format!("define a `kind: Role` with `apiName: {role}`"),
                |s| format!("did you mean `{s}`?"),
            );
        diags.push(
            Diagnostic::error(
                codes::V21,
                format!("RoleBinding refers to role `{role}`, which is not defined"),
                &at,
            )
            .with_help(help),
        );
    }
}
