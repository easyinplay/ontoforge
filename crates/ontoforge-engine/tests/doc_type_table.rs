//! `docs/source-binding.md` is the one place the type mapping table is written down (D-13).
//! This test reads that table and asks `compare_binding` the same question for every row, so the
//! document cannot drift from the code.

use std::collections::BTreeSet;

use ontoforge_engine::bind_check::compare_binding;
use ontoforge_engine::{PhysicalColumn, PhysicalType};
use ontoforge_schema::diag::{Severity, codes};
use ontoforge_schema::ir::ProfileName;
use ontoforge_schema::lint::KNOWN_RULE_IDS;
use ontoforge_schema::load_from_memory;

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/source-binding.md");

const START: &str = "<!-- type-table:start -->";
const END: &str = "<!-- type-table:end -->";

fn read_doc() -> String {
    std::fs::read_to_string(DOC).expect("docs/source-binding.md")
}

/// The data rows of the table between the markers: three cells each, backticks removed.
fn table_rows(doc: &str) -> Vec<[String; 3]> {
    let start = doc.find(START).expect("type-table:start marker") + START.len();
    let end = doc.find(END).expect("type-table:end marker");
    assert!(start <= end, "the markers are in the wrong order");
    doc[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        // The header row and the separator row.
        .skip(2)
        .map(|line| {
            let cells: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect();
            let [a, b, c]: [String; 3] = cells.try_into().unwrap_or_else(|cells: Vec<String>| {
                panic!("row `{line}` has {} cells", cells.len())
            });
            [a, b, c]
        })
        .collect()
}

/// `decimal(12,2)` becomes `("decimal", ", precision: 12, scale: 2")`; other types are kept.
fn scalar_yaml(text: &str) -> (String, String) {
    match text
        .strip_prefix("decimal(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(args) => {
            let (precision, scale) = args.split_once(',').expect("decimal(p,s)");
            (
                "decimal".to_owned(),
                format!(", precision: {}, scale: {}", precision.trim(), scale.trim()),
            )
        }
        None => (text.to_owned(), String::new()),
    }
}

/// The YAML flow mapping of the property and the declared `timezone`, from a declaration such
/// as `timestamp (timezone: aware)` or `array<decimal(10,2)>`.
fn parse_declaration(text: &str) -> (String, Option<String>) {
    let mut text = text.to_owned();
    let mut timezone = None;
    if let Some(at) = text.find(" (timezone: ") {
        let rest = &text[at + " (timezone: ".len()..];
        let close = rest.find(')').expect("closing parenthesis of timezone");
        timezone = Some(rest[..close].to_owned());
        text = format!("{}{}", &text[..at], &rest[close + 1..]);
    }
    let element = text
        .strip_prefix("array<")
        .and_then(|rest| rest.strip_suffix('>'));
    let property = if let Some(element) = element {
        let (base, extra) = scalar_yaml(element);
        format!("type: array, items: {base}{extra}")
    } else {
        let (base, extra) = scalar_yaml(&text);
        format!("type: {base}{extra}")
    };
    (property, timezone)
}

fn judge(declaration: &str, physical: &str) -> String {
    let (property, timezone) = parse_declaration(declaration);
    let columns = timezone.map_or_else(String::new, |zone| {
        format!("columns:\n  v: {{ timezone: {zone} }}\n")
    });
    let objects = format!(
        "kind: ObjectType\napiName: Thing\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  v: {{ {property} }}\nbacking: source\n"
    );
    let binding = format!(
        "kind: SourceBinding\ntarget: {{ objectType: Thing }}\nsource: {{ catalog: demo, schema: s, table: t }}\n{columns}"
    );
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", objects.as_str()),
        ("bindings/dev/thing.yaml", binding.as_str()),
    ]);
    assert!(
        out.diagnostics.is_empty(),
        "declaration `{declaration}`: {:#?}",
        out.diagnostics
    );
    let ontology = out.ontology.expect("ontology");
    let dev = ProfileName::try_from("dev".to_owned()).expect("profile");
    let binding = ontology
        .profile_bindings(&dev)
        .expect("dev bindings")
        .objects
        .values()
        .next()
        .expect("thing binding")
        .clone();
    let table = vec![
        PhysicalColumn {
            name: "id".to_owned(),
            physical: PhysicalType::Varchar { length: None },
            nullable: false,
        },
        PhysicalColumn {
            name: "v".to_owned(),
            physical: PhysicalType::parse(physical),
            nullable: true,
        },
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    match found.as_slice() {
        [] => "OK".to_owned(),
        [one] if one.severity == Severity::Warning => "W".to_owned(),
        [one] if one.severity == Severity::Error => "E".to_owned(),
        many => format!("{} mismatches", many.len()),
    }
}

/// One line per row whose verdict in the document differs from `compare_binding`.
fn disagreements(rows: &[[String; 3]]) -> Vec<String> {
    rows.iter()
        .filter_map(|[declaration, physical, verdict]| {
            let actual = judge(declaration, physical);
            (actual != *verdict).then(|| {
                format!("`{declaration}` <- `{physical}`: the document says {verdict}, compare_binding says {actual}")
            })
        })
        .collect()
}

#[test]
fn doc_type_table_matches_compare_binding() {
    let rows = table_rows(&read_doc());
    assert!(
        rows.len() >= 30,
        "only {} rows in the type table",
        rows.len()
    );
    let wrong = disagreements(&rows);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));

    // The check itself notices a wrong row.
    let tampered = [["integer".to_owned(), "bigint".to_owned(), "OK".to_owned()]];
    assert_eq!(disagreements(&tampered).len(), 1);
}

#[test]
fn doc_diagnostic_codes_exist() {
    let doc = read_doc();
    let pattern = regex::Regex::new(r"ONT-[VEW]\d{2}").expect("regex");
    let mentioned: BTreeSet<&str> = pattern.find_iter(&doc).map(|m| m.as_str()).collect();

    let validate_codes = [
        codes::V01,
        codes::V02,
        codes::V03,
        codes::V04,
        codes::V05,
        codes::V06,
        codes::V07,
        codes::V08,
        codes::V09,
        codes::V10,
        codes::V11,
        codes::V12,
        codes::V13,
        codes::V14,
        codes::V15,
        codes::V16,
        codes::V17,
        codes::V18,
        codes::V19,
        codes::V20,
    ];
    let known: BTreeSet<&str> = validate_codes.into_iter().chain(KNOWN_RULE_IDS).collect();
    for code in &mentioned {
        assert!(
            known.contains(code),
            "the document mentions unknown code {code}"
        );
    }

    for required in [
        codes::V16,
        codes::V17,
        codes::V18,
        codes::V19,
        codes::V20,
        "ONT-E13",
        "ONT-E14",
        "ONT-E15",
        "ONT-E16",
        "ONT-W17",
    ] {
        assert!(
            mentioned.contains(required),
            "the document does not describe {required}"
        );
    }
}
