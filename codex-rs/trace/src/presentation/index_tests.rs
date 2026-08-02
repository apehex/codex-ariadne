use pretty_assertions::assert_eq;
use serde_json::json;

use crate::EvidenceGrade;
use crate::GroupCompleteness;
use crate::GroupId;
use crate::GroupKind;
use crate::OrderBandKind;
use crate::PresentationBuildStatus;
use crate::PresentationDiagnosticCode;
use crate::PresentationDisposition;
use crate::PresentationIndex;
use crate::PresentationScope;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::TraceCapabilities;
use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOrderPoint;
use crate::TraceOwnership;
use crate::TraceRecordClass;
use crate::TraceRecordPresentation;
use crate::TraceRelation;
use crate::TraceSourceKind;
use crate::TraceStatus;

#[test]
fn empty_trace_builds_an_empty_complete_snapshot() {
    let trace = empty_trace(TraceSourceKind::Ordinary);
    let index = trace.presentation_index();

    assert_eq!(index, PresentationIndex::new(&trace));
    assert_eq!(index.status(), PresentationBuildStatus::Complete);
    assert_eq!(index.groups(), &[]);
    assert_eq!(index.events_in_scope(&PresentationScope::Session), &[]);
    assert_eq!(index.diagnostics(), &[]);
}

#[test]
fn structural_reference_and_primary_dispositions_are_distinct() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    add(
        &mut trace,
        node(
            "session",
            TraceNodeKind::Session,
            TraceRecordClass::Structure,
        ),
        TraceNodeFacts::default(),
    );
    add(
        &mut trace,
        node(
            "raw",
            TraceNodeKind::RawPayload,
            TraceRecordClass::RawArtifact,
        ),
        TraceNodeFacts::default(),
    );
    add(
        &mut trace,
        node(
            "user",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::User,
        ),
        ordinary_facts(/*ordinal*/ 1, &[]),
    );

    let index = trace.presentation_index();

    assert_eq!(
        (0..3)
            .map(|position| index.disposition(position))
            .collect::<Vec<_>>(),
        vec![
            Some(PresentationDisposition::StructuralOnly),
            Some(PresentationDisposition::ReferenceOnly),
            Some(PresentationDisposition::Primary),
        ]
    );
    assert_eq!(index.group_for_node(/*node_position*/ 0), None);
    assert_eq!(index.group_for_node(/*node_position*/ 1), None);
    assert_eq!(
        index.group_for_node(/*node_position*/ 2).map(|group| (
            &group.id,
            group.kind,
            group.completeness
        )),
        Some((
            &GroupId::Singleton(trace.nodes[2].locator.clone()),
            GroupKind::UserMessage,
            GroupCompleteness::Complete,
        ))
    );
}

