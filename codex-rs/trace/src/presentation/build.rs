//! Bounded construction of the minimal static presentation snapshot.

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::BuildContext;
use super::DiagnosticSink;
use super::PresentationLimits;
use super::index::PresentationIndexParts;
use super::policy;
use crate::GroupId;
use crate::GroupKind;
use crate::GroupMember;
use crate::GroupOrigin;
use crate::PresentationBuildStatus;
use crate::PresentationDiagnosticCode;
use crate::PresentationGroup;
use crate::PresentationIndex;
use crate::PresentationScope;
use crate::SessionTrace;

pub(crate) fn build(trace: &SessionTrace, limits: PresentationLimits) -> PresentationIndex {
    let context = BuildContext::new(trace, limits);
    let limits = context.limits();
    let dispositions = trace
        .nodes
        .iter()
        .map(policy::disposition)
        .collect::<Vec<_>>();
    let mut diagnostics = DiagnosticSink::new(trace.nodes.len(), limits);
    let primary_ids = super::grouping::correlation::assign_primary_groups(
        &context,
        &dispositions,
        &mut diagnostics,
    );
    let mut order = super::order::build_order(&context, &primary_ids);
    diagnostics.extend(order.diagnostics.drain(..));
    let mut groups = materialize_groups(
        &context,
        &primary_ids,
        &order.band_by_node,
        &order.scope_events,
        &mut diagnostics,
    );
    super::grouping::hierarchy::add_hierarchy(&context, &mut groups, &mut diagnostics);
    let mut primary_groups = vec![None; trace.nodes.len()];
    groups.sort_by(|left, right| left.id.cmp(&right.id));
    let group_positions = groups
        .iter()
        .enumerate()
        .map(|(position, group)| (group.id.clone(), position))
        .collect::<HashMap<_, _>>();
    for (position, group_id) in primary_ids.iter().enumerate() {
        if let Some(group_id) = group_id {
            primary_groups[position] = group_positions.get(group_id).copied();
        }
    }
    let mut scope_groups = BTreeMap::<PresentationScope, Vec<usize>>::new();
    let child_groups = groups
        .iter()
        .flat_map(|group| group.child_groups.iter().cloned())
        .collect::<std::collections::HashSet<_>>();
    for (position, group) in groups.iter().enumerate() {
        if child_groups.contains(&group.id) {
            continue;
        }
        scope_groups
            .entry(group.scope.clone())
            .or_default()
            .push(position);
    }
    for positions in scope_groups.values_mut() {
        positions.sort_by(|left, right| {
            groups[*left]
                .anchor_band
                .cmp(&groups[*right].anchor_band)
                .then(groups[*left].kind.cmp(&groups[*right].kind))
                .then(groups[*left].id.cmp(&groups[*right].id))
        });
    }
    let mut scope_bands = BTreeMap::<PresentationScope, Vec<usize>>::new();
    for (position, band) in order.bands.iter().enumerate() {
        scope_bands
            .entry(band.scope.clone())
            .or_default()
            .push(position);
    }

    let mut status = PresentationBuildStatus::Complete;
    super::validate::validate(
        &context,
        &groups,
        &primary_groups,
        &dispositions,
        &order.bands,
        &order.scope_events,
        &mut diagnostics,
        &mut status,
    );
    PresentationIndex::from_parts(PresentationIndexParts {
        groups,
        primary_groups,
        dispositions,
        bands: order.bands,
        scope_bands,
        scope_groups,
        scope_events: order.scope_events,
        diagnostics: diagnostics.finish(),
        status,
    })
}

fn materialize_groups(
    context: &BuildContext<'_>,
    primary_ids: &[Option<GroupId>],
    band_by_node: &[Option<crate::OrderBandId>],
    scope_events: &BTreeMap<PresentationScope, Vec<crate::PresentationEvent>>,
    diagnostics: &mut DiagnosticSink,
) -> Vec<PresentationGroup> {
    let trace = context.trace();
    let limits = context.limits();
    let mut members = BTreeMap::<GroupId, Vec<usize>>::new();
    for (position, group_id) in primary_ids.iter().enumerate() {
        if let Some(group_id) = group_id {
            members.entry(group_id.clone()).or_default().push(position);
        }
    }
    let event_rank = scope_events
        .values()
        .flatten()
        .enumerate()
        .map(|(rank, event)| (event.node_position, rank))
        .collect::<HashMap<_, _>>();
    let global_reference_limit = trace
        .nodes
        .len()
        .saturating_mul(limits.max_references_per_node);
    let mut retained_references = 0;
    let group_count = members.len();
    let summary_limit = limits
        .max_summary_bytes_per_group
        .saturating_mul(group_count)
        .min(limits.max_total_summary_bytes);
    let mut retained_summary_bytes = 0;
    let mut groups = Vec::with_capacity(group_count);
    for (id, mut positions) in members {
        positions.sort_by_key(|position| event_rank.get(position).copied().unwrap_or(usize::MAX));
        let scope = positions
            .first()
            .map(|position| context.scope_at(*position))
            .unwrap_or(PresentationScope::Session);
        let kind = match &id {
            GroupId::Correlated { kind, .. } | GroupId::Batch { kind, .. } => *kind,
            GroupId::Singleton(_) => positions
                .first()
                .map(|position| policy::group_kind(&trace.nodes[*position]))
                .unwrap_or(GroupKind::Unknown),
        };
        let anchor_band = positions
            .iter()
            .filter_map(|position| band_by_node[*position])
            .min()
            .unwrap_or(crate::OrderBandId(0));
        let group_members = positions
            .iter()
            .map(|position| GroupMember {
                node_position: *position,
                role: policy::member_role(&trace.nodes[*position]),
            })
            .collect::<Vec<_>>();
        let mut references = super::references::collect(
            context,
            &positions,
            limits.max_references_per_group,
            global_reference_limit.saturating_sub(retained_references),
        );
        retained_references = retained_references.saturating_add(references.values.len());
        if references.truncated {
            diagnostics.push(
                PresentationDiagnosticCode::ResourceLimit,
                Some(id.clone()),
                positions.first().copied(),
                "secondary presentation references reached their snapshot limit",
            );
        }
        if references.missing {
            diagnostics.push(
                PresentationDiagnosticCode::MissingReference,
                Some(id.clone()),
                positions.first().copied(),
                "typed source reference has no retained canonical target",
            );
        }
        let summary_remaining = summary_limit.saturating_sub(retained_summary_bytes);
        let summary = super::summary::summarize(
            context,
            kind,
            &positions,
            /*child_count*/ 0,
            references.values.len(),
            summary_remaining.min(limits.max_summary_bytes_per_group),
            limits.max_preview_chars,
        );
        retained_summary_bytes = retained_summary_bytes
            .saturating_add(summary.label_bytes)
            .saturating_add(summary.preview_bytes);
        if summary.truncated {
            diagnostics.push(
                PresentationDiagnosticCode::ResourceLimit,
                Some(id.clone()),
                positions.first().copied(),
                "group summary reached its bounded text budget",
            );
        }
        let completeness =
            super::summary::completeness(context, kind, &positions, references.missing);
        groups.push(PresentationGroup {
            id,
            kind,
            scope,
            anchor_band,
            members: group_members,
            child_groups: Vec::new(),
            references: std::mem::take(&mut references.values),
            metadata: summary.metadata,
            completeness,
            origin: GroupOrigin::Persisted,
            default_visibility: policy::default_visibility(kind),
        });
    }
    groups
}
