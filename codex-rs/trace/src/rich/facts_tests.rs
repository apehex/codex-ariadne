use codex_rollout_trace::ExecutionStatus;
use codex_rollout_trace::ExecutionWindow;
use codex_rollout_trace::InteractionEdge;
use codex_rollout_trace::InteractionEdgeKind;
use codex_rollout_trace::ToolCall;
use codex_rollout_trace::ToolCallKind;
use codex_rollout_trace::ToolCallRequester;
use codex_rollout_trace::ToolCallSummary;
use codex_rollout_trace::TraceAnchor;
use pretty_assertions::assert_eq;

use super::interaction;
use super::tool;
use crate::TraceActivity;
use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRelation;
use crate::TraceToolActivity;
use crate::TraceToolRequester;

#[test]
fn tool_facts_retain_completion_position_and_runtime_identifiers() {
    let source_tool = ToolCall {
        tool_call_id: "tool".to_string(),
        mcp_call_id: Some("mcp".to_string()),
        model_visible_call_id: Some("call".to_string()),
        code_mode_runtime_tool_id: Some("runtime-tool".to_string()),
        thread_id: "thread".to_string(),
        started_by_codex_turn_id: Some("turn".to_string()),
        execution: execution(/*started_seq*/ 3, /*ended_seq*/ Some(11)),
        requester: ToolCallRequester::Model,
        kind: ToolCallKind::Web,
        model_visible_call_item_ids: vec!["call-item".to_string()],
        model_visible_output_item_ids: vec!["output-item".to_string()],
        terminal_operation_id: None,
        summary: ToolCallSummary::Generic {
            label: "web".to_string(),
            input_preview: None,
            output_preview: None,
        },
        raw_invocation_payload_id: None,
        raw_result_payload_id: None,
        raw_runtime_payload_ids: Vec::new(),
    };

    let facts = tool("tool", &source_tool, TraceFactAvailability::Complete);

    assert_eq!(
        facts.order,
        TraceOrder::rich(
            /*started_seq*/ 3,
            Some(11),
            /*started_at_unix_ms*/ 30,
            Some(110),
        )
    );
    assert_eq!(
        facts.ownership,
        TraceOwnership {
            thread_id: Some("thread".to_string()),
            turn_id: Some("turn".to_string()),
        }
    );
    assert_eq!(
        facts.policy.activity,
        Some(TraceActivity::Tool {
            kind: TraceToolActivity::Web,
            requester: TraceToolRequester::Model,
        }),
    );
    assert_eq!(
        facts.correlations,
        vec![
            correlation(
                TraceRelation::SourceIdentity,
                TraceObjectRef::ToolCall("tool".to_string()),
            ),
            correlation(
                TraceRelation::SourceIdentity,
                TraceObjectRef::McpCall("mcp".to_string()),
            ),
            correlation(
                TraceRelation::ModelVisibleCall,
                TraceObjectRef::ModelVisibleCall("call".to_string()),
            ),
            correlation(
                TraceRelation::SourceIdentity,
                TraceObjectRef::CodeModeRuntimeTool("runtime-tool".to_string()),
            ),
            correlation(
                TraceRelation::ModelVisibleCallItem,
                TraceObjectRef::ConversationItem("call-item".to_string()),
            ),
            correlation(
                TraceRelation::ModelVisibleOutputItem,
                TraceObjectRef::ConversationItem("output-item".to_string()),
            ),
        ]
    );
}

#[test]
fn interaction_facts_use_raw_sequence_and_retain_endpoints() {
    let edge = InteractionEdge {
        edge_id: "edge".to_string(),
        kind: InteractionEdgeKind::SendMessage,
        source: TraceAnchor::Thread {
            thread_id: "source".to_string(),
        },
        target: TraceAnchor::Thread {
            thread_id: "target".to_string(),
        },
        started_at_unix_ms: 500,
        started_seq: 4,
        ended_at_unix_ms: Some(100),
        carried_item_ids: vec!["item".to_string()],
        carried_raw_payload_ids: Vec::new(),
    };

    let facts = interaction(
        "edge",
        &edge,
        TraceOwnership {
            thread_id: Some("source".to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Complete,
    );

    assert_eq!(
        facts.order,
        TraceOrder::rich(
            /*started_seq*/ 4,
            /*ended_seq*/ None,
            /*started_at_unix_ms*/ 500,
            Some(100),
        )
    );
    assert_eq!(
        facts.correlations,
        vec![
            correlation(
                TraceRelation::SourceIdentity,
                TraceObjectRef::InteractionEdge("edge".to_string()),
            ),
            correlation(
                TraceRelation::InteractionSource,
                TraceObjectRef::Thread("source".to_string()),
            ),
            correlation(
                TraceRelation::InteractionTarget,
                TraceObjectRef::Thread("target".to_string()),
            ),
            correlation(
                TraceRelation::InteractionCarriedItem,
                TraceObjectRef::ConversationItem("item".to_string()),
            ),
        ]
    );
}

fn execution(started_seq: u64, ended_seq: Option<u64>) -> ExecutionWindow {
    ExecutionWindow {
        started_at_unix_ms: 30,
        started_seq,
        ended_at_unix_ms: ended_seq.map(|sequence| sequence as i64 * 10),
        ended_seq,
        status: ExecutionStatus::Completed,
    }
}

fn correlation(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation { relation, target }
}
