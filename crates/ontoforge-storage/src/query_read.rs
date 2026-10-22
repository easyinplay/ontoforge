//! Side-effect-free, bounded reads of typed edit rows.

use std::collections::BTreeMap;

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyDef};
use sea_query::{Alias, Condition, Expr, ExprTrait, Order, Query};

use crate::commit::{bind, cell_json, decode_object, read_expr};
use crate::error::StoreError;
use crate::physical::{ColumnType, LinkTable, ObjectTable};
use crate::sql_exit::{self, Cell, CellKind};
use crate::state::{EditRow, LinkRow};
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

#[derive(Debug, Clone, PartialEq)]
pub enum LinkEndpoint {
    From(PropertyValue),
    To(PropertyValue),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkEdit {
    pub from: PropertyValue,
    pub to: PropertyValue,
    pub row: LinkRow,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkScanCursor {
    from: PropertyValue,
    to: PropertyValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkScanPage {
    pub edits: Vec<LinkEdit>,
    pub next: Option<LinkScanCursor>,
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

fn logical_read_error(name: &impl std::fmt::Display, error: StoreError) -> StoreError {
    match error {
        StoreError::Query { sqlstate, .. } => StoreError::Query {
            sqlstate,
            message: format!("logical edit `{name}` read failed"),
        },
        other => other,
    }
}

fn decode_key(
    cell: Cell,
    ty: &ColumnType,
    def: &PropertyDef,
    name: &impl std::fmt::Display,
) -> Result<PropertyValue, StoreError> {
    let json = cell_json(cell, ty)?;
    PropertyValue::from_json(def, &json)
        .map_err(|e| StoreError::Corrupt(format!("logical key `{name}` is invalid: {e}")))
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
    let pk = decode_key(key, &table.pk.ty, pk_def, &ty.primary_key)?;
    let row = decode_object(ty, table, cells)?;
    Ok((row.version > 0).then_some(ObjectEdit { pk, row }))
}

struct LinkMapping<'a> {
    table: &'a LinkTable,
    from_ty: &'a ontoforge_schema::ir::ObjectType,
    to_ty: &'a ontoforge_schema::ir::ObjectType,
    from_def: &'a PropertyDef,
    to_def: &'a PropertyDef,
}

fn link_mapping<'a>(
    active: &'a ActiveDefinition,
    link_name: &LinkTypeName,
) -> Result<LinkMapping<'a>, StoreError> {
    active
        .ontology
        .link_types
        .get(link_name)
        .ok_or_else(|| StoreError::Config(format!("unknown logical link `{link_name}`")))?;
    let table = active.physical.links.get(link_name).ok_or_else(|| {
        StoreError::Config(format!(
            "logical link `{link_name}` is not a many-to-many link"
        ))
    })?;
    let from_ty = active
        .ontology
        .object_types
        .get(&table.from_type)
        .ok_or_else(|| {
            StoreError::Corrupt(format!("logical link `{link_name}` has no from type"))
        })?;
    let to_ty = active
        .ontology
        .object_types
        .get(&table.to_type)
        .ok_or_else(|| StoreError::Corrupt(format!("logical link `{link_name}` has no to type")))?;
    let from_def = from_ty
        .properties
        .get(&from_ty.primary_key)
        .ok_or_else(|| {
            StoreError::Corrupt(format!("logical link `{link_name}` has no from key"))
        })?;
    let to_def = to_ty
        .properties
        .get(&to_ty.primary_key)
        .ok_or_else(|| StoreError::Corrupt(format!("logical link `{link_name}` has no to key")))?;
    Ok(LinkMapping {
        table,
        from_ty,
        to_ty,
        from_def,
        to_def,
    })
}

fn decode_link(
    cells: Vec<Cell>,
    mapping: &LinkMapping<'_>,
    link_name: &LinkTypeName,
) -> Result<Option<LinkEdit>, StoreError> {
    let mut row = cells.into_iter();
    let from = decode_key(
        row.next().unwrap(),
        &mapping.table.a.ty,
        mapping.from_def,
        &mapping.from_ty.primary_key,
    )?;
    let to = decode_key(
        row.next().unwrap(),
        &mapping.table.b.ty,
        mapping.to_def,
        &mapping.to_ty.primary_key,
    )?;
    let (Some(Cell::Bool(deleted)), Some(Cell::Int(version))) = (row.next(), row.next()) else {
        return Err(StoreError::Corrupt(format!(
            "logical link `{link_name}` has invalid edit flags"
        )));
    };
    Ok((version > 0).then_some(LinkEdit {
        from,
        to,
        row: LinkRow { deleted, version },
    }))
}

impl EditStore {
    /// Reads one bounded, stable page of M:N edits adjacent to a logical endpoint.
    pub async fn read_link_edits(
        &self,
        active: &ActiveDefinition,
        link_name: &LinkTypeName,
        endpoint: LinkEndpoint,
        after: Option<&LinkScanCursor>,
        limit: usize,
    ) -> Result<LinkScanPage, StoreError> {
        bounded(limit)?;
        let mapping = link_mapping(active, link_name)?;
        let LinkMapping {
            table,
            from_def,
            to_def,
            ..
        } = &mapping;
        let backend = self.backend();
        let (from_expr, from_kind) = read_expr(&table.a.column, &table.a.ty, backend);
        let (to_expr, to_kind) = read_expr(&table.b.column, &table.b.ty, backend);
        let mut select = Query::select();
        select
            .expr(from_expr)
            .expr(to_expr)
            .column(Alias::new("of_deleted"))
            .column(Alias::new("of_version"))
            .from(sql_exit::edit_table(self.schema_edit(), &table.table))
            .and_where(Expr::col(Alias::new("of_version")).gt(0))
            .limit((limit + 1) as u64);
        let (filter_column, filter_type, filter_def, other_column, other_type, other_def, value) =
            match &endpoint {
                LinkEndpoint::From(value) => (
                    &table.a.column,
                    &table.a.ty,
                    from_def,
                    &table.b.column,
                    &table.b.ty,
                    to_def,
                    value,
                ),
                LinkEndpoint::To(value) => (
                    &table.b.column,
                    &table.b.ty,
                    to_def,
                    &table.a.column,
                    &table.a.ty,
                    from_def,
                    value,
                ),
            };
        let key = value.canonical(filter_def).map_err(|e| {
            StoreError::Config(format!(
                "invalid endpoint for logical link `{link_name}`: {e}"
            ))
        })?;
        select
            .and_where(Expr::col(Alias::new(filter_column)).eq(bind(&key, filter_type, backend)))
            .order_by(Alias::new(other_column), Order::Asc)
            .order_by(Alias::new(&table.a.column), Order::Asc)
            .order_by(Alias::new(&table.b.column), Order::Asc);
        if let Some(cursor) = after {
            let (fixed, other) = match &endpoint {
                LinkEndpoint::From(_) => (&cursor.from, &cursor.to),
                LinkEndpoint::To(_) => (&cursor.to, &cursor.from),
            };
            if fixed != &key {
                return Err(StoreError::Config(format!(
                    "cursor does not match logical link `{link_name}` endpoint"
                )));
            }
            let other = other.canonical(other_def).map_err(|e| {
                StoreError::Config(format!(
                    "invalid cursor for logical link `{link_name}`: {e}"
                ))
            })?;
            select.and_where(
                Expr::col(Alias::new(other_column)).gt(bind(&other, other_type, backend)),
            );
        }
        let kinds = [from_kind, to_kind, CellKind::Bool, CellKind::Int64];
        let cells =
            sql_exit::fetch_all_cells_read(&self.db, sql_exit::build_for(&select, backend), &kinds)
                .await
                .map_err(|error| logical_read_error(link_name, error))?;
        let mut edits = cells
            .into_iter()
            .map(|row| decode_link(row, &mapping, link_name))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let has_more = edits.len() > limit;
        edits.truncate(limit);
        let next = edits.last().map(|edit| LinkScanCursor {
            from: edit.from.clone(),
            to: edit.to.clone(),
        });
        Ok(LinkScanPage {
            edits,
            next,
            has_more,
        })
    }

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
