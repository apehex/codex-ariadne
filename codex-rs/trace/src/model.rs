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
    /// Ordinary rollout JSONL evidence only.
    Ordinary,
    /// Rich rollout-trace bundle evidence only.
    Rich,
    /// Reconciled ordinary and rich evidence.
    Merged,
}

/// Strength of the evidence represented by a projected node or search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGrade {
    /// Exact bytes or source-authored identity.
    Exact,
    /// Typed meaning reconstructed without retaining exact bytes.
    Semantic,
    /// Best-effort meaning from incomplete evidence.
    Reconstructed,
    /// Evidence that could not be obtained or interpreted.
    Unavailable,
    /// Multiple retained observations disagree.
    Conflicting,
}

/// Evidence that a trace source can authoritatively provide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceCapabilities {
    /// Ordinary conversational transcript records are available.
    pub ordinary_transcript: bool,
    /// Raw ordinary rollout records are available.
    pub raw_rollout_records: bool,
    /// Exact model inference context is available.
    pub exact_inference_context: bool,
    /// Usage per model generation is available.
    pub per_generation_usage: bool,
    /// Runtime tools, terminals, and interaction edges are available.
    pub runtime_graph: bool,
    /// Lazy raw payload references are available.
    pub raw_payloads: bool,
    /// Typed compaction lifecycle detail is available.
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
    /// No authoritative completion status is available.
    Unknown,
    /// The recorded operation was still running.
    Running,
    /// The recorded operation completed successfully.
    Completed,
    /// The recorded operation failed.
    Failed,
    /// The recorded operation was interrupted or aborted.
    Aborted,
}

/// Stable kind component of a [`TraceNodeLocator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceNodeKind {
    /// Root browsing session.
    Session,
    /// Root or spawned agent thread.
    Thread,
    /// One Codex turn.
    Turn,
    /// One model inference request and response.
    Inference,
    /// One model-visible conversation item.
    ConversationItem,
    /// One tool invocation.
    ToolCall,
    /// One code-mode execution cell.
    CodeCell,
    /// One persistent terminal session.
    TerminalSession,
    /// One terminal operation.
    TerminalOperation,
    /// One installed context compaction.
    Compaction,
    /// One compaction-generation request.
    CompactionRequest,
    /// One inter-agent interaction edge.
    InteractionEdge,
    /// One lazy exact source artifact.
    RawPayload,
    /// One ordinary rollout JSONL record.
    RolloutRecord,
    /// One non-fatal inspection problem.
    Diagnostic,
}

/// Primary semantic class used by trace views for styling and filtering.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TraceRecordClass {
    /// Navigation or hierarchy structure.
    Structure,
    /// System-authored model context.
    System,
    /// Developer-authored model context.
    Developer,
    /// User-authored content.
    User,
    /// Assistant content without a narrower class.
    Assistant,
    /// Assistant commentary.
    Commentary,
    /// Assistant final answer.
    FinalAnswer,
    /// Model reasoning.
    Reasoning,
    /// Tool invocation input.
    ToolInput,
    /// Tool result output.
    ToolOutput,
    /// Source code or execution.
    Code,
    /// Multi-agent delegation.
    Delegation,
    /// Context compaction lifecycle.
    Compaction,
    /// Diagnostic information.
    Diagnostic,
    /// Exact raw artifact.
    RawArtifact,
    /// Content not classified more specifically.
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
    /// System role.
    System,
    /// Developer role.
    Developer,
    /// User role.
    User,
    /// Assistant role.
    Assistant,
    /// Tool role.
    Tool,
}

/// Codex content channel associated with a normalized record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceRecordChannel {
    /// Private analysis channel.
    Analysis,
    /// User-visible commentary channel.
    Commentary,
    /// Final-answer channel.
    Final,
    /// Compacted-summary channel.
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
    Code {
        /// Optional highlighter language name.
        language: String,
    },
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
    /// Root session containing the node.
    pub session_id: String,
    /// Stable semantic node kind.
    pub kind: TraceNodeKind,
    /// Source identity, disambiguated for repeated observations.
    pub id: String,
}

impl TraceNodeLocator {
    /// Creates a stable address from a session, kind, and source identity.
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
    /// Affected node when known.
    pub locator: Option<TraceNodeLocator>,
    /// Affected source path when known.
    pub path: Option<PathBuf>,
    /// Evidence consequence of the problem.
    pub evidence: EvidenceGrade,
    /// Bounded explanation without source contents.
    pub message: String,
}

impl TraceDiagnostic {
    /// Creates an unavailable-evidence diagnostic associated with one source path.
    pub(crate) fn unavailable_at(path: &std::path::Path, message: String) -> Self {
        Self {
            locator: None,
            path: Some(path.to_path_buf()),
            evidence: EvidenceGrade::Unavailable,
            message,
        }
    }
}

/// Catalog row for one root session tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Catalog identity used to load the session.
    pub session_id: String,
    /// Root agent-thread identity.
    pub root_thread_id: String,
    /// Sources contributing evidence.
    pub source: TraceSourceKind,
    /// Evidence families available for inspection.
    pub capabilities: TraceCapabilities,
    /// Source-recorded creation time.
    pub created_at: Option<String>,
    /// Source-recorded working directory.
    pub cwd: Option<PathBuf>,
    /// Source-recorded model provider.
    pub model_provider: Option<String>,
    /// Coarse rich-trace completion status.
    pub status: TraceStatus,
    /// Whether every ordinary observation was archived.
    pub archived: bool,
    /// Known thread-observation count.
    pub thread_count: Option<usize>,
}

