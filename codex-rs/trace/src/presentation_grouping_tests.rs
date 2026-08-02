use pretty_assertions::assert_eq;

use super::*;
use crate::EvidenceGrade;
use crate::GroupActivity;
use crate::GroupAggregate;
use crate::GroupId;
use crate::GroupKind;
use crate::PRESENTATION_POLICY_VERSION;
use crate::PresentationScope;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::TraceActivity;
use crate::TraceAgentActivity;
use crate::TraceCapabilities;
use crate::TraceCorrelation;
use crate::TraceExplorationEligibility;
use crate::TraceFactAvailability;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRecordClass;
use crate::TraceRecordPresentation;
use crate::TraceRelation;
use crate::TraceSourceKind;
use crate::TraceStatus;
use crate::TraceToolActivity;
use crate::TraceToolRequester;

#[test]
fn consecutive_exploration_tools_form_a_versioned_parent_group() {
    let trace = trace_from(vec![
        tool_node("read", /*ordinal*/ 1, exploration_activity()),
        tool_node("search", /*ordinal*/ 2, exploration_activity()),
    ]);

    let index = PresentationIndex::new(&trace);
    let scope = PresentationScope::Thread("root".to_string());
    let top_level = index.groups_in_scope(&scope).collect::<Vec<_>>();

    assert_eq!(top_level.len(), 1);
    assert_eq!(top_level[0].kind, GroupKind::ExplorationBatch);
    assert_eq!(top_level[0].child_groups.len(), 2);
    assert_eq!(
        top_level[0].metadata.activity,
        GroupAggregate::Value(GroupActivity::Exploration),
    );
    assert_eq!(
        top_level[0].id,
        GroupId::Batch {
            thread_id: "root".to_string(),
            kind: GroupKind::ExplorationBatch,
            policy_version: PRESENTATION_POLICY_VERSION,
            first_child: Box::new(tool_group_id("read")),
        },
    );
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 0)
            .map(|group| &group.id),
        Some(&tool_group_id("read"))
    );
    assert_eq!(index.status(), crate::PresentationBuildStatus::Complete);
}

#[test]
fn incrementally_admitted_facts_converge_to_the_batch_snapshot() {
    let first = tool_node("read", /*ordinal*/ 1, exploration_activity());
    let second = tool_node("search", /*ordinal*/ 2, exploration_activity());
    let batch = trace_from(vec![first.clone(), second.clone()]);
    let mut incremental = trace_from(vec![first]);
    let first_index = PresentationIndex::new(&incremental);
    assert_eq!(
        first_index
            .group_for_node(/*node_position*/ 0)
            .map(|group| group.id.clone()),
        Some(tool_group_id("read"))
    );

    incremental.facts.insert(second.0.locator.clone(), second.1);
    incremental.nodes.push(second.0);
    let incremental_index = PresentationIndex::new(&incremental);

    assert_eq!(incremental_index, PresentationIndex::new(&batch));
    assert_eq!(
        incremental_index
            .groups()
            .iter()
            .find(|group| group.kind == GroupKind::ExplorationBatch)
            .and_then(|group| group.child_groups.first()),
        Some(&tool_group_id("read")),
    );
}

#[test]
fn a_message_is_an_explicit_exploration_flush_boundary() {
    let trace = trace_from(vec![
        tool_node("read", /*ordinal*/ 1, exploration_activity()),
        (
            node(
                "message",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::User,
            ),
            facts(
                /*ordinal*/ 2,
                vec![identity(TraceObjectRef::ConversationItem(
                    "message".to_string(),
                ))],
            ),
        ),
        tool_node("search", /*ordinal*/ 3, exploration_activity()),
    ]);

    let index = PresentationIndex::new(&trace);

    assert_eq!(
        index
            .groups()
            .iter()
            .filter(|group| group.kind == GroupKind::ExplorationBatch)
            .count(),
        0,
    );
}

