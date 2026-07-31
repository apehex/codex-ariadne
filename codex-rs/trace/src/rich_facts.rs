//! Typed fact projection from reduced rich rollout objects.

use codex_rollout_trace::AgentOrigin;
use codex_rollout_trace::AgentThread;
use codex_rollout_trace::CodeCell;
use codex_rollout_trace::CodexTurn;
use codex_rollout_trace::Compaction;
use codex_rollout_trace::CompactionRequest;
use codex_rollout_trace::ConversationItem;
use codex_rollout_trace::ExecutionWindow;
use codex_rollout_trace::InferenceCall;
use codex_rollout_trace::InteractionEdge;
use codex_rollout_trace::ProducerRef;
use codex_rollout_trace::ToolCall;
use codex_rollout_trace::ToolCallRequester;
use codex_rollout_trace::TraceAnchor;

use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRelation;

pub(crate) fn thread(
    id: &str,
    thread: &AgentThread,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::Thread(id.to_string()));
    match &thread.origin {
        AgentOrigin::Root => {}
        AgentOrigin::Spawned {
            parent_thread_id,
            spawn_edge_id,
            ..
        } => {
            correlations.push(link(
                TraceRelation::ParentThread,
                TraceObjectRef::Thread(parent_thread_id.clone()),
            ));
            correlations.push(link(
                TraceRelation::SpawnEdge,
                TraceObjectRef::InteractionEdge(spawn_edge_id.clone()),
            ));
        }
    }
    facts(
        TraceOrder::structural(
            thread.execution.started_at_unix_ms,
            thread.execution.ended_at_unix_ms,
        ),
        Some(id),
        None,
        availability,
        correlations,
    )
}

