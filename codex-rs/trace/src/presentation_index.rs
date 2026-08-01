//! Immutable renderer-neutral presentation snapshot and indexed queries.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::GroupId;
use crate::OrderBandId;
use crate::PresentationBuildStatus;
use crate::PresentationDiagnostic;
use crate::PresentationDisposition;
use crate::PresentationEvent;
use crate::PresentationGroup;
use crate::PresentationOrderBand;
use crate::PresentationScope;
use crate::SessionTrace;

/// Immutable presentation snapshot over one loaded [`SessionTrace`].
///
/// Inserting, removing, or reordering canonical nodes, or changing their
/// locators or typed facts, invalidates this snapshot. Rebuild it after any
/// such mutation. Queries return precomputed groups, bands, events, and
/// memberships without inspecting node detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationIndex {
    groups: Vec<PresentationGroup>,
    group_positions: HashMap<GroupId, usize>,
    primary_groups: Vec<Option<usize>>,
    dispositions: Vec<PresentationDisposition>,
    bands: Vec<PresentationOrderBand>,
    scope_bands: BTreeMap<PresentationScope, Vec<usize>>,
    scope_groups: BTreeMap<PresentationScope, Vec<usize>>,
    scope_events: BTreeMap<PresentationScope, Vec<PresentationEvent>>,
    diagnostics: Vec<PresentationDiagnostic>,
    status: PresentationBuildStatus,
}

impl PresentationIndex {
    /// Builds one bounded deterministic snapshot from canonical typed facts.
    pub fn new(trace: &SessionTrace) -> Self {
        crate::presentation_build::build(trace)
    }

    /// Returns whether every required primary membership was admitted.
    pub fn status(&self) -> PresentationBuildStatus {
        self.status
    }

    /// Returns every materialized group in deterministic snapshot order.
    pub fn groups(&self) -> &[PresentationGroup] {
        &self.groups
    }

    /// Looks up one presentation group by stable snapshot identity.
    pub fn group(&self, id: &GroupId) -> Option<&PresentationGroup> {
        self.group_positions
            .get(id)
            .and_then(|position| self.groups.get(*position))
    }

    /// Returns the primary group owning one canonical node position.
    pub fn group_for_node(&self, node_position: usize) -> Option<&PresentationGroup> {
        self.primary_groups
            .get(node_position)
            .and_then(|position| *position)
            .and_then(|position| self.groups.get(position))
    }

    /// Returns the presentation disposition of one canonical node position.
    pub fn disposition(&self, node_position: usize) -> Option<PresentationDisposition> {
        self.dispositions.get(node_position).copied()
    }

    /// Iterates collapsed groups for one scope in precomputed anchor order.
    pub fn groups_in_scope<'a>(
        &'a self,
        scope: &'a PresentationScope,
    ) -> impl Iterator<Item = &'a PresentationGroup> + 'a {
        self.scope_groups
            .get(scope)
            .into_iter()
            .flatten()
            .filter_map(|position| self.groups.get(*position))
    }

    /// Returns expanded canonical events for one scope.
    pub fn events_in_scope(&self, scope: &PresentationScope) -> &[PresentationEvent] {
        self.scope_events.get(scope).map_or(&[], Vec::as_slice)
    }

    /// Iterates partial-order bands belonging to one scope.
    pub fn bands_in_scope<'a>(
        &'a self,
        scope: &'a PresentationScope,
    ) -> impl Iterator<Item = &'a PresentationOrderBand> + 'a {
        self.scope_bands
            .get(scope)
            .into_iter()
            .flatten()
            .filter_map(|position| self.bands.get(*position))
    }

    /// Looks up one snapshot-local order band.
    pub fn band(&self, id: OrderBandId) -> Option<&PresentationOrderBand> {
        self.bands.get(id.0).filter(|band| band.id == id)
    }

    /// Returns bounded construction and validation diagnostics.
    pub fn diagnostics(&self) -> &[PresentationDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn from_parts(parts: PresentationIndexParts) -> Self {
        let group_positions = parts
            .groups
            .iter()
            .enumerate()
            .map(|(position, group)| (group.id.clone(), position))
            .collect();
        Self {
            groups: parts.groups,
            group_positions,
            primary_groups: parts.primary_groups,
            dispositions: parts.dispositions,
            bands: parts.bands,
            scope_bands: parts.scope_bands,
            scope_groups: parts.scope_groups,
            scope_events: parts.scope_events,
            diagnostics: parts.diagnostics,
            status: parts.status,
        }
    }
}

/// Fully validated pieces transferred from the construction pipeline.
pub(crate) struct PresentationIndexParts {
    pub(crate) groups: Vec<PresentationGroup>,
    pub(crate) primary_groups: Vec<Option<usize>>,
    pub(crate) dispositions: Vec<PresentationDisposition>,
    pub(crate) bands: Vec<PresentationOrderBand>,
    pub(crate) scope_bands: BTreeMap<PresentationScope, Vec<usize>>,
    pub(crate) scope_groups: BTreeMap<PresentationScope, Vec<usize>>,
    pub(crate) scope_events: BTreeMap<PresentationScope, Vec<PresentationEvent>>,
    pub(crate) diagnostics: Vec<PresentationDiagnostic>,
    pub(crate) status: PresentationBuildStatus,
}

#[cfg(test)]
#[path = "presentation_index_tests.rs"]
mod tests;
