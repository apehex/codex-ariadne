//! Construction of per-scope canonical partial-order bands.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::GroupId;
use crate::OrderBandId;
use crate::OrderBandKind;
use crate::OrderSegment;
use crate::PresentationDiagnostic;
use crate::PresentationDiagnosticCode;
use crate::PresentationEvent;
use crate::PresentationOrderBand;
use crate::PresentationScope;
use crate::SessionTrace;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceOrderDomain;
use crate::TraceRelation;

pub(crate) struct OrderOutput {
    pub(crate) bands: Vec<PresentationOrderBand>,
    pub(crate) scope_events: BTreeMap<PresentationScope, Vec<PresentationEvent>>,
    pub(crate) band_by_node: Vec<Option<OrderBandId>>,
    pub(crate) diagnostics: Vec<PresentationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AlignmentKey {
    Source(TraceObjectRef, usize),
    Group(GroupId, usize),
}

pub(crate) fn build_order(
    trace: &SessionTrace,
    primary_group_ids: &[Option<GroupId>],
) -> OrderOutput {
    let mut scoped = BTreeMap::<PresentationScope, BTreeMap<TraceOrderDomain, Vec<usize>>>::new();
    for (position, group_id) in primary_group_ids.iter().enumerate() {
        if group_id.is_none() {
            continue;
        }
        let facts = facts_at(trace, position);
        scoped
            .entry(scope(&facts))
            .or_default()
            .entry(facts.order.start.domain())
            .or_default()
            .push(position);
    }
    for domains in scoped.values_mut() {
        for positions in domains.values_mut() {
            positions.sort_by(|left, right| {
                let left_facts = facts_at(trace, *left);
                let right_facts = facts_at(trace, *right);
                left_facts
                    .source_cmp(&right_facts)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(left_facts.order.tie_break.cmp(&right_facts.order.tie_break))
                    .then(left.cmp(right))
            });
        }
    }

    let mut output = OrderOutput {
        bands: Vec::new(),
        scope_events: BTreeMap::new(),
        band_by_node: vec![None; trace.nodes.len()],
        diagnostics: Vec::new(),
    };
    for (scope, mut domains) in scoped {
        let ordinary = domains
            .remove(&TraceOrderDomain::Ordinary)
            .unwrap_or_default();
        let rich = domains.remove(&TraceOrderDomain::Rich).unwrap_or_default();
        add_causal_bands(
            trace,
            primary_group_ids,
            &scope,
            &ordinary,
            &rich,
            &mut output,
        );
        let mut unpositioned = domains.into_values().flatten().collect::<Vec<_>>();
        unpositioned.sort_by_key(|position| facts_at(trace, *position).order.tie_break);
        if !unpositioned.is_empty() {
            add_band(
                &scope,
                OrderBandKind::Unpositioned,
                vec![OrderSegment {
                    domain: TraceOrderDomain::Unspecified,
                    node_positions: unpositioned,
                }],
                primary_group_ids,
                &mut output,
            );
        }
    }
    output
}

fn add_causal_bands(
    trace: &SessionTrace,
    primary_group_ids: &[Option<GroupId>],
    scope: &PresentationScope,
    ordinary: &[usize],
    rich: &[usize],
    output: &mut OrderOutput,
) {
    if ordinary.is_empty() || rich.is_empty() {
        let (domain, positions) = if ordinary.is_empty() {
            (TraceOrderDomain::Rich, rich)
        } else {
            (TraceOrderDomain::Ordinary, ordinary)
        };
        for position in positions {
            add_band(
                scope,
                OrderBandKind::Ordered,
                vec![OrderSegment {
                    domain,
                    node_positions: vec![*position],
                }],
                primary_group_ids,
                output,
            );
        }
        return;
    }

    let ordinary_keys = alignment_keys(trace, ordinary, primary_group_ids);
    let rich_keys = alignment_keys(trace, rich, primary_group_ids);
    let rich_positions = rich_keys
        .iter()
        .enumerate()
        .filter_map(|(index, key)| key.clone().map(|key| (key, index)))
        .collect::<HashMap<_, _>>();
    let mut anchors = Vec::new();
    let mut last_rich = None;
    for (ordinary_index, key) in ordinary_keys.iter().enumerate() {
        let Some(key) = key else {
            continue;
        };
        let Some(rich_index) = rich_positions.get(key).copied() else {
            continue;
        };
        if last_rich.is_some_and(|last| rich_index <= last) {
            output.diagnostics.push(PresentationDiagnostic {
                code: PresentationDiagnosticCode::OrderAlignment,
                group_id: primary_group_ids[ordinary[ordinary_index]].clone(),
                node_position: Some(ordinary[ordinary_index]),
                message: "crossing source alignment was retained as unordered evidence".to_string(),
            });
            continue;
        }
        anchors.push((ordinary_index, rich_index));
        last_rich = Some(rich_index);
    }

    if anchors.is_empty() {
        add_unmatched(scope, ordinary, rich, primary_group_ids, output);
        return;
    }
    let mut ordinary_cursor = 0;
    let mut rich_cursor = 0;
    for (ordinary_anchor, rich_anchor) in anchors {
        add_unmatched(
            scope,
            &ordinary[ordinary_cursor..ordinary_anchor],
            &rich[rich_cursor..rich_anchor],
            primary_group_ids,
            output,
        );
        add_band(
            scope,
            OrderBandKind::Aligned,
            vec![
                OrderSegment {
                    domain: TraceOrderDomain::Ordinary,
                    node_positions: vec![ordinary[ordinary_anchor]],
                },
                OrderSegment {
                    domain: TraceOrderDomain::Rich,
                    node_positions: vec![rich[rich_anchor]],
                },
            ],
            primary_group_ids,
            output,
        );
        ordinary_cursor = ordinary_anchor + 1;
        rich_cursor = rich_anchor + 1;
    }
    add_unmatched(
        scope,
        &ordinary[ordinary_cursor..],
        &rich[rich_cursor..],
        primary_group_ids,
        output,
    );
}

fn add_unmatched(
    scope: &PresentationScope,
    ordinary: &[usize],
    rich: &[usize],
    primary_group_ids: &[Option<GroupId>],
    output: &mut OrderOutput,
) {
    match (ordinary.is_empty(), rich.is_empty()) {
        (true, true) => {}
        (false, true) => {
            for position in ordinary {
                add_band(
                    scope,
                    OrderBandKind::Ordered,
                    vec![OrderSegment {
                        domain: TraceOrderDomain::Ordinary,
                        node_positions: vec![*position],
                    }],
                    primary_group_ids,
                    output,
                );
            }
        }
        (true, false) => {
            for position in rich {
                add_band(
                    scope,
                    OrderBandKind::Ordered,
                    vec![OrderSegment {
                        domain: TraceOrderDomain::Rich,
                        node_positions: vec![*position],
                    }],
                    primary_group_ids,
                    output,
                );
            }
        }
        (false, false) => add_band(
            scope,
            OrderBandKind::Unordered,
            vec![
                OrderSegment {
                    domain: TraceOrderDomain::Ordinary,
                    node_positions: ordinary.to_vec(),
                },
                OrderSegment {
                    domain: TraceOrderDomain::Rich,
                    node_positions: rich.to_vec(),
                },
            ],
            primary_group_ids,
            output,
        ),
    }
}

fn add_band(
    scope: &PresentationScope,
    kind: OrderBandKind,
    segments: Vec<OrderSegment>,
    primary_group_ids: &[Option<GroupId>],
    output: &mut OrderOutput,
) {
    let id = OrderBandId(output.bands.len());
    for position in segments
        .iter()
        .flat_map(|segment| segment.node_positions.iter().copied())
    {
        output.band_by_node[position] = Some(id);
        if let Some(group_id) = &primary_group_ids[position] {
            output
                .scope_events
                .entry(scope.clone())
                .or_default()
                .push(PresentationEvent {
                    node_position: position,
                    primary_group: group_id.clone(),
                    band: id,
                });
        }
    }
    output.bands.push(PresentationOrderBand {
        id,
        scope: scope.clone(),
        kind,
        segments,
    });
}

fn alignment_keys(
    trace: &SessionTrace,
    positions: &[usize],
    primary_group_ids: &[Option<GroupId>],
) -> Vec<Option<AlignmentKey>> {
    let mut occurrences = HashMap::<TraceObjectRef, usize>::new();
    let mut group_occurrences = HashMap::<GroupId, usize>::new();
    positions
        .iter()
        .map(|position| {
            let facts = facts_at(trace, *position);
            if let Some(identity) = facts.correlations.iter().find_map(|correlation| {
                (correlation.relation == TraceRelation::SourceIdentity)
                    .then(|| correlation.target.clone())
            }) {
                let occurrence = occurrences.entry(identity.clone()).or_default();
                let key = AlignmentKey::Source(identity, *occurrence);
                *occurrence += 1;
                Some(key)
            } else {
                primary_group_ids[*position]
                    .clone()
                    .filter(|id| matches!(id, GroupId::Correlated { .. }))
                    .map(|id| {
                        let occurrence = group_occurrences.entry(id.clone()).or_default();
                        let key = AlignmentKey::Group(id, *occurrence);
                        *occurrence += 1;
                        key
                    })
            }
        })
        .collect()
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
