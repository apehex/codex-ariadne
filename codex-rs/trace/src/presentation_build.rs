//! Bounded construction of the minimal static presentation snapshot.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::GroupId;
use crate::GroupKind;
use crate::GroupMember;
use crate::GroupOrigin;
use crate::PresentationBuildStatus;
use crate::PresentationDiagnostic;
use crate::PresentationDiagnosticCode;
use crate::PresentationGroup;
use crate::PresentationIndex;
use crate::PresentationScope;
use crate::SessionTrace;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceRelation;
use crate::presentation_index::PresentationIndexParts;
use crate::presentation_policy;

const MAX_REFERENCES_PER_GROUP: usize = 4_096;
const MAX_SUMMARY_BYTES: usize = 4 * 1_024;
const MAX_TOTAL_SUMMARY_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_PREVIEW_CHARS: usize = 256;

pub(crate) fn build(trace: &SessionTrace) -> PresentationIndex {
    let dispositions = trace
        .nodes
        .iter()
        .map(presentation_policy::disposition)
        .collect::<Vec<_>>();
    let mut diagnostics = DiagnosticSink::new(trace.nodes.len());
    let primary_ids =
        crate::presentation_tools::assign_primary_groups(trace, &dispositions, &mut diagnostics);
    let mut order = crate::presentation_order::build_order(trace, &primary_ids);
    diagnostics.extend(order.diagnostics.drain(..));
    let mut groups = materialize_groups(
        trace,
        &primary_ids,
        &order.band_by_node,
        &order.scope_events,
        &mut diagnostics,
    );
    crate::presentation_hierarchy::add_hierarchy(trace, &mut groups, &mut diagnostics);
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
    crate::presentation_validate::validate(
        trace,
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
    trace: &SessionTrace,
    primary_ids: &[Option<GroupId>],
    band_by_node: &[Option<crate::OrderBandId>],
    scope_events: &BTreeMap<PresentationScope, Vec<crate::PresentationEvent>>,
    diagnostics: &mut DiagnosticSink,
) -> Vec<PresentationGroup> {
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
    let identity_positions = identity_positions(trace);
    let global_reference_limit = trace.nodes.len().saturating_mul(4);
    let mut retained_references = 0;
    let group_count = members.len();
    let summary_limit = MAX_SUMMARY_BYTES
        .saturating_mul(group_count)
        .min(MAX_TOTAL_SUMMARY_BYTES);
    let mut retained_summary_bytes = 0;
    let mut groups = Vec::with_capacity(group_count);
    for (id, mut positions) in members {
        positions.sort_by_key(|position| event_rank.get(position).copied().unwrap_or(usize::MAX));
        let scope = positions
            .first()
            .map(|position| scope(&facts_at(trace, *position)))
            .unwrap_or(PresentationScope::Session);
        let kind = match &id {
            GroupId::Correlated { kind, .. } | GroupId::Batch { kind, .. } => *kind,
            GroupId::Singleton(_) => positions
                .first()
                .map(|position| presentation_policy::group_kind(&trace.nodes[*position]))
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
                role: presentation_policy::member_role(&trace.nodes[*position]),
            })
            .collect::<Vec<_>>();
        let mut references = crate::presentation_summaries::references(
            trace,
            &positions,
            &identity_positions,
            MAX_REFERENCES_PER_GROUP,
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
        let summary = crate::presentation_summaries::summarize(
            trace,
            kind,
            &positions,
            /*child_count*/ 0,
            references.values.len(),
            summary_remaining.min(MAX_SUMMARY_BYTES),
            MAX_PREVIEW_CHARS,
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
        let completeness = crate::presentation_summaries::completeness(
            trace,
            kind,
            &positions,
            references.missing,
        );
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
            default_visibility: presentation_policy::default_visibility(kind),
        });
    }
    groups
}

fn identity_positions(trace: &SessionTrace) -> HashMap<TraceObjectRef, Vec<usize>> {
    let mut positions = HashMap::<TraceObjectRef, Vec<usize>>::new();
    for position in 0..trace.nodes.len() {
        let facts = facts_at(trace, position);
        for identity in facts
            .correlations
            .iter()
            .filter(|correlation| correlation.relation == TraceRelation::SourceIdentity)
            .map(|correlation| correlation.target.clone())
        {
            positions.entry(identity).or_default().push(position);
        }
    }
    positions
}

fn facts_at(trace: &SessionTrace, position: usize) -> TraceNodeFacts {
    trace
        .nodes
        .get(position)
        .and_then(|node| trace.facts.get(&node.locator))
        .cloned()
        .unwrap_or_default()
}

fn scope(facts: &TraceNodeFacts) -> PresentationScope {
    facts
        .ownership
        .thread_id
        .as_ref()
        .map_or(PresentationScope::Session, |thread_id| {
            PresentationScope::Thread(thread_id.clone())
        })
}

pub(crate) struct DiagnosticSink {
    limit: usize,
    diagnostics: Vec<PresentationDiagnostic>,
    saturated: bool,
}

impl DiagnosticSink {
    fn new(node_count: usize) -> Self {
        Self {
            limit: node_count.saturating_add(1).min(1_024),
            diagnostics: Vec::new(),
            saturated: false,
        }
    }

    pub(crate) fn push(
        &mut self,
        code: PresentationDiagnosticCode,
        group_id: Option<GroupId>,
        node_position: Option<usize>,
        message: &str,
    ) {
        if self.diagnostics.len() < self.limit {
            self.diagnostics.push(PresentationDiagnostic {
                code,
                group_id,
                node_position,
                message: message.chars().take(MAX_SUMMARY_BYTES).collect(),
            });
        } else {
            self.saturated = true;
        }
    }

    fn extend(&mut self, diagnostics: impl IntoIterator<Item = PresentationDiagnostic>) {
        for diagnostic in diagnostics {
            self.push(
                diagnostic.code,
                diagnostic.group_id,
                diagnostic.node_position,
                &diagnostic.message,
            );
        }
    }

    fn finish(mut self) -> Vec<PresentationDiagnostic> {
        if self.saturated && !self.diagnostics.is_empty() {
            let last = self.diagnostics.len() - 1;
            self.diagnostics[last] = PresentationDiagnostic {
                code: PresentationDiagnosticCode::ResourceLimit,
                group_id: None,
                node_position: None,
                message: "additional presentation diagnostics were bounded away".to_string(),
            };
        }
        self.diagnostics
    }
}
