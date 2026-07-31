//! Typed fact projection for rich terminal runtime objects.

use codex_rollout_trace::TerminalOperation;
use codex_rollout_trace::TerminalSession;

use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRelation;

pub(crate) fn operation(
    id: &str,
    operation: &TerminalOperation,
    ownership: TraceOwnership,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::TerminalOperation(id.to_string()));
    correlations.push(link(
        TraceRelation::OwningTool,
        TraceObjectRef::ToolCall(operation.tool_call_id.clone()),
    ));
    correlations.extend(operation.terminal_id.iter().map(|id| {
        link(
            TraceRelation::TerminalSession,
            TraceObjectRef::Terminal(id.clone()),
        )
    }));
    for observation in &operation.model_observations {
        correlations.extend(observation.call_item_ids.iter().map(|id| {
            link(
                TraceRelation::TerminalObservationCall,
                TraceObjectRef::ConversationItem(id.clone()),
            )
        }));
        correlations.extend(observation.output_item_ids.iter().map(|id| {
            link(
                TraceRelation::TerminalObservationOutput,
                TraceObjectRef::ConversationItem(id.clone()),
            )
        }));
    }
    correlations.extend(operation.raw_payload_ids.iter().map(|id| {
        link(
            TraceRelation::RawPayload,
            TraceObjectRef::RawPayload(id.clone()),
        )
    }));
    TraceNodeFacts::new(
        execution_order(&operation.execution),
        ownership,
        availability,
        correlations,
    )
}

pub(crate) fn session(
    id: &str,
    terminal: &TerminalSession,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::Terminal(id.to_string()));
    correlations.push(link(
        TraceRelation::CreatedByTerminalOperation,
        TraceObjectRef::TerminalOperation(terminal.created_by_operation_id.clone()),
    ));
    correlations.extend(terminal.operation_ids.iter().map(|id| {
        link(
            TraceRelation::TerminalSessionOperation,
            TraceObjectRef::TerminalOperation(id.clone()),
        )
    }));
    TraceNodeFacts::new(
        execution_order(&terminal.execution),
        TraceOwnership {
            thread_id: Some(terminal.thread_id.clone()),
            turn_id: None,
        },
        availability,
        correlations,
    )
}

fn execution_order(execution: &codex_rollout_trace::ExecutionWindow) -> TraceOrder {
    TraceOrder::rich(
        execution.started_seq,
        execution.ended_seq,
        execution.started_at_unix_ms,
        execution.ended_at_unix_ms,
    )
}

fn identity(target: TraceObjectRef) -> Vec<TraceCorrelation> {
    vec![link(TraceRelation::SourceIdentity, target)]
}

fn link(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}

#[cfg(test)]
#[path = "rich_terminal_facts_tests.rs"]
mod tests;