pub(crate) fn turn(
    id: &str,
    turn: &CodexTurn,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::Turn(id.to_string()));
    correlations.extend(turn.input_item_ids.iter().map(|id| {
        link(
            TraceRelation::TurnInput,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    execution_facts(
        &turn.execution,
        Some(&turn.thread_id),
        Some(id),
        availability,
        correlations,
    )
}

pub(crate) fn conversation_item(
    id: &str,
    item: &ConversationItem,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::ConversationItem(id.to_string()));
    if let Some(call_id) = &item.call_id {
        correlations.push(link(
            TraceRelation::ModelVisibleCall,
            TraceObjectRef::ModelVisibleCall(call_id.clone()),
        ));
    }
    correlations.extend(
        item.produced_by
            .iter()
            .map(|producer| link(TraceRelation::Producer, producer_ref(producer))),
    );
    facts(
        TraceOrder::rich(item.first_seen_seq, None, item.first_seen_at_unix_ms, None),
        Some(&item.thread_id),
        item.codex_turn_id.as_deref(),
        availability,
        correlations,
    )
}

pub(crate) fn inference(
    id: &str,
    inference: &InferenceCall,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::Inference(id.to_string()));
    correlations.extend(inference.request_item_ids.iter().map(|id| {
        link(
            TraceRelation::InferenceInput,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(inference.response_item_ids.iter().map(|id| {
        link(
            TraceRelation::InferenceOutput,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(
        inference
            .tool_call_ids_started_by_response
            .iter()
            .map(|id| {
                link(
                    TraceRelation::InferenceStartedTool,
                    TraceObjectRef::ToolCall(id.clone()),
                )
            }),
    );
    correlations.push(raw_payload_link(&inference.raw_request_payload_id));
    correlations.extend(
        inference
            .raw_response_payload_id
            .iter()
            .map(raw_payload_link),
    );
    execution_facts(
        &inference.execution,
        Some(&inference.thread_id),
        Some(&inference.codex_turn_id),
        availability,
        correlations,
    )
}

pub(crate) fn tool(
    id: &str,
    tool: &ToolCall,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::ToolCall(id.to_string()));
    correlations.extend(tool.mcp_call_id.iter().map(|id| {
        link(
            TraceRelation::SourceIdentity,
            TraceObjectRef::McpCall(id.clone()),
        )
    }));
    correlations.extend(tool.model_visible_call_id.iter().map(|id| {
        link(
            TraceRelation::ModelVisibleCall,
            TraceObjectRef::ModelVisibleCall(id.clone()),
        )
    }));
    correlations.extend(tool.code_mode_runtime_tool_id.iter().map(|id| {
        link(
            TraceRelation::SourceIdentity,
            TraceObjectRef::CodeModeRuntimeTool(id.clone()),
        )
    }));
    if let ToolCallRequester::CodeCell { code_cell_id } = &tool.requester {
        correlations.push(link(
            TraceRelation::RequestingCodeCell,
            TraceObjectRef::CodeCell(code_cell_id.clone()),
        ));
    }
    correlations.extend(tool.model_visible_call_item_ids.iter().map(|id| {
        link(
            TraceRelation::ModelVisibleCallItem,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(tool.model_visible_output_item_ids.iter().map(|id| {
        link(
            TraceRelation::ModelVisibleOutputItem,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(tool.terminal_operation_id.iter().map(|id| {
        link(
            TraceRelation::ToolTerminalOperation,
            TraceObjectRef::TerminalOperation(id.clone()),
        )
    }));
    correlations.extend(tool.raw_invocation_payload_id.iter().map(raw_payload_link));
    correlations.extend(tool.raw_result_payload_id.iter().map(raw_payload_link));
    correlations.extend(tool.raw_runtime_payload_ids.iter().map(raw_payload_link));
    execution_facts(
        &tool.execution,
        Some(&tool.thread_id),
        tool.started_by_codex_turn_id.as_deref(),
        availability,
        correlations,
    )
}

pub(crate) fn code_cell(
    id: &str,
    cell: &CodeCell,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::CodeCell(id.to_string()));
    correlations.push(link(
        TraceRelation::ModelVisibleCall,
        TraceObjectRef::ModelVisibleCall(cell.model_visible_call_id.clone()),
    ));
    correlations.push(link(
        TraceRelation::CodeSource,
        TraceObjectRef::ConversationItem(cell.source_item_id.clone()),
    ));
    correlations.extend(cell.output_item_ids.iter().map(|id| {
        link(
            TraceRelation::CodeOutput,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(cell.nested_tool_call_ids.iter().map(|id| {
        link(
            TraceRelation::NestedTool,
            TraceObjectRef::ToolCall(id.clone()),
        )
    }));
    correlations.extend(cell.wait_tool_call_ids.iter().map(|id| {
        link(
            TraceRelation::WaitTool,
            TraceObjectRef::ToolCall(id.clone()),
        )
    }));
    execution_facts(
        &cell.execution,
        Some(&cell.thread_id),
        Some(&cell.codex_turn_id),
        availability,
        correlations,
    )
}

pub(crate) fn compaction(
    id: &str,
    compaction: &Compaction,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::Compaction(id.to_string()));
    correlations.push(link(
        TraceRelation::CompactionMarker,
        TraceObjectRef::ConversationItem(compaction.marker_item_id.clone()),
    ));
    correlations.extend(compaction.request_ids.iter().map(|id| {
        link(
            TraceRelation::CompactionRequest,
            TraceObjectRef::CompactionRequest(id.clone()),
        )
    }));
    correlations.extend(compaction.input_item_ids.iter().map(|id| {
        link(
            TraceRelation::CompactionInput,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(compaction.replacement_item_ids.iter().map(|id| {
        link(
            TraceRelation::CompactionReplacement,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    facts(
        TraceOrder::rich(
            compaction.installed_seq,
            None,
            compaction.installed_at_unix_ms,
            None,
        ),
        Some(&compaction.thread_id),
        Some(&compaction.codex_turn_id),
        availability,
        correlations,
    )
}

pub(crate) fn compaction_request(
    id: &str,
    request: &CompactionRequest,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::CompactionRequest(id.to_string()));
    correlations.push(link(
        TraceRelation::OwningCompaction,
        TraceObjectRef::Compaction(request.compaction_id.clone()),
    ));
    correlations.push(raw_payload_link(&request.raw_request_payload_id));
    correlations.extend(request.raw_response_payload_id.iter().map(raw_payload_link));
    execution_facts(
        &request.execution,
        Some(&request.thread_id),
        Some(&request.codex_turn_id),
        availability,
        correlations,
    )
}

pub(crate) fn interaction(
    id: &str,
    edge: &InteractionEdge,
    availability: TraceFactAvailability,
) -> TraceNodeFacts {
    let mut correlations = identity(TraceObjectRef::InteractionEdge(id.to_string()));
    correlations.push(link(
        TraceRelation::InteractionSource,
        anchor_ref(&edge.source),
    ));
    correlations.push(link(
        TraceRelation::InteractionTarget,
        anchor_ref(&edge.target),
    ));
    correlations.extend(edge.carried_item_ids.iter().map(|id| {
        link(
            TraceRelation::InteractionCarriedItem,
            TraceObjectRef::ConversationItem(id.clone()),
        )
    }));
    correlations.extend(edge.carried_raw_payload_ids.iter().map(raw_payload_link));
    facts(
        TraceOrder::rich(
            edge.started_seq,
            None,
            edge.started_at_unix_ms,
            edge.ended_at_unix_ms,
        ),
        None,
        None,
        availability,
        correlations,
    )
}

pub(crate) fn raw_payload(id: &str) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::default(),
        TraceOwnership::default(),
        TraceFactAvailability::Complete,
        identity(TraceObjectRef::RawPayload(id.to_string())),
    )
}

fn execution_facts(
    execution: &ExecutionWindow,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    availability: TraceFactAvailability,
    correlations: Vec<TraceCorrelation>,
) -> TraceNodeFacts {
    facts(
        execution_order(execution),
        thread_id,
        turn_id,
        availability,
        correlations,
    )
}

fn execution_order(execution: &ExecutionWindow) -> TraceOrder {
    TraceOrder::rich(
        execution.started_seq,
        execution.ended_seq,
        execution.started_at_unix_ms,
        execution.ended_at_unix_ms,
    )
}

fn facts(
    order: TraceOrder,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    availability: TraceFactAvailability,
    correlations: Vec<TraceCorrelation>,
) -> TraceNodeFacts {
    TraceNodeFacts::new(
        order,
        TraceOwnership {
            thread_id: thread_id.map(str::to_owned),
            turn_id: turn_id.map(str::to_owned),
        },
        availability,
        correlations,
    )
}

fn identity(target: TraceObjectRef) -> Vec<TraceCorrelation> {
    vec![link(TraceRelation::SourceIdentity, target)]
}

fn raw_payload_link(id: impl AsRef<str>) -> TraceCorrelation {
    link(
        TraceRelation::RawPayload,
        TraceObjectRef::RawPayload(id.as_ref().to_string()),
    )
}

fn link(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}

fn producer_ref(producer: &ProducerRef) -> TraceObjectRef {
    match producer {
        ProducerRef::UserInput => TraceObjectRef::UserInput,
        ProducerRef::Inference { inference_call_id } => {
            TraceObjectRef::Inference(inference_call_id.clone())
        }
        ProducerRef::Tool { tool_call_id } => TraceObjectRef::ToolCall(tool_call_id.clone()),
        ProducerRef::CodeCell { code_cell_id } => TraceObjectRef::CodeCell(code_cell_id.clone()),
        ProducerRef::InteractionEdge { edge_id } => {
            TraceObjectRef::InteractionEdge(edge_id.clone())
        }
        ProducerRef::Compaction { compaction_id } => {
            TraceObjectRef::Compaction(compaction_id.clone())
        }
        ProducerRef::Harness => TraceObjectRef::Harness,
    }
}

fn anchor_ref(anchor: &TraceAnchor) -> TraceObjectRef {
    match anchor {
        TraceAnchor::ConversationItem { item_id } => {
            TraceObjectRef::ConversationItem(item_id.clone())
        }
        TraceAnchor::ToolCall { tool_call_id } => TraceObjectRef::ToolCall(tool_call_id.clone()),
        TraceAnchor::Thread { thread_id } => TraceObjectRef::Thread(thread_id.clone()),
    }
}

#[cfg(test)]
#[path = "rich_facts_tests.rs"]
mod tests;
