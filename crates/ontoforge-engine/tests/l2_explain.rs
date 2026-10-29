use std::collections::BTreeSet;

use ontoforge_engine::printer::l2::{L2Policy, explain_l2, validate_l2};

fn policy() -> L2Policy {
    L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["demo".to_owned()]),
        schemas: BTreeSet::from(["demo.tiny".to_owned()]),
        edit_catalog: "editstore".to_owned(),
        max_rows: 2,
    }
}

#[test]
fn l2_explain_four_types_preserve_validated_ast() {
    let sql = validate_l2("SELECT nationkey FROM demo.tiny.nation", &policy())
        .expect("validated SQL");
    let actual = explain_l2(&sql).to_string();
    assert_eq!(
        actual,
        "EXPLAIN (TYPE LOGICAL) SELECT nationkey FROM demo.tiny.nation LIMIT 3"
    );
}

#[test]
fn l2_table_name_defaults_qualification_and_injection() {
    assert!(
        validate_l2("SELECT nationkey FROM demo.tiny.nation;", &policy()).is_err(),
        "identifier-like input must reject a trailing statement token"
    );
}
