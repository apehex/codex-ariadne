use std::collections::BTreeMap;
use std::path::PathBuf;

use codex_rollout_trace::RawPayloadRef;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Storage representation contributing evidence to a session projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceSourceKind {
    Ordinary,
    Rich,
    Merged,
}

/// Strength of the evidence represented by a projected node or search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGrade {
    Exact,
    Semantic,
    Reconstructed,
    Unavailable,
    Conflicting,
}

/// Evidence that a trace source can authoritatively provide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceCapabilities {
    pub ordinary_transcript: bool,
    pub raw_rollout_records: bool,
    pub exact_inference_context: bool,
    pub per_generation_usage: bool,
    pub runtime_graph: bool,
    pub raw_payloads: bool,
    pub compaction_detail: bool,
}

impl TraceCapabilities {
    pub(crate) const ORDINARY: Self = Self {
        ordinary_transcript: true,
        raw_rollout_records: true,
        exact_inference_context: false,
        per_generation_usage: false,
        runtime_graph: false,
        raw_payloads: false,
        compaction_detail: false,
    };
    pub(crate) const RICH: Self = Self {
        ordinary_transcript: true,
        raw_rollout_records: false,
        exact_inference_context: true,
        per_generation_usage: true,
        runtime_graph: true,
        raw_payloads: true,
        compaction_detail: true,
    };

    pub(crate) fn union(self, other: Self) -> Self {
        Self {
            ordinary_transcript: self.ordinary_transcript || other.ordinary_transcript,
            raw_rollout_records: self.raw_rollout_records || other.raw_rollout_records,
            exact_inference_context: self.exact_inference_context || other.exact_inference_context,
            per_generation_usage: self.per_generation_usage || other.per_generation_usage,
            runtime_graph: self.runtime_graph || other.runtime_graph,
            raw_payloads: self.raw_payloads || other.raw_payloads,
            compaction_detail: self.compaction_detail || other.compaction_detail,
        }
    }
}

/// Coarse session completion state used by the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Unknown,
    Running,
    Completed,
    Failed,
    Aborted,
}

/// Stable kind component of a [`TraceNodeLocator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceNodeKind {
    Session,
    Thread,
    Turn,
    Inference,
    ConversationItem,
    ToolCall,
    CodeCell,
    TerminalSession,
    TerminalOperation,
    Compaction,
    CompactionRequest,
    InteractionEdge,
    RawPayload,
    RolloutRecord,
    Diagnostic,
}

/// Primary semantic class used by trace views for styling and filtering.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TraceRecordClass {
    Structure,
    System,
    Developer,
    User,
    Assistant,
    Commentary,
    FinalAnswer,
    Reasoning,
    ToolInput,
    ToolOutput,
    Code,
    Delegation,
    Compaction,
    Diagnostic,
    RawArtifact,
    #[default]
    Other,
}

impl TraceRecordClass {
    /// Stable complete class order used by filter controls.
    pub const ALL: [Self; 16] = [
        Self::Structure,
        Self::System,
        Self::Developer,
        Self::User,
        Self::Assistant,
        Self::Commentary,
        Self::FinalAnswer,
        Self::Reasoning,
        Self::ToolInput,
        Self::ToolOutput,
        Self::Code,
        Self::Delegation,
        Self::Compaction,
        Self::Diagnostic,
        Self::RawArtifact,
        Self::Other,
    ];
}

/// Model-visible role associated with a normalized record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceRecordRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

/// Codex content channel associated with a normalized record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceRecordChannel {
    Analysis,
    Commentary,
    Final,
    Summary,
}

/// Small, eagerly retained facts needed to present a record without rescanning its detail.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceRecordPresentation {
    /// Primary styling and filtering class.
    pub class: TraceRecordClass,
    /// Model-visible role when the source provides one.
    pub role: Option<TraceRecordRole>,
    /// Codex content channel when the source provides one.
    pub channel: Option<TraceRecordChannel>,
    /// Runtime or completion state when present.
    pub status: Option<TraceStatus>,
    /// Bounded single-line content summary for wide listings.
    pub preview: Option<String>,
}