#[test]
fn interleaved_tool_members_share_one_group_without_moving_events() {
    let mut trace = empty_trace(TraceSourceKind::Rich);
    add(
        &mut trace,
        node(
            "input",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolInput,
        ),
        rich_facts(
            /*sequence*/ 1,
            &[
                source(TraceObjectRef::ConversationItem("input".to_string())),
                model_call("call"),
            ],
        ),
    );
    add(
        &mut trace,
        node(
            "user",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::User,
        ),
        rich_facts(
            /*sequence*/ 2,
            &[source(TraceObjectRef::ConversationItem("user".to_string()))],
        ),
    );
    add(
        &mut trace,
        node("tool", TraceNodeKind::ToolCall, TraceRecordClass::Other),
        rich_facts(
            /*sequence*/ 3,
            &[
                source(TraceObjectRef::ToolCall("tool".to_string())),
                model_call("call"),
                relation(
                    TraceRelation::ModelVisibleCallItem,
                    TraceObjectRef::ConversationItem("input".to_string()),
                ),
                relation(
                    TraceRelation::ModelVisibleOutputItem,
                    TraceObjectRef::ConversationItem("output".to_string()),
                ),
            ],
        ),
    );
    add(
        &mut trace,
        node(
            "output",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolOutput,
        ),
        rich_facts(
            /*sequence*/ 4,
            &[
                source(TraceObjectRef::ConversationItem("output".to_string())),
                model_call("call"),
            ],
        ),
    );

    let index = trace.presentation_index();
    let tool_group = index.group_for_node(/*node_position*/ 0).unwrap();

    assert_eq!(
        tool_group,
        index.group_for_node(/*node_position*/ 2).unwrap()
    );
    assert_eq!(
        tool_group,
        index.group_for_node(/*node_position*/ 3).unwrap()
    );
    assert_eq!(
        tool_group
            .members
            .iter()
            .map(|member| member.node_position)
            .collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
    assert_eq!(tool_group.kind, GroupKind::DirectTool);
    assert_eq!(tool_group.completeness, GroupCompleteness::Complete);
    assert_eq!(
        index
            .events_in_scope(&thread_scope())
            .iter()
            .map(|event| event.node_position)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        index
            .groups_in_scope(&thread_scope())
            .map(|group| group.id.clone())
            .collect::<Vec<_>>(),
        vec![
            tool_group.id.clone(),
            GroupId::Singleton(trace.nodes[1].locator.clone())
        ]
    );
}

#[test]
fn merged_sources_align_identity_and_preserve_unordered_intervals() {
    let mut trace = empty_trace(TraceSourceKind::Merged);
    add(
        &mut trace,
        node(
            "ordinary-a",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::User,
        ),
        ordinary_facts(
            /*ordinal*/ 1,
            &[source(TraceObjectRef::ConversationItem("a".to_string()))],
        ),
    );
    add(
        &mut trace,
        node(
            "ordinary-b",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::Assistant,
        ),
        ordinary_facts(/*ordinal*/ 2, &[]),
    );
    add(
        &mut trace,
        node(
            "rich-a",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::User,
        ),
        rich_facts(
            /*sequence*/ 10,
            &[source(TraceObjectRef::ConversationItem("a".to_string()))],
        ),
    );
    add(
        &mut trace,
        node(
            "rich-c",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::Assistant,
        ),
        rich_facts(/*sequence*/ 11, &[]),
    );

    let index = trace.presentation_index();
    let scope = thread_scope();
    let bands = index.bands_in_scope(&scope).collect::<Vec<_>>();

    assert_eq!(
        bands.iter().map(|band| band.kind).collect::<Vec<_>>(),
        vec![OrderBandKind::Aligned, OrderBandKind::Unordered]
    );
    assert_eq!(bands[0].segments[0].node_positions, vec![0]);
    assert_eq!(bands[0].segments[1].node_positions, vec![2]);
    assert_eq!(bands[1].segments[0].node_positions, vec![1]);
    assert_eq!(bands[1].segments[1].node_positions, vec![3]);
    assert_eq!(index, trace.presentation_index());
}

#[test]
fn repeated_call_identity_is_disambiguated_without_guessing_result_ownership() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    for (id, ordinal) in [("first", 1), ("second", 3)] {
        add(
            &mut trace,
            node(
                id,
                TraceNodeKind::ConversationItem,
                TraceRecordClass::ToolInput,
            ),
            ordinary_facts(
                ordinal,
                &[
                    source(TraceObjectRef::ConversationItem(id.to_string())),
                    model_call("reused"),
                ],
            ),
        );
    }
    add(
        &mut trace,
        node(
            "result",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolOutput,
        ),
        ordinary_facts(/*ordinal*/ 4, &[model_call("reused")]),
    );

    let index = trace.presentation_index();

    assert_eq!(
        index
            .group_for_node(/*node_position*/ 0)
            .map(|group| group.id.clone()),
        Some(GroupId::Correlated {
            thread_id: "root".to_string(),
            kind: GroupKind::DirectTool,
            correlation: TraceObjectRef::ModelVisibleCall("reused".to_string()),
            occurrence: 0,
        })
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 1)
            .map(|group| group.id.clone()),
        Some(GroupId::Correlated {
            thread_id: "root".to_string(),
            kind: GroupKind::DirectTool,
            correlation: TraceObjectRef::ModelVisibleCall("reused".to_string()),
            occurrence: 1,
        })
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 2)
            .map(|group| group.id.clone()),
        Some(GroupId::Singleton(trace.nodes[2].locator.clone()))
    );
    assert!(index.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == PresentationDiagnosticCode::AmbiguousCorrelation
            && diagnostic.node_position == Some(2)
    }));
}

