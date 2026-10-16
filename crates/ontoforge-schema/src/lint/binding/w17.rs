//! ONT-W17: a bound column is missing from the binding's fixture (D-03).
//!
//! The offline part of "a property has no column in the bound table": a binding that
//! names a fixture is compared with the header the loader recorded for it. Columns
//! that exist only in the real table are checked by `validate --online` (ONT-V17).
//! A fixture whose header cannot be read is reported as well, so a broken fixture
//! never makes the lint look clean (E20).

use std::collections::HashSet;

use super::{all_bindings, binding_def};
use crate::lint::{Finding, Level, LintCx, LintRule};
use crate::loader::sourcemap::{Part, Seg};

pub(super) struct W17;

pub(super) static RULE: W17 = W17;

impl LintRule for W17 {
    fn id(&self) -> &'static str {
        "ONT-W17"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for (profile, binding) in all_bindings(cx.ontology) {
            let Some(fixture) = &binding.fixture else {
                continue;
            };
            let def = binding_def(profile, binding);
            let Some(header) = cx.sources.fixture_columns(&def.api_name) else {
                continue;
            };
            let finding = |path: &[&str], message: String, help: String| Finding {
                def: def.clone(),
                path: path.iter().map(|k| Seg::Key((*k).to_owned())).collect(),
                part: Part::Value,
                message,
                help: Some(help),
                severity: None,
            };
            let header = match header {
                Ok(header) => header,
                Err(reason) => {
                    out.push(finding(
                        &["fixture", "file"],
                        format!(
                            "cannot read the header of fixture `{}`: {reason}; bound columns cannot be checked offline",
                            fixture.file
                        ),
                        "fix the fixture, or run `ontoforge validate --online` to check the binding"
                            .to_owned(),
                    ));
                    continue;
                }
            };
            // Column names compare without regard to case, like the engine's tables.
            let present: HashSet<String> = header.iter().map(|c| c.to_lowercase()).collect();
            for column in cx.ontology.bound_columns(binding) {
                if present.contains(&column.column.to_lowercase()) {
                    continue;
                }
                let path: &[&str] = if column.explicit {
                    &["columns", column.key, "column"]
                } else {
                    &["fixture", "file"]
                };
                out.push(finding(
                    path,
                    format!(
                        "fixture `{}` has no column `{}` for `{}`",
                        fixture.file, column.column, column.key
                    ),
                    "add the column to the fixture, or map the property with `columns.<key>.column`"
                        .to_owned(),
                ));
            }
        }
    }
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
  flightNumber: { type: string }
  status: { type: string }
