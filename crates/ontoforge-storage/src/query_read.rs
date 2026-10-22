//! Side-effect-free reads of typed object edit rows.

use ontoforge_schema::ir::ObjectTypeName;
use sea_query::{Alias, Expr, ExprTrait, Query};

use crate::commit::{bind, decode_object, read_expr};
use crate::error::StoreError;
use crate::sql_exit::{self, CellKind};
use crate::state::EditRow;
use crate::store::{ActiveDefinition, EditStore};
use crate::value::PropertyValue;

/// One edit row with its canonical logical key.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectEdit {
    pub pk: PropertyValue,
    pub row: EditRow,
}

impl EditStore {
    /// Reads one edit row without creating or locking a row.
    pub async fn read_object_edit(
        &self,
        active: &ActiveDefinition,
        type_name: &ObjectTypeName,
        pk: &PropertyValue,
    ) -> Result<Option<ObjectEdit>, StoreError> {
        let ty = active.ontology.object_types.get(type_name).ok_or_else(|| {
            StoreError::Config("object type is not in the active definition".to_owned())
        })?;
        let table = active
            .physical
            .objects
            .get(type_name)
            .ok_or_else(|| StoreError::Corrupt("object type has no edit table".to_owned()))?;
        let backend = self.backend();
        let schema = self.schema_edit();
        let mut select = Query::select();
        let mut kinds = Vec::with_capacity(table.properties.len() * 2 + 3);
        for column in table.properties.values() {
            select.column(Alias::new(&column.set_column));
            kinds.push(CellKind::Bool);
            let (expr, kind) = read_expr(&column.column, &column.ty, backend);
            select.expr(expr);
            kinds.push(kind);
        }
        select
            .column(Alias::new("of_version"))
            .column(Alias::new("of_deleted"))
            .column(Alias::new("of_released"))
            .from(sql_exit::edit_table(schema, &table.table))
            .and_where(Expr::col(Alias::new(&table.pk.column)).eq(bind(pk, &table.pk.ty, backend)));
        kinds.extend([CellKind::Int64, CellKind::Bool, CellKind::Bool]);
        let built = sql_exit::build_for(&select, backend);
        sql_exit::fetch_cells_read(&self.db, built, &kinds)
            .await?
            .map(|cells| {
                Ok(ObjectEdit {
                    pk: pk.clone(),
                    row: decode_object(ty, table, cells)?,
                })
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use camino::{Utf8Path, Utf8PathBuf};
    use ontoforge_schema::load_dir;

    use crate::reload::{ReloadOptions, reload};
    use crate::store::{BackendSettings, StoreSettings};

    use super::*;

    #[tokio::test]
    async fn query_read_missing_key_does_not_create_edit_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
        let store = EditStore::open(StoreSettings {
            backend: BackendSettings::Sqlite { path },
            lock_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_secs(5),
            trino_catalog: None,
        })
        .await
        .unwrap();
        let root = Utf8Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation"
        ));
        let ontology = load_dir(root).ontology.unwrap();
        reload(&store, &ontology, &ReloadOptions::default())
            .await
            .unwrap();
        let active = store.active_definition().await.unwrap();
        let ty = ObjectTypeName::try_from("Airport".to_owned()).unwrap();
        let pk = PropertyValue::String("NONE".to_owned());
        assert!(
            store
                .read_object_edit(&active, &ty, &pk)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .read_object_edit(&active, &ty, &pk)
                .await
                .unwrap()
                .is_none()
        );
    }
}
