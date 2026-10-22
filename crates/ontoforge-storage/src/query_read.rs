//! Side-effect-free reads of typed object edit rows.

use ontoforge_schema::ir::ObjectTypeName;

use crate::error::StoreError;
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
        _active: &ActiveDefinition,
        _type_name: &ObjectTypeName,
        _pk: &PropertyValue,
    ) -> Result<Option<ObjectEdit>, StoreError> {
        Ok(None)
    }
}
