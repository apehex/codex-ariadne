//! Loaded-session nodes, navigation methods, search hits, and lazy payload handles.

use std::collections::BTreeMap;
use std::path::PathBuf;

use codex_rollout_trace::RawPayloadRef;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::EvidenceGrade;
use super::SessionSummary;
use super::TraceDiagnostic;
use super::TraceNodeLocator;
use super::TraceRecordPresentation;
use super::TraceSourceKind;
use crate::payload::BundlePayload;

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

impl TraceNode {
    /// Creates a projected node whose presentation matches its kind and detail.
    pub(crate) fn projected(
        locator: TraceNodeLocator,
        parent: Option<TraceNodeLocator>,
        provenance: TraceSourceKind,
        evidence: EvidenceGrade,
        timestamp: Option<String>,
        label: String,
        detail: Value,
    ) -> Self {
        let presentation = TraceRecordPresentation::from_detail(locator.kind, &detail);
        Self {
            locator,
            parent,
            provenance,
            evidence,
            timestamp,
            label,
            presentation,
            detail,
        }
    }
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
    pub(crate) facts: BTreeMap<TraceNodeLocator, crate::TraceNodeFacts>,
    pub(crate) payloads: BTreeMap<String, BundlePayload>,
}

impl SessionTrace {
    /// Builds an immutable navigation index over retained nodes.
    pub fn index(&self) -> crate::TraceIndex {
        crate::TraceIndex::new(self)
    }

    /// Builds an immutable typed order and correlation index over retained nodes.
    pub fn fact_index(&self) -> crate::TraceFactIndex {
        crate::TraceFactIndex::new(self)
    }

    /// Builds an immutable renderer-neutral presentation snapshot.
    pub fn presentation_index(&self) -> crate::PresentationIndex {
        crate::PresentationIndex::new(self)
    }

    /// Builds a renderer-neutral presentation snapshot with explicit limits.
    pub fn presentation_index_with_limits(
        &self,
        limits: crate::PresentationLimits,
    ) -> crate::PresentationIndex {
        crate::PresentationIndex::new_with_limits(self, limits)
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
