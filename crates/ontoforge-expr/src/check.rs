//! Static checks of submission criteria (ONT-V13, D-09).
//!
//! `cel` has no declarative variable typing, so the checks are ours: every free
//! variable must be `params`, `user` or `objects` and resolve against the action's
//! parameters and the ontology. All AST recursion happens inside the
//! [`analyze`](crate::analyze()) closure; this module only handles flat result lists.

use std::collections::HashSet;

use ontoforge_schema::diag::{Diagnostic, SourceLocation, codes, did_you_mean};
use ontoforge_schema::ir::{ActionType, Ontology, ParamType, ParameterName, PropertyName};
use ontoforge_schema::loader::SourceMap;
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part, Seg};

use crate::infer::{TypeEnv, type_issues};
use crate::walk::{MemberPath, member_paths};
use crate::{ExprError, analyze};

const ROOTS: &str = "`params`, `user` and `objects`";
const USER_FIELDS: [&str; 2] = ["id", "groups"];

/// Checks every submission criterion of every action type; each problem is an
/// `ONT-V13` error located at the criterion's `condition` value.
#[must_use]
pub fn check_ontology(ontology: &Ontology, sources: &SourceMap) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (name, action) in &ontology.action_types {
        let def = DefKey {
            kind: DefKind::ActionType,
            api_name: name.as_str().to_owned(),
        };
        let env = TypeEnv::for_action(action, ontology);
        for (i, criterion) in action.submission_criteria.iter().enumerate() {
            let path = [
                Seg::Key("submissionCriteria".into()),
                Seg::Index(i),
                Seg::Key("condition".into()),
            ];
            let at = sources.locate(&def, &path, Part::Value);
            out.extend(
                check_condition(&criterion.condition, ontology, action, &env)
                    .into_iter()
                    .map(|p| p.into_diagnostic(&at)),
            );
        }
    }
    out
}

/// One finding about a condition, positioned inside the expression text.
struct Problem {
    message: String,
    help: Option<String>,
    /// 1-based `(line, column)` inside the expression.
    pos: Option<(usize, usize)>,
}

impl Problem {
    fn new(message: String, help: impl Into<Option<String>>, pos: Option<(usize, usize)>) -> Self {
        Self {
            message,
            help: help.into(),
            pos,
        }
    }

    fn into_diagnostic(self, at: &SourceLocation) -> Diagnostic {
        let message = match self.pos {
            Some((line, column)) => {
                format!("{} (at {line}:{column} in expression)", self.message)
            }
            None => self.message,
        };
        let diagnostic = Diagnostic::error(codes::V13, message, at);
        match self.help {
            Some(help) => diagnostic.with_help(help),
            None => diagnostic,
        }
    }
}

fn check_condition(
    src: &str,
    ontology: &Ontology,
    action: &ActionType,
    env: &TypeEnv,
) -> Vec<Problem> {
    // Both walks recurse per AST level, so they run inside the analyzer thread.
    let analyzed = analyze(src, |program| {
        (member_paths(program, src), type_issues(program, src, env))
    });
    match analyzed {
        Ok((paths, issues)) => {
            let mut problems = reference_problems(&paths, ontology, action);
            problems.extend(
                issues
                    .into_iter()
                    .map(|i| Problem::new(i.message, None, i.pos)),
            );
            problems
        }
        Err(error) => vec![analyzer_problem(&error)],
    }
}

fn analyzer_problem(error: &ExprError) -> Problem {
    match error {
        ExprError::Syntax {
            line,
            column,
            message,
        } => Problem::new(
            format!("invalid CEL expression: {message}"),
            None,
            Some((*line, *column)),
        ),
        other => Problem::new(format!("invalid CEL expression: {other}"), None, None),
    }
}

/// One problem per distinct message, at its first occurrence.
fn reference_problems(
    paths: &[MemberPath],
    ontology: &Ontology,
    action: &ActionType,
) -> Vec<Problem> {
    let mut seen = HashSet::new();
    paths
        .iter()
        .filter_map(|path| path_problem(path, ontology, action))
        .filter(|problem| seen.insert(problem.message.clone()))
        .collect()
}

fn path_problem(path: &MemberPath, ontology: &Ontology, action: &ActionType) -> Option<Problem> {
    let [root, rest @ ..] = path.segments.as_slice() else {
        return None;
    };
    let pos = path.pos;
    match (root.as_str(), rest) {
        ("params", [name, ..]) => {
            if action.parameters.contains_key(name.as_str()) {
                return None;
            }
            let declared: Vec<&str> = action
                .parameters
                .keys()
                .map(ParameterName::as_str)
                .collect();
            Some(Problem::new(
                format!("unknown parameter `{name}` in `params.{name}`"),
                suggest(name, &declared, "declared parameters"),
                pos,
            ))
        }
        ("objects", [name, tail @ ..]) => object_problem(name, tail.first(), ontology, action, pos),
        ("user", [field, ..]) => {
            if USER_FIELDS.contains(&field.as_str()) {
                return None;
            }
            Some(Problem::new(
                format!("unknown user field `{field}`"),
                suggest(field, &USER_FIELDS, "user fields"),
                pos,
            ))
        }
        ("params" | "objects" | "user", []) => None,
        (other, _) => Some(Problem::new(
            format!("unknown variable `{other}`"),
            format!("expressions can only use {ROOTS}"),
            pos,
        )),
    }
}

/// `objects.<name>` and `objects.<name>.<property>`.
fn object_problem(
    name: &str,
    property: Option<&String>,
    ontology: &Ontology,
    action: &ActionType,
    pos: Option<(usize, usize)>,
) -> Option<Problem> {
    let references: Vec<&str> = action
        .parameters
        .iter()
        .filter(|(_, def)| matches!(def.param_type, ParamType::ObjectReference(_)))
        .map(|(key, _)| key.as_str())
        .collect();
    let Some(def) = action.parameters.get(name) else {
        return Some(Problem::new(
            format!("unknown parameter `{name}` in `objects.{name}`"),
            suggest(name, &references, "objectReference parameters"),
            pos,
        ));
    };
    let ParamType::ObjectReference(target) = &def.param_type else {
        return Some(Problem::new(
            format!("`objects.{name}` needs an objectReference parameter, but `{name}` is not one"),
            suggest(name, &references, "objectReference parameters"),
            pos,
        ));
    };
    let property = property?;
    // A missing target type is reported by validate (V09) before this runs.
    let object_type = ontology.object_types.get(target.as_str())?;
    if object_type.properties.contains_key(property.as_str()) {
        return None;
    }
    let known: Vec<&str> = object_type
        .properties
        .keys()
        .map(PropertyName::as_str)
        .collect();
    Some(Problem::new(
        format!(
            "object type `{target}` has no property `{property}` in `objects.{name}.{property}`"
        ),
        suggest(property, &known, &format!("properties of `{target}`")),
        pos,
    ))
}

/// A "did you mean" hint, or the list of valid names when nothing is close.
fn suggest(name: &str, candidates: &[&str], what: &str) -> String {
    match did_you_mean(name, candidates.iter().copied()) {
        Some(close) => format!("did you mean `{close}`?"),
        None if candidates.is_empty() => format!("there are no {what}"),
        None => format!("{what}: {}", candidates.join(", ")),
    }
}
