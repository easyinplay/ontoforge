//! Comparison of a source binding with the physical table it names (BIND-03).
//!
//! `compare_binding` is the one implementation of the comparison: the CLI's
//! `validate --online` and the drift check ([`check_drift`]) both call it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use ontoforge_schema::diag::Severity;
use ontoforge_schema::ir::{
    BaseType, BindingTarget, LinkTypeName, ObjectTypeName, Ontology, ProfileName, SourceBinding,
    binding_def_name,
};
use ontoforge_schema::loader::sourcemap::Seg;

use crate::{AnyEngine, Engine, EngineError, PhysicalColumn, PhysicalType, QueryContext, TableRef};

/// Identifies one binding: the profile it belongs to and what it binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingKey {
    pub profile: ProfileName,
    pub target: BindingTarget,
}

impl BindingKey {
    /// The name the binding is registered under in the source map.
    #[must_use]
    pub fn def_name(&self) -> String {
        let target = match &self.target {
            BindingTarget::ObjectType(name) => name.as_str(),
            BindingTarget::LinkType(name) => name.as_str(),
        };
        binding_def_name(&self.profile, target)
    }
}

/// What kind of difference a [`Mismatch`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    TableMissing,
    ColumnMissing,
    TypeIncompatible,
    PrecisionChanged,
    TimezoneMismatch,
    CharPadding,
    CaseCollision,
    UnsupportedType,
}

/// One difference between a binding and its physical table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub kind: MismatchKind,
    pub severity: Severity,
    /// Where to point, relative to the binding document.
    pub path: Vec<Seg>,
    pub message: String,
    pub help: Option<String>,
}

/// A column a binding expects, with the type information the comparison needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedColumn {
    /// The property apiName, or `from` / `to` for a link.
    pub key: String,
    /// The physical column name.
    pub column: String,
    /// Whether the binding writes the column name.
    pub explicit: bool,
    pub base: BaseType,
    pub items: Option<BaseType>,
    pub precision: u32,
    pub scale: u32,
    pub timezone: Option<String>,
}

/// The columns `binding` expects, with the types of the properties they carry.
#[must_use]
pub fn expected_columns(ontology: &Ontology, binding: &SourceBinding) -> Vec<ExpectedColumn> {
    ontology
        .bound_columns(binding)
        .into_iter()
        .map(|bound| {
            let (precision, scale) = bound.property.effective_decimal();
            ExpectedColumn {
                key: bound.key.to_owned(),
                column: bound.column.to_owned(),
                explicit: bound.explicit,
                base: bound.property.base_type,
                items: bound.property.items,
                precision,
                scale,
                timezone: binding.timezone_for(bound.key).map(str::to_owned),
            }
        })
        .collect()
}

/// The type families the comparison tells apart. Rules within a family belong to 02-08.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Integer,
    Float,
    Decimal,
    String,
    Boolean,
    Date,
    Timestamp,
    Array,
    Other,
}

impl Family {
    fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Float => "floating point",
            Self::Decimal => "decimal",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Timestamp => "timestamp",
            Self::Array => "array",
            Self::Other => "other",
        }
    }
}

/// The family a property type is stored as. A geopoint is stored as text.
fn expected_family(base: BaseType) -> Family {
    match base {
        BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => Family::Integer,
        BaseType::Float | BaseType::Double => Family::Float,
        BaseType::Decimal => Family::Decimal,
        BaseType::String | BaseType::Geopoint => Family::String,
        BaseType::Boolean => Family::Boolean,
        BaseType::Date => Family::Date,
        BaseType::Timestamp => Family::Timestamp,
        BaseType::Array => Family::Array,
    }
}

fn physical_family(physical: &PhysicalType) -> Family {
    match physical {
        PhysicalType::Tinyint
        | PhysicalType::Smallint
        | PhysicalType::Integer
        | PhysicalType::Bigint => Family::Integer,
        PhysicalType::Real | PhysicalType::Double => Family::Float,
        PhysicalType::Decimal { .. } | PhysicalType::Number => Family::Decimal,
        PhysicalType::Varchar { .. } | PhysicalType::Char { .. } => Family::String,
        PhysicalType::Boolean => Family::Boolean,
        PhysicalType::Date => Family::Date,
        PhysicalType::Timestamp { .. } => Family::Timestamp,
        PhysicalType::Array(_) => Family::Array,
        PhysicalType::Other(_) => Family::Other,
    }
}

fn table_path() -> Vec<Seg> {
    vec![Seg::Key("source".to_owned()), Seg::Key("table".to_owned())]
}