/// Read-only catalog of discovered root session trees.
#[derive(Debug, Clone)]
pub struct TraceCatalog {
    /// Deterministically ordered session rows.
    pub sessions: Vec<SessionSummary>,
    /// Non-fatal discovery diagnostics.
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
    /// Maximum bytes read from one JSON payload needed for semantic replay.
    pub max_rich_semantic_payload_bytes: usize,
}

impl Default for TraceLimits {
    fn default() -> Self {
        Self {
            max_discovered_files_per_root: 100_000,
            max_nodes_per_session: 100_000,
            max_ordinary_record_bytes: 1024 * 1024,
            max_rich_events: 100_000,
            max_rich_event_bytes: 1024 * 1024,
            max_rich_semantic_payload_bytes: 1024 * 1024,
        }
    }
}

/// One normalized semantic or raw-record node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceNode {
    /// Stable node address.
    pub locator: TraceNodeLocator,
    /// Parent address, or none for roots.
    pub parent: Option<TraceNodeLocator>,
    /// Source contributing this observation.
    pub provenance: TraceSourceKind,
    /// Strength of the retained evidence.
    pub evidence: EvidenceGrade,
    /// Source timestamp when available.
    pub timestamp: Option<String>,
    /// Short navigation label.
    pub label: String,
    /// Eager bounded listing facts.
    #[serde(default)]
    pub presentation: TraceRecordPresentation,
    /// Normalized structured detail.
    pub detail: Value,
}

/// A loaded root session, ready for tree browsing and local search.
#[derive(Debug, Clone)]
pub struct SessionTrace {
    /// Catalog summary updated during materialization.
    pub summary: SessionSummary,
    /// Bounded normalized node observations.
    pub nodes: Vec<TraceNode>,
    /// Non-fatal discovery, replay, and projection diagnostics.
    pub diagnostics: Vec<TraceDiagnostic>,
    pub(crate) payloads: BTreeMap<String, BundlePayload>,
}

impl SessionTrace {
    /// Builds an immutable navigation index over retained nodes.
    pub fn index(&self) -> crate::TraceIndex {
        crate::TraceIndex::new(self)
    }

    /// Iterates nodes without a parent.
    pub fn root_nodes(&self) -> impl Iterator<Item = &TraceNode> {
        self.nodes.iter().filter(|node| node.parent.is_none())
    }

    /// Iterates direct children of one retained parent.
    pub fn children<'a>(
        &'a self,
        parent: &'a TraceNodeLocator,
    ) -> impl Iterator<Item = &'a TraceNode> {
        self.nodes
            .iter()
            .filter(move |node| node.parent.as_ref() == Some(parent))
    }

    /// Looks up one node by stable locator.
    pub fn node(&self, locator: &TraceNodeLocator) -> Option<&TraceNode> {
        self.nodes.iter().find(|node| node.locator == *locator)
    }

    /// Searches bounded labels and structured string fields.
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        crate::search::search_nodes(&self.nodes, query)
    }

    /// Returns a lazy handle for one retained raw payload.
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
    /// Matching node.
    pub locator: TraceNodeLocator,
    /// Matching observation source.
    pub provenance: TraceSourceKind,
    /// Strength of matching evidence.
    pub evidence: EvidenceGrade,
    /// Node field containing the match (`label` or a JSON pointer).
    pub field: String,
    /// Byte range within the bounded field text used for this result.
    pub match_range: std::ops::Range<usize>,
    /// Bounded single-line context around the match.
    pub snippet: String,
}

/// Lazy handle to a bundle-local raw payload.
#[derive(Debug, Clone)]
pub struct RawPayloadHandle {
    bundle_root: PathBuf,
    reference: RawPayloadRef,
}

impl RawPayloadHandle {
    /// Returns the source-authored raw payload identity.
    pub fn id(&self) -> &str {
        &self.reference.raw_payload_id
    }

    /// Reads, sanitizes, and byte-bounds the referenced artifact.
    pub async fn read(
        &self,
        limit: crate::PayloadReadLimit,
    ) -> anyhow::Result<crate::SanitizedPayload> {
        crate::SafePayloadReader::new(self.bundle_root.clone())
            .read(&self.reference, limit)
            .await
    }
}

/// Metadata for one discovered ordinary rollout observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrdinaryThread {
    pub path: PathBuf,
    pub session_id: String,
    pub thread_id: String,
    pub parent_thread_id: Option<String>,
    pub forked_from_thread_id: Option<String>,
    pub history_base: Option<codex_protocol::protocol::HistoryPosition>,
    pub timestamp: String,
    pub cwd: PathBuf,
    pub model_provider: Option<String>,
    pub archived: bool,
}

/// One discovered rich bundle and immutable manifest metadata.
#[derive(Debug, Clone)]
pub(crate) struct RichBundle {
    pub path: PathBuf,
    pub manifest: codex_rollout_trace::TraceBundleMetadata,
}

/// Source observations grouped under one root catalog session.
#[derive(Debug, Clone, Default)]
pub(crate) struct CatalogEntry {
    pub ordinary: Vec<OrdinaryThread>,
    pub rich: Vec<RichBundle>,
}

/// Bundle root and reference needed for an on-demand payload read.
#[derive(Debug, Clone)]
pub(crate) struct BundlePayload {
    pub bundle_root: PathBuf,
    pub reference: RawPayloadRef,
}
