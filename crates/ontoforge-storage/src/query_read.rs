//! Side-effect-free, bounded reads of typed edit rows.

use std::collections::BTreeMap;

use ontoforge_schema::ir::{ObjectTypeName, PropertyDef};
use sea_query::{Alias, Condition, Expr, ExprTrait, Order, Query};

use crate::commit::{bind, cell_json, decode_object, read_expr};
use crate::error::StoreError;
use crate::physical::ObjectTable;
use crate::sql_exit::{self, Cell, CellKind};
use crate::state::EditRow;
use crate::store::{ActiveDefinition, Backend, EditStore};
use crate::value::PropertyValue;

pub const MAX_EDIT_READ: usize = 256;

/// One edit row with its canonical logical key.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectEdit {
    pub pk: PropertyValue,
    pub row: EditRow,
}

/// Key of the last returned object; opaque to query callers.
#[derive(Debug, Clone, PartialEq)]
pub struct EditScanCursor {
    pk: PropertyValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditScanPage {
    pub edits: Vec<ObjectEdit>,
    pub next: Option<EditScanCursor>,
    pub has_more: bool,
}

fn bounded(limit: usize) -> Result<(), StoreError> {
    if (1..=MAX_EDIT_READ).contains(&limit) {
        Ok(())
    } else {
        Err(StoreError::Config(format!(
            "edit read limit must be in 1..={MAX_EDIT_READ}"
        )))
    }
}

fn logical_read_error(type_name: &ObjectTypeName, error: StoreError) -> StoreError {
    match error {
        StoreError::Query { sqlstate, .. } => StoreError::Query {
            sqlstate,
            message: format!("logical object type `{type_name}` read failed"),
        },
        other => other,
    }
}

fn object_definition<'a>(
    active: &'a ActiveDefinition,
    type_name: &ObjectTypeName,
) -> Result<
    (
        &'a ontoforge_schema::ir::ObjectType,
        &'a ObjectTable,
        &'a PropertyDef,
    ),
    StoreError,
> {
    let ty =
        active.ontology.object_types.get(type_name).ok_or_else(|| {
            StoreError::Config(format!("unknown logical object type `{type_name}`"))
        })?;
    let table = active.physical.objects.get(type_name).ok_or_else(|| {
        StoreError::Corrupt(format!(
            "logical object type `{type_name}` has no edit mapping"
        ))
    })?;
    let pk = ty.properties.get(&ty.primary_key).ok_or_else(|| {
        StoreError::Corrupt(format!(
            "logical object type `{type_name}` has no primary key"
        ))
    })?;
    Ok((ty, table, pk))
}

fn select_object_columns(
    select: &mut sea_query::SelectStatement,
    table: &ObjectTable,
    backend: Backend,
) -> Vec<CellKind> {
    let (pk_expr, pk_kind) = read_expr(&table.pk.column, &table.pk.ty, backend);
    select.expr(pk_expr);
    let mut kinds = vec![pk_kind];
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
        .column(Alias::new("of_released"));
    kinds.extend([CellKind::Int64, CellKind::Bool, CellKind::Bool]);
    kinds
}

fn decode_edit(
    ty: &ontoforge_schema::ir::ObjectType,
    table: &ObjectTable,
    pk_def: &PropertyDef,
    mut cells: Vec<Cell>,
) -> Result<Option<ObjectEdit>, StoreError> {
    let key = cells.remove(0);
    let json = cell_json(key, &table.pk.ty)?;
    let pk = PropertyValue::from_json(pk_def, &json).map_err(|e| {
        StoreError::Corrupt(format!(
            "logical primary key `{}` is invalid: {e}",
            ty.primary_key
        ))
    })?;
    let row = decode_object(ty, table, cells)?;
    Ok((row.version > 0).then_some(ObjectEdit { pk, row }))
}

