use std::collections::BTreeSet;

use ontoforge_engine::TableRef;
use ontoforge_engine::printer::l2::{
    ExplainType, L2Policy, L2TableNameError, McpDefaults, explain_l2, explain_l2_with_type,
    resolve_l2_table_name, validate_l2,
};

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
    let sql =
        validate_l2("SELECT nationkey FROM demo.tiny.nation", &policy()).expect("validated SQL");
    for (kind, keyword) in [
        (ExplainType::Logical, "LOGICAL"),
        (ExplainType::Distributed, "DISTRIBUTED"),
        (ExplainType::Validate, "VALIDATE"),
        (ExplainType::Io, "IO"),
    ] {
        assert_eq!(
            explain_l2_with_type(&sql, kind).to_string(),
            format!("EXPLAIN (TYPE {keyword}) SELECT nationkey FROM demo.tiny.nation LIMIT 3")
        );
    }
    assert_eq!(
        explain_l2(&sql).to_string(),
        "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation LIMIT 3"
    );
}

#[test]
fn l2_table_name_defaults_qualification_and_injection() {
    let defaults = McpDefaults {
        catalog: Some("demo".to_owned()),
        schema: Some("tiny".to_owned()),
    };
    assert_eq!(
        resolve_l2_table_name("nation", &defaults).unwrap(),
        TableRef {
            catalog: "demo".to_owned(),
            schema: "tiny".to_owned(),
            table: "nation".to_owned(),
        }
    );
    assert_eq!(
        resolve_l2_table_name("tiny.nation", &defaults)
            .unwrap()
            .schema,
        "tiny"
    );
    assert_eq!(
        resolve_l2_table_name("other.s.t", &defaults)
            .unwrap()
            .catalog,
        "other"
    );
    assert_eq!(
        resolve_l2_table_name("\"tiny\".\"nation\"", &defaults)
            .unwrap()
            .table,
        "nation"
    );
    for input in [
        "",
        "demo.tiny.nation;",
        "demo.tiny.nation; DROP TABLE x",
        "demo.tiny.nation trailing",
        "a.b.c.d",
    ] {
        assert!(
            resolve_l2_table_name(input, &defaults).is_err(),
            "{input:?}"
        );
    }
    assert!(matches!(
        resolve_l2_table_name(
            "nation",
            &McpDefaults {
                catalog: None,
                schema: Some("tiny".to_owned()),
            }
        ),
        Err(L2TableNameError::MissingContext)
    ));
    assert!(validate_l2("SELECT nationkey FROM demo.tiny.nation;", &policy()).is_ok());
}
