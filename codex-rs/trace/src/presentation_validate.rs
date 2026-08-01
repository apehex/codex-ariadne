//! Validation of presentation membership, order, and resource invariants.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::GroupId;
use crate::OrderBandId;
use crate::PresentationBuildStatus;
use crate::PresentationDisposition;
use crate::PresentationGroup;
use crate::PresentationOrderBand;
use crate::PresentationScope;
use crate::SessionTrace;
use crate::presentation_build::DiagnosticSink;
use crate::presentation_model::PresentationDiagnosticCode;

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate(
    trace: &SessionTrace,
    groups: &[PresentationGroup],
    primary_groups: &[Option<usize>],
    dispositions: &[PresentationDisposition],
    bands: &[PresentationOrderBand],
    scope_events: &std::collections::BTreeMap<PresentationScope, Vec<crate::PresentationEvent>>,
    diagnostics: &mut DiagnosticSink,
    status: &mut PresentationBuildStatus,
) {
    let node_count = trace.nodes.len();
    let primary_count = dispositions
        .iter()
        .filter(|disposition| **disposition == PresentationDisposition::Primary)
        .count();
    if primary_groups.len() != node_count || dispositions.len() != node_count {
        invalid(
            diagnostics,
            status,
            "presentation position tables do not match the trace",
        );
    }
    if primary_groups.iter().flatten().count() != primary_count {
        invalid(
            diagnostics,
            status,
            "a primary event lacks exactly one group",
        );
    }
    if groups.len() > node_count.saturating_mul(2) {
        invalid(
            diagnostics,
            status,
            "group count exceeds the 2N structural bound",
        );
    }
    let membership_count = groups
        .iter()
        .map(|group| group.members.len().saturating_add(group.child_groups.len()))
        .sum::<usize>();
    if membership_count > node_count.saturating_mul(3) {
        invalid(
            diagnostics,
            status,
            "membership count exceeds the 3N structural bound",
        );
    }
    let reference_count = groups
        .iter()
        .map(|group| group.references.len())
        .sum::<usize>();
    if reference_count > node_count.saturating_mul(4)
        || groups.iter().any(|group| group.references.len() > 4_096)
    {
        invalid(
            diagnostics,
            status,
            "secondary references exceed their structural bound",
        );
    }

    let group_positions = groups
        .iter()
        .enumerate()
        .map(|(position, group)| (group.id.clone(), position))
        .collect::<HashMap<_, _>>();
    if group_positions.len() != groups.len() {
        invalid(diagnostics, status, "duplicate presentation group identity");
    }
    let mut owned = vec![0_usize; node_count];
    for (group_position, group) in groups.iter().enumerate() {
        if group.members.len() > node_count {
            invalid(diagnostics, status, "group direct membership exceeds N");
        }
        for member in &group.members {
            let Some(owner) = owned.get_mut(member.node_position) else {
                invalid(
                    diagnostics,
                    status,
                    "group member position is outside the trace",
                );
                continue;
            };
            *owner += 1;
            if primary_groups.get(member.node_position).copied().flatten() != Some(group_position) {
                invalid(
                    diagnostics,
                    status,
                    "primary membership reverse lookup disagrees",
                );
            }
            if scope_for_position(trace, member.node_position) != group.scope {
                invalid(
                    diagnostics,
                    status,
                    "primary group crosses recorded thread scopes",
                );
            }
        }
    }
    for (position, disposition) in dispositions.iter().enumerate() {
        let expected = usize::from(*disposition == PresentationDisposition::Primary);
        if owned.get(position).copied().unwrap_or_default() != expected {
            invalid(
                diagnostics,
                status,
                "canonical position has invalid primary ownership",
            );
        }
    }
    validate_children(groups, &group_positions, diagnostics, status);
    let node_bands = validate_order(
        node_count,
        groups,
        primary_groups,
        bands,
        scope_events,
        diagnostics,
        status,
    );
    for group in groups {
        let expected_anchor = group
            .members
            .iter()
            .filter_map(|member| node_bands.get(member.node_position).copied().flatten())
            .min();
        if expected_anchor != Some(group.anchor_band) {
            invalid(
                diagnostics,
                status,
                "group anchor does not match its first order band",
            );
        }
        if bands
            .get(group.anchor_band.0)
            .is_none_or(|band| band.scope != group.scope)
        {
            invalid(
                diagnostics,
                status,
                "group anchor belongs to a different scope",
            );
        }
    }
}

fn validate_children(
    groups: &[PresentationGroup],
    positions: &HashMap<GroupId, usize>,
    diagnostics: &mut DiagnosticSink,
    status: &mut PresentationBuildStatus,
) {
    let mut parent_count = HashMap::<GroupId, usize>::new();
    for group in groups {
        for child in &group.child_groups {
            if !positions.contains_key(child) {
                invalid(diagnostics, status, "group names a missing child");
            }
            *parent_count.entry(child.clone()).or_default() += 1;
        }
    }
    if parent_count.values().any(|count| *count > 1) {
        invalid(
            diagnostics,
            status,
            "group has more than one containing group",
        );
    }
    for group in groups {
        let mut path = HashSet::new();
        if depth(group, groups, positions, &mut path) > 4 {
            invalid(
                diagnostics,
                status,
                "group nesting is cyclic or exceeds depth four",
            );
            break;
        }
    }
}

