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
    pub detail: Value,
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