impl EditStore {
    /// Reads one edit row without creating or locking a row.
    pub async fn read_object_edit(
        &self,
        active: &ActiveDefinition,
        type_name: &ObjectTypeName,
        pk: &PropertyValue,
    ) -> Result<Option<ObjectEdit>, StoreError> {
        Ok(self
            .read_object_edits(active, type_name, std::slice::from_ref(pk))
            .await?
            .pop()
            .flatten())
    }

    /// Reads at most 256 keys, returning one slot per requested key in request order.
    pub async fn read_object_edits(
        &self,
        active: &ActiveDefinition,
        type_name: &ObjectTypeName,
        keys: &[PropertyValue],
    ) -> Result<Vec<Option<ObjectEdit>>, StoreError> {
        if keys.len() > MAX_EDIT_READ {
            return Err(StoreError::Config(format!(
                "object edit batch for `{type_name}` exceeds {MAX_EDIT_READ} keys"
            )));
        }
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let (ty, table, pk_def) = object_definition(active, type_name)?;
        let canonical = keys
            .iter()
            .map(|key| {
                key.canonical(pk_def).map_err(|e| {
                    StoreError::Config(format!("invalid primary key for `{type_name}`: {e}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let backend = self.backend();
        let mut select = Query::select();
        let kinds = select_object_columns(&mut select, table, backend);
        let mut predicates = Condition::any();
        let mut unique = BTreeMap::new();
        for key in &canonical {
            let identity = key.to_json().to_string();
            if unique.insert(identity, key).is_none() {
                predicates = predicates.add(Expr::col(Alias::new(&table.pk.column)).eq(bind(
                    key,
                    &table.pk.ty,
                    backend,
                )));
            }
        }
        select
            .from(sql_exit::edit_table(self.schema_edit(), &table.table))
            .cond_where(predicates)
            .and_where(Expr::col(Alias::new("of_version")).gt(0));
        let cells =
            sql_exit::fetch_all_cells_read(&self.db, sql_exit::build_for(&select, backend), &kinds)
                .await
                .map_err(|error| logical_read_error(type_name, error))?;
        let mut found = BTreeMap::new();
        for row in cells {
            if let Some(edit) = decode_edit(ty, table, pk_def, row)? {
                found.insert(edit.pk.to_json().to_string(), edit);
            }
        }
        Ok(canonical
            .iter()
            .map(|key| found.get(&key.to_json().to_string()).cloned())
            .collect())
    }

    /// Reads one stable keyset page of object edits. Version-zero rows are invisible.
    pub async fn scan_object_edits(
        &self,
        active: &ActiveDefinition,
        type_name: &ObjectTypeName,
        after: Option<&EditScanCursor>,
        limit: usize,
    ) -> Result<EditScanPage, StoreError> {
        bounded(limit)?;
        let (ty, table, pk_def) = object_definition(active, type_name)?;
        let backend = self.backend();
        let mut select = Query::select();
        let kinds = select_object_columns(&mut select, table, backend);
        select
            .from(sql_exit::edit_table(self.schema_edit(), &table.table))
            .and_where(Expr::col(Alias::new("of_version")).gt(0))
            .order_by(Alias::new(&table.pk.column), Order::Asc)
            .limit((limit + 1) as u64);
        if let Some(cursor) = after {
            let pk = cursor.pk.canonical(pk_def).map_err(|e| {
                StoreError::Config(format!("invalid cursor for `{type_name}`: {e}"))
            })?;
            select.and_where(Expr::col(Alias::new(&table.pk.column)).gt(bind(
                &pk,
                &table.pk.ty,
                backend,
            )));
        }
        let cells =
            sql_exit::fetch_all_cells_read(&self.db, sql_exit::build_for(&select, backend), &kinds)
                .await
                .map_err(|error| logical_read_error(type_name, error))?;
        let mut edits = cells
            .into_iter()
            .map(|row| decode_edit(ty, table, pk_def, row))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let has_more = edits.len() > limit;
        edits.truncate(limit);
        let next = edits.last().map(|edit| EditScanCursor {
            pk: edit.pk.clone(),
        });
        Ok(EditScanPage {
            edits,
            next,
            has_more,
        })
    }
}
