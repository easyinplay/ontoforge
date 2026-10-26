use std::collections::BTreeSet;

use ontoforge_engine::printer::l2::{L2Error, L2Policy, validate_l2};

fn policy() -> L2Policy {
    L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["hive".to_owned()]),
        schemas: BTreeSet::from(["hive.sales".to_owned()]),
        edit_catalog: "edit".to_owned(),
        max_rows: 100,
    }
}

#[test]
fn l2_gate_accepts_only_validated_display() {
    let sql = validate_l2("SELECT id FROM hive.sales.orders; -- discarded", &policy())
        .expect("an authorized SELECT must be accepted");
    assert_eq!(sql.as_str(), "SELECT id FROM hive.sales.orders LIMIT 101");
    assert!(!sql.as_str().contains("discarded"));
}

#[test]
fn l2_gate_rejects_unauthorized_relation() {
    assert_eq!(
        validate_l2("SELECT * FROM edit.s.secret", &policy()),
        Err(L2Error::Denied)
    );
}
