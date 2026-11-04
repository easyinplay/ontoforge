use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::bind_check::{check_drift, describe_bindings};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};

struct Probe {
    budgets: Mutex<Vec<Duration>>,
}
#[allow(
    clippy::unused_async_trait_impl,
    reason = "metadata test doubles have immediate unused business operations"
)]
impl Engine for Probe {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        ontoforge_engine::Dialect::Trino
    }
    async fn execute(
        &self,
        _sql: &PrintedSql,
        _ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        unreachable!("metadata must not execute business SQL")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
    async fn describe_tables(
        &self,
        _tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.budgets.lock().unwrap().push(ctx.timeout);
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(BTreeMap::new())
    }
}

#[tokio::test(start_paused = true)]
async fn online_description_retains_per_query_timeout_while_drift_shares_budget() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let mut ontology = ontoforge_schema::load_dir(root).ontology.unwrap();
    let profile = ontology.default_profile.clone().unwrap();
    ontology
        .bindings
        .get_mut(&profile)
        .unwrap()
        .objects
        .get_mut("Pilot")
        .unwrap()
        .source
        .catalog = "second".to_owned();
    let mut context = QueryContext::new("budget-contract".to_owned(), vec![]);
    context.timeout = Duration::from_secs(5);
    let online = Probe {
        budgets: Mutex::new(Vec::new()),
    };
    describe_bindings(&online, &ontology, &profile, &context)
        .await
        .unwrap();
    assert_eq!(
        *online.budgets.lock().unwrap(),
        vec![Duration::from_secs(5); 2]
    );
    let drift = Probe {
        budgets: Mutex::new(Vec::new()),
    };
    check_drift(&drift, &ontology, &profile, &context)
        .await
        .unwrap();
    let budgets = drift.budgets.lock().unwrap();
    assert_eq!(
        *budgets,
        vec![Duration::from_secs(5), Duration::from_millis(4980)]
    );
}

#[tokio::test(start_paused = true)]
async fn expired_drift_budget_sends_no_catalog_probe() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let ontology = ontoforge_schema::load_dir(root).ontology.unwrap();
    let profile = ontology.default_profile.as_ref().unwrap();
    let mut context = QueryContext::new("expired-budget".to_owned(), vec![]);
    context.timeout = Duration::ZERO;
    let engine = Probe {
        budgets: Mutex::new(Vec::new()),
    };
    assert!(
        check_drift(&engine, &ontology, profile, &context)
            .await
            .is_err()
    );
    assert!(engine.budgets.lock().unwrap().is_empty());
}