/// Where a problem with `column` is pointed at: the written column name when there is one,
/// otherwise the table.
fn column_path(column: &ExpectedColumn) -> Vec<Seg> {
    if column.explicit {
        vec![
            Seg::Key("columns".to_owned()),
            Seg::Key(column.key.clone()),
            Seg::Key("column".to_owned()),
        ]
    } else {
        table_path()
    }
}

/// Compares `binding` with `physical`: the columns of its table, or `None` when the table does
/// not exist.
///
/// Column names are compared in lower case, because Trino exposes identifiers in lower case.
/// Messages carry only apiNames and physical names, never data.
#[must_use]
pub fn compare_binding(
    ontology: &Ontology,
    binding: &SourceBinding,
    physical: Option<&[PhysicalColumn]>,
) -> Vec<Mismatch> {
    let source = &binding.source;
    let table = format!("{}.{}.{}", source.catalog, source.schema, source.table);
    let Some(physical) = physical else {
        return vec![Mismatch {
            kind: MismatchKind::TableMissing,
            severity: Severity::Error,
            path: table_path(),
            message: format!(
                "table `{table}` bound to `{}` does not exist",
                binding.target_name()
            ),
            help: Some("create the table or view, or correct `source` in the binding".to_owned()),
        }];
    };
    let what = match &binding.target {
        BindingTarget::ObjectType(_) => "property",
        BindingTarget::LinkType(_) => "end",
    };
    let mut by_name: HashMap<String, &PhysicalColumn> = HashMap::new();
    for column in physical {
        by_name.entry(column.name.to_lowercase()).or_insert(column);
    }

    let mut found = Vec::new();
    for expected in expected_columns(ontology, binding) {
        let subject = format!(
            "{what} `{}` of `{}` maps to column `{}`",
            expected.key,
            binding.target_name(),
            expected.column
        );
        let Some(actual) = by_name.get(&expected.column.to_lowercase()) else {
            found.push(Mismatch {
                kind: MismatchKind::ColumnMissing,
                severity: Severity::Error,
                path: column_path(&expected),
                message: format!("{subject}, but table `{table}` has no such column"),
                help: Some(
                    "add the column, or map it to an existing one with `columns.<key>.column`"
                        .to_owned(),
                ),
            });
            continue;
        };
        let want = expected_family(expected.base);
        let have = physical_family(&actual.physical);
        if want != have {
            found.push(Mismatch {
                kind: MismatchKind::TypeIncompatible,
                severity: Severity::Error,
                path: column_path(&expected),
                message: format!(
                    "{subject}, which is `{}` ({} family) but the {what} is a {} value",
                    actual.physical.render(),
                    have.name(),
                    want.name()
                ),
                help: Some(
                    "change the column type, or map the property to a column of a matching type"
                        .to_owned(),
                ),
            });
        }
    }
    found
}

/// A binding with the description of its table.
#[derive(Debug)]
pub struct DescribedBinding {
    pub key: BindingKey,
    pub table: TableRef,
    /// `Err` only for an error that is not [`EngineError::is_engine_wide`].
    pub physical: Result<Option<Vec<PhysicalColumn>>, EngineError>,
}

type Described = BTreeMap<TableRef, Vec<PhysicalColumn>>;

/// Describes the tables of every binding of `profile`, one request per catalog.
///
/// An error that is not engine wide ([`EngineError::is_engine_wide`]) is copied onto every
/// binding of the catalog it came from and the other catalogs are still described. An engine
/// wide error aborts the round.
pub async fn describe_bindings(
    engine: &AnyEngine,
    ontology: &Ontology,
    profile: &ProfileName,
    ctx: &QueryContext,
) -> Result<Vec<DescribedBinding>, EngineError> {
    let Some(bindings) = ontology.profile_bindings(profile) else {
        return Ok(Vec::new());
    };
    let entries: Vec<(BindingKey, TableRef)> = bindings
        .objects
        .values()
        .chain(bindings.links.values())
        .map(|binding| {
            let key = BindingKey {
                profile: profile.clone(),
                target: binding.target.clone(),
            };
            let table = TableRef {
                catalog: binding.source.catalog.clone(),
                schema: binding.source.schema.clone(),
                table: binding.source.table.clone(),
            };
            (key, table)
        })
        .collect();

    let mut by_catalog: BTreeMap<&str, BTreeSet<&TableRef>> = BTreeMap::new();
    for (_, table) in &entries {
        by_catalog.entry(&table.catalog).or_default().insert(table);
    }
    let mut results: HashMap<String, Result<Described, EngineError>> = HashMap::new();
    for (catalog, tables) in by_catalog {
        let tables: Vec<TableRef> = tables.into_iter().cloned().collect();
        match engine.describe_tables(&tables, ctx).await {
            Ok(described) => {
                results.insert(catalog.to_owned(), Ok(described));
            }
            Err(e) if e.is_engine_wide() => return Err(e),
            Err(e) => {
                results.insert(catalog.to_owned(), Err(e));
            }
        }
    }

    Ok(entries
        .into_iter()
        .map(|(key, table)| {
            let physical = match results.get(table.catalog.as_str()) {
                Some(Ok(described)) => Ok(described.get(&table).cloned()),
                Some(Err(e)) => Err(e.clone()),
                None => Ok(None),
            };
            DescribedBinding {
                key,
                table,
                physical,
            }
        })
        .collect())
}

