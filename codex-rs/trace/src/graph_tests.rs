use pretty_assertions::assert_eq;
use serde_json::json;

use super::Admission;
use super::TraceGraphBuilder;
use crate::EvidenceGrade;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceRecordPresentation;
use crate::TraceSourceKind;

#[test]
fn preserves_duplicate_observations_and_reports_only_conflicts() {
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 4, Vec::new());
    let original = node("same");
    let duplicate = original.clone();
    let mut conflict = original.clone();
    conflict.detail = json!({"value": "different"});

    assert!(matches!(
        graph.admit(original, /*source_path*/ None),
        Admission::Retained(_)
    ));
    let Admission::Retained(duplicate_locator) = graph.admit(duplicate, /*source_path*/ None)
    else {
        panic!("duplicate should be retained");
    };
    let Admission::Retained(conflict_locator) = graph.admit(conflict, /*source_path*/ None) else {
        panic!("conflict should be retained");
    };

    assert_eq!(duplicate_locator.id, "same:observation:2");
    assert_eq!(conflict_locator.id, "same:observation:3");
    assert_eq!(graph.diagnostics().len(), 1);
}

#[test]
fn node_limit_and_missing_parent_never_admit_an_orphan() {
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 1, Vec::new());
    let mut child = node("child");
    child.parent = Some(TraceNodeLocator::new(
        "session",
        TraceNodeKind::ToolCall,
        "missing",
    ));
    assert_eq!(
        graph.admit_with_required_parent(child, /*source_path*/ None),
        Admission::MissingParent
    );
    assert!(graph.is_empty());
    assert!(matches!(
        graph.admit(node("first"), /*source_path*/ None),
        Admission::Retained(_)
    ));
    assert_eq!(
        graph.admit(node("second"), /*source_path*/ None),
        Admission::LimitReached
    );
    assert_eq!(graph.len(), 1);
}

#[test]
fn zero_and_exact_limits_have_deterministic_accounting() {
    let mut zero = TraceGraphBuilder::new(/*max_nodes*/ 0, Vec::new());
    assert_eq!(
        zero.admit(node("rejected"), /*source_path*/ None),
        Admission::LimitReached
    );
    assert!(zero.is_empty());
    assert_eq!(zero.diagnostics().len(), 1);

    let mut exact = TraceGraphBuilder::new(/*max_nodes*/ 2, Vec::new());
    assert!(matches!(
        exact.admit(node("first"), /*source_path*/ None),
        Admission::Retained(_)
    ));
    assert!(matches!(
        exact.admit(node("second"), /*source_path*/ None),
        Admission::Retained(_)
    ));
    assert_eq!(exact.len(), 2);
    assert!(exact.diagnostics().is_empty());
}

#[test]
fn repeated_parent_observations_keep_their_children_together() {
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 4, Vec::new());
    let parent = node("parent");
    let mut child = node("child");
    child.parent = Some(parent.locator.clone());

    graph.admit(parent.clone(), /*source_path*/ None);
    graph.admit_with_required_parent(child.clone(), /*source_path*/ None);
    let Admission::Retained(repeated_parent) = graph.admit(parent, /*source_path*/ None) else {
        panic!("repeated parent must be retained");
    };
    let Admission::Retained(repeated_child) =
        graph.admit_with_required_parent(child, /*source_path*/ None)
    else {
        panic!("repeated child must be retained");
    };
    let (nodes, _) = graph.finish();

    let child = nodes
        .iter()
        .find(|node| node.locator == repeated_child)
        .expect("repeated child");
    assert_eq!(child.parent.as_ref(), Some(&repeated_parent));
}

fn node(id: &str) -> TraceNode {
    TraceNode {
        locator: TraceNodeLocator::new("session", TraceNodeKind::ToolCall, id),
        parent: None,
        provenance: TraceSourceKind::Rich,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: id.to_string(),
        presentation: TraceRecordPresentation::default(),
        detail: json!({"value": "same"}),
    }
}