#[test]
fn nested_tools_remain_primary_children_of_their_code_cell() {
    let code_id = GroupId::Correlated {
        thread_id: "root".to_string(),
        kind: GroupKind::Code,
        correlation: TraceObjectRef::CodeCell("cell".to_string()),
        occurrence: 0,
    };
    let trace = trace_from(vec![
        (
            node("cell", TraceNodeKind::CodeCell, TraceRecordClass::Code),
            facts(
                /*ordinal*/ 1,
                vec![identity(TraceObjectRef::CodeCell("cell".to_string()))],
            )
            .with_activity(TraceActivity::CodeCell),
        ),
        (
            node(
                "nested",
                TraceNodeKind::ToolCall,
                TraceRecordClass::ToolInput,
            ),
            facts(
                /*ordinal*/ 2,
                vec![
                    identity(TraceObjectRef::ToolCall("nested".to_string())),
                    correlation(
                        TraceRelation::RequestingCodeCell,
                        TraceObjectRef::CodeCell("cell".to_string()),
                    ),
                ],
            )
            .with_activity(TraceActivity::Tool {
                kind: TraceToolActivity::ExecCommand(TraceExplorationEligibility::Eligible),
                requester: TraceToolRequester::CodeCell,
            }),
        ),
    ]);

    let index = PresentationIndex::new(&trace);
    let parent = index.group(&code_id).expect("code group");

    assert_eq!(parent.child_groups, vec![tool_group_id("nested")]);
    assert_eq!(parent.metadata.child_count, 1);
    assert_eq!(
        index
            .groups_in_scope(&PresentationScope::Thread("root".to_string()))
            .map(|group| group.id.clone())
            .collect::<Vec<_>>(),
        vec![code_id],
    );
}

#[test]
fn compaction_keeps_replaced_history_as_references() {
    let marker_ref = TraceObjectRef::ConversationItem("marker".to_string());
    let input_ref = TraceObjectRef::ConversationItem("input".to_string());
    let trace = trace_from(vec![
        (
            node(
                "input",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::User,
            ),
            facts(/*ordinal*/ 1, vec![identity(input_ref.clone())]),
        ),
        (
            node(
                "marker",
                TraceNodeKind::ConversationItem,
                TraceRecordClass::Compaction,
            ),
            facts(
                /*ordinal*/ 2,
                vec![
                    identity(marker_ref.clone()),
                    correlation(
                        TraceRelation::Producer,
                        TraceObjectRef::Compaction("compact".to_string()),
                    ),
                ],
            ),
        ),
        (
            node(
                "compact",
                TraceNodeKind::Compaction,
                TraceRecordClass::Compaction,
            ),
            facts(
                /*ordinal*/ 3,
                vec![
                    identity(TraceObjectRef::Compaction("compact".to_string())),
                    correlation(TraceRelation::CompactionMarker, marker_ref),
                    correlation(TraceRelation::CompactionInput, input_ref),
                ],
            )
            .with_activity(TraceActivity::Compaction(
                crate::TraceCompactionActivity::Checkpoint,
            )),
        ),
    ]);

    let index = PresentationIndex::new(&trace);
    let group = index
        .group_for_node(/*node_position*/ 2)
        .expect("compaction group");

    assert_eq!(group.kind, GroupKind::Compaction);
    assert_eq!(group.members.len(), 2);
    assert_eq!(
        index
            .group_for_node(/*node_position*/ 0)
            .map(|group| group.kind),
        Some(GroupKind::UserMessage)
    );
    assert!(group.references.iter().any(|reference| {
        reference.relation == TraceRelation::CompactionInput && reference.node_position == Some(0)
    }));
}