fn depth(
    group: &PresentationGroup,
    groups: &[PresentationGroup],
    positions: &HashMap<GroupId, usize>,
    path: &mut HashSet<GroupId>,
) -> usize {
    if !path.insert(group.id.clone()) {
        return usize::MAX;
    }
    let child_depth = group
        .child_groups
        .iter()
        .filter_map(|child| {
            positions
                .get(child)
                .and_then(|position| groups.get(*position))
        })
        .map(|child| depth(child, groups, positions, path))
        .max()
        .unwrap_or(0);
    path.remove(&group.id);
    child_depth.saturating_add(1)
}

fn validate_order(
    node_count: usize,
    groups: &[PresentationGroup],
    primary_groups: &[Option<usize>],
    bands: &[PresentationOrderBand],
    scope_events: &std::collections::BTreeMap<PresentationScope, Vec<crate::PresentationEvent>>,
    diagnostics: &mut DiagnosticSink,
    status: &mut PresentationBuildStatus,
) -> Vec<Option<OrderBandId>> {
    let mut band_counts = vec![0_usize; node_count];
    let mut node_bands = vec![None; node_count];
    let mut expected_events = std::collections::BTreeMap::<PresentationScope, Vec<usize>>::new();
    for (position, band) in bands.iter().enumerate() {
        if band.id.0 != position {
            invalid(
                diagnostics,
                status,
                "order-band identity is not deterministic",
            );
        }
        for node_position in band
            .segments
            .iter()
            .flat_map(|segment| segment.node_positions.iter().copied())
        {
            expected_events
                .entry(band.scope.clone())
                .or_default()
                .push(node_position);
            let Some(count) = band_counts.get_mut(node_position) else {
                invalid(
                    diagnostics,
                    status,
                    "order band contains a position outside the trace",
                );
                continue;
            };
            *count += 1;
            if node_bands[node_position].replace(band.id).is_some() {
                invalid(
                    diagnostics,
                    status,
                    "primary event appears in multiple order bands",
                );
            }
        }
    }
    let mut event_counts = vec![0_usize; node_count];
    if expected_events.len() != scope_events.len() {
        invalid(
            diagnostics,
            status,
            "order-band and expanded-event scopes disagree",
        );
    }
    for (scope, events) in scope_events {
        let expected = expected_events.get(scope).map_or(&[][..], Vec::as_slice);
        if events
            .iter()
            .map(|event| event.node_position)
            .ne(expected.iter().copied())
        {
            invalid(
                diagnostics,
                status,
                "expanded event order disagrees with order bands",
            );
        }
        for event in events {
            let Some(count) = event_counts.get_mut(event.node_position) else {
                invalid(diagnostics, status, "event position is outside the trace");
                continue;
            };
            *count += 1;
            let Some(band) = bands.get(event.band.0) else {
                invalid(diagnostics, status, "event names a missing order band");
                continue;
            };
            if &band.scope != scope || node_bands[event.node_position] != Some(event.band) {
                invalid(
                    diagnostics,
                    status,
                    "event names an inconsistent order band",
                );
            }
            let expected_group = primary_groups
                .get(event.node_position)
                .copied()
                .flatten()
                .and_then(|position| groups.get(position))
                .map(|group| &group.id);
            if expected_group != Some(&event.primary_group) {
                invalid(
                    diagnostics,
                    status,
                    "event primary group annotation disagrees",
                );
            }
        }
    }
    for (position, group) in primary_groups.iter().enumerate() {
        let expected = usize::from(group.is_some());
        if event_counts.get(position).copied().unwrap_or_default() != expected {
            invalid(
                diagnostics,
                status,
                "primary event does not appear once in event order",
            );
        }
        if band_counts.get(position).copied().unwrap_or_default() != expected {
            invalid(
                diagnostics,
                status,
                "primary event does not appear once in order bands",
            );
        }
    }
    node_bands
}

fn scope_for_position(trace: &SessionTrace, position: usize) -> PresentationScope {
    trace
        .nodes
        .get(position)
        .and_then(|node| trace.facts.get(&node.locator))
        .and_then(|facts| facts.ownership.thread_id.as_ref())
        .map_or(PresentationScope::Session, |thread| {
            PresentationScope::Thread(thread.clone())
        })
}

fn invalid(diagnostics: &mut DiagnosticSink, status: &mut PresentationBuildStatus, message: &str) {
    *status = PresentationBuildStatus::Incomplete;
    diagnostics.push(
        PresentationDiagnosticCode::InvalidStructure,
        None,
        None,
        message,
    );
}
