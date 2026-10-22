//! Criteria column coverage (ONT-V23, SCOPE-06).
//!
//! A submission criterion that reads `objects.<param>.<prop>` is evaluated on the current
//! object before the action runs, so a caller can learn the value of `<prop>` from the
//! outcome of a dry run (bisecting with different parameters). A role granted the action
//! must therefore be allowed to read every property its criteria read, and to see the
//! object type at all. Reading the object as a whole (`objects.<param> != null`, a comparison,
//! an argument of a macro) counts as reading every property of its type. What the criteria read has to be known statically: a computed index
//! such as `objects.flight[params.field]` is refused (fail-closed).
//!
//! The check is a plain function over one [`Role`], so that both the offline scope check
//! (`compile_scope`) and the reload re-check of stored roles call it.

use std::collections::BTreeSet;

use ontoforge_expr::{MemberPath, analyze, member_paths};
use ontoforge_schema::diag::{Diagnostic, SourceLocation, codes};
use ontoforge_schema::ir::{ActionType, ActionTypeName, Ontology, ParamType, Role};

/// ONT-V23 for every property or object type the criteria of `role`'s actions read but the
/// role cannot see. `at` gives the position of an action in the role's `actions` list.
pub(crate) fn criteria_coverage(
    role: &Role,
    ontology: &Ontology,
    at: &dyn Fn(&ActionTypeName) -> SourceLocation,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for name in &role.actions {
        // An unknown action is ONT-V21, reported by the caller.
        let Some(action) = ontology.action_types.get(name.as_str()) else {
            continue;
        };
        let location = at(name);
        let mut seen = BTreeSet::new();
        for criterion in &action.submission_criteria {
            let src = criterion.condition.as_str();
            // A condition that does not parse is ONT-V13's business.
            let Ok(paths) = analyze(src, |program| member_paths(program, src)) else {
                continue;
            };
            for path in paths.iter().filter(|p| is_object_read(p)) {
                let Some((message, help)) = problem(role, action, ontology, path) else {
                    continue;
                };
                if seen.insert(message.clone()) {
                    out.push(Diagnostic::error(codes::V23, message, &location).with_help(help));
                }
            }
        }
    }
    out
}

fn is_object_read(path: &MemberPath) -> bool {
    path.segments.first().map(String::as_str) == Some("objects")
}

/// What is wrong with one read of `objects.…`, as `(message, help)`.
fn problem(
    role: &Role,
    action: &ActionType,
    ontology: &Ontology,
    path: &MemberPath,
) -> Option<(String, String)> {
    let who = format!(
        "role `{}` is granted action `{}`, whose submission criteria read",
        role.api_name, action.api_name
    );
    let text = path.segments.join(".");
    let undetermined = || {
        Some((
            format!(
                "{who} `{text}` through an index or without naming an object parameter; \
                 which property is read cannot be determined statically"
            ),
            "write the property name literally, for example `objects.flight.status`".to_owned(),
        ))
    };
    if path.dynamic || path.segments.len() < 2 {
        return undetermined();
    }
    // A name that is not an object reference parameter is ONT-V13's business.
    let def = action.parameters.get(path.segments[1].as_str())?;
    let ParamType::ObjectReference(object_type) = &def.param_type else {
        return None;
    };
    let Some(grant) = role.object_types.get(object_type.as_str()) else {
        return Some((
            format!(
                "{who} `{text}` (object type `{object_type}`), but the role is not granted `{object_type}`"
            ),
            format!(
                "grant `{object_type}` with the property the criteria read, or do not grant the action"
            ),
        ));
    };
    let Some(property) = path.segments.get(2) else {
        // The whole object is read: every property of its type must be in the allow-list.
        let Some(def) = ontology.object_types.get(object_type.as_str()) else {
            return undetermined();
        };
        let missing: Vec<String> = def
            .properties
            .keys()
            .filter(|p| !grant.properties.contains(p))
            .map(|p| format!("`{p}`"))
            .collect();
        if missing.is_empty() {
            return None;
        }
        return Some((
            format!(
                "{who} `{text}`, the whole object, so the allow-list of `{object_type}` in this role \
                 must include every property; missing: {}",
                missing.join(", ")
            ),
            format!(
                "compare a property such as `objects.<param>.<prop>`, add the missing properties to `properties` of `{object_type}`, or do not grant the action"
            ),
        ));
    };
    if grant.properties.iter().any(|p| p.as_str() == property) {
        return None;
    }
    Some((
        format!(
            "{who} `{text}`, but the allow-list of `{object_type}` in this role does not include `{property}`"
        ),
        format!("add `{property}` to `properties` of `{object_type}`, or do not grant the action"),
    ))
}
