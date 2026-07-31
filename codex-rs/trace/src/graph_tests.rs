use pretty_assertions::assert_eq;
use serde_json::json;

use super::Admission;
use super::TraceGraphBuilder;
use crate::EvidenceGrade;
use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRecordPresentation;
use crate::TraceRelation;
use crate::TraceSourceKind;

#[test]
fn preserves_duplicate_observations_and_reports_only_conflicts() {
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 5, Vec::new());
    graph.admit(
        node("earlier"),
        TraceNodeFacts::default(),
        /*source_path*/ None,
    );
    let original = node("same");
    let duplicate = original.clone();
    let mut conflict = original.clone();
    conflict.detail = json!({"value": "different"});

    assert!(matches!(
        graph.admit(
            original,
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::Retained(_)
    ));
    let Admission::Retained(duplicate_locator) = graph.admit(
        duplicate,
        TraceNodeFacts::default(),
        /*source_path*/ None,
    ) else {
        panic!("duplicate should be retained");
    };
    let Admission::Retained(conflict_locator) = graph.admit(
        conflict,
        TraceNodeFacts::default(),
        /*source_path*/ None,
    ) else {
        panic!("conflict should be retained");
    };

    assert_eq!(duplicate_locator.id, "same:observation:2");
    assert_eq!(conflict_locator.id, "same:observation:3");
    assert_eq!(graph.diagnostics().len(), 1);
    let (_, facts, _) = graph.finish();
    assert_eq!(
        (
            facts
                .get(&TraceNodeLocator::new(
                    "session",
                    TraceNodeKind::ToolCall,
                    "same",
                ))
                .map(|facts| facts.availability),
            facts.get(&conflict_locator).map(|facts| facts.availability),
        ),
        (
            Some(TraceFactAvailability::Conflicting),
            Some(TraceFactAvailability::Conflicting),
        )
    );
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
        graph.admit_with_required_parent(
            child,
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::MissingParent
    );
    assert!(graph.is_empty());
    assert!(matches!(
        graph.admit(
            node("first"),
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::Retained(_)
    ));
    assert_eq!(
        graph.admit(
            node("second"),
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::LimitReached
    );
    assert_eq!(graph.len(), 1);
}

#[test]
fn zero_and_exact_limits_have_deterministic_accounting() {
    let mut zero = TraceGraphBuilder::new(/*max_nodes*/ 0, Vec::new());
    assert_eq!(
        zero.admit(
            node("rejected"),
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::LimitReached
    );
    assert!(zero.is_empty());
    assert_eq!(zero.diagnostics().len(), 1);

    let mut exact = TraceGraphBuilder::new(/*max_nodes*/ 2, Vec::new());
    assert!(matches!(
        exact.admit(
            node("first"),
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
        Admission::Retained(_)
    ));
    assert!(matches!(
        exact.admit(
            node("second"),
            TraceNodeFacts::default(),
            /*source_path*/ None,
        ),
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

    graph.admit(
        parent.clone(),
        TraceNodeFacts::default(),
        /*source_path*/ None,
    );
    graph.admit_with_required_parent(
        child.clone(),
        TraceNodeFacts::default(),
        /*source_path*/ None,
    );
    let Admission::Retained(repeated_parent) =
        graph.admit(parent, TraceNodeFacts::default(), /*source_path*/ None)
    else {
        panic!("repeated parent must be retained");
    };
    let Admission::Retained(repeated_child) = graph.admit_with_required_parent(
        child,
        TraceNodeFacts::default(),
        /*source_path*/ None,
    ) else {
        panic!("repeated child must be retained");
    };
    let (nodes, _, _) = graph.finish();

    let child = nodes
        .iter()
        .find(|node| node.locator == repeated_child)
        .expect("repeated child");
    assert_eq!(child.parent.as_ref(), Some(&repeated_parent));
}

#[test]
fn sibling_positions_override_locator_kind_and_lexicographic_id_order() {
    let parent = TraceNodeLocator::new("session", TraceNodeKind::Thread, "parent");
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 5, Vec::new());
    for (id, kind, position) in [
        ("ordinary:parent:10", TraceNodeKind::Turn, 10),
        ("ordinary:parent:2", TraceNodeKind::ConversationItem, 2),
        ("ordinary:parent:1", TraceNodeKind::RolloutRecord, 1),
    ] {
        let mut child = node(id);
        child.locator.kind = kind;
        child.parent = Some(parent.clone());
        graph.admit(child, ordinary_facts(position), /*source_path*/ None);
    }
    let mut diagnostic = node("diagnostic");
    diagnostic.locator.kind = TraceNodeKind::Diagnostic;
    diagnostic.parent = Some(parent);
    graph.admit(
        diagnostic,
        TraceNodeFacts::default(),
        /*source_path*/ None,
    );

    let (nodes, _, diagnostics) = graph.finish();

    assert!(diagnostics.is_empty());
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.locator.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "ordinary:parent:1",
            "ordinary:parent:2",
            "ordinary:parent:10",
            "diagnostic",
        ]
    );
}

#[test]
fn correlations_are_bounded_globally_and_report_saturation_once() {
    let mut graph = TraceGraphBuilder::new(/*max_nodes*/ 2, Vec::new());
    for id in ["first", "second"] {
        graph.admit(
            node(id),
            TraceNodeFacts::new(
                TraceOrder::default(),
                TraceOwnership::default(),
                TraceFactAvailability::Complete,
                (0..5).map(|index| {
                    TraceCorrelation::new(
                        TraceRelation::RawPayload,
                        TraceObjectRef::RawPayload(format!("{id}:{index}")),
                    )
                }),
            ),
            /*source_path*/ None,
        );
    }

    let (_, facts, diagnostics) = graph.finish();

    assert_eq!(
        facts
            .values()
            .map(|facts| facts.correlations.len())
            .sum::<usize>(),
        8
    );
    assert_eq!(diagnostics.len(), 1);
}

fn ordinary_facts(ordinal: u64) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::ordinary(ordinal, None),
        TraceOwnership {
            thread_id: Some("thread".to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Partial,
        [],
    )
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
