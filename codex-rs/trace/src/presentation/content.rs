//! Interpreted, terminal-safe content documents for trace records.

use serde_json::Value;

use super::bounded_text::sanitize_terminal_text;
use super::bounded_text::truncate_utf8;
use super::record::find_string;
use crate::TraceContentDocument;
use crate::TraceContentFormat;
use crate::TraceNode;
use crate::TraceRecordClass;

impl TraceNode {
    /// Builds a bounded semantic document without reading any referenced raw payload.
    pub fn content_document(&self, byte_limit: usize) -> TraceContentDocument {
        let (format, text) = semantic_content(self);
        let text = sanitize_terminal_text(&text);
        let (text, truncated) = truncate_utf8(&text, byte_limit);
        TraceContentDocument {
            format,
            text,
            truncated,
        }
    }
}

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
