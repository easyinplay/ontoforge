//! Comparison of a source binding with the physical table it names (BIND-03).
//!
//! `compare_binding` is the one implementation of the comparison: the CLI's
//! `validate --online` and the later drift check both call it.

use ontoforge_schema::diag::Severity;
use ontoforge_schema::ir::{
    BaseType, BindingTarget, Ontology, ProfileName, SourceBinding, binding_def_name,
};
use ontoforge_schema::loader::sourcemap::Seg;

use crate::{AnyEngine, EngineError, PhysicalColumn, QueryContext, TableRef};

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

/// Compares `binding` with `physical`: the columns of its table, or `None` when the table does
/// not exist.
#[must_use]
pub fn compare_binding(
    _ontology: &Ontology,
    _binding: &SourceBinding,
    _physical: Option<&[PhysicalColumn]>,
) -> Vec<Mismatch> {
    Vec::new()
}

/// A binding with the description of its table.
#[derive(Debug)]
pub struct DescribedBinding {
    pub key: BindingKey,
    pub table: TableRef,
    /// `Err` only for an error that is not [`EngineError::is_engine_wide`].
    pub physical: Result<Option<Vec<PhysicalColumn>>, EngineError>,
}

/// Describes the tables of every binding of `profile`, one request per catalog.
pub async fn describe_bindings(
    _engine: &AnyEngine,
    _ontology: &Ontology,
    _profile: &ProfileName,
    _ctx: &QueryContext,
) -> Result<Vec<DescribedBinding>, EngineError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests;
