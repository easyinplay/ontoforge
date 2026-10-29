use serde_json::{json, Value};

use ontoforge_api::dto::{ActionRequest, ObjectSet, WhereClause};

fn checked_in_openapi() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/openapi.json");
    let text = std::fs::read_to_string(path).expect("checked-in OpenAPI document");
    serde_json::from_str(&text).expect("valid OpenAPI JSON")
}

#[test]
fn openapi_31_all_operations_and_security() {
    let document = checked_in_openapi();
    assert_eq!(document["openapi"], "3.1.0");
    let paths = document["paths"].as_object().expect("paths object");
    for path in [
        "/api/ontology",
        "/api/objects/{object_type}/{pk}",
        "/api/query",
        "/api/aggregate",
        "/api/actions/{action}",
        "/api/commands/{command_id}",
        "/api/objects/{object_type}/{pk}/history",
    ] {
        assert!(paths.contains_key(path), "missing path {path}");
    }
    assert!(document["components"]["securitySchemes"]["bearerAuth"].is_object());
}

#[test]
fn openapi_recursive_nullable_required_and_unknown_fields() {
    let nested = ObjectSet::Where {
        object_type: "文档".into(),
        predicate: WhereClause::And {
            clauses: vec![WhereClause::IsNull {
                property: "备注".into(),
            }],
        },
    };
    let encoded = serde_json::to_value(&nested).expect("serialize recursive union");
    assert_eq!(encoded["kind"], "where");
    assert_eq!(encoded["predicate"]["op"], "and");
    assert!(serde_json::from_value::<ObjectSet>(json!({
        "kind": "type",
        "object_type": "文档",
        "unknown": true
    }))
    .is_err());
    assert!(serde_json::from_value::<ActionRequest>(json!({
        "items": [],
        "validateOnly": false,
        "unknown": true
    }))
    .is_err());
}

#[test]
fn wire_numbers_unicode_and_variant_names() {
    let value = serde_json::to_value(ObjectSet::Pk {
        object_type: "订单".into(),
        pk: json!(9_007_199_254_740_993_i64),
    })
    .expect("serialize pk");
    assert_eq!(value["kind"], "pk");
    assert_eq!(value["object_type"], "订单");
    assert_eq!(value["pk"], 9_007_199_254_740_993_i64);
}