/// Why a binding target is degraded (BIND-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradeReason {
    /// The physical table no longer matches the binding.
    SourceSchemaDrift,
    /// The table's metadata could not be read; `error` is the engine error's text.
    SourceUnreadable { error: String },
}

impl DegradeReason {
    /// Stable upper-snake code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::SourceSchemaDrift => "SOURCE_SCHEMA_DRIFT",
            Self::SourceUnreadable { .. } => "SOURCE_UNREADABLE",
        }
    }
}

/// The state of one bound object type or link type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetHealth {
    /// No error: the warnings, if any, are kept for the report.
    Healthy { warnings: Vec<Mismatch> },
    Degraded {
        reason: DegradeReason,
        mismatches: Vec<Mismatch>,
    },
}

/// The outcome of one drift check of one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    pub profile: ProfileName,
    pub objects: BTreeMap<ObjectTypeName, TargetHealth>,
    pub links: BTreeMap<LinkTypeName, TargetHealth>,
}

impl DriftReport {
    /// Whether every target is [`TargetHealth::Healthy`].
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.objects
            .values()
            .chain(self.links.values())
            .all(|health| matches!(health, TargetHealth::Healthy { .. }))
    }
}

/// The health of a target whose comparison found `mismatches`: any error is drift, warnings
/// alone leave the target healthy and are kept (E12).
fn health_of(mismatches: Vec<Mismatch>) -> TargetHealth {
    if mismatches.iter().any(|m| m.severity == Severity::Error) {
        TargetHealth::Degraded {
            reason: DegradeReason::SourceSchemaDrift,
            mismatches,
        }
    } else {
        TargetHealth::Healthy {
            warnings: mismatches,
        }
    }
}

/// Judges every described binding of `profile`.
///
/// A binding whose table could not be read is degraded on its own (E3); the others are
/// compared with [`compare_binding`], the same comparison `validate --online` uses (D-15).
#[must_use]
pub fn drift_report(
    ontology: &Ontology,
    profile: &ProfileName,
    described: &[DescribedBinding],
) -> DriftReport {
    let bindings = ontology.profile_bindings(profile);
    let mut report = DriftReport {
        profile: profile.clone(),
        objects: BTreeMap::new(),
        links: BTreeMap::new(),
    };
    for entry in described {
        let health = match &entry.physical {
            Err(e) => TargetHealth::Degraded {
                reason: DegradeReason::SourceUnreadable {
                    error: e.to_string(),
                },
                mismatches: Vec::new(),
            },
            Ok(physical) => {
                let binding = bindings.and_then(|b| match &entry.key.target {
                    BindingTarget::ObjectType(name) => b.objects.get(name),
                    BindingTarget::LinkType(name) => b.links.get(name),
                });
                let Some(binding) = binding else {
                    // `described` comes from this profile's bindings, so this is not reachable.
                    continue;
                };
                health_of(compare_binding(ontology, binding, physical.as_deref()))
            }
        };
        match &entry.key.target {
            BindingTarget::ObjectType(name) => report.objects.insert(name.clone(), health),
            BindingTarget::LinkType(name) => report.links.insert(name.clone(), health),
        };
    }
    report
}

/// Describes the bindings of `profile` and judges them (BIND-03).
///
/// Only an engine wide error ([`EngineError::is_engine_wide`]: unavailable, authentication
/// failure and the like) is returned as `Err`; a report is never made from an engine that could
/// not answer, because engine health and schema drift are different things. One target whose
/// metadata cannot be read is [`DegradeReason::SourceUnreadable`] and the others are judged as
/// usual.
///
/// Starting and scheduling the check, and exposing the result on `/health`, belong to the
/// service process of Phase 7 (D-15). The duplicate primary key check (`--check-pk`) is not
/// drift: ADR-0015 lets it degrade a type only when asked for explicitly, so drift never scans
/// a table.
pub async fn check_drift(
    engine: &AnyEngine,
    ontology: &Ontology,
    profile: &ProfileName,
    ctx: &QueryContext,
) -> Result<DriftReport, EngineError> {
    let described = describe_bindings(engine, ontology, profile, ctx).await?;
    Ok(drift_report(ontology, profile, &described))
}

#[cfg(test)]
mod tests;
