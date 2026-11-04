use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::Toolset;
use ontoforge_server::mcp_catalog::ToolCatalog;

fn raw_scope() -> PrincipalScope {
    PrincipalScope {
        toolsets: [Toolset::SqlRaw].into_iter().collect(),
        sql_levels: [ontoforge_schema::ir::SqlLevel::Raw].into_iter().collect(),
        l2_catalogs: [String::from("demo")].into_iter().collect(),
        l2_schemas: [String::from("demo.tiny")].into_iter().collect(),
        ..PrincipalScope::default()
    }
}

#[test]
fn mcp_l2_all_six_names_parameters_and_allowlist() {
    let catalog = ToolCatalog::for_scope(&raw_scope());
    let names = catalog
        .all()
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "execute_query",
            "explain_query",
            "get_table_schema",
            "list_catalogs",
            "list_schemas",
            "list_tables",
        ]
    );

    let execute = catalog
        .all()
        .iter()
        .find(|tool| tool.name == "execute_query")
        .expect("execute_query tool");
    assert_eq!(
        execute.input_schema["required"],
        serde_json::json!(["query"])
    );
    assert_eq!(
        execute.input_schema["properties"]["query"]["type"],
        "string"
    );

    let explain = catalog
        .all()
        .iter()
        .find(|tool| tool.name == "explain_query")
        .expect("explain_query tool");
    assert_eq!(
        explain.input_schema["required"],
        serde_json::json!(["query"])
    );
    assert_eq!(
        explain.input_schema["properties"]["format"]["enum"],
        serde_json::json!(["LOGICAL", "DISTRIBUTED", "VALIDATE", "IO"])
    );
}

#[test]
fn mcp_l2_gate_expiry_revoke_and_audit() {
    let allowed = raw_scope();
    assert!(allowed.toolsets.contains(&Toolset::SqlRaw));
    assert!(
        allowed
            .sql_levels
            .contains(&ontoforge_schema::ir::SqlLevel::Raw)
    );

    let revoked = PrincipalScope::default();
    let catalog = ToolCatalog::for_scope(&revoked);
    assert!(catalog.all().is_empty());
}
