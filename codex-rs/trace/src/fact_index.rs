//! Immutable lookup snapshots over retained trace facts.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::SessionTrace;
use crate::TraceCorrelation;
use crate::TraceNodeFacts;
use crate::TraceNodeLocator;
use crate::TraceOrder;
use crate::TraceOrderDomain;
use crate::TraceOrderPoint;

/// Immutable lookup snapshot for typed node facts.
///
/// Like [`crate::TraceIndex`], this index is invalidated by inserting,
/// removing, or reordering nodes or changing a node locator. Rebuild it after
/// any such mutation. Label, detail, and presentation-only changes do not alter
/// its order and correlation snapshots.
#[derive(Debug, Clone)]
pub struct TraceFactIndex {
    node_positions: HashMap<TraceNodeLocator, usize>,
    facts: Vec<TraceNodeFacts>,
    thread_positions: HashMap<String, HashMap<TraceOrderDomain, Vec<usize>>>,
    correlation_positions: HashMap<TraceCorrelation, Vec<usize>>,
}

impl TraceFactIndex {
    /// Builds typed fact lookup maps for one immutable session trace.
    pub fn new(trace: &SessionTrace) -> Self {
        let mut node_positions = HashMap::with_capacity(trace.nodes.len());
        let mut facts = Vec::with_capacity(trace.nodes.len());
        let mut thread_positions = HashMap::<String, HashMap<TraceOrderDomain, Vec<usize>>>::new();
        let mut correlation_positions = HashMap::<TraceCorrelation, Vec<usize>>::new();

        for (position, node) in trace.nodes.iter().enumerate() {
            node_positions
                .entry(node.locator.clone())
                .or_insert(position);
            let fact = trace
                .facts
                .get(&node.locator)
                .cloned()
                .unwrap_or_else(|| TraceNodeFacts {
                    order: TraceOrder {
                        tie_break: position,
                        ..TraceOrder::default()
                    },
                    ..TraceNodeFacts::default()
                });
            if let Some(thread_id) = &fact.ownership.thread_id {
                thread_positions
                    .entry(thread_id.clone())
                    .or_default()
                    .entry(fact.order.start.domain())
                    .or_default()
                    .push(position);
            }
            for correlation in &fact.correlations {
                correlation_positions
                    .entry(correlation.clone())
                    .or_default()
                    .push(position);
            }
            facts.push(fact);
        }
        for domains in thread_positions.values_mut() {
            for positions in domains.values_mut() {
                positions.sort_by(|left, right| {
                    stable_order_cmp(&facts[*left].order, &facts[*right].order)
                });
            }
        }

        Self {
            node_positions,
            facts,
            thread_positions,
            correlation_positions,
        }
    }

    /// Returns facts for the first retained node with this locator.
    pub fn facts<'a>(
        &'a self,
        trace: &SessionTrace,
        locator: &TraceNodeLocator,
    ) -> Option<&'a TraceNodeFacts> {
        let position = *self.node_positions.get(locator)?;
        trace
            .nodes
            .get(position)
            .filter(|node| node.locator == *locator)?;
        self.facts.get(position)
    }

    /// Returns facts at one compact node position.
    pub fn facts_at<'a>(
        &'a self,
        trace: &SessionTrace,
        position: usize,
    ) -> Option<&'a TraceNodeFacts> {
        trace.nodes.get(position)?;
        self.facts.get(position)
    }

    /// Iterates one thread's positions in deterministic order within one source domain.
    pub fn thread_positions<'a>(
        &'a self,
        trace: &'a SessionTrace,
        thread_id: &str,
        domain: TraceOrderDomain,
    ) -> impl Iterator<Item = usize> + 'a {
        self.thread_positions
            .get(thread_id)
            .and_then(|domains| domains.get(&domain))
            .into_iter()
            .flatten()
            .copied()
            .filter(|position| trace.nodes.get(*position).is_some())
    }

    /// Iterates node positions carrying one exact typed correlation.
    pub fn correlated_positions<'a>(
        &'a self,
        trace: &'a SessionTrace,
        correlation: &'a TraceCorrelation,
    ) -> impl Iterator<Item = usize> + 'a {
        self.correlation_positions
            .get(correlation)
            .into_iter()
            .flatten()
            .copied()
            .filter(|position| trace.nodes.get(*position).is_some())
    }
}

fn stable_order_cmp(left: &TraceOrder, right: &TraceOrder) -> Ordering {
    match (left.start, right.start) {
        (
            TraceOrderPoint::Ordinary { ordinal: left },
            TraceOrderPoint::Ordinary { ordinal: right },
        ) => left.cmp(&right),
        (TraceOrderPoint::Rich { sequence: left }, TraceOrderPoint::Rich { sequence: right }) => {
            left.cmp(&right)
        }
        (
            TraceOrderPoint::Structural { unix_ms: left },
            TraceOrderPoint::Structural { unix_ms: right },
        ) => left.cmp(&right),
        (TraceOrderPoint::Unspecified, TraceOrderPoint::Unspecified)
        | (
            TraceOrderPoint::Ordinary { .. },
            TraceOrderPoint::Rich { .. }
            | TraceOrderPoint::Structural { .. }
            | TraceOrderPoint::Unspecified,
        )
        | (
            TraceOrderPoint::Rich { .. },
            TraceOrderPoint::Ordinary { .. }
            | TraceOrderPoint::Structural { .. }
            | TraceOrderPoint::Unspecified,
        )
        | (
            TraceOrderPoint::Structural { .. },
            TraceOrderPoint::Ordinary { .. }
            | TraceOrderPoint::Rich { .. }
            | TraceOrderPoint::Unspecified,
        )
        | (
            TraceOrderPoint::Unspecified,
            TraceOrderPoint::Ordinary { .. }
            | TraceOrderPoint::Rich { .. }
            | TraceOrderPoint::Structural { .. },
        ) => Ordering::Equal,
    }
    .then(left.tie_break.cmp(&right.tie_break))
}

#[cfg(test)]
#[path = "fact_index_tests.rs"]
mod tests;
