//! Source-typed presentation policy classification.

use codex_protocol::models::LocalShellAction;
use codex_protocol::models::ResponseItem;
use codex_protocol::parse_command::ParsedCommand;
use codex_protocol::protocol::RolloutItem;
use codex_rollout_trace::InteractionEdgeKind;
use codex_rollout_trace::TerminalRequest;
use codex_rollout_trace::ToolCallKind;
use codex_rollout_trace::ToolCallRequester;

use crate::TraceActivity;
use crate::TraceAgentActivity;
use crate::TraceCompactionActivity;
use crate::TraceExplorationEligibility;
use crate::TraceToolActivity;
use crate::TraceToolRequester;

/// Classifies one ordinary rollout record when durable fields suffice.
pub(crate) fn ordinary(item: &RolloutItem) -> Option<TraceActivity> {
    match item {
        RolloutItem::ResponseItem(item) => ordinary_response(item),
        RolloutItem::Compacted(_) => Some(TraceActivity::Compaction(
            TraceCompactionActivity::Checkpoint,
        )),
        RolloutItem::SessionMeta(_)
        | RolloutItem::InterAgentCommunication(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. }
        | RolloutItem::TurnContext(_)
        | RolloutItem::WorldState(_)
        | RolloutItem::EventMsg(_) => None,
    }
}

/// Classifies one rich runtime tool without inspecting serialized detail.
pub(crate) fn rich_tool(tool: &codex_rollout_trace::ToolCall) -> TraceActivity {
    let requester = match tool.requester {
        ToolCallRequester::Model => TraceToolRequester::Model,
        ToolCallRequester::CodeCell { .. } => TraceToolRequester::CodeCell,
    };
    match &tool.kind {
        ToolCallKind::SpawnAgent => TraceActivity::Agent(TraceAgentActivity::Spawn),
        ToolCallKind::AssignAgentTask => TraceActivity::Agent(TraceAgentActivity::Assign),
        ToolCallKind::SendMessage => TraceActivity::Agent(TraceAgentActivity::Send),
        ToolCallKind::WaitAgent => TraceActivity::Agent(TraceAgentActivity::Wait),
        ToolCallKind::CloseAgent => TraceActivity::Agent(TraceAgentActivity::Close),
        ToolCallKind::Other { name } if name == "resume_agent" => {
            TraceActivity::Agent(TraceAgentActivity::Resume)
        }
        ToolCallKind::ExecCommand => tool_activity(
            TraceToolActivity::ExecCommand(TraceExplorationEligibility::Unavailable),
            requester,
        ),
        ToolCallKind::WriteStdin => tool_activity(TraceToolActivity::WriteStdin, requester),
        ToolCallKind::ApplyPatch => tool_activity(TraceToolActivity::ApplyPatch, requester),
        ToolCallKind::Mcp { .. } => tool_activity(TraceToolActivity::Mcp, requester),
        ToolCallKind::Web => tool_activity(TraceToolActivity::Web, requester),
        ToolCallKind::ImageGeneration => {
            tool_activity(TraceToolActivity::ImageGeneration, requester)
        }
        ToolCallKind::Other { .. } => tool_activity(TraceToolActivity::Other, requester),
    }
}

/// Refines terminal activity with command exploration and poll semantics.
pub(crate) fn terminal(operation: &codex_rollout_trace::TerminalOperation) -> TraceActivity {
    let kind = match &operation.request {
        TerminalRequest::ExecCommand { command, .. } => {
            TraceToolActivity::ExecCommand(exploration(command))
        }
        TerminalRequest::WriteStdin { stdin, .. } if stdin.is_empty() => {
            TraceToolActivity::PollTerminal
        }
        TerminalRequest::WriteStdin { .. } => TraceToolActivity::WriteStdin,
    };
    tool_activity(kind, TraceToolRequester::Unknown)
}

