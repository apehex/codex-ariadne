//! Bounded, evidence-preserving admission of normalized trace nodes.

use std::collections::BTreeMap;
use std::path::Path;

use crate::EvidenceGrade;
use crate::TraceDiagnostic;
use crate::TraceNode;
use crate::TraceNodeLocator;

/// Result of attempting to retain one source observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Admission {
    /// The observation was retained under this stable locator.
    Retained(TraceNodeLocator),
    /// The session node limit rejected the observation.
    LimitReached,
    /// A required parent was not retained.
    MissingParent,
}

/// Recorded position used to order nodes that share one parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SiblingOrder {
    /// Wall-clock position for structural containers that lack a shared event sequence.
    Timestamp(i64),
    /// Causal source position, such as an ordinary ordinal or rich event sequence.
    Sequence(u64),
    /// No meaningful position is available; retain deterministic admission order.
    Unspecified,
}

/// One retained node plus ordering metadata that is not part of the public model.
#[derive(Debug)]
struct StoredNode {
    node: TraceNode,
    sibling_order: SiblingOrder,
    admission_index: usize,
}

/// Owns node limits, duplicate identities, and conflict diagnostics.
#[derive(Debug)]
pub(crate) struct TraceGraphBuilder {
    max_nodes: usize,
    nodes: BTreeMap<TraceNodeLocator, StoredNode>,
    occurrences: BTreeMap<TraceNodeLocator, usize>,
    latest_observations: BTreeMap<TraceNodeLocator, TraceNodeLocator>,
    diagnostics: Vec<TraceDiagnostic>,
    limit_reported: bool,
    next_admission_index: usize,
}

impl TraceGraphBuilder {
    /// Creates a builder with pre-existing discovery or replay diagnostics.
    pub(crate) fn new(max_nodes: usize, diagnostics: Vec<TraceDiagnostic>) -> Self {
        Self {
            max_nodes,
            nodes: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            latest_observations: BTreeMap::new(),
            diagnostics,
            limit_reported: false,
            next_admission_index: 0,
        }
    }

    /// Retains one observation, preserving duplicate identities as separate nodes.
    pub(crate) fn admit(
        &mut self,
        mut node: TraceNode,
        sibling_order: SiblingOrder,
        source_path: Option<&Path>,
    ) -> Admission {
        self.remap_parent(&mut node);
        self.retain(node, sibling_order, source_path)
    }

    /// Retains a child only when its parent is already present.
    pub(crate) fn admit_with_required_parent(
        &mut self,
        mut node: TraceNode,
        sibling_order: SiblingOrder,
        source_path: Option<&Path>,
    ) -> Admission {
        self.remap_parent(&mut node);
        if let Some(parent) = node.parent.clone()
            && !self.nodes.contains_key(&parent)
        {
            self.diagnostics.push(TraceDiagnostic {
                locator: Some(node.locator.clone()),
                path: source_path.map(Path::to_path_buf),
                evidence: EvidenceGrade::Unavailable,
                message: format!("parent {} was not admitted", parent.id),
            });
            return Admission::MissingParent;
        }
        self.retain(node, sibling_order, source_path)
    }

    /// Applies the most recently retained identity for a repeated parent.
    fn remap_parent(&self, node: &mut TraceNode) {
        if let Some(parent) = &mut node.parent
            && let Some(retained) = self.latest_observations.get(parent)
        {
            *parent = retained.clone();
        }
    }

    /// Performs bounded duplicate-aware insertion after parent resolution.
    fn retain(
        &mut self,
        mut node: TraceNode,
        sibling_order: SiblingOrder,
        source_path: Option<&Path>,
    ) -> Admission {
        if self.nodes.len() >= self.max_nodes {
            self.report_limit(source_path);
            return Admission::LimitReached;
        }
        let base = node.locator.clone();
        if let Some(previous) = self.nodes.get(&base) {
            let occurrence = self.occurrences.entry(base.clone()).or_insert(1);
            *occurrence += 1;
            node.locator.id = format!("{}:observation:{}", base.id, *occurrence);
            if !same_observation(&previous.node, &node) {
                self.diagnostics.push(TraceDiagnostic {
                    locator: Some(node.locator.clone()),
                    path: source_path.map(Path::to_path_buf),
                    evidence: EvidenceGrade::Conflicting,
                    message: format!(
                        "conflicting observation retained separately from {}",
                        base.id
                    ),
                });
            }
        } else {
            self.occurrences.insert(base.clone(), 1);
        }
        let locator = node.locator.clone();
        self.latest_observations.insert(base, locator.clone());
        let admission_index = self.next_admission_index;
        self.next_admission_index += 1;
        self.nodes.insert(
            locator.clone(),
            StoredNode {
                node,
                sibling_order,
                admission_index,
            },
        );
        Admission::Retained(locator)
    }

