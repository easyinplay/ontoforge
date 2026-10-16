//! ONT-E15: collation applies to string columns only (BIND-02).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct E15;

pub(super) static RULE: E15 = E15;

impl LintRule for E15 {
    fn id(&self) -> &'static str {
        "ONT-E15"
    }

    fn default_level(&self) -> Level {
        Level::Error
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
}

#[cfg(test)]
mod tests {
    use crate::diag::{Diagnostic, Severity};
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    const OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: id
properties:
  id: { type: string, required: true }
  status: { type: string }
  tags: { type: array, items: string }
  delayMinutes: { type: integer }
  codes: { type: array, items: integer }
backing: source
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeNumber
properties:
  employeeNumber: { type: integer, required: true }
backing: source
---
kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: flights }
backing: source
";

    const SOURCE: &str = "source: { catalog: demo, schema: aviation, table: t }\n";

    fn binding(target: &str, columns: &str) -> String {
        format!("kind: SourceBinding\ntarget: {target}\n{SOURCE}columns:\n{columns}")
    }

    fn run(bindings: &[(&str, String)], config: Option<&str>) -> Vec<Diagnostic> {
        let mut files = vec![("ontology.yaml", MANIFEST), ("objects.yaml", OBJECTS)];
        files.extend(bindings.iter().map(|(path, text)| (*path, text.as_str())));
        lint_rule_on("ONT-E15", &files, config)
    }

    #[test]
    fn triggers() {
        let bind = binding(
            "{ objectType: Flight }",
            "  delayMinutes: { collation: caseInsensitive }\n  codes: { collation: caseSensitive }\n",
        );
        let found = run(&[("bindings/dev/flight.yaml", bind.clone())], None);
        assert_eq!(found.len(), 2, "{found:#?}");
        assert!(found.iter().all(|d| d.severity == Severity::Error));
        let texts: Vec<&str> = found
            .iter()
            .map(|d| &bind[d.span.offset..d.span.offset + d.span.length])
            .collect();
        assert_eq!(texts, ["collation", "collation"]);
        assert_eq!(found[0].line, 5);
        assert!(
            found[0].message.contains("`delayMinutes`") && found[0].message.contains("integer"),
            "{}",
            found[0].message
        );
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let bind = binding(
            "{ objectType: Flight }",
            "  status: { collation: caseInsensitive }\n  tags: { collation: caseSensitive }\n  delayMinutes: { column: delay }\n",
        );
        let found = run(&[("bindings/dev/flight.yaml", bind)], None);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    /// A link end takes its type from the primary key of the object type at that end.
    #[test]
    fn e15_link_end() {
        let bind = binding(
            "{ linkType: flightCrew }",
            "  from: { collation: caseInsensitive }\n  to: { collation: caseInsensitive }\n",
        );
        let found = run(&[("bindings/dev/flight-crew.yaml", bind)], None);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].message.contains("`to`"), "{}", found[0].message);
        assert_eq!(found[0].line, 6);
    }
}