/// Maps a rich interaction edge to its agent activity.
pub(crate) fn interaction(kind: &InteractionEdgeKind) -> TraceActivity {
    TraceActivity::Agent(match kind {
        InteractionEdgeKind::SpawnAgent => TraceAgentActivity::Spawn,
        InteractionEdgeKind::AssignAgentTask => TraceAgentActivity::Assign,
        InteractionEdgeKind::SendMessage => TraceAgentActivity::Send,
        InteractionEdgeKind::AgentResult => TraceAgentActivity::Result,
        InteractionEdgeKind::CloseAgent => TraceAgentActivity::Close,
    })
}

/// Classifies a typed model-visible response item.
fn ordinary_response(item: &ResponseItem) -> Option<TraceActivity> {
    match item {
        ResponseItem::LocalShellCall { action, .. } => {
            let LocalShellAction::Exec(action) = action;
            Some(tool_activity(
                TraceToolActivity::ExecCommand(exploration(&action.command)),
                TraceToolRequester::Model,
            ))
        }
        ResponseItem::FunctionCall { name, .. } | ResponseItem::CustomToolCall { name, .. } => {
            Some(named_tool(name))
        }
        ResponseItem::WebSearchCall { .. } => Some(tool_activity(
            TraceToolActivity::Web,
            TraceToolRequester::Model,
        )),
        ResponseItem::ImageGenerationCall { .. } => Some(tool_activity(
            TraceToolActivity::ImageGeneration,
            TraceToolRequester::Model,
        )),
        ResponseItem::Compaction { .. }
        | ResponseItem::CompactionTrigger {}
        | ResponseItem::ContextCompaction { .. } => {
            Some(TraceActivity::Compaction(TraceCompactionActivity::Marker))
        }
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::Message { .. }
        | ResponseItem::AgentMessage { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::Other => None,
    }
}

/// Maps stable Codex tool names while retaining an explicit unknown family.
fn named_tool(name: &str) -> TraceActivity {
    match name {
        "spawn_agent" => TraceActivity::Agent(TraceAgentActivity::Spawn),
        "followup_task" | "assign_task" => TraceActivity::Agent(TraceAgentActivity::Assign),
        "send_message" => TraceActivity::Agent(TraceAgentActivity::Send),
        "wait_agent" => TraceActivity::Agent(TraceAgentActivity::Wait),
        "resume_agent" => TraceActivity::Agent(TraceAgentActivity::Resume),
        "close_agent" | "interrupt_agent" => TraceActivity::Agent(TraceAgentActivity::Close),
        "exec_command" | "local_shell" | "shell" | "shell_command" => tool_activity(
            TraceToolActivity::ExecCommand(TraceExplorationEligibility::Unavailable),
            TraceToolRequester::Model,
        ),
        "write_stdin" => tool_activity(TraceToolActivity::WriteStdin, TraceToolRequester::Model),
        "apply_patch" => tool_activity(TraceToolActivity::ApplyPatch, TraceToolRequester::Model),
        "web_search" | "web_search_preview" => {
            tool_activity(TraceToolActivity::Web, TraceToolRequester::Model)
        }
        "image_generation" | "image_query" | "imagegen" => tool_activity(
            TraceToolActivity::ImageGeneration,
            TraceToolRequester::Model,
        ),
        _ => tool_activity(TraceToolActivity::Other, TraceToolRequester::Model),
    }
}

/// Constructs a typed tool activity at readable call sites.
fn tool_activity(kind: TraceToolActivity, requester: TraceToolRequester) -> TraceActivity {
    TraceActivity::Tool { kind, requester }
}

/// Applies the parent Codex read/list/search exploration rule.
fn exploration(command: &[String]) -> TraceExplorationEligibility {
    let parsed = codex_shell_command::parse_command::parse_command(command);
    if !parsed.is_empty()
        && parsed.iter().all(|command| {
            matches!(
                command,
                ParsedCommand::Read { .. }
                    | ParsedCommand::ListFiles { .. }
                    | ParsedCommand::Search { .. }
            )
        })
    {
        TraceExplorationEligibility::Eligible
    } else {
        TraceExplorationEligibility::Ineligible
    }
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
