//! Index reconciliation for the properties row predicates read (plan 03-16).
//!
//! A role's `rowFilter` turns into a `WHERE` clause on a property, so the property needs an index
//! (orchestrator decision 8). `scope apply` and, later, `reload` ask here which plain indexes the
//! row filters need. Indexes are only ever added: dropping a role never drops an index.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_schema::ir::{ObjectTypeName, PropertyName};

use crate::ddl::DdlStep;
use crate::naming;
use crate::physical::{IndexDef, IndexKind, PhysicalSchema};

/// `CREATE INDEX IF NOT EXISTS` steps for the value column of every property in `props`.
///
/// The primary key is skipped (it has an index already), and so is a property that is not in
/// `physical` (the caller checked the scope against the ontology the tables follow) and one that
/// `physical` already indexes under the same name. The steps come in the order of type and
/// property names.
pub(crate) fn role_index_steps(
    physical: &PhysicalSchema,
    props: &BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>,
) -> Vec<DdlStep> {
    let mut steps = Vec::new();
    for (type_name, properties) in props {
        let Some(table) = physical.objects.get(type_name) else {
            continue;
        };
        for property in properties {
            if table.pk.property == *property {
                continue;
            }
            let Some(column) = table.properties.get(property) else {
                continue;
            };
            let name = naming::index_name(&table.table, &column.column);
            if table.indexes.iter().any(|index| index.name == name) {
                continue;
            }
            steps.push(DdlStep::CreateIndex {
                table: table.table.clone(),
                index: IndexDef {
                    name,
                    column: column.column.clone(),
                    kind: IndexKind::Plain,
                },
            });
        }
    }
    steps
}
