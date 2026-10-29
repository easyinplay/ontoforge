use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::Toolset;
use ontoforge_server::mcp_catalog::{AGGREGATE_TOOL, DESCRIBE_TOOL, HISTORY_TOOL, ToolCatalog};

fn scope(toolsets: &[Toolset]) -> PrincipalScope {
    PrincipalScope {
        toolsets: toolsets.iter().copied().collect(),
        ..PrincipalScope::default()
    }
}

#[test]
fn mcp_core_full_describe_query_history_equal_rest() {
    let core = ToolCatalog::for_scope(&scope(&[Toolset::Core]));
    assert!(core.all().iter().any(|tool| tool.name == DESCRIBE_TOOL));
    assert!(core.all().iter().any(|tool| tool.name == "object_set"));
    assert!(core.all().iter().any(|tool| tool.name == "searchAround"));
    assert!(!core.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Core, Toolset::Full]));
    assert!(full.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    assert!(full.all().iter().any(|tool| tool.name == HISTORY_TOOL));
}

#[test]
fn mcp_list_filters_before_keyset_and_direct_call_denies() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    let first = full.page(None, 1);
    assert_eq!(first.tools.len(), 1);
    assert!(first.has_more);
    let second = full.page(first.next.as_deref(), 16);
    assert!(!second.tools.is_empty());
    assert!(!full.allows("admin_reload"));
    assert!(!full.allows("scope_apply"));
}

#[test]
fn mcp_inflight_snapshot_and_revoke_on_next_call() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    assert!(full.allows(DESCRIBE_TOOL));
    let revoked = ToolCatalog::for_scope(&scope(&[]));
    assert!(!revoked.allows(DESCRIBE_TOOL));
}
