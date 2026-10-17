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

/// What comparing one expected column with the physical column it reads came to.
///
/// Only lossless readings are accepted (D-13): anything that could drop or reinterpret a value
/// is an error, and a reading that is lossless but not exact is a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Compat {
    Ok,
    Warn {
        kind: MismatchKind,
        reason: String,
        help: Option<String>,
    },
    Error {
        kind: MismatchKind,
        reason: String,
        help: Option<String>,
    },
}

const CAST_HELP: &str =
    "create a view in Trino that casts the column to a supported type and bind the view";

impl Compat {
    fn error(kind: MismatchKind, reason: impl Into<String>, help: &str) -> Self {
        Self::Error {
            kind,
            reason: reason.into(),
            help: Some(help.to_owned()),
        }
    }

    fn warn(kind: MismatchKind, reason: impl Into<String>, help: &str) -> Self {
        Self::Warn {
            kind,
            reason: reason.into(),
            help: Some(help.to_owned()),
        }
    }

    /// The column is not of the kind of type the property needs.
    fn family(want: &str, physical: &PhysicalType) -> Self {
        Self::error(
            MismatchKind::TypeIncompatible,
            format!(
                "a {want} property cannot be read from a {} column",
                family_name(physical)
            ),
            "change the column type, or map the property to a column of a matching type",
        )
    }

    /// A type this comparison has no lossless reading for.
    fn unsupported(reason: impl Into<String>) -> Self {
        Self::error(MismatchKind::UnsupportedType, reason, CAST_HELP)
    }
}

/// The kind of a physical type, for messages.
fn family_name(physical: &PhysicalType) -> &'static str {
    match physical {
        PhysicalType::Tinyint
        | PhysicalType::Smallint
        | PhysicalType::Integer
        | PhysicalType::Bigint => "integer",
        PhysicalType::Real | PhysicalType::Double => "floating point",
        PhysicalType::Decimal { .. } | PhysicalType::Number => "decimal",
        PhysicalType::Varchar { .. } | PhysicalType::Char { .. } => "string",
        PhysicalType::Boolean => "boolean",
        PhysicalType::Date => "date",
        PhysicalType::Timestamp { .. } => "timestamp",
        PhysicalType::Array(_) => "array",
        PhysicalType::Other(_) => "unsupported",
    }
}

/// How the property is written in the definition: `integer`, `decimal(12,2)`, `array<string>`.
fn declared(expected: &ExpectedColumn) -> String {
    let scalar = |base: BaseType| match base {
        BaseType::Decimal => format!("decimal({},{})", expected.precision, expected.scale),
        other => other.as_str().to_owned(),
    };
    match (expected.base, expected.items) {
        (BaseType::Array, Some(items)) => format!("array<{}>", scalar(items)),
        (base, _) => scalar(base),
    }
}

/// Rank of an integer property type; a column may be read when its rank is not larger.
fn integer_rank(base: BaseType) -> Option<u8> {
    match base {
        BaseType::Byte => Some(1),
        BaseType::Short => Some(2),
        BaseType::Integer => Some(3),
        BaseType::Long => Some(4),
        _ => None,
    }
}

fn physical_integer_rank(physical: &PhysicalType) -> Option<u8> {
    match physical {
        PhysicalType::Tinyint => Some(1),
        PhysicalType::Smallint => Some(2),
        PhysicalType::Integer => Some(3),
        PhysicalType::Bigint => Some(4),
        _ => None,
    }
}

/// Whether the column `physical` can be read as the property `expected` without loss (D-13,
/// ADR-0021). Arrays are compared by element.
///
/// `expected.precision` / `expected.scale` are already the effective values of the property
/// (`PropertyDef::effective_decimal`); no default is applied here.
fn compatibility(expected: &ExpectedColumn, physical: &PhysicalType) -> Compat {
    if expected.base != BaseType::Array {
        return element_compatibility(expected.base, expected, physical);
    }
    let Some(element) = expected.items else {
        return Compat::unsupported("an array property without an element type");
    };
    match physical {
        PhysicalType::Array(inner) => element_compatibility(element, expected, inner),
        PhysicalType::Other(_) => element_compatibility(element, expected, physical),
        other => Compat::family("array", other),
    }
}

