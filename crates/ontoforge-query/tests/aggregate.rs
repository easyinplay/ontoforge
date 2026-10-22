use ontoforge_query::aggregate::AggregateLimits;
use ontoforge_query::ast::AggregateRequest;
use serde_json::json;

#[test]
fn count_sum_avg_limits_phase4_defaults_are_exact() {
    let limits = AggregateLimits::phase4_default();
    assert_eq!(limits.max_groups, 1_024);
    assert_eq!(limits.max_buckets, 4_096);
    assert_eq!(limits.max_distinct_values, 10_000);
    assert_eq!(limits.max_distinct_memory_bytes, 16 * 1024 * 1024);
    assert!(AggregateLimits::try_new(0, 1, 1, 1).is_err());
    assert!(AggregateLimits::try_new(1, 0, 1, 1).is_err());
    assert!(AggregateLimits::try_new(1, 1, 0, 1).is_err());
    assert!(AggregateLimits::try_new(1, 1, 1, 0).is_err());
    assert!(AggregateLimits::try_new(usize::MAX, 1, 1, 1).is_err());
    let injected = json!({
        "set": {"kind": "type", "object_type": "Flight"},
        "groupBy": [],
        "measures": [{"op": "count", "property": null}],
        "maxGroups": 999999
    });
    assert!(serde_json::from_value::<AggregateRequest>(injected).is_err());
}
