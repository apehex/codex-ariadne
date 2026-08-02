//! Aggregate typed facts retained for one normalized trace node.

use std::cmp::Ordering;

use super::TraceActivity;
use super::TraceCorrelation;
use super::TraceOrder;
use super::TraceOrderPoint;
use super::TraceOwnership;

pub(crate) const MAX_CORRELATIONS_PER_NODE: usize = 4_096;

/// Completeness of the typed order and correlation facts for one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceFactAvailability {
    /// The contributing source exposed every supported fact.
    Complete,
    /// Some supported facts were absent or bounded away.
    Partial,
    /// The source record could not be interpreted into typed facts.
    #[default]
    Unavailable,
    /// Retained observations disagree about the same source identity.
    Conflicting,
}

/// Small typed facts consumed only by presentation policies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TracePolicyFacts {
    /// Source-derived activity, or `None` when the record has no supported policy role.
    pub activity: Option<TraceActivity>,
}

/// Typed order, ownership, and correlation facts for one retained node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceNodeFacts {
    /// Source order and display time.
    pub order: TraceOrder,
    /// Thread and turn ownership.
    pub ownership: TraceOwnership,
    /// Whether supported facts were completely retained.
    pub availability: TraceFactAvailability,
    /// Bounded typed relations to other source objects.
    pub correlations: Vec<TraceCorrelation>,
    /// Typed renderer-neutral facts needed by grouping policies.
    pub policy: TracePolicyFacts,
}

impl TraceNodeFacts {
    pub(crate) fn new(
        order: TraceOrder,
        ownership: TraceOwnership,
        availability: TraceFactAvailability,
        correlations: impl IntoIterator<Item = TraceCorrelation>,
    ) -> Self {
        let mut correlations = correlations.into_iter();
        let retained = correlations
            .by_ref()
            .take(MAX_CORRELATIONS_PER_NODE)
            .collect::<Vec<_>>();
        let truncated = correlations.next().is_some();
        Self {
            order,
            ownership,
            availability: if truncated {
                TraceFactAvailability::Partial
            } else {
                availability
            },
            correlations: retained,
            policy: TracePolicyFacts::default(),
        }
    }

    /// Attaches one typed activity while preserving order and correlation facts.
    pub(crate) fn with_activity(mut self, activity: TraceActivity) -> Self {
        self.policy.activity = Some(activity);
        self
    }

    /// Compares causal source positions only when their domains are compatible.
    pub fn source_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.order.start, other.order.start) {
            (TraceOrderPoint::Ordinary { .. }, TraceOrderPoint::Ordinary { .. })
                if self.ownership.thread_id == other.ownership.thread_id
                    && self.ownership.thread_id.is_some() =>
            {
                self.order.start.same_domain_cmp(other.order.start)
            }
            (TraceOrderPoint::Rich { .. }, TraceOrderPoint::Rich { .. }) => {
                self.order.start.same_domain_cmp(other.order.start)
            }
            (
                TraceOrderPoint::Ordinary { .. }
                | TraceOrderPoint::Rich { .. }
                | TraceOrderPoint::Structural { .. }
                | TraceOrderPoint::Unspecified,
                TraceOrderPoint::Ordinary { .. }
                | TraceOrderPoint::Rich { .. }
                | TraceOrderPoint::Structural { .. }
                | TraceOrderPoint::Unspecified,
            ) => None,
        }
    }
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
