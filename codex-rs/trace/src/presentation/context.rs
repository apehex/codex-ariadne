//! Shared immutable facts and indexes for one presentation build.

use std::collections::HashMap;

use super::PresentationLimits;
use super::PresentationScope;
use crate::SessionTrace;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceRelation;

/// Shared trace access and precomputed lookups for one presentation build.
pub(crate) struct BuildContext<'a> {
    trace: &'a SessionTrace,
    limits: PresentationLimits,
    missing_facts: TraceNodeFacts,
    identity_positions: HashMap<TraceObjectRef, Vec<usize>>,
}

impl<'a> BuildContext<'a> {
    pub(crate) fn new(trace: &'a SessionTrace, limits: PresentationLimits) -> Self {
        let missing_facts = TraceNodeFacts::default();
        let mut identity_positions = HashMap::<TraceObjectRef, Vec<usize>>::new();
        for (position, node) in trace.nodes.iter().enumerate() {
            let facts = trace.facts.get(&node.locator).unwrap_or(&missing_facts);
            for identity in facts
                .correlations
                .iter()
                .filter(|correlation| correlation.relation == TraceRelation::SourceIdentity)
                .map(|correlation| correlation.target.clone())
            {
                identity_positions
                    .entry(identity)
                    .or_default()
                    .push(position);
            }
        }
        Self {
            trace,
            limits: limits.effective(),
            missing_facts,
            identity_positions,
        }
    }

    pub(crate) fn trace(&self) -> &'a SessionTrace {
        self.trace
    }

    pub(crate) fn limits(&self) -> PresentationLimits {
        self.limits
    }

    pub(crate) fn node_at(&self, position: usize) -> Option<&'a TraceNode> {
        self.trace.nodes.get(position)
    }

    pub(crate) fn facts_at(&self, position: usize) -> &TraceNodeFacts {
        self.node_at(position)
            .and_then(|node| self.trace.facts.get(&node.locator))
            .unwrap_or(&self.missing_facts)
    }

    pub(crate) fn scope_at(&self, position: usize) -> PresentationScope {
        self.facts_at(position)
            .ownership
            .thread_id
            .as_ref()
            .map_or(PresentationScope::Session, |thread_id| {
                PresentationScope::Thread(thread_id.clone())
            })
    }

    pub(crate) fn identity_positions(&self) -> &HashMap<TraceObjectRef, Vec<usize>> {
        &self.identity_positions
    }
}
