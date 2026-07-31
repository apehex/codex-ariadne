use codex_rollout_trace::ExecutionStatus;
use codex_rollout_trace::ExecutionWindow;
use codex_rollout_trace::TerminalModelObservation;
use codex_rollout_trace::TerminalObservationSource;
use codex_rollout_trace::TerminalOperation;
use codex_rollout_trace::TerminalOperationKind;
use codex_rollout_trace::TerminalRequest;
use pretty_assertions::assert_eq;

use super::operation;
use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceObjectRef;
use crate::TraceOwnership;
use crate::TraceRelation;

#[test]
fn terminal_operation_retains_runtime_and_model_visible_observations() {
    let source_operation = TerminalOperation {
        operation_id: "operation".to_string(),
        terminal_id: Some("terminal".to_string()),
        tool_call_id: "tool".to_string(),
        kind: TerminalOperationKind::WriteStdin,
        execution: ExecutionWindow {
            started_at_unix_ms: 20,
            started_seq: 2,
            ended_at_unix_ms: Some(50),
            ended_seq: Some(5),
            status: ExecutionStatus::Completed,
        },
        request: TerminalRequest::WriteStdin {
            stdin: String::new(),
            yield_time_ms: None,
            max_output_tokens: None,
        },
        result: None,
        model_observations: vec![TerminalModelObservation {
            call_item_ids: vec!["call-item".to_string()],
            output_item_ids: vec!["output-item".to_string()],
            source: TerminalObservationSource::DirectToolCall,
        }],
        raw_payload_ids: Vec::new(),
    };

    let facts = operation(
        "operation",
        &source_operation,
        TraceOwnership {
            thread_id: Some("thread".to_string()),
            turn_id: Some("turn".to_string()),
        },
        TraceFactAvailability::Complete,
    );

    assert_eq!(
        facts.correlations,
        vec![
            correlation(
                TraceRelation::SourceIdentity,
                TraceObjectRef::TerminalOperation("operation".to_string()),
            ),
            correlation(
                TraceRelation::OwningTool,
                TraceObjectRef::ToolCall("tool".to_string()),
            ),
            correlation(
                TraceRelation::TerminalSession,
                TraceObjectRef::Terminal("terminal".to_string()),
            ),
            correlation(
                TraceRelation::TerminalObservationCall,
                TraceObjectRef::ConversationItem("call-item".to_string()),
            ),
            correlation(
                TraceRelation::TerminalObservationOutput,
                TraceObjectRef::ConversationItem("output-item".to_string()),
            ),
        ]
    );
}

fn correlation(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation { relation, target }
}
