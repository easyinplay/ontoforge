//! ONT-E14: a timestamp binding must say how the column is read (BIND-02).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct E14;

pub(super) static RULE: E14 = E14;

impl LintRule for E14 {
    fn id(&self) -> &'static str {
        "ONT-E14"
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
  scheduledDeparture: { type: timestamp }
  status: { type: string }
backing: source
---
kind: ObjectType
apiName: Maintenance
primaryKey: id
properties:
  id: { type: string, required: true }
  performedAt: { type: timestamp }
backing: source
---
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
  openedAt: { type: timestamp }
backing: editOnly
";

    const SOURCE: &str = "source: { catalog: demo, schema: aviation, table: t }\n";

    fn binding(target: &str, columns: &str) -> String {
        format!("kind: SourceBinding\ntarget: {{ objectType: {target} }}\n{SOURCE}{columns}")
    }

    fn run(bindings: &[(&str, String)], config: Option<&str>) -> Vec<Diagnostic> {
        let mut files = vec![("ontology.yaml", MANIFEST), ("objects.yaml", OBJECTS)];
        files.extend(bindings.iter().map(|(path, text)| (*path, text.as_str())));
        lint_rule_on("ONT-E14", &files, config)
    }

    fn flagged<'a>(text: &'a str, d: &Diagnostic) -> &'a str {
        &text[d.span.offset..d.span.offset + d.span.length]
    }

    /// An entry without `timezone`, a timestamp without an entry, and a `timezone` on
    /// a string.
    fn triggering() -> Vec<(&'static str, String)> {
        vec![
            (
                "bindings/dev/flight.yaml",
                binding(
                    "Flight",
                    "columns:\n  scheduledDeparture: { column: sched }\n  status: { timezone: UTC }\n",
                ),
            ),
            ("bindings/dev/maintenance.yaml", binding("Maintenance", "")),
        ]
    }

    #[test]
    fn triggers() {
        let bindings = triggering();
        let found = run(&bindings, None);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(found.iter().all(|d| d.code == "ONT-E14"));
        assert!(found.iter().all(|d| d.severity == Severity::Error));

        let flight = &bindings[0].1;
        let maintenance = &bindings[1].1;
        let by_file = |file: &str| -> Vec<&Diagnostic> {
            found.iter().filter(|d| d.file == file).collect()
        };
        let in_flight = by_file("bindings/dev/flight.yaml");
        let texts: Vec<&str> = in_flight.iter().map(|d| flagged(flight, d)).collect();
        assert_eq!(texts, ["scheduledDeparture", "timezone"]);
        assert_eq!(in_flight[0].line, 5);
        assert!(
            in_flight[0].message.contains("`scheduledDeparture`")
                && in_flight[0].message.contains("`Flight`"),
            "{}",
            in_flight[0].message
        );
        assert!(
            in_flight[1].message.contains("`status`"),
            "{}",
            in_flight[1].message
        );

        // No entry for the property: the finding points at `target`.
        let in_maintenance = by_file("bindings/dev/maintenance.yaml");
        assert_eq!(in_maintenance.len(), 1);
        assert_eq!(flagged(maintenance, in_maintenance[0]), "target");
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let found = run(
            &[
                (
                    "bindings/dev/flight.yaml",
                    binding(
                        "Flight",
                        "columns:\n  scheduledDeparture: { timezone: Europe/Berlin }\n",
                    ),
                ),
                (
                    "bindings/prod/flight.yaml",
                    binding("Flight", "columns:\n  scheduledDeparture: { timezone: aware }\n"),
                ),
                (
                    "bindings/dev/maintenance.yaml",
                    binding("Maintenance", "columns:\n  performedAt: { timezone: aware }\n"),
                ),
            ],
            None,
        );
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn e14_multi_profile() {
        let found = run(
            &[
                (
                    "bindings/dev/flight.yaml",
                    binding(
                        "Flight",
                        "columns:\n  scheduledDeparture: { timezone: UTC }\n",
                    ),
                ),
                (
                    "bindings/prod/flight.yaml",
                    binding("Flight", "columns:\n  scheduledDeparture: { column: sched }\n"),
                ),
            ],
            None,
        );
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].file, "bindings/prod/flight.yaml");
    }

    #[test]
    fn e14_array_of_timestamps() {
        let objects = "\
kind: ObjectType
apiName: Run
primaryKey: id
properties:
  id: { type: string, required: true }
  stops: { type: array, items: timestamp }
backing: source
";
        let bind = binding("Run", "");
        let files = [
            ("ontology.yaml", MANIFEST),
            ("objects.yaml", objects),
            ("bindings/dev/run.yaml", bind.as_str()),
        ];
        let found = lint_rule_on("ONT-E14", &files, None);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].message.contains("`stops`"), "{}", found[0].message);
    }

    /// The end of a many-to-many link whose key is a timestamp needs a zone too.
    #[test]
    fn e14_link_end_with_timestamp_key() {
        let objects = "\
kind: ObjectType
apiName: Slot
primaryKey: at
properties:
  at: { type: timestamp, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Crew
primaryKey: id
properties:
  id: { type: string, required: true }
backing: editOnly
---
kind: LinkType
apiName: slotCrew
cardinality: manyToMany
from: { objectType: Slot, apiName: crew }
to: { objectType: Crew, apiName: slots }
backing: source
";
        let bind = "\
kind: SourceBinding
target: { linkType: slotCrew }
source: { catalog: demo, schema: aviation, table: slot_crew }
columns:
  from: { column: slot_at }
  to: { column: crew_id }
";
        let files = [
            ("ontology.yaml", MANIFEST),
            ("objects.yaml", objects),
            ("bindings/dev/slot-crew.yaml", bind),
        ];
        let found = lint_rule_on("ONT-E14", &files, None);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(flagged(bind, &found[0]), "from");
    }

    #[test]
    fn e14_can_be_turned_off() {
        let found = run(&triggering(), Some("[lint]\n\"ONT-E14\" = \"off\"\n"));
        assert!(found.is_empty(), "{found:#?}");
    }
}