#[test]
fn agent_edges_join_the_tool_but_reference_child_threads() {
    let trace = trace_from(vec![
        (
            node("child", TraceNodeKind::Thread, TraceRecordClass::Structure),
            facts(
                /*ordinal*/ 1,
                vec![identity(TraceObjectRef::Thread("child".to_string()))],
            ),
        ),
        (
            node(
                "spawn",
                TraceNodeKind::ToolCall,
                TraceRecordClass::Delegation,
            ),
            facts(
                /*ordinal*/ 2,
                vec![identity(TraceObjectRef::ToolCall("spawn".to_string()))],
            )
            .with_activity(TraceActivity::Agent(TraceAgentActivity::Spawn)),
        ),
        (
            node(
                "edge",
                TraceNodeKind::InteractionEdge,
                TraceRecordClass::Delegation,
            ),
            facts(
                /*ordinal*/ 3,
                vec![
                    identity(TraceObjectRef::InteractionEdge("edge".to_string())),
                    correlation(
                        TraceRelation::InteractionSource,
                        TraceObjectRef::ToolCall("spawn".to_string()),
                    ),
                    correlation(
                        TraceRelation::InteractionTarget,
                        TraceObjectRef::Thread("child".to_string()),
                    ),
                ],
            )
            .with_activity(TraceActivity::Agent(TraceAgentActivity::Spawn)),
        ),
    ]);

    let index = PresentationIndex::new(&trace);
    let group = index
        .group_for_node(/*node_position*/ 1)
        .expect("agent group");

    assert_eq!(group.kind, GroupKind::Delegation);
    assert_eq!(group.members.len(), 2);
    assert_eq!(
        index.disposition(/*node_position*/ 0),
        Some(crate::PresentationDisposition::StructuralOnly)
    );
    assert!(group.references.iter().any(|reference| {
        reference.relation == TraceRelation::InteractionTarget && reference.node_position == Some(0)
    }));
}

fn exploration_activity() -> TraceActivity {
    TraceActivity::Tool {
        kind: TraceToolActivity::ExecCommand(TraceExplorationEligibility::Eligible),
        requester: TraceToolRequester::Model,
    }
}

fn tool_node(id: &str, ordinal: u64, activity: TraceActivity) -> (TraceNode, TraceNodeFacts) {
    (
        node(id, TraceNodeKind::ToolCall, TraceRecordClass::ToolInput),
        facts(
            ordinal,
            vec![identity(TraceObjectRef::ToolCall(id.to_string()))],
        )
        .with_activity(activity),
    )
}

fn tool_group_id(id: &str) -> GroupId {
    GroupId::Correlated {
        thread_id: "root".to_string(),
        kind: GroupKind::DirectTool,
        correlation: TraceObjectRef::ToolCall(id.to_string()),
        occurrence: 0,
    }
}

fn trace_from(entries: Vec<(TraceNode, TraceNodeFacts)>) -> SessionTrace {
    let mut trace = SessionTrace {
        summary: SessionSummary {
            session_id: "session".to_string(),
            root_thread_id: "root".to_string(),
            source: TraceSourceKind::Rich,
            capabilities: TraceCapabilities::RICH,
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
    };
    for (node, facts) in entries {
        trace.facts.insert(node.locator.clone(), facts);
        trace.nodes.push(node);
    }
    trace
}

fn node(id: &str, kind: TraceNodeKind, class: TraceRecordClass) -> TraceNode {
    TraceNode {
        locator: TraceNodeLocator::new("session", kind, id),
        parent: None,
        provenance: TraceSourceKind::Rich,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: id.to_string(),
        presentation: TraceRecordPresentation {
            class,
            ..TraceRecordPresentation::default()
        },
        detail: serde_json::json!({"id": id}),
    }
}

fn facts(ordinal: u64, correlations: Vec<TraceCorrelation>) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::ordinary(ordinal, /*wall_clock_start_ms*/ None),
        TraceOwnership {
            thread_id: Some("root".to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Complete,
        correlations,
    )
}

fn identity(target: TraceObjectRef) -> TraceCorrelation {
    correlation(TraceRelation::SourceIdentity, target)
}

fn correlation(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}
