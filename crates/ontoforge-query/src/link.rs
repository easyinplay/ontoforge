use std::collections::BTreeMap;

use ontoforge_storage::query_read::LinkEdit;
use ontoforge_storage::value::PropertyValue;

use crate::QueryError;

#[derive(Debug, Clone, PartialEq)]
pub struct LinkEdge {
    pub from: PropertyValue,
    pub to: PropertyValue,
}

pub fn merge_edges(source: Vec<LinkEdge>, edits: &[LinkEdit]) -> Result<Vec<LinkEdge>, QueryError> {
    let mut live = BTreeMap::new();
    for edge in source {
        live.insert(edge_key(&edge), edge);
    }
    for edit in edits {
        if edit.row.version <= 0 {
            return Err(QueryError::Store);
        }
        let edge = LinkEdge {
            from: edit.from.clone(),
            to: edit.to.clone(),
        };
        if edit.row.deleted {
            live.remove(&edge_key(&edge));
        } else {
            live.insert(edge_key(&edge), edge);
        }
    }
    Ok(live.into_values().collect())
}

fn edge_key(edge: &LinkEdge) -> (String, String) {
    (edge.from.to_json().to_string(), edge.to.to_json().to_string())
}
