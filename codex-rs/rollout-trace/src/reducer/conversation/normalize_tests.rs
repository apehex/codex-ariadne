use pretty_assertions::assert_eq;
use serde_json::json;

use super::summarize_json;

const MAX_JSON_SUMMARY_LEN: usize = 240;

#[test]
fn json_summary_truncates_chinese_at_utf8_boundary() {
    let value = json!({"query": "中".repeat(100)});
    let serialized = serde_json::to_string(&value).expect("serialize JSON");

    assert!(!serialized.is_char_boundary(MAX_JSON_SUMMARY_LEN));
    assert_eq!(
        summarize_json(&value),
        format!("{{\"query\":\"{}...", "中".repeat(76)),
    );
}

#[test]
fn json_summary_truncates_emoji_at_utf8_boundary() {
    let value = json!({"query": "🙂".repeat(100)});
    let serialized = serde_json::to_string(&value).expect("serialize JSON");

    assert!(!serialized.is_char_boundary(MAX_JSON_SUMMARY_LEN));
    assert_eq!(
        summarize_json(&value),
        format!("{{\"query\":\"{}...", "🙂".repeat(57)),
    );
}

#[test]
fn json_summary_preserves_ascii_byte_limit() {
    let value = json!({"query": "a".repeat(300)});
    let serialized = serde_json::to_string(&value).expect("serialize JSON");

    assert_eq!(
        summarize_json(&value),
        format!("{}...", &serialized[..MAX_JSON_SUMMARY_LEN]),
    );
}

#[test]
fn json_summary_truncates_at_existing_utf8_boundary() {
    let value = json!({"q": "中".repeat(100)});
    let serialized = serde_json::to_string(&value).expect("serialize JSON");

    assert!(serialized.is_char_boundary(MAX_JSON_SUMMARY_LEN));
    assert_eq!(
        summarize_json(&value),
        format!("{}...", &serialized[..MAX_JSON_SUMMARY_LEN]),
    );
}
