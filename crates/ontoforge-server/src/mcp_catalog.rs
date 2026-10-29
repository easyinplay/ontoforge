use std::collections::BTreeSet;

use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::ActionType;
use ontoforge_schema::ir::Toolset;
use rmcp::model::{Tool, ToolAnnotations};
use serde_json::json;

pub const DESCRIBE_TOOL: &str = "describe";
pub const GET_OBJECT_TOOL: &str = "get_object";
pub const QUERY_TOOL: &str = "query";
pub const OBJECT_SET_TOOL: &str = "object_set";
pub const SEARCH_AROUND_TOOL: &str = "searchAround";
pub const AGGREGATE_TOOL: &str = "aggregate";
pub const HISTORY_TOOL: &str = "history";
pub const EXECUTE_QUERY_TOOL: &str = "execute_query";
pub const LIST_CATALOGS_TOOL: &str = "list_catalogs";
pub const LIST_SCHEMAS_TOOL: &str = "list_schemas";
pub const LIST_TABLES_TOOL: &str = "list_tables";
pub const GET_TABLE_SCHEMA_TOOL: &str = "get_table_schema";
pub const EXPLAIN_QUERY_TOOL: &str = "explain_query";

#[derive(Debug, Clone, PartialEq)]
pub struct ToolPage {
    pub tools: Vec<Tool>,
    pub has_more: bool,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolCatalog {
    tools: Vec<Tool>,
}

impl ToolCatalog {
    #[must_use]
    pub fn for_scope(scope: &PrincipalScope) -> Self {
        let mut names = BTreeSet::new();
        if scope.toolsets.contains(&Toolset::Core) || scope.toolsets.contains(&Toolset::Full) {
            names.extend([
                DESCRIBE_TOOL,
                GET_OBJECT_TOOL,
                QUERY_TOOL,
                OBJECT_SET_TOOL,
                SEARCH_AROUND_TOOL,
            ]);
        }
        if scope.toolsets.contains(&Toolset::Full) {
            names.extend([AGGREGATE_TOOL, HISTORY_TOOL]);
        }
        if scope.toolsets.contains(&Toolset::SqlRaw) {
            names.extend([
                EXECUTE_QUERY_TOOL,
                EXPLAIN_QUERY_TOOL,
                GET_TABLE_SCHEMA_TOOL,
                LIST_CATALOGS_TOOL,
                LIST_SCHEMAS_TOOL,
                LIST_TABLES_TOOL,
            ]);
        }
        let tools = names.into_iter().map(static_tool).collect();
        Self { tools }
    }

    #[must_use]
    pub fn with_actions<'a, I>(mut self, scope: &PrincipalScope, actions: I) -> Self
    where
        I: IntoIterator<Item = &'a ActionType>,
    {
        if scope.toolsets.contains(&Toolset::Core) || scope.toolsets.contains(&Toolset::Full) {
            for action in actions {
                if scope.actions.contains(&action.api_name) {
                    let schema = crate::mcp_actions::ActionToolSchema::from_action(action);
                    self.tools.push(schema.tool(action.description.as_deref()));
                }
            }
            self.tools.sort_by(|left, right| left.name.cmp(&right.name));
            self.tools.dedup_by(|left, right| left.name == right.name);
        }
        self
    }

    #[must_use]
    pub fn all(&self) -> &[Tool] {
        &self.tools
    }

    #[must_use]
    pub fn allows(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool.name == name)
    }

    #[must_use]
    pub fn page(&self, after: Option<&str>, first: usize) -> ToolPage {
        let start = after
            .and_then(|cursor| self.tools.iter().position(|tool| tool.name == cursor))
            .map_or(0, |index| index + 1);
        let end = (start + first.max(1)).min(self.tools.len());
        let tools = self.tools[start..end].to_vec();
        let has_more = end < self.tools.len();
        ToolPage {
            next: has_more.then(|| self.tools[end - 1].name.to_string()),
            tools,
            has_more,
        }
    }
}

fn static_tool(name: &str) -> Tool {
    let schema = match name {
        GET_OBJECT_TOOL => json!({
            "type":"object",
            "required":["objectType","pk"],
            "properties":{"objectType":{"type":"string"},"pk":{}},
            "additionalProperties":false
        }),
        QUERY_TOOL | OBJECT_SET_TOOL | SEARCH_AROUND_TOOL | AGGREGATE_TOOL => {
            json!({"type":"object","required":["request"],"properties":{"request":{"type":"object"}},"additionalProperties":false})
        }
        EXECUTE_QUERY_TOOL => json!({
            "type":"object",
            "required":["query"],
            "properties":{"query":{"type":"string"}},
            "additionalProperties":false
        }),
        LIST_CATALOGS_TOOL => json!({"type":"object","additionalProperties":false}),
        LIST_SCHEMAS_TOOL => json!({
            "type":"object",
            "properties":{"catalog":{"type":"string"}},
            "additionalProperties":false
        }),
        LIST_TABLES_TOOL => json!({
            "type":"object",
            "properties":{"catalog":{"type":"string"},"schema":{"type":"string"}},
            "additionalProperties":false
        }),
        GET_TABLE_SCHEMA_TOOL => json!({
            "type":"object",
            "required":["table"],
            "properties":{"table":{"type":"string"},"catalog":{"type":"string"},"schema":{"type":"string"}},
            "additionalProperties":false
        }),
        EXPLAIN_QUERY_TOOL => json!({
            "type":"object",
            "required":["query"],
            "properties":{"query":{"type":"string"},"format":{"type":"string","enum":["LOGICAL","DISTRIBUTED","VALIDATE","IO"],"default":"DISTRIBUTED"}},
            "additionalProperties":false
        }),
        HISTORY_TOOL => {
            json!({"type":"object","required":["objectType","pk"],"properties":{"objectType":{"type":"string"},"pk":{},"limit":{"type":"integer","minimum":1},"beforeSeq":{"type":"integer"}},"additionalProperties":false})
        }
        _ => json!({"type":"object","additionalProperties":false}),
    };
    Tool::new(
        name.to_owned(),
        format!("Ontoforge {name}"),
        schema.as_object().unwrap().clone(),
    )
    .with_annotations(ToolAnnotations::new().read_only(true))
}
