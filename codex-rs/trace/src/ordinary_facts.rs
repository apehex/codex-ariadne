//! Typed fact extraction from ordinary durable rollout records.

use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::RolloutItem;

use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRelation;

/// Extracts supported facts without coupling consumers to record JSON.
pub(crate) fn record(
    item: &RolloutItem,
    thread_id: &str,
    ordinal: u64,
    wall_clock_start_ms: Option<i64>,
    current_turn_id: Option<&str>,
) -> TraceNodeFacts {
    let turn_id = match item {
        RolloutItem::ResponseItem(item) => item.turn_id().or(current_turn_id),
        RolloutItem::InterAgentCommunication(communication) => communication
            .internal_chat_message_metadata_passthrough
            .as_ref()
            .and_then(|metadata| metadata.turn_id.as_deref())
            .filter(|turn_id| !turn_id.is_empty())
            .or(current_turn_id),
        RolloutItem::TurnContext(context) => context.turn_id.as_deref(),
        RolloutItem::SessionMeta(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. }
        | RolloutItem::Compacted(_)
        | RolloutItem::WorldState(_)
        | RolloutItem::EventMsg(_) => current_turn_id,
    };
    let facts = TraceNodeFacts::new(
        TraceOrder::ordinary(ordinal, wall_clock_start_ms),
        TraceOwnership {
            thread_id: Some(thread_id.to_string()),
            turn_id: turn_id.map(str::to_owned),
        },
        TraceFactAvailability::Partial,
        correlations(item),
    );
    match crate::presentation_facts::ordinary(item) {
        Some(activity) => facts.with_activity(activity),
        None => facts,
    }
}

/// Maps durable identities and protocol call IDs into typed links.
fn correlations(item: &RolloutItem) -> Vec<TraceCorrelation> {
    let mut correlations = Vec::new();
    match item {
        RolloutItem::ResponseItem(item) => {
            if let Some(id) = item.id() {
                correlations.push(link(
                    TraceRelation::SourceIdentity,
                    TraceObjectRef::ConversationItem(id.to_string()),
                ));
            }
            if let Some(call_id) = response_call_id(item) {
                correlations.push(link(
                    TraceRelation::ModelVisibleCall,
                    TraceObjectRef::ModelVisibleCall(call_id.to_string()),
                ));
            }
        }
        RolloutItem::InterAgentCommunication(communication) => {
            if let Some(id) = &communication.id {
                correlations.push(link(
                    TraceRelation::SourceIdentity,
                    TraceObjectRef::ConversationItem(id.to_string()),
                ));
            }
        }
        RolloutItem::Compacted(compaction) => {
            if let Some(id) = &compaction.window_id {
                correlations.push(link(
                    TraceRelation::SourceIdentity,
                    TraceObjectRef::Compaction(id.clone()),
                ));
            }
            correlations.extend(
                compaction
                    .replacement_history
                    .iter()
                    .flatten()
                    .filter_map(ResponseItem::id)
                    .map(|id| {
                        link(
                            TraceRelation::CompactionReplacement,
                            TraceObjectRef::ConversationItem(id.to_string()),
                        )
                    }),
            );
        }
        RolloutItem::TurnContext(context) => {
            if let Some(turn_id) = &context.turn_id {
                correlations.push(link(
                    TraceRelation::SourceIdentity,
                    TraceObjectRef::Turn(turn_id.clone()),
                ));
            }
        }
        RolloutItem::SessionMeta(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. }
        | RolloutItem::WorldState(_)
        | RolloutItem::EventMsg(_) => {}
    }
    correlations
}

/// Returns the model-visible protocol call ID carried by a response item.
fn response_call_id(item: &ResponseItem) -> Option<&str> {
    match item {
        ResponseItem::LocalShellCall { call_id, .. }
        | ResponseItem::ToolSearchCall { call_id, .. }
        | ResponseItem::ToolSearchOutput { call_id, .. } => call_id.as_deref(),
        ResponseItem::FunctionCall { call_id, .. }
        | ResponseItem::FunctionCallOutput { call_id, .. }
        | ResponseItem::CustomToolCall { call_id, .. }
        | ResponseItem::CustomToolCallOutput { call_id, .. } => Some(call_id),
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::Message { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::CompactionTrigger {}
        | ResponseItem::ContextCompaction { .. }
        | ResponseItem::Other => None,
    }
}

fn link(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}

#[cfg(test)]
#[path = "ordinary_facts_tests.rs"]
mod tests;
