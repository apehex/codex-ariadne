use pretty_assertions::assert_eq;
use serde_json::json;

use super::sorted_json;

#[test]
fn normalized_json_sorting_is_recursive_and_stable() {
    let sorted = sorted_json(&json!({
        "z": {"b": 2, "a": 1},
        "a": [{"d": 4, "c": 3}]
    }));

    assert_eq!(
        serde_json::to_string(&sorted).unwrap(),
        r#"{"a":[{"c":3,"d":4}],"z":{"a":1,"b":2}}"#
    );
}