#[test]
fn partial_and_conflicting_evidence_remain_visible() {
    let mut trace = empty_trace(TraceSourceKind::Merged);
    add(
        &mut trace,
        node(
            "input",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolInput,
        ),
        ordinary_facts(/*ordinal*/ 1, &[model_call("partial")]),
    );
    let mut conflicting = node(
        "conflict",
        TraceNodeKind::ConversationItem,
        TraceRecordClass::Assistant,
    );
    conflicting.evidence = EvidenceGrade::Conflicting;
    add(&mut trace, conflicting, rich_facts(/*sequence*/ 2, &[]));

    let index = trace.presentation_index();

    assert_eq!(
        index
            .group_for_node(/*node_position*/ 0)
            .unwrap()
            .completeness,
        GroupCompleteness::Partial
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 1)
            .unwrap()
            .completeness,
        GroupCompleteness::Partial
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 1)
            .unwrap()
            .metadata
            .evidence
            .conflicting,
        1
    );
    assert_eq!(index.status(), PresentationBuildStatus::Complete);
}

#[test]
fn truncated_facts_degrade_an_otherwise_complete_lifecycle() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    let mut input_facts = ordinary_facts(/*ordinal*/ 1, &[model_call("partial-facts")]);
    input_facts.availability = TraceFactAvailability::Partial;
    add(
        &mut trace,
        node(
            "input",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolInput,
        ),
        input_facts,
    );
    add(
        &mut trace,
        node(
            "output",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::ToolOutput,
        ),
        ordinary_facts(/*ordinal*/ 2, &[model_call("partial-facts")]),
    );

    let index = trace.presentation_index();

    assert_eq!(
        index.group_for_node(/*node_position*/ 0),
        index.group_for_node(/*node_position*/ 1)
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 0)
            .unwrap()
            .completeness,
        GroupCompleteness::Partial
    );
}

#[test]
fn summary_text_and_preview_respect_utf8_and_scalar_bounds() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    let mut bounded = node(
        "bounded",
        TraceNodeKind::ConversationItem,
        TraceRecordClass::Assistant,
    );
    bounded.label = "é".repeat(2_100);
    bounded.presentation.preview = Some("λ".repeat(300));
    add(&mut trace, bounded, ordinary_facts(/*ordinal*/ 1, &[]));

    let index = trace.presentation_index();
    let metadata = &index.group_for_node(/*node_position*/ 0).unwrap().metadata;
    let label = metadata.label.as_ref().unwrap();

    assert_eq!(label.len(), 4_096);
    assert!(label.is_char_boundary(label.len()));
    assert_eq!(metadata.preview.as_deref(), Some(""));
    assert!(
        index
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == PresentationDiagnosticCode::ResourceLimit })
    );

    let mut preview_trace = empty_trace(TraceSourceKind::Ordinary);
    let mut preview = node(
        "preview",
        TraceNodeKind::ConversationItem,
        TraceRecordClass::Assistant,
    );
    preview.presentation.preview = Some("λ".repeat(300));
    add(
        &mut preview_trace,
        preview,
        ordinary_facts(/*ordinal*/ 1, &[]),
    );
    assert_eq!(
        preview_trace
            .presentation_index()
            .group_for_node(/*node_position*/ 0)
            .unwrap()
            .metadata
            .preview
            .as_ref()
            .unwrap()
            .chars()
            .count(),
        256
    );
}