fn element_compatibility(
    base: BaseType,
    expected: &ExpectedColumn,
    physical: &PhysicalType,
) -> Compat {
    if base == BaseType::Geopoint {
        return Compat::unsupported(
            "a geopoint property cannot be read from a source column; no lossless mapping exists",
        );
    }
    if let PhysicalType::Other(text) = physical {
        return Compat::unsupported(format!("the column type `{text}` is not supported"));
    }
    if let Some(rank) = integer_rank(base) {
        let Some(have) = physical_integer_rank(physical) else {
            return Compat::family("integer", physical);
        };
        return if have <= rank {
            Compat::Ok
        } else {
            Compat::error(
                MismatchKind::TypeIncompatible,
                format!(
                    "`{}` is wider than {}, so values could be lost",
                    physical.render(),
                    base.as_str()
                ),
                "widen the property type, or create a view in Trino that casts the column",
            )
        };
    }
    match base {
        BaseType::Float | BaseType::Double => match physical {
            PhysicalType::Real => Compat::Ok,
            PhysicalType::Double if base == BaseType::Double => Compat::Ok,
            PhysicalType::Double => Compat::error(
                MismatchKind::TypeIncompatible,
                "`double` is wider than float, so values could be rounded",
                "declare the property as double",
            ),
            other => Compat::family("floating point", other),
        },
        BaseType::Decimal => decimal_compatibility(expected, physical),
        BaseType::String => match physical {
            PhysicalType::Varchar { .. } => Compat::Ok,
            PhysicalType::Char { .. } => Compat::warn(
                MismatchKind::CharPadding,
                "`char` columns pad values with trailing spaces, which the property would keep",
                "use a varchar column, or a view that trims the value",
            ),
            other => Compat::family("string", other),
        },
        BaseType::Boolean => match physical {
            PhysicalType::Boolean => Compat::Ok,
            other => Compat::family("boolean", other),
        },
        BaseType::Date => match physical {
            PhysicalType::Date => Compat::Ok,
            other => Compat::family("date", other),
        },
        BaseType::Timestamp => match physical {
            PhysicalType::Timestamp { with_time_zone, .. } => {
                timezone_compatibility(expected.timezone.as_deref(), *with_time_zone)
            }
            other => Compat::family("timestamp", other),
        },
        // Integers were handled above; geopoint above; an array element cannot be an array.
        BaseType::Array
        | BaseType::Geopoint
        | BaseType::Byte
        | BaseType::Short
        | BaseType::Integer
        | BaseType::Long => {
            Compat::unsupported("this property type has no lossless column mapping")
        }
    }
}

/// `decimal(p, s)` fits `decimal(P, S)` exactly when `s <= S` and `p - s <= P - S`: neither the
/// fraction nor the integer digits can be lost. The integer part matters, because
/// `decimal(10, 2)` does not fit `decimal(10, 5)`.
fn decimal_compatibility(expected: &ExpectedColumn, physical: &PhysicalType) -> Compat {
    let (want_p, want_s) = (i64::from(expected.precision), i64::from(expected.scale));
    match physical {
        PhysicalType::Decimal { precision, scale } => {
            let (p, s) = (i64::from(*precision), i64::from(*scale));
            let name = physical.render();
            if s > want_s {
                Compat::error(
                    MismatchKind::TypeIncompatible,
                    format!(
                        "`{name}` has {s} fraction digits but the property keeps {want_s}, so digits could be lost"
                    ),
                    "raise `scale` on the property, or create a view in Trino that rounds the column",
                )
            } else if p - s > want_p - want_s {
                Compat::error(
                    MismatchKind::TypeIncompatible,
                    format!(
                        "`{name}` has {} integer digits but the property holds {}, so values could overflow",
                        p - s,
                        want_p - want_s
                    ),
                    "raise `precision` on the property, or create a view in Trino that narrows the column",
                )
            } else if p < want_p || s < want_s {
                Compat::warn(
                    MismatchKind::PrecisionChanged,
                    format!("`{name}` is narrower than the declared decimal({want_p},{want_s})"),
                    "align `precision` and `scale` on the property with the column",
                )
            } else {
                Compat::Ok
            }
        }
        PhysicalType::Number => Compat::error(
            MismatchKind::UnsupportedType,
            "`number` is an unconstrained numeric with no fixed precision (ADR-0021)",
            "create a view in Trino that casts the column to a decimal(p, s) and bind the view",
        ),
        other => Compat::family("decimal", other),
    }
}

