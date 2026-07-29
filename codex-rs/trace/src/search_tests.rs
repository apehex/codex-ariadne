use pretty_assertions::assert_eq;
use serde_json::json;

use super::MAX_SEARCH_FIELD_CHARS;
use super::MAX_SEARCH_FIELDS_PER_NODE;
use super::MAX_SEARCH_HITS;
use super::search_nodes;
use crate::EvidenceGrade;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceSourceKind;

#[test]
fn search_attributes_the_field_and_centers_the_match() {
    let node = node(
        "turn",
        "turn",
        json!({"payload": {"message": format!("{}needle{}", "a".repeat(100), "b".repeat(100))}}),
    );
    let hit = search_nodes(&[node], "needle").pop().unwrap();
    assert_eq!(hit.field, "/payload/message");
    assert_eq!(hit.match_range, 100..106);
    assert!(hit.snippet.starts_with('a'));
    assert!(hit.snippet.contains("needle"));
    assert!(hit.snippet.ends_with('b'));
    assert!(hit.snippet.len() < 200);
}

#[test]
fn search_hit_limit_is_exact() {
    let nodes = (0..MAX_SEARCH_HITS + 1)
        .map(|index| node(&index.to_string(), "needle", serde_json::Value::Null))
        .collect::<Vec<_>>();

    let hits = search_nodes(&nodes, "needle");

    assert_eq!(hits.len(), MAX_SEARCH_HITS);
    assert_eq!(hits.last().unwrap().locator.id, "999");
}

#[test]
fn searchable_character_limit_preserves_utf8_boundaries() {
    let exact = format!("{}needle", "界".repeat(MAX_SEARCH_FIELD_CHARS - 6));
    let over = format!("{}needle", "界".repeat(MAX_SEARCH_FIELD_CHARS - 5));
    let nodes = vec![
        node("exact", "other", json!({"text": exact})),
        node("over", "other", json!({"text": over})),
    ];

    let hits = search_nodes(&nodes, "needle");

    assert_eq!(
        hits.into_iter()
            .map(|hit| hit.locator.id)
            .collect::<Vec<_>>(),
        vec!["exact"]
    );
}

#[test]
fn field_limit_and_json_pointer_escaping_are_exact() {
    let mut fields =
        vec![serde_json::Value::String("other".to_string()); MAX_SEARCH_FIELDS_PER_NODE];
    fields[MAX_SEARCH_FIELDS_PER_NODE - 1] = serde_json::Value::String("needle".to_string());
    fields.push(serde_json::Value::String("needle beyond cap".to_string()));
    let exact = node("exact", "other", serde_json::Value::Array(fields));
    let escaped = node("escaped", "other", json!({"a/b~c": "needle"}));

    let hits = search_nodes(&[exact, escaped], "NEEDLE");

    assert_eq!(
        hits[0].field,
        format!("/{}", MAX_SEARCH_FIELDS_PER_NODE - 1)
    );
    assert_eq!(hits[1].field, "/a~1b~0c");
}

#[test]
fn empty_queries_and_control_characters_are_handled_without_rewriting_offsets() {
    let nodes = vec![node("control", "a\nneedle\rb", serde_json::Value::Null)];

    assert!(search_nodes(&nodes, " \t ").is_empty());
    let hit = search_nodes(&nodes, "needle").pop().unwrap();
    assert_eq!(hit.match_range, 2..8);
    assert_eq!(hit.snippet, "a needle b");
}

fn node(id: &str, label: &str, detail: serde_json::Value) -> TraceNode {
    TraceNode {
        locator: TraceNodeLocator::new("session", TraceNodeKind::Turn, id),
        parent: None,
        provenance: TraceSourceKind::Ordinary,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: label.to_string(),
        presentation: Default::default(),
        detail,
    }
}