#[test]
fn secondary_references_obey_the_global_bound_and_keep_primary_membership() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    for (id, class, ordinal) in [
        ("input", TraceRecordClass::ToolInput, 1),
        ("output", TraceRecordClass::ToolOutput, 2),
    ] {
        let mut correlations = vec![model_call("bounded")];
        correlations.extend((0..6).map(|reference| {
            relation(
                TraceRelation::RawPayload,
                TraceObjectRef::RawPayload(format!("{id}-{reference}")),
            )
        }));
        add(
            &mut trace,
            node(id, TraceNodeKind::ConversationItem, class),
            ordinary_facts(ordinal, &correlations),
        );
    }

    let index = trace.presentation_index();
    let group = index.group_for_node(/*node_position*/ 0).unwrap();

    assert_eq!(group, index.group_for_node(/*node_position*/ 1).unwrap());
    assert_eq!(group.members.len(), 2);
    assert_eq!(group.references.len(), 8);
    assert_eq!(group.metadata.reference_count, 8);
    assert!(
        index
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == PresentationDiagnosticCode::ResourceLimit })
    );
    assert!(
        index
            .diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code == PresentationDiagnosticCode::MissingReference })
    );
}

#[test]
fn rebuilding_after_structural_mutation_leaves_the_old_snapshot_unchanged() {
    let mut trace = empty_trace(TraceSourceKind::Ordinary);
    add(
        &mut trace,
        node(
            "first",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::User,
        ),
        ordinary_facts(/*ordinal*/ 1, &[]),
    );
    let original = trace.presentation_index();
    add(
        &mut trace,
        node(
            "second",
            TraceNodeKind::ConversationItem,
            TraceRecordClass::Assistant,
        ),
        ordinary_facts(/*ordinal*/ 2, &[]),
    );
    let rebuilt = trace.presentation_index();

    assert_eq!(original.groups().len(), 1);
    assert_eq!(rebuilt.groups().len(), 2);
    assert_eq!(original.events_in_scope(&thread_scope()).len(), 1);
    assert_eq!(rebuilt.events_in_scope(&thread_scope()).len(), 2);
}

#[test]
fn whole_snapshot_is_deterministic_for_supported_evidence_shapes() {
    let ordinary = trace_from(
        TraceSourceKind::Ordinary,
        vec![(
            node(
                "ordinary",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::User,
            ),
            ordinary_facts(/*ordinal*/ 1, &[]),
        )],
    );
    let rich = trace_from(
        TraceSourceKind::Rich,
        vec![(
            node(
                "rich",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::Assistant,
            ),
            rich_facts(/*sequence*/ 1, &[]),
        )],
    );
    let merged = trace_from(
        TraceSourceKind::Merged,
        vec![
            (
                node(
                    "ordinary-a",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::User,
                ),
                ordinary_facts(
                    /*ordinal*/ 1,
                    &[source(TraceObjectRef::ConversationItem("a".to_string()))],
                ),
            ),
            (
                node(
                    "rich-a",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::User,
                ),
                rich_facts(
                    /*sequence*/ 1,
                    &[source(TraceObjectRef::ConversationItem("a".to_string()))],
                ),
            ),
        ],
    );
    let complete = trace_from(
        TraceSourceKind::Ordinary,
        vec![
            (
                node(
                    "input",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::ToolInput,
                ),
                ordinary_facts(/*ordinal*/ 1, &[model_call("complete")]),
            ),
            (
                node(
                    "output",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::ToolOutput,
                ),
                ordinary_facts(/*ordinal*/ 2, &[model_call("complete")]),
            ),
        ],
    );
    let interleaved = trace_from(
        TraceSourceKind::Rich,
        vec![
            (
                node(
                    "input",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::ToolInput,
                ),
                rich_facts(/*sequence*/ 1, &[model_call("interleaved")]),
            ),
            (
                node(
                    "message",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::User,
                ),
                rich_facts(/*sequence*/ 2, &[]),
            ),
            (
                node(
                    "output",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::ToolOutput,
                ),
                rich_facts(/*sequence*/ 3, &[model_call("interleaved")]),
            ),
        ],
    );
    let mut partial_facts = ordinary_facts(/*ordinal*/ 1, &[model_call("partial")]);
    partial_facts.availability = TraceFactAvailability::Partial;
    let partial = trace_from(
        TraceSourceKind::Ordinary,
        vec![(
            node(
                "partial",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::ToolInput,
            ),
            partial_facts,
        )],
    );
    let mut conflicting_node = node(
        "conflicting",
        TraceNodeKind::ConversationItem,
        TraceRecordClass::Assistant,
    );
    conflicting_node.evidence = EvidenceGrade::Conflicting;
    let conflicting = trace_from(
        TraceSourceKind::Merged,
        vec![(conflicting_node, rich_facts(/*sequence*/ 1, &[]))],
    );
    let repeated = trace_from(
        TraceSourceKind::Ordinary,
        vec![
            (
                node(
                    "repeat-1",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::User,
                ),
                ordinary_facts(
                    /*ordinal*/ 1,
                    &[source(TraceObjectRef::ConversationItem(
                        "repeat".to_string(),
                    ))],
                ),
            ),
            (
                node(
                    "repeat-2",
                    TraceNodeKind::ConversationItem,
                    TraceRecordClass::User,
                ),
                ordinary_facts(
                    /*ordinal*/ 2,
                    &[source(TraceObjectRef::ConversationItem(
                        "repeat".to_string(),
                    ))],
                ),
            ),
        ],
    );

    for trace in [
        ordinary,
        rich,
        merged,
        complete,
        interleaved,
        partial,
        conflicting,
        repeated,
    ] {
        assert_eq!(trace.presentation_index(), trace.presentation_index());
    }
}

