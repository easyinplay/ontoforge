//! Difference between two physical schemas (STORE-02).
//!
//! A change that only adds a type, a many-to-many link or an optional property becomes DDL.
//! Everything that could lose or reinterpret stored data is a [`DestructiveChange`]: `reload`
//! refuses it and points at `ontoforge migrate`. The definition has no property defaults, so a
//! new `required` property is destructive too: existing rows could not satisfy it. A rename shows
//! up as a removal plus an addition (D-14).

use ontoforge_schema::diag::{Diagnostic, Severity, codes};

use crate::ddl::DdlStep;
use crate::physical::{LinkTable, ObjectTable, PhysicalSchema};

/// What `reload` would do, and what it refuses to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<DdlStep>,
    pub destructive: Vec<DestructiveChange>,
}

/// A definition change that would lose or reinterpret data. `reload` never applies one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestructiveChange {
    ObjectTypeRemoved { type_name: String },
    LinkTypeRemoved { link: String },
    PropertyRemoved { type_name: String, property: String },
    PropertyTypeChanged { type_name: String, property: String },
    PrimaryKeyChanged { type_name: String },
    RequiredPropertyAdded { type_name: String, property: String },
    LinkEndsChanged { link: String },
}

impl DestructiveChange {
    /// The ONT-V25 diagnostic of this change. It has no source position: the change is the
    /// difference between two definitions, not a spot in one file.
    pub fn diagnostic(&self) -> Diagnostic {
        let message = match self {
            Self::ObjectTypeRemoved { type_name } => {
                format!("object type `{type_name}` was removed")
            }
            Self::LinkTypeRemoved { link } => format!("link type `{link}` was removed"),
            Self::PropertyRemoved {
                type_name,
                property,
            } => format!("property `{type_name}.{property}` was removed"),
            Self::PropertyTypeChanged {
                type_name,
                property,
            } => format!("the type of property `{type_name}.{property}` changed"),
            Self::PrimaryKeyChanged { type_name } => {
                format!("the primary key of `{type_name}` changed")
            }
            Self::RequiredPropertyAdded {
                type_name,
                property,
            } => format!(
                "required property `{type_name}.{property}` was added; existing objects have no value for it"
            ),
            Self::LinkEndsChanged { link } => {
                format!("the object types at the ends of link type `{link}` changed")
            }
        };
        Diagnostic::without_location(
            codes::V25,
            Severity::Error,
            format!("destructive definition change: {message}"),
            "ontology",
        )
        .with_help("run `ontoforge migrate --plan` to review the migration")
    }
}

/// Compares the active physical schema `old` (none before the first reload) with `new`.
pub fn diff(old: Option<&PhysicalSchema>, new: &PhysicalSchema) -> Plan {
    let mut plan = Plan::default();
    let Some(old) = old else {
        plan.steps.extend(
            new.objects
                .values()
                .map(|table| DdlStep::CreateObjectTable(table.clone())),
        );
        plan.steps.extend(
            new.links
                .values()
                .map(|table| DdlStep::CreateLinkTable(table.clone())),
        );
        return plan;
    };

    for (name, table) in &new.objects {
        match old.objects.get(name) {
            None => plan.steps.push(DdlStep::CreateObjectTable(table.clone())),
            Some(previous) => diff_object(name.as_str(), previous, table, &mut plan),
        }
    }
    for name in old.objects.keys().filter(|n| !new.objects.contains_key(*n)) {
        plan.destructive.push(DestructiveChange::ObjectTypeRemoved {
            type_name: name.to_string(),
        });
    }

    for (name, table) in &new.links {
        match old.links.get(name) {
            None => plan.steps.push(DdlStep::CreateLinkTable(table.clone())),
            Some(previous) => {
                if link_ends_changed(previous, table) {
                    plan.destructive.push(DestructiveChange::LinkEndsChanged {
                        link: name.to_string(),
                    });
                }
            }
        }
    }
    for name in old.links.keys().filter(|n| !new.links.contains_key(*n)) {
        plan.destructive.push(DestructiveChange::LinkTypeRemoved {
            link: name.to_string(),
        });
    }
    plan
}

fn diff_object(type_name: &str, old: &ObjectTable, new: &ObjectTable, plan: &mut Plan) {
    if old.pk.property != new.pk.property || old.pk.ty != new.pk.ty {
        plan.destructive.push(DestructiveChange::PrimaryKeyChanged {
            type_name: type_name.to_owned(),
        });
    }
    for (name, column) in &new.properties {
        match old.properties.get(name) {
            None if column.required => {
                plan.destructive
                    .push(DestructiveChange::RequiredPropertyAdded {
                        type_name: type_name.to_owned(),
                        property: name.to_string(),
                    });
            }
            None => plan.steps.push(DdlStep::AddPropertyColumn {
                table: new.table.clone(),
                column: column.clone(),
            }),
            Some(previous) if previous.ty != column.ty => {
                plan.destructive
                    .push(DestructiveChange::PropertyTypeChanged {
                        type_name: type_name.to_owned(),
                        property: name.to_string(),
                    });
            }
            Some(_) => {}
        }
    }
    for name in old
        .properties
        .keys()
        .filter(|n| !new.properties.contains_key(*n))
    {
        plan.destructive.push(DestructiveChange::PropertyRemoved {
            type_name: type_name.to_owned(),
            property: name.to_string(),
        });
    }
}

fn link_ends_changed(old: &LinkTable, new: &LinkTable) -> bool {
    old.from_type != new.from_type || old.to_type != new.to_type
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_is_v25_with_migrate_help() {
        let d = DestructiveChange::PropertyRemoved {
            type_name: "Gate".to_owned(),
            property: "name".to_owned(),
        }
        .diagnostic();
        assert_eq!(d.code, "ONT-V25");
        assert!(d.message.contains("`Gate.name`"), "{}", d.message);
        assert!(
            d.help
                .as_deref()
                .is_some_and(|h| h.contains("migrate --plan")),
            "{:?}",
            d.help
        );
    }
}
