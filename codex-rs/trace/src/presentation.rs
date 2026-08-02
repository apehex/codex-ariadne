//! Semantic classification and bounded terminal presentation of trace records.

use serde_json::Value;

use crate::TraceContentDocument;
use crate::TraceContentFormat;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceRecordChannel;
use crate::TraceRecordClass;
use crate::TraceRecordPresentation;
use crate::TraceRecordRole;
use crate::TraceStatus;

impl TraceNode {
    /// Builds a bounded semantic document without reading any referenced raw payload.
    pub fn content_document(&self, byte_limit: usize) -> TraceContentDocument {
        let (format, text) = semantic_content(self);
        let text = sanitize_terminal_text(&text);
        let (text, truncated) = truncate_utf8(text, byte_limit);
        TraceContentDocument {
            format,
            text,
            truncated,
        }
    }
}

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

/// Builds an interpreted content document before terminal sanitization and truncation.
fn semantic_content(node: &TraceNode) -> (TraceContentFormat, String) {
    if node.presentation.class == TraceRecordClass::Code
        && let Some(source) = find_string(&node.detail, &["source"])
    {
        let language = find_string(&node.detail, &["language"]).unwrap_or("text");
        return (
            TraceContentFormat::Code {
                language: language.to_string(),
            },
            source.to_string(),
        );
    }
    let text = collect_content_text(&node.detail);
    if !text.is_empty() {
        if matches!(
            node.presentation.class,
            TraceRecordClass::ToolInput | TraceRecordClass::ToolOutput
        ) && let Ok(value) = serde_json::from_str::<Value>(&text)
        {
            return (
                TraceContentFormat::Json,
                serde_json::to_string_pretty(&value).unwrap_or(text),
            );
        }
        let format = match node.presentation.class {
            TraceRecordClass::Assistant
            | TraceRecordClass::Commentary
            | TraceRecordClass::FinalAnswer
            | TraceRecordClass::Reasoning => TraceContentFormat::Markdown,
            TraceRecordClass::System | TraceRecordClass::Developer | TraceRecordClass::User => {
                TraceContentFormat::Text
            }
            TraceRecordClass::ToolInput | TraceRecordClass::ToolOutput => TraceContentFormat::Text,
            TraceRecordClass::Structure
            | TraceRecordClass::Code
            | TraceRecordClass::Delegation
            | TraceRecordClass::Compaction
            | TraceRecordClass::Diagnostic
            | TraceRecordClass::RawArtifact
            | TraceRecordClass::Other => TraceContentFormat::Json,
        };
        if !matches!(format, TraceContentFormat::Json) {
            return (format, text);
        }
    }
    (
        TraceContentFormat::Json,
        serde_json::to_string_pretty(&node.detail).unwrap_or_else(|_| node.detail.to_string()),
    )
}

/// Collects ordered textual fields used by semantic detail rendering.
fn collect_content_text(value: &Value) -> String {
    let mut parts = Vec::new();
    collect_named_strings(
        value,
        &[
            "text",
            "message",
            "summary",
            "source",
            "output",
            "arguments",
            "value",
        ],
        &mut parts,
    );
    parts.join("\n\n")
}

/// Recursively collects named string values without serializing the complete record.
fn collect_named_strings(value: &Value, names: &[&str], output: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (name, value) in map {
                if names.contains(&name.as_str())
                    && let Some(text) = value.as_str()
                {
                    output.push(text.to_string());
                    continue;
                }
                collect_named_strings(value, names, output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_named_strings(value, names, output);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
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
fn find_string<'a>(value: &'a Value, names: &[&str]) -> Option<&'a str> {
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

/// Collapses whitespace and caps a preview by Unicode scalar count.
fn one_line_preview(text: &str, limit: usize) -> String {
    let mut preview = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if preview.chars().count() > limit {
        preview = preview.chars().take(limit).collect();
        preview.push('…');
    }
    preview
}

/// Truncates a string at a valid UTF-8 boundary.
fn truncate_utf8(mut text: String, byte_limit: usize) -> (String, bool) {
    if text.len() <= byte_limit {
        return (text, false);
    }
    let mut end = byte_limit.min(text.len());
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.truncate(end);
    (text, true)
}

/// Replaces terminal control characters while preserving newlines and tabs.
fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if matches!(character, '\n' | '\t') || !character.is_control() {
                character
            } else {
                '�'
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