fn empty_trace(source: TraceSourceKind) -> SessionTrace {
    SessionTrace {
        summary: SessionSummary {
            session_id: "session".to_string(),
            root_thread_id: "root".to_string(),
            source,
            capabilities: match source {
                TraceSourceKind::Ordinary => TraceCapabilities::ORDINARY,
                TraceSourceKind::Rich => TraceCapabilities::RICH,
                TraceSourceKind::Merged => {
                    TraceCapabilities::ORDINARY.union(TraceCapabilities::RICH)
                }
            },
            created_at: None,
            cwd: None,
            model_provider: None,
            status: TraceStatus::Unknown,
            archived: false,
            thread_count: Some(1),
        },
        nodes: Vec::new(),
        diagnostics: Vec::new(),
        facts: Default::default(),
        payloads: Default::default(),
    }
}

fn trace_from(source: TraceSourceKind, entries: Vec<(TraceNode, TraceNodeFacts)>) -> SessionTrace {
    let mut trace = empty_trace(source);
    for (node, facts) in entries {
        add(&mut trace, node, facts);
    }
    trace
}

fn add(trace: &mut SessionTrace, node: TraceNode, facts: TraceNodeFacts) {
    trace.facts.insert(node.locator.clone(), facts);
    trace.nodes.push(node);
}

fn node(id: &str, kind: TraceNodeKind, class: TraceRecordClass) -> TraceNode {
    TraceNode {
        locator: TraceNodeLocator::new("session", kind, id),
        parent: None,
        provenance: TraceSourceKind::Ordinary,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: id.to_string(),
        presentation: TraceRecordPresentation {
            class,
            preview: Some(format!("preview {id}")),
            ..TraceRecordPresentation::default()
        },
        detail: json!({"id": id}),
    }
}

fn ordinary_facts(ordinal: u64, correlations: &[TraceCorrelation]) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder {
            tie_break: usize::try_from(ordinal).unwrap(),
            ..TraceOrder::ordinary(ordinal, /*wall_clock_start_ms*/ None)
        },
        ownership(),
        TraceFactAvailability::Complete,
        correlations.iter().cloned(),
    )
}

fn rich_facts(sequence: u64, correlations: &[TraceCorrelation]) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder {
            start: TraceOrderPoint::Rich { sequence },
            tie_break: usize::try_from(sequence).unwrap(),
            ..TraceOrder::default()
        },
        ownership(),
        TraceFactAvailability::Complete,
        correlations.iter().cloned(),
    )
}

fn ownership() -> TraceOwnership {
    TraceOwnership {
        thread_id: Some("root".to_string()),
        turn_id: None,
    }
}

fn thread_scope() -> PresentationScope {
    PresentationScope::Thread("root".to_string())
}

fn source(target: TraceObjectRef) -> TraceCorrelation {
    relation(TraceRelation::SourceIdentity, target)
}

fn model_call(id: &str) -> TraceCorrelation {
    relation(
        TraceRelation::ModelVisibleCall,
        TraceObjectRef::ModelVisibleCall(id.to_string()),
    )
}

fn relation(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}