/// Semantic format selected for a record's bounded detail document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceContentFormat {
    /// Markdown suitable for Codex's semantic renderer.
    Markdown,
    /// Plain, already interpreted text.
    Text,
    /// Normalized JSON.
    Json,
    /// Source code with an optional highlighter language name.
    Code { language: String },
}

/// Bounded semantic content prepared for full-screen presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContentDocument {
    /// Semantic rendering format.
    pub format: TraceContentFormat,
    /// Bounded interpreted content.
    pub text: String,
    /// Whether the source content exceeded the requested byte limit.
    pub truncated: bool,
}

/// Stable address for one node within a session.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceNodeLocator {
    pub session_id: String,
    pub kind: TraceNodeKind,
    pub id: String,
}

impl TraceNodeLocator {
    pub fn new(session_id: impl Into<String>, kind: TraceNodeKind, id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            kind,
            id: id.into(),
        }
    }
}

/// Non-fatal problem encountered while discovering or projecting a trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceDiagnostic {
    pub locator: Option<TraceNodeLocator>,
    pub path: Option<PathBuf>,
    pub evidence: EvidenceGrade,
    pub message: String,
}

/// Catalog row for one root session tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub root_thread_id: String,
    pub source: TraceSourceKind,
    pub capabilities: TraceCapabilities,
    pub created_at: Option<String>,
    pub cwd: Option<PathBuf>,
    pub model_provider: Option<String>,
    pub status: TraceStatus,
    pub archived: bool,
    pub thread_count: Option<usize>,
}

/// Read-only catalog of discovered root session trees.
#[derive(Debug, Clone)]
pub struct TraceCatalog {
    pub sessions: Vec<SessionSummary>,
    pub diagnostics: Vec<TraceDiagnostic>,
    pub(crate) entries: BTreeMap<String, CatalogEntry>,
    pub(crate) limits: TraceLimits,
}

/// Resource bounds for discovery and selected-session materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLimits {
    /// Maximum matching rollout or manifest files retained per source root.
    pub max_discovered_files_per_root: usize,
    /// Maximum projected nodes retained for one selected root session.
    pub max_nodes_per_session: usize,
    /// Maximum encoded bytes retained for an ordinary rollout record.
    pub max_ordinary_record_bytes: usize,
    /// Maximum rich events applied from one selected event spine.
    pub max_rich_events: usize,
    /// Maximum encoded bytes retained for one rich event line.
    pub max_rich_event_bytes: usize,
}

impl Default for TraceLimits {
    fn default() -> Self {
        Self {
            max_discovered_files_per_root: 100_000,
            max_nodes_per_session: 100_000,
            max_ordinary_record_bytes: 1024 * 1024,
            max_rich_events: 100_000,
            max_rich_event_bytes: 1024 * 1024,
        }
    }
}

/// One normalized semantic or raw-record node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceNode {
    pub locator: TraceNodeLocator,
    pub parent: Option<TraceNodeLocator>,
    pub provenance: TraceSourceKind,
    pub evidence: EvidenceGrade,
    pub timestamp: Option<String>,
    pub label: String,
    #[serde(default)]
    pub presentation: TraceRecordPresentation,
    pub detail: Value,
}

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
    pub(crate) fn from_detail(kind: TraceNodeKind, detail: &Value) -> Self {
        let role = find_string(detail, &["role"]).and_then(parse_role);
        let channel = find_string(detail, &["channel"]).and_then(parse_channel);
        let class = classify_record(kind, role, channel, detail);
        let status = find_string(detail, &["status", "outcome"]).and_then(parse_status);
        let preview = preview_text(detail).map(|text| one_line_preview(&text, 256));
        Self {
            class,
            role,
            channel,
            status,
            preview,
        }
    }
}

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

