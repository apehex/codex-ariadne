//! Semantic classification of trace records.

use serde_json::Value;

use super::bounded_text::one_line_preview;
use crate::TraceNodeKind;
use crate::TraceRecordChannel;
use crate::TraceRecordClass;
use crate::TraceRecordPresentation;
use crate::TraceRecordRole;
use crate::TraceStatus;

impl TraceRecordPresentation {
    /// Derives eagerly retained presentation facts from one source detail value.
    pub(crate) fn from_detail(kind: TraceNodeKind, detail: &Value) -> Self {
        let role = find_string(detail, &["role"]).and_then(parse_role);
        let channel = find_string(detail, &["channel"]).and_then(parse_channel);
        let class = classify_record(kind, role, channel, detail);
        let status = find_string(detail, &["status", "outcome"]).and_then(parse_status);
        let preview = preview_text(detail).map(|text| one_line_preview(&text, /*limit*/ 256));
        Self {
            class,
            role,
            channel,
            status,
            preview,
        }
    }
}

/// Selects the primary semantic class for one normalized record.
fn classify_record(
    kind: TraceNodeKind,
    role: Option<TraceRecordRole>,
    channel: Option<TraceRecordChannel>,
    detail: &Value,
) -> TraceRecordClass {
    if matches!(
        kind,
        TraceNodeKind::ConversationItem | TraceNodeKind::RolloutRecord
    ) && let Some(class) = semantic_record_class(role, channel, detail)
    {
        return class;
    }
    match kind {
        TraceNodeKind::Session
        | TraceNodeKind::Thread
        | TraceNodeKind::Turn
        | TraceNodeKind::Inference
        | TraceNodeKind::TerminalSession
        | TraceNodeKind::TerminalOperation
        | TraceNodeKind::RolloutRecord => TraceRecordClass::Structure,
        TraceNodeKind::ToolCall => {
            if detail
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.contains("agent"))
            {
                TraceRecordClass::Delegation
            } else {
                TraceRecordClass::ToolInput
            }
        }
        TraceNodeKind::CodeCell => TraceRecordClass::Code,
        TraceNodeKind::Compaction | TraceNodeKind::CompactionRequest => {
            TraceRecordClass::Compaction
        }
        TraceNodeKind::InteractionEdge => TraceRecordClass::Delegation,
        TraceNodeKind::RawPayload => TraceRecordClass::RawArtifact,
        TraceNodeKind::Diagnostic => TraceRecordClass::Diagnostic,
        TraceNodeKind::ConversationItem => TraceRecordClass::Other,
    }
}

/// Classifies model-facing records from source role, channel, and kind fields.
fn semantic_record_class(
    role: Option<TraceRecordRole>,
    channel: Option<TraceRecordChannel>,
    detail: &Value,
) -> Option<TraceRecordClass> {
    if contains_named_string(detail, &["type", "kind"], &["reasoning", "agent_reasoning"]) {
        return Some(TraceRecordClass::Reasoning);
    }
    if contains_named_string(
        detail,
        &["type", "kind"],
        &[
            "function_call_output",
            "custom_tool_call_output",
            "mcp_tool_call_output",
            "tool_output",
        ],
    ) {
        return Some(TraceRecordClass::ToolOutput);
    }
    if contains_named_string(
        detail,
        &["type", "kind"],
        &[
            "function_call",
            "custom_tool_call",
            "mcp_tool_call",
            "tool_call",
        ],
    ) {
        return Some(TraceRecordClass::ToolInput);
    }
    if contains_named_string(detail, &["type", "kind"], &["user_message"]) {
        return Some(TraceRecordClass::User);
    }
    if contains_named_string(detail, &["type", "kind"], &["developer_message"]) {
        return Some(TraceRecordClass::Developer);
    }
    if contains_named_string(detail, &["type", "kind"], &["system_message"]) {
        return Some(TraceRecordClass::System);
    }
    if contains_named_string(
        detail,
        &["type", "kind"],
        &["agent_message", "assistant_message"],
    ) {
        return Some(assistant_class(channel));
    }
    match role {
        Some(TraceRecordRole::Tool) => Some(TraceRecordClass::ToolInput),
        Some(TraceRecordRole::System) => Some(TraceRecordClass::System),
        Some(TraceRecordRole::Developer) => Some(TraceRecordClass::Developer),
        Some(TraceRecordRole::User) => Some(TraceRecordClass::User),
        Some(TraceRecordRole::Assistant) => Some(assistant_class(channel)),
        None => None,
    }
}

/// Maps assistant channels to their presentation classes.
fn assistant_class(channel: Option<TraceRecordChannel>) -> TraceRecordClass {
    match channel {
        Some(TraceRecordChannel::Commentary) => TraceRecordClass::Commentary,
        Some(TraceRecordChannel::Final) => TraceRecordClass::FinalAnswer,
        Some(TraceRecordChannel::Analysis | TraceRecordChannel::Summary) | None => {
            TraceRecordClass::Assistant
        }
    }
}

/// Finds the first suitable source string for a one-line preview.
fn preview_text(value: &Value) -> Option<String> {
    find_string(
        value,
        &[
            "text",
            "message",
            "summary",
            "source",
            "command",
            "arguments",
            "output",
        ],
    )
    .map(str::to_owned)
}

/// Finds the first recursively nested string under one of the supplied names.
pub(super) fn find_string<'a>(value: &'a Value, names: &[&str]) -> Option<&'a str> {
    match value {
        Value::Object(map) => {
            for name in names {
                if let Some(text) = map.get(*name).and_then(Value::as_str) {
                    return Some(text);
                }
            }
            map.values().find_map(|value| find_string(value, names))
        }
        Value::Array(values) => values.iter().find_map(|value| find_string(value, names)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

/// Reports whether a nested named field contains one of the candidate strings.
fn contains_named_string(value: &Value, names: &[&str], candidates: &[&str]) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(name, value)| {
            (names.contains(&name.as_str())
                && value
                    .as_str()
                    .is_some_and(|text| candidates.contains(&text)))
                || contains_named_string(value, names, candidates)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| contains_named_string(value, names, candidates)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

/// Parses a source role without inventing unknown variants.
fn parse_role(role: &str) -> Option<TraceRecordRole> {
    match role {
        "system" => Some(TraceRecordRole::System),
        "developer" => Some(TraceRecordRole::Developer),
        "user" => Some(TraceRecordRole::User),
        "assistant" => Some(TraceRecordRole::Assistant),
        "tool" => Some(TraceRecordRole::Tool),
        _ => None,
    }
}

/// Parses a Codex content channel without inventing unknown variants.
fn parse_channel(channel: &str) -> Option<TraceRecordChannel> {
    match channel {
        "analysis" => Some(TraceRecordChannel::Analysis),
        "commentary" => Some(TraceRecordChannel::Commentary),
        "final" => Some(TraceRecordChannel::Final),
        "summary" => Some(TraceRecordChannel::Summary),
        _ => None,
    }
}

/// Normalizes common source status spellings into the trace status model.
fn parse_status(status: &str) -> Option<TraceStatus> {
    match status {
        "running" | "started" => Some(TraceStatus::Running),
        "completed" | "complete" | "succeeded" | "success" => Some(TraceStatus::Completed),
        "failed" | "error" => Some(TraceStatus::Failed),
        "aborted" | "cancelled" | "canceled" | "interrupted" => Some(TraceStatus::Aborted),
        "unknown" => Some(TraceStatus::Unknown),
        _ => None,
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
