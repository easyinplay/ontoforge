use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::ActionRequest;

pub(crate) fn request_hash(request: &ActionRequest) -> String {
    digest(&json!({
        "format": 1,
        "action": request.action,
        "items": request.items,
        "validate_only": request.validate_only,
    }))
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            let mut ordered = Map::new();
            for key in keys {
                ordered.insert(key.clone(), canonical_json(&map[key]));
            }
            Value::Object(ordered)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

pub(crate) fn digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(&canonical_json(value)).expect("JSON values serialize");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::request_hash;
    use crate::ActionRequest;

    #[test]
    fn recursive_keys_are_canonical_but_arrays_keep_order() {
        let first: ActionRequest = serde_json::from_str(
            r#"{"action":"a","commandId":"one","items":[{"params":{"nested":{"z":1,"a":[{"y":2,"x":3},4]}}}]}"#,
        ).unwrap();
        let second: ActionRequest = serde_json::from_str(
            r#"{"action":"a","commandId":"two","items":[{"params":{"nested":{"a":[{"x":3,"y":2},4],"z":1}}}]}"#,
        ).unwrap();
        assert_eq!(request_hash(&first), request_hash(&second));
        let mut reordered = second;
        reordered.items[0].params["nested"]["a"] = serde_json::json!([4, {"x":3,"y":2}]);
        assert_ne!(request_hash(&first), request_hash(&reordered));
    }
}
