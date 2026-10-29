use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use ontoforge_auth::{Principal, PrincipalId};
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::{Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PhysicalType, PrintedSql, QueryContext, QueryOutput, TableRef, Value};
use ontoforge_schema::ir::{SqlLevel, Toolset};
use ontoforge_server::mcp::mcp_l2::{McpL2Error, McpL2Tools};
use ontoforge_server::ServerLimits;

mod support;

#[derive(Clone, Default)]
struct RecordingEngine {
    sql: Arc<Mutex<Vec<String>>>,
    tables: Arc<Mutex<Vec<TableRef>>>,
}

impl Engine for RecordingEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Trino
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        _ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.sql.lock().unwrap().push(sql.as_str().to_owned());
        Ok(QueryOutput {
            columns: vec![Column {
                name: "name".to_owned(),
                physical: PhysicalType::Varchar { length: None },
            }],
            rows: vec![vec![Value::String("demo".to_owned())]],
            truncated: false,
        })
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.tables.lock().unwrap().extend(tables.iter().cloned());
        Ok(tables
            .iter()
            .cloned()
            .map(|table| {
                (
                    table,
                    vec![PhysicalColumn {
                        name: "id".to_owned(),
                        physical: PhysicalType::Varchar { length: None },
                        nullable: false,
                    }],
                )
            })
            .collect())
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

fn principal() -> Principal {
    Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Default::default(),
    }
}

fn raw_scope() -> PrincipalScope {
    PrincipalScope {
        toolsets: [Toolset::SqlRaw].into_iter().collect(),
        sql_levels: [SqlLevel::Raw].into_iter().collect(),
        l2_catalogs: ["demo".to_owned()].into_iter().collect(),
        l2_schemas: ["demo.tiny".to_owned()].into_iter().collect(),
        ..PrincipalScope::default()
    }
}

#[tokio::test]
async fn mcp_l2_optional_context_defaults_and_qualified_override() {
    let engine = RecordingEngine::default();
    let fixture = support::fixture().await;
    let active = fixture.store.active_definition().await.unwrap();
    let principal = principal();
    let scope = raw_scope();
    let tools = McpL2Tools {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &fixture.store,
        limits: ServerLimits::default(),
    };

    let result = tools
        .get_table_schema("\"other\".\"空间\".\"航班\"", Some("demo"), Some("tiny"))
        .await;
    assert!(result.is_ok(), "qualified table should be accepted: {result:?}");
    assert_eq!(
        engine.tables.lock().unwrap().as_slice(),
        &[TableRef {
            catalog: "other".to_owned(),
            schema: "空间".to_owned(),
            table: "航班".to_owned(),
        }]
    );
}

#[tokio::test]
async fn mcp_l2_missing_context_and_outside_allowlist_deny_before_send() {
    let engine = RecordingEngine::default();
    let fixture = support::fixture().await;
    let active = fixture.store.active_definition().await.unwrap();
    let principal = principal();
    let mut scope = raw_scope();
    scope.l2_catalogs.clear();
    scope.l2_schemas.clear();
    let tools = McpL2Tools {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &fixture.store,
        limits: ServerLimits::default(),
    };

    let missing = tools.list_schemas(None).await;
    assert!(
        matches!(missing, Err(McpL2Error::Invalid(ref message)) if message.contains("trusted L2 qualification context")),
        "missing context must be explicit: {missing:?}"
    );
    assert!(engine.sql.lock().unwrap().is_empty());

    let denied = tools
        .get_table_schema("outside", Some("other"), Some("secret"))
        .await;
    assert!(matches!(denied, Err(McpL2Error::Denied)));
    assert!(engine.sql.lock().unwrap().is_empty());

    let invalid = tools
        .get_table_schema("tiny; DROP TABLE secret", Some("demo"), Some("tiny"))
        .await;
    assert!(matches!(invalid, Err(McpL2Error::Invalid(_))));
    assert!(engine.tables.lock().unwrap().is_empty());
}
