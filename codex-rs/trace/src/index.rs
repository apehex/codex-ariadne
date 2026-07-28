use std::collections::HashMap;

use crate::SessionTrace;
use crate::TraceNode;
use crate::TraceNodeLocator;

/// Snapshot index for routine navigation through a [`SessionTrace`].
///
/// Construction is linear in the number of retained nodes. Locator and
/// adjacency selection are expected constant time, and iterating roots or
/// children is linear only in the number of returned nodes. Root and child
/// iteration preserves the order of [`SessionTrace::nodes`].
///
/// The index is a snapshot because the trace's public node fields remain
/// mutable. Inserting, removing, or reordering nodes, or changing a node's
/// locator or parent, requires rebuilding the index.
#[derive(Debug, Clone)]
pub struct TraceIndex {
    node_positions: HashMap<TraceNodeLocator, usize>,
    root_positions: Vec<usize>,
    child_positions: HashMap<TraceNodeLocator, Vec<usize>>,
}

impl TraceIndex {
    pub fn new(trace: &SessionTrace) -> Self {
        let mut node_positions = HashMap::with_capacity(trace.nodes.len());
        let mut root_positions = Vec::new();
        let mut child_positions = HashMap::<TraceNodeLocator, Vec<usize>>::new();

        for (position, node) in trace.nodes.iter().enumerate() {
            node_positions
                .entry(node.locator.clone())
                .or_insert(position);
            match &node.parent {
                Some(parent) => child_positions
                    .entry(parent.clone())
                    .or_default()
                    .push(position),
                None => root_positions.push(position),
            }
        }

        Self {
            node_positions,
            root_positions,
            child_positions,
        }
    }

    pub fn node<'a>(
        &self,
        trace: &'a SessionTrace,
        locator: &TraceNodeLocator,
    ) -> Option<&'a TraceNode> {
        self.node_position(trace, locator)
            .and_then(|position| trace.nodes.get(position))
    }

    /// Returns the indexed vector position for a locator.
    ///
    /// This is useful for UI caches that need compact stable references into an
    /// immutable loaded trace. The same snapshot invalidation contract applies.
    pub fn node_position(&self, trace: &SessionTrace, locator: &TraceNodeLocator) -> Option<usize> {
        let position = *self.node_positions.get(locator)?;
        let node = trace.nodes.get(position)?;
        (node.locator == *locator).then_some(position)
    }

    pub fn root_nodes<'a>(
        &'a self,
        trace: &'a SessionTrace,
    ) -> impl Iterator<Item = &'a TraceNode> + 'a {
        self.root_positions(trace)
            .filter_map(|position| trace.nodes.get(position))
    }

    /// Returns compact positions for roots in source node order.
    pub fn root_positions<'a>(
        &'a self,
        trace: &'a SessionTrace,
    ) -> impl Iterator<Item = usize> + 'a {
        self.root_positions.iter().copied().filter(|position| {
            trace
                .nodes
                .get(*position)
                .is_some_and(|node| node.parent.is_none())
        })
    }

    pub fn children<'a>(
        &'a self,
        trace: &'a SessionTrace,
        parent: &'a TraceNodeLocator,
    ) -> impl Iterator<Item = &'a TraceNode> + 'a {
        self.child_positions(trace, parent)
            .filter_map(|position| trace.nodes.get(position))
    }

    /// Returns compact positions for direct children in source node order.
    pub fn child_positions<'a>(
        &'a self,
        trace: &'a SessionTrace,
        parent: &'a TraceNodeLocator,
    ) -> impl Iterator<Item = usize> + 'a {
        self.child_positions
            .get(parent)
            .into_iter()
            .flatten()
            .copied()
            .filter(|position| {
                trace
                    .nodes
                    .get(*position)
                    .is_some_and(|node| node.parent.as_ref() == Some(parent))
            })
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
