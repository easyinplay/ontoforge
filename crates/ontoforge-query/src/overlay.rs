use std::collections::BTreeMap;

use ontoforge_schema::ir::PropertyName;
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::value::PropertyValue;
use serde_json::Value;

use crate::QueryError;

pub fn overlay_object(
    source: Option<&BTreeMap<PropertyName, Value>>,
    _edit: Option<&ObjectEdit>,
    _pk_name: &PropertyName,
    _pk: &PropertyValue,
) -> Result<Option<BTreeMap<PropertyName, Value>>, QueryError> {
    Ok(source.cloned())
}