backing: source
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
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

    fn binding(target: &str, columns: &str, fixture: &str) -> String {
        format!(
            "kind: SourceBinding\ntarget: {target}\n{SOURCE}{columns}fixture: {{ file: {fixture} }}\n"
        )
    }

    const FLIGHT_COLUMNS: &str = "columns:\n  flightNumber: { column: flight_no }\n";
    const CREW_COLUMNS: &str =
        "columns:\n  from: { column: flight_id }\n  to: { column: pilot_id }\n";

    fn run(files: &[(&str, &str)], config: Option<&str>) -> Vec<Diagnostic> {
        let mut all = vec![("ontology.yaml", MANIFEST), ("objects.yaml", OBJECTS)];
        all.extend_from_slice(files);
        lint_rule_on("ONT-W17", &all, config)
    }

    fn flagged<'a>(text: &'a str, d: &Diagnostic) -> &'a str {
        &text[d.span.offset..d.span.offset + d.span.length]
    }

    #[test]
    fn triggers() {
        let flight = binding(
            "{ objectType: Flight }",
            FLIGHT_COLUMNS,
            "fixtures/flights.csv",
        );
        let crew = binding(
            "{ linkType: flightCrew }",
            CREW_COLUMNS,
            "fixtures/crew.csv",
        );
        let files = [
            ("bindings/dev/flight.yaml", flight.as_str()),
            ("bindings/dev/fixtures/flights.csv", "id\nF-1\n"),
            ("bindings/dev/flight-crew.yaml", crew.as_str()),
            ("bindings/dev/fixtures/crew.csv", "flight_id\nF-1\n"),
        ];
        let found = run(&files, None);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(found.iter().all(|d| d.severity == Severity::Warning));

        let of =
            |file: &str| -> Vec<&Diagnostic> { found.iter().filter(|d| d.file == file).collect() };
        let in_flight = of("bindings/dev/flight.yaml");
        assert_eq!(in_flight.len(), 2);
        // An explicit mapping points at the `column` value, a same-name default at
        // the fixture file.
        let texts: Vec<&str> = in_flight.iter().map(|d| flagged(&flight, d)).collect();
        assert_eq!(texts, ["flight_no", "fixtures/flights.csv"]);
        assert!(
            in_flight[0].message.contains("`flight_no`")
                && in_flight[0].message.contains("`flightNumber`"),
            "{}",
            in_flight[0].message
        );
        assert!(
            in_flight[1].message.contains("`status`"),
            "{}",
            in_flight[1].message
        );

        let in_crew = of("bindings/dev/flight-crew.yaml");
        assert_eq!(in_crew.len(), 1);
        assert_eq!(flagged(&crew, in_crew[0]), "pilot_id");
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let flight = binding(
            "{ objectType: Flight }",
            FLIGHT_COLUMNS,
            "fixtures/flights.csv",
        );
        // No fixture, no check.
        let pilot = format!("kind: SourceBinding\ntarget: {{ objectType: Pilot }}\n{SOURCE}");
        let files = [
            ("bindings/dev/flight.yaml", flight.as_str()),
            // The header matches without regard to case.
            (
                "bindings/dev/fixtures/flights.csv",
                "ID,Flight_No,STATUS\nF-1,LH1,ok\n",
            ),
            ("bindings/dev/pilot.yaml", pilot.as_str()),
        ];
        let found = run(&files, None);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    /// E20: a fixture whose header cannot be read is reported, not skipped.
    #[test]
    fn w17_unreadable_header_warns() {
        let flight = binding(
            "{ objectType: Flight }",
            FLIGHT_COLUMNS,
            "fixtures/flights.json",
        );
        let files = [
            ("bindings/dev/flight.yaml", flight.as_str()),
            ("bindings/dev/fixtures/flights.json", r#"{"id": "F-1"}"#),
        ];
        let found = run(&files, None);
        assert_eq!(found.len(), 1, "{found:#?}");
        let d = &found[0];
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(flagged(&flight, d), "fixtures/flights.json");
        assert!(
            d.message
                .contains("cannot read the header of fixture `fixtures/flights.json`"),
            "{}",
            d.message
        );
        assert!(
            d.message.contains("expected an array of objects"),
            "{}",
            d.message
        );
        assert!(
            d.help
                .as_deref()
                .unwrap_or_default()
                .contains("validate --online"),
            "{:?}",
            d.help
        );
        insta::assert_json_snapshot!("unreadable", found);
    }

    #[test]
    fn w17_json_fixture_header_is_the_union_of_keys() {
        let flight = binding("{ objectType: Flight }", "", "fixtures/flights.json");
        let files = [
            ("bindings/dev/flight.yaml", flight.as_str()),
            (
                "bindings/dev/fixtures/flights.json",
                r#"[{"id": "F-1"}, {"flightNumber": "LH1", "status": "ok"}]"#,
            ),
        ];
        assert!(run(&files, None).is_empty());
    }

    #[test]
    fn w17_off() {
        let flight = binding(
            "{ objectType: Flight }",
            FLIGHT_COLUMNS,
            "fixtures/flights.csv",
        );
        let broken = binding("{ objectType: Pilot }", "", "fixtures/pilots.json");
        let files = [
            ("bindings/dev/flight.yaml", flight.as_str()),
            ("bindings/dev/fixtures/flights.csv", "id\n"),
            ("bindings/dev/pilot.yaml", broken.as_str()),
            ("bindings/dev/fixtures/pilots.json", "not json"),
        ];
        assert_eq!(run(&files, None).len(), 3);
        let off = Some("[lint]\n\"ONT-W17\" = \"off\"\n");
        assert!(run(&files, off).is_empty());
    }
}
