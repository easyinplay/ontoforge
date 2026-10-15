//! Object type checks. Plan 01-04 completes `ONT-V10` and `ONT-V06`.

use crate::diag::{Diagnostic, codes, did_you_mean};
use crate::ir::{Ontology, PropertyName};
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for object in ontology.object_types.values() {
        if object.properties.contains_key(object.primary_key.as_str()) {
            continue;
        }
        let key = DefKey {
            kind: DefKind::ObjectType,
            api_name: object.api_name.to_string(),
        };
        let at = sources.locate(&key, &[Seg::Key("primaryKey".into())], Part::Value);
        let d = Diagnostic::error(
            codes::V10,
            format!(
                "primaryKey `{}` is not a property of `{}`",
                object.primary_key, object.api_name
            ),
            &at,
        );
        let candidates = object.properties.keys().map(PropertyName::as_str);
        out.push(
            match did_you_mean(object.primary_key.as_str(), candidates) {
                Some(s) => d.with_help(format!("did you mean `{s}`?")),
                None => d,
            },
        );
    }
}
