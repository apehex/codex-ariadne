use serde_json::Value;

use crate::SearchHit;
use crate::TraceNode;

const MAX_SEARCH_HITS: usize = 1_000;
const MAX_SEARCH_FIELD_CHARS: usize = 16 * 1_024;
const MAX_SEARCH_FIELDS_PER_NODE: usize = 4_096;
const SNIPPET_CONTEXT_CHARS: usize = 72;

pub(crate) fn search_nodes(nodes: &[TraceNode], query: &str) -> Vec<SearchHit> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    nodes
        .iter()
        .filter_map(|node| search_node(node, &needle))
        .take(MAX_SEARCH_HITS)
        .collect()
}

fn search_node(node: &TraceNode, needle: &str) -> Option<SearchHit> {
    if let Some(found) = match_field("label", &node.label, needle) {
        return Some(hit(node, found));
    }
    let mut fields = Vec::new();
    collect_fields(&node.detail, "", &mut fields);
    fields
        .into_iter()
        .find_map(|(field, value)| match_field(&field, value, needle).map(|found| hit(node, found)))
}

fn hit(node: &TraceNode, found: FieldMatch) -> SearchHit {
    SearchHit {
        locator: node.locator.clone(),
        provenance: node.provenance,
        evidence: node.evidence,
        field: found.field,
        match_range: found.range,
        snippet: found.snippet,
    }
}

struct FieldMatch {
    field: String,
    range: std::ops::Range<usize>,
    snippet: String,
}

fn match_field(field: &str, value: &str, needle: &str) -> Option<FieldMatch> {
    let bounded = value
        .chars()
        .take(MAX_SEARCH_FIELD_CHARS)
        .collect::<String>();
    // ASCII folding preserves byte offsets for the inspector's match range.
    let lowered = bounded.to_ascii_lowercase();
    let start = lowered.find(needle)?;
    let end = start + needle.len();
    Some(FieldMatch {
        field: field.to_string(),
        range: start..end,
        snippet: match_snippet(&bounded, start, end),
    })
}

fn match_snippet(text: &str, match_start: usize, match_end: usize) -> String {
    let start = text[..match_start]
        .char_indices()
        .rev()
        .nth(SNIPPET_CONTEXT_CHARS)
        .map_or(0, |(index, _)| index);
    let end = text[match_end..]
        .char_indices()
        .nth(SNIPPET_CONTEXT_CHARS)
        .map_or(text.len(), |(index, _)| match_end + index);
    text[start..end].replace(['\n', '\r'], " ")
}

fn collect_fields<'a>(value: &'a Value, path: &str, fields: &mut Vec<(String, &'a str)>) {
    if fields.len() >= MAX_SEARCH_FIELDS_PER_NODE {
        return;
    }
    match value {
        Value::String(text) => fields.push((pointer_or_root(path), text)),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_fields(item, &format!("{path}/{index}"), fields);
                if fields.len() >= MAX_SEARCH_FIELDS_PER_NODE {
                    break;
                }
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                collect_fields(item, &format!("{path}/{key}"), fields);
                if fields.len() >= MAX_SEARCH_FIELDS_PER_NODE {
                    break;
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn pointer_or_root(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::EvidenceGrade;
    use crate::TraceNodeKind;
    use crate::TraceNodeLocator;
    use crate::TraceSourceKind;

    #[test]
    fn search_attributes_the_field_and_centers_the_match() {
        let node = TraceNode {
            locator: TraceNodeLocator::new("session", TraceNodeKind::Turn, "turn"),
            parent: None,
            provenance: TraceSourceKind::Ordinary,
            evidence: EvidenceGrade::Semantic,
            timestamp: None,
            label: "turn".to_string(),
            detail: json!({"payload": {"message": format!("{}needle{}", "a".repeat(100), "b".repeat(100))}}),
        };
        let hit = search_nodes(&[node], "needle").pop().unwrap();
        assert_eq!(hit.field, "/payload/message");
        assert_eq!(hit.match_range, 100..106);
        assert!(hit.snippet.starts_with('a'));
        assert!(hit.snippet.contains("needle"));
        assert!(hit.snippet.ends_with('b'));
        assert!(hit.snippet.len() < 200);
    }
}