/// The online recheck of ADR-0021: the declared zone must agree with whether the column has
/// one. `declared` is the binding's `timezone`: an IANA name, `aware`, or nothing.
fn timezone_compatibility(declared: Option<&str>, column_has_zone: bool) -> Compat {
    match (declared, column_has_zone) {
        (Some("aware"), true) => Compat::Ok,
        (Some("aware"), false) => Compat::error(
            MismatchKind::TimezoneMismatch,
            "the binding says the column is time zone aware, but it has no time zone",
            "declare the IANA zone the values are written in, for example `timezone: Europe/Berlin`",
        ),
        (Some(zone), true) => Compat::warn(
            MismatchKind::TimezoneMismatch,
            format!(
                "the binding declares zone `{zone}`, but the column already carries a time zone"
            ),
            "write `timezone: aware`; the declared zone would not be applied",
        ),
        (Some(_), false) => Compat::Ok,
        (None, false) => Compat::error(
            MismatchKind::TimezoneMismatch,
            "the column has no time zone and the binding declares none",
            "declare `timezone` under `columns` in the binding: an IANA name, or `aware`",
        ),
        (None, true) => Compat::warn(
            MismatchKind::TimezoneMismatch,
            "the column carries a time zone but the binding does not say so",
            "write `timezone: aware` under `columns` in the binding",
        ),
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
/// Column names are compared in lower case, because Trino exposes identifiers in lower case. A
/// bound column with several physical candidates that differ only in case is an error, since
/// the choice would be arbitrary. Messages carry only apiNames and physical names, never data.
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
    let mut by_name: HashMap<String, Vec<&PhysicalColumn>> = HashMap::new();
    for column in physical {
        by_name
            .entry(column.name.to_lowercase())
            .or_default()
            .push(column);
    }

    let mut found = Vec::new();
    for expected in expected_columns(ontology, binding) {
        let subject = format!(
            "{what} `{}` of `{}` maps to column `{}`",
            expected.key,
            binding.target_name(),
            expected.column
        );
        let candidates = by_name
            .get(&expected.column.to_lowercase())
            .map_or(&[][..], Vec::as_slice);
        let actual = match candidates {
            [] => {
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
            }
            [one] => *one,
            many => {
                let names: Vec<String> = many.iter().map(|c| format!("`{}`", c.name)).collect();
                found.push(Mismatch {
                    kind: MismatchKind::CaseCollision,
                    severity: Severity::Error,
                    path: column_path(&expected),
                    message: format!(
                        "{subject}, but table `{table}` has columns {} that differ only in case, so the column is ambiguous",
                        names.join(" and ")
                    ),
                    help: Some(
                        "create a view in Trino that exposes one of them under a unique name and bind the view"
                            .to_owned(),
                    ),
                });
                continue;
            }
        };
        let (kind, severity, reason, help) = match compatibility(&expected, &actual.physical) {
            Compat::Ok => continue,
            Compat::Warn { kind, reason, help } => (kind, Severity::Warning, reason, help),
            Compat::Error { kind, reason, help } => (kind, Severity::Error, reason, help),
        };
        found.push(Mismatch {
            kind,
            severity,
            path: column_path(&expected),
            message: format!(
                "{what} `{}` ({}) of `{}` reads column `{}`, which is `{}`: {reason}",
                expected.key,
                declared(&expected),
                binding.target_name(),
                expected.column,
                actual.physical.render(),
            ),
            help,
        });
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
    /// The table's metadata could not be read. `code` and `error_name` may be shown to any
    /// caller (the `/health` of Phase 7); `detail` may not (WR-E07).
    SourceUnreadable {
        /// [`EngineError::code`] of the failure, e.g. `ENGINE_QUERY_FAILED`.
        code: &'static str,
        /// The error name the engine reported (Trino `errorName`, e.g. `PERMISSION_DENIED`),
        /// kept only when it has the shape of one: 1 to 64 of `[A-Z0-9_]`, starting with a
        /// letter.
        error_name: Option<String>,
        /// The engine error text, for logs and operators only.
        detail: OperatorDetail,
    },
}

/// Most bytes of engine error text an [`OperatorDetail`] keeps.
pub const MAX_DETAIL_BYTES: usize = 2048;

/// Engine error text for logs and operator tools only. A Trino message can name internal
/// hosts, metastore and storage paths, or quote data values, so it is kept apart from the
/// exposable fields of [`DegradeReason`]: `Debug` prints only its length, and the text is read
/// with [`OperatorDetail::text`]. At most [`MAX_DETAIL_BYTES`] bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct OperatorDetail(String);

impl OperatorDetail {
    /// Keeps `text`, cut to [`MAX_DETAIL_BYTES`] at a character boundary.
    #[must_use]
    pub fn new(text: &str) -> Self {
        if text.len() <= MAX_DETAIL_BYTES {
            return Self(text.to_owned());
        }
        let mut end = MAX_DETAIL_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Self(format!("{}...", &text[..end]))
    }

    /// The text. Not for responses to API callers.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for OperatorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OperatorDetail(<{} bytes>)", self.0.len())
    }
}

/// The exposable shape of an engine failure on one target.
fn unreadable(e: &EngineError) -> DegradeReason {
    let error_name = match e {
        EngineError::Query { error_name, .. } if is_error_name(error_name) => {
            Some(error_name.clone())
        }
        _ => None,
    };
    DegradeReason::SourceUnreadable {
        code: e.code(),
        error_name,
        detail: OperatorDetail::new(&e.to_string()),
    }
}

/// Whether `s` looks like an engine error name such as `PERMISSION_DENIED`.
fn is_error_name(s: &str) -> bool {
    s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_uppercase())
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
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
                reason: unreadable(e),
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

#[cfg(test)]
mod compat_tests;
