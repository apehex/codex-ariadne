//! Bounded, evidence-preserving admission of normalized trace nodes.

use std::collections::BTreeMap;
use std::path::Path;

use crate::EvidenceGrade;
use crate::TraceDiagnostic;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeLocator;
use crate::TraceOrderPoint;

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

/// One retained node plus typed source facts.
#[derive(Debug)]
struct StoredNode {
    node: TraceNode,
    facts: TraceNodeFacts,
    admission_index: usize,
}

/// Owns node limits, duplicate identities, and conflict diagnostics.
#[derive(Debug)]
pub(crate) struct TraceGraphBuilder {
    max_nodes: usize,
    max_correlations: usize,
    retained_correlations: usize,
    nodes: BTreeMap<TraceNodeLocator, StoredNode>,
    occurrences: BTreeMap<TraceNodeLocator, usize>,
    latest_observations: BTreeMap<TraceNodeLocator, TraceNodeLocator>,
    diagnostics: Vec<TraceDiagnostic>,
    limit_reported: bool,
    correlation_limit_reported: bool,
    next_admission_index: usize,
}

impl TraceGraphBuilder {
    /// Creates a builder with pre-existing discovery or replay diagnostics.
    pub(crate) fn new(max_nodes: usize, diagnostics: Vec<TraceDiagnostic>) -> Self {
        Self {
            max_nodes,
            max_correlations: max_nodes.saturating_mul(4),
            retained_correlations: 0,
            nodes: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            latest_observations: BTreeMap::new(),
            diagnostics,
            limit_reported: false,
            correlation_limit_reported: false,
            next_admission_index: 0,
        }
    }

    /// Retains one observation, preserving duplicate identities as separate nodes.
    pub(crate) fn admit(
        &mut self,
        mut node: TraceNode,
        facts: TraceNodeFacts,
        source_path: Option<&Path>,
    ) -> Admission {
        self.remap_parent(&mut node);
        self.retain(node, facts, source_path)
    }

