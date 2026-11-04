//! Fixed metadata captured for one request.

use std::sync::Arc;

use ontoforge_schema::ir::ScopeSet;

use crate::error::StoreError;
use crate::reload;
use crate::scope_store;
use crate::sql_exit;
use crate::store::{ActiveDefinition, EditStore};

/// The complete metadata captured for one request, with no live transaction.
#[derive(Debug, Clone)]
pub struct RequestSnapshot {
    pub active: Arc<ActiveDefinition>,
    pub scope: ScopeSet,
}

impl EditStore {
    /// Reads the active definition and stored scope under the shared metadata lock, then ends the
    /// transaction before callers compile scope or perform source/network work.
    pub async fn request_snapshot(&self) -> Result<RequestSnapshot, StoreError> {
        let mut tx = reload::begin_definition_tx(self).await?;
        let snapshot = async {
            let active = sql_exit::read_active_definition_in(&mut tx)
                .await?
                .ok_or_else(|| StoreError::Config("no active definition".to_owned()))?;
            let scope = scope_store::load_scope_in(&mut tx).await?;
            Ok(RequestSnapshot {
                active: Arc::new(active),
                scope,
            })
        }
        .await;
        match snapshot {
            Ok(snapshot) => {
                sql_exit::commit(tx).await?;
                Ok(snapshot)
            }
            Err(error) => {
                sql_exit::rollback(tx).await;
                Err(error)
            }
        }
    }
}
