use std::collections::BTreeMap;

use pretty_assertions::assert_eq;
use serde_json::json;

use super::*;
use crate::EvidenceGrade;
use crate::SessionSummary;
use crate::TraceCapabilities;
use crate::TraceNodeKind;
use crate::TraceSourceKind;
use crate::TraceStatus;

#[test]
fn index_preserves_node_order_for_roots_and_children() {
    let session = locator(TraceNodeKind::Session, "session");
    let second_root = locator(TraceNodeKind::Diagnostic, "second-root");
    let first_child = locator(TraceNodeKind::Turn, "first-child");
    let grandchild = locator(TraceNodeKind::ConversationItem, "grandchild");
    let second_child = locator(TraceNodeKind::ToolCall, "second-child");
    let trace = trace(vec![
        node(session.clone(), None),
        node(first_child.clone(), Some(session.clone())),
        node(grandchild, Some(first_child.clone())),
        node(second_child.clone(), Some(session.clone())),
        node(second_root.clone(), None),
    ]);

    let index = trace.index();

    assert_eq!(
        locators(index.root_nodes(&trace)),
        vec![session.clone(), second_root]
    );
    assert_eq!(
        locators(index.children(&trace, &session)),
        vec![first_child, second_child]
    );
    assert_eq!(index.root_positions(&trace).collect::<Vec<_>>(), vec![0, 4]);
    assert_eq!(
        index.child_positions(&trace, &session).collect::<Vec<_>>(),
        vec![1, 3]
    );
}

#[test]
fn locator_lookup_preserves_first_match_semantics() {
    let duplicate = locator(TraceNodeKind::Turn, "duplicate");
    let mut first = node(duplicate.clone(), None);
    first.label = "first".to_string();
    let mut second = node(duplicate.clone(), None);
    second.label = "second".to_string();
    let trace = trace(vec![first.clone(), second]);

    assert_eq!(trace.index().node(&trace, &duplicate), Some(&first));
}

#[test]
fn structural_changes_require_rebuilding_the_snapshot() {
    let root = locator(TraceNodeKind::Session, "session");
    let child = locator(TraceNodeKind::Turn, "child");
    let mut trace = trace(vec![node(root.clone(), None)]);
    let stale = trace.index();
    trace.nodes.push(node(child.clone(), Some(root.clone())));

    assert_eq!(locators(stale.children(&trace, &root)), Vec::new());

    let rebuilt = trace.index();
    assert_eq!(locators(rebuilt.children(&trace, &root)), vec![child]);
}

fn trace(nodes: Vec<TraceNode>) -> SessionTrace {
    SessionTrace {
        summary: SessionSummary {
            session_id: "session".to_string(),
            root_thread_id: "thread".to_string(),
            source: TraceSourceKind::Ordinary,
            capabilities: TraceCapabilities::default(),
            created_at: None,
            cwd: None,
            model_provider: None,
            status: TraceStatus::Unknown,
            archived: false,
            thread_count: None,
        },
        nodes,
        facts: BTreeMap::new(),
        diagnostics: Vec::new(),
        payloads: BTreeMap::new(),
    }
}

fn node(locator: TraceNodeLocator, parent: Option<TraceNodeLocator>) -> TraceNode {
    TraceNode {
        locator,
        parent,
        provenance: TraceSourceKind::Ordinary,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: "node".to_string(),
        presentation: Default::default(),
        detail: json!({}),
    }
}

fn locator(kind: TraceNodeKind, id: &str) -> TraceNodeLocator {
    TraceNodeLocator::new("session", kind, id)
}

fn locators<'a>(nodes: impl Iterator<Item = &'a TraceNode>) -> Vec<TraceNodeLocator> {
    nodes.map(|node| node.locator.clone()).collect()
}