    /// Retains a child only when its parent is already present.
    pub(crate) fn admit_with_required_parent(
        &mut self,
        mut node: TraceNode,
        facts: TraceNodeFacts,
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
        self.retain(node, facts, source_path)
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
        mut facts: TraceNodeFacts,
        source_path: Option<&Path>,
    ) -> Admission {
        if self.nodes.len() >= self.max_nodes {
            self.report_limit(source_path);
            return Admission::LimitReached;
        }
        let base = node.locator.clone();
        let conflicting = self.nodes.get(&base).is_some_and(|previous| {
            !same_observation(&previous.node, &node, &previous.facts, &facts)
        });
        if self.nodes.contains_key(&base) {
            let occurrence = self.occurrences.entry(base.clone()).or_insert(1);
            *occurrence += 1;
            node.locator.id = format!("{}:observation:{}", base.id, *occurrence);
            if conflicting {
                facts.availability = crate::TraceFactAvailability::Conflicting;
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
        if conflicting && let Some(previous) = self.nodes.get_mut(&base) {
            previous.facts.availability = crate::TraceFactAvailability::Conflicting;
        }
        let locator = node.locator.clone();
        self.bound_correlations(&locator, &mut facts, source_path);
        self.latest_observations.insert(base, locator.clone());
        let admission_index = self.next_admission_index;
        self.next_admission_index += 1;
        facts.order.tie_break = admission_index;
        self.nodes.insert(
            locator.clone(),
            StoredNode {
                node,
                facts,
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
    pub(crate) fn finish(
        self,
    ) -> (
        Vec<TraceNode>,
        BTreeMap<TraceNodeLocator, TraceNodeFacts>,
        Vec<TraceDiagnostic>,
    ) {
        let mut nodes = self.nodes.into_values().collect::<Vec<_>>();
        nodes.sort_by(|left, right| {
            left.node
                .parent
                .cmp(&right.node.parent)
                .then_with(|| source_order_cmp(left.facts.order.start, right.facts.order.start))
                .then(left.admission_index.cmp(&right.admission_index))
                .then_with(|| left.node.locator.cmp(&right.node.locator))
        });
        let facts = nodes
            .iter()
            .map(|stored| (stored.node.locator.clone(), stored.facts.clone()))
            .collect();
        let nodes = nodes.into_iter().map(|stored| stored.node).collect();
        (nodes, facts, self.diagnostics)
    }

    /// Applies the global correlation budget without hiding retained nodes.
    fn bound_correlations(
        &mut self,
        locator: &TraceNodeLocator,
        facts: &mut TraceNodeFacts,
        source_path: Option<&Path>,
    ) {
        let remaining = self
            .max_correlations
            .saturating_sub(self.retained_correlations);
        if facts.correlations.len() > remaining {
            facts.correlations.truncate(remaining);
            if facts.availability == crate::TraceFactAvailability::Complete {
                facts.availability = crate::TraceFactAvailability::Partial;
            }
            if !self.correlation_limit_reported {
                self.correlation_limit_reported = true;
                self.diagnostics.push(TraceDiagnostic {
                    locator: Some(locator.clone()),
                    path: source_path.map(Path::to_path_buf),
                    evidence: EvidenceGrade::Unavailable,
                    message: format!(
                        "typed correlations stopped at the session limit of {}",
                        self.max_correlations
                    ),
                });
            }
        }
        self.retained_correlations = self
            .retained_correlations
            .saturating_add(facts.correlations.len());
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
fn source_order_cmp(left: TraceOrderPoint, right: TraceOrderPoint) -> std::cmp::Ordering {
    match (left, right) {
        (
            TraceOrderPoint::Structural { unix_ms: left },
            TraceOrderPoint::Structural { unix_ms: right },
        ) => left.cmp(&right),
        (
            TraceOrderPoint::Ordinary { ordinal: left },
            TraceOrderPoint::Ordinary { ordinal: right },
        ) => left.cmp(&right),
        (TraceOrderPoint::Rich { sequence: left }, TraceOrderPoint::Rich { sequence: right }) => {
            left.cmp(&right)
        }
        (
            TraceOrderPoint::Structural { .. },
            TraceOrderPoint::Ordinary { .. }
            | TraceOrderPoint::Rich { .. }
            | TraceOrderPoint::Unspecified,
        )
        | (
            TraceOrderPoint::Ordinary { .. } | TraceOrderPoint::Rich { .. },
            TraceOrderPoint::Unspecified,
        ) => std::cmp::Ordering::Less,
        (
            TraceOrderPoint::Ordinary { .. }
            | TraceOrderPoint::Rich { .. }
            | TraceOrderPoint::Unspecified,
            TraceOrderPoint::Structural { .. },
        )
        | (
            TraceOrderPoint::Unspecified,
            TraceOrderPoint::Ordinary { .. } | TraceOrderPoint::Rich { .. },
        ) => std::cmp::Ordering::Greater,
        (TraceOrderPoint::Ordinary { .. }, TraceOrderPoint::Rich { .. })
        | (TraceOrderPoint::Rich { .. }, TraceOrderPoint::Ordinary { .. })
        | (TraceOrderPoint::Unspecified, TraceOrderPoint::Unspecified) => std::cmp::Ordering::Equal,
    }
}

/// Compares semantic observation fields while ignoring the disambiguated locator.
fn same_observation(
    previous: &TraceNode,
    current: &TraceNode,
    previous_facts: &TraceNodeFacts,
    current_facts: &TraceNodeFacts,
) -> bool {
    previous.parent == current.parent
        && previous.provenance == current.provenance
        && previous.evidence == current.evidence
        && previous.timestamp == current.timestamp
        && previous.label == current.label
        && previous.presentation == current.presentation
        && previous.detail == current.detail
        && previous_facts.order.start == current_facts.order.start
        && previous_facts.order.end == current_facts.order.end
        && previous_facts.order.wall_clock_start_ms == current_facts.order.wall_clock_start_ms
        && previous_facts.order.wall_clock_end_ms == current_facts.order.wall_clock_end_ms
        && previous_facts.ownership == current_facts.ownership
        && previous_facts.availability == current_facts.availability
        && previous_facts.correlations == current_facts.correlations
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