fn assistant_class(channel: Option<TraceRecordChannel>) -> TraceRecordClass {
    match channel {
        Some(TraceRecordChannel::Commentary) => TraceRecordClass::Commentary,
        Some(TraceRecordChannel::Final) => TraceRecordClass::FinalAnswer,
        Some(TraceRecordChannel::Analysis | TraceRecordChannel::Summary) | None => {
            TraceRecordClass::Assistant
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

fn parse_channel(channel: &str) -> Option<TraceRecordChannel> {
    match channel {
        "analysis" => Some(TraceRecordChannel::Analysis),
        "commentary" => Some(TraceRecordChannel::Commentary),
        "final" => Some(TraceRecordChannel::Final),
        "summary" => Some(TraceRecordChannel::Summary),
        _ => None,
    }
}

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

fn one_line_preview(text: &str, limit: usize) -> String {
    let mut preview = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if preview.chars().count() > limit {
        preview = preview.chars().take(limit).collect();
        preview.push('…');
    }
    preview
}

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

/// A loaded root session, ready for tree browsing and local search.
#[derive(Debug, Clone)]
pub struct SessionTrace {
    pub summary: SessionSummary,
    pub nodes: Vec<TraceNode>,
    pub diagnostics: Vec<TraceDiagnostic>,
    pub(crate) payloads: BTreeMap<String, BundlePayload>,
}

impl SessionTrace {
    pub fn index(&self) -> crate::TraceIndex {
        crate::TraceIndex::new(self)
    }

    pub fn root_nodes(&self) -> impl Iterator<Item = &TraceNode> {
        self.nodes.iter().filter(|node| node.parent.is_none())
    }

    pub fn children<'a>(
        &'a self,
        parent: &'a TraceNodeLocator,
    ) -> impl Iterator<Item = &'a TraceNode> {
        self.nodes
            .iter()
            .filter(move |node| node.parent.as_ref() == Some(parent))
    }

    pub fn node(&self, locator: &TraceNodeLocator) -> Option<&TraceNode> {
        self.nodes.iter().find(|node| node.locator == *locator)
    }

    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        crate::search::search_nodes(&self.nodes, query)
    }

    pub fn raw_payload(&self, payload_id: &str) -> Option<RawPayloadHandle> {
        self.payloads
            .get(payload_id)
            .map(|payload| RawPayloadHandle {
                bundle_root: payload.bundle_root.clone(),
                reference: payload.reference.clone(),
            })
    }
}

/// Search match with a compact display snippet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub locator: TraceNodeLocator,
    pub provenance: TraceSourceKind,
    pub evidence: EvidenceGrade,
    /// Node field containing the match (`label` or a JSON pointer).
    pub field: String,
    /// Byte range within the bounded field text used for this result.
    pub match_range: std::ops::Range<usize>,
    pub snippet: String,
}

/// Lazy handle to a bundle-local raw payload.
#[derive(Debug, Clone)]
pub struct RawPayloadHandle {
    bundle_root: PathBuf,
    reference: RawPayloadRef,
}

impl RawPayloadHandle {
    pub fn id(&self) -> &str {
        &self.reference.raw_payload_id
    }

    pub async fn read(
        &self,
        limit: crate::PayloadReadLimit,
    ) -> anyhow::Result<crate::SanitizedPayload> {
        crate::SafePayloadReader::new(self.bundle_root.clone())
            .read(&self.reference, limit)
            .await
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OrdinaryThread {
    pub path: PathBuf,
    pub thread_id: String,
    pub parent_thread_id: Option<String>,
    pub timestamp: String,
    pub cwd: PathBuf,
    pub model_provider: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RichBundle {
    pub path: PathBuf,
    pub manifest: codex_rollout_trace::TraceBundleManifest,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CatalogEntry {
    pub ordinary: Vec<OrdinaryThread>,
    pub rich: Option<RichBundle>,
}

#[derive(Debug, Clone)]
pub(crate) struct BundlePayload {
    pub bundle_root: PathBuf,
    pub reference: RawPayloadRef,
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
