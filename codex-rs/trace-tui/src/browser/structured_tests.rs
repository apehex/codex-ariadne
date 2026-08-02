use pretty_assertions::assert_eq;
use serde_json::json;

use super::JsonPath;
use super::JsonPathComponent;
use super::MAX_STRUCTURED_DEPTH;
use super::bounded_scalar;
use super::preview;

#[test]
fn typed_paths_resolve_and_escape_json_pointer_components() {
    let value = json!({"a/b": {"~key": [false, "value"]}});
    let path = JsonPath::root()
        .child(JsonPathComponent::Key("a/b".to_string()))
        .unwrap()
        .child(JsonPathComponent::Key("~key".to_string()))
        .unwrap()
        .child(JsonPathComponent::Index(1))
        .unwrap();

    assert_eq!(path.resolve(&value), Some(&json!("value")));
    assert_eq!(path.pointer(), "/a~1b/~0key/1");
}

#[test]
fn path_depth_and_scalar_bytes_are_bounded_on_utf8_boundaries() {
    let path = (0..MAX_STRUCTURED_DEPTH).fold(JsonPath::root(), |path, index| {
        path.child(JsonPathComponent::Index(index)).unwrap()
    });
    assert_eq!(path.child(JsonPathComponent::Index(0)), None);

    let value = json!("é".repeat(super::MAX_STRUCTURED_SCALAR_BYTES));
    let (text, truncated) = bounded_scalar(&value);
    assert!(truncated);
    assert!(text.len() <= super::MAX_STRUCTURED_SCALAR_BYTES);
    assert!(text.is_char_boundary(text.len()));
}

#[test]
fn previews_preserve_short_multiline_values_and_disclose_truncation() {
    assert_eq!(preview(&json!("first\nsecond")), "first\nsecond");
    let long = "x".repeat(super::MAX_STRUCTURED_PREVIEW_CHARS + 1);
    assert_eq!(
        preview(&json!(long)),
        format!("{}…", "x".repeat(super::MAX_STRUCTURED_PREVIEW_CHARS))
    );
}