    /// Returns a retained node for an in-place summary update.
    pub(crate) fn node_mut(&mut self, locator: &TraceNodeLocator) -> Option<&mut TraceNode> {
        self.nodes.get_mut(locator).map(|stored| &mut stored.node)
    }

    /// Returns the number of retained nodes.
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether no nodes have been retained.
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Adds a non-fatal source or projection diagnostic.
    pub(crate) fn record_diagnostic(&mut self, diagnostic: TraceDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Returns all diagnostics accumulated so far.
    pub(crate) fn diagnostics(&self) -> &[TraceDiagnostic] {
        &self.diagnostics
    }

    /// Records that materialization encountered the hard node limit.
    pub(crate) fn note_limit(&mut self, source_path: Option<&Path>) {
        self.report_limit(source_path);
    }

    /// Consumes the builder into deterministically ordered nodes and diagnostics.
    pub(crate) fn finish(self) -> (Vec<TraceNode>, Vec<TraceDiagnostic>) {
        let mut nodes = self.nodes.into_values().collect::<Vec<_>>();
        nodes.sort_by(|left, right| {
            left.node
                .parent
                .cmp(&right.node.parent)
                .then_with(|| sibling_order_cmp(left.sibling_order, right.sibling_order))
                .then(left.admission_index.cmp(&right.admission_index))
                .then_with(|| left.node.locator.cmp(&right.node.locator))
        });
        (
            nodes.into_iter().map(|stored| stored.node).collect(),
            self.diagnostics,
        )
    }

    /// Records the hard node limit only once.
    fn report_limit(&mut self, source_path: Option<&Path>) {
        if self.limit_reported {
            return;
        }
        self.limit_reported = true;
        self.diagnostics.push(TraceDiagnostic {
            locator: None,
            path: source_path.map(Path::to_path_buf),
            evidence: EvidenceGrade::Unavailable,
            message: format!(
                "session node materialization stopped at {} nodes",
                self.max_nodes
            ),
        });
    }
}

/// Orders known source positions before stable unpositioned observations.
fn sibling_order_cmp(left: SiblingOrder, right: SiblingOrder) -> std::cmp::Ordering {
    match (left, right) {
        (SiblingOrder::Timestamp(left), SiblingOrder::Timestamp(right)) => left.cmp(&right),
        (SiblingOrder::Sequence(left), SiblingOrder::Sequence(right)) => left.cmp(&right),
        (SiblingOrder::Timestamp(_), SiblingOrder::Sequence(_))
        | (SiblingOrder::Timestamp(_), SiblingOrder::Unspecified)
        | (SiblingOrder::Sequence(_), SiblingOrder::Unspecified) => std::cmp::Ordering::Less,
        (SiblingOrder::Sequence(_), SiblingOrder::Timestamp(_))
        | (SiblingOrder::Unspecified, SiblingOrder::Timestamp(_))
        | (SiblingOrder::Unspecified, SiblingOrder::Sequence(_)) => std::cmp::Ordering::Greater,
        (SiblingOrder::Unspecified, SiblingOrder::Unspecified) => std::cmp::Ordering::Equal,
    }
}

/// Compares semantic observation fields while ignoring the disambiguated locator.
fn same_observation(previous: &TraceNode, current: &TraceNode) -> bool {
    previous.parent == current.parent
        && previous.provenance == current.provenance
        && previous.evidence == current.evidence
        && previous.timestamp == current.timestamp
        && previous.label == current.label
        && previous.presentation == current.presentation
        && previous.detail == current.detail
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
