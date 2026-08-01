//! Bounded group references, metadata, and completeness reduction.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::EvidenceGrade;
use crate::GroupAggregate;
use crate::GroupCompleteness;
use crate::GroupEvidenceCounts;
use crate::GroupKind;
use crate::GroupMetadata;
use crate::GroupReference;
use crate::SessionTrace;
use crate::TraceFactAvailability;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceRecordClass;
use crate::TraceRelation;
use crate::TraceStatus;

pub(crate) struct ReferenceResult {
    pub(crate) values: Vec<GroupReference>,
    pub(crate) missing: bool,
    pub(crate) truncated: bool,
}

pub(crate) struct SummaryResult {
    pub(crate) metadata: GroupMetadata,
    pub(crate) label_bytes: usize,
    pub(crate) preview_bytes: usize,
    pub(crate) truncated: bool,
}

pub(crate) fn references(
    trace: &SessionTrace,
    members: &[usize],
    identity_positions: &HashMap<TraceObjectRef, Vec<usize>>,
    group_limit: usize,
    global_remaining: usize,
) -> ReferenceResult {
    let member_set = members.iter().copied().collect::<HashSet<_>>();
    let limit = group_limit.min(global_remaining);
    let mut values = Vec::new();
    let mut missing = false;
    let mut truncated = false;
    let mut observed = HashSet::new();
    for position in members {
        let facts = facts_at(trace, *position);
        for correlation in facts
            .correlations
            .iter()
            .filter(|correlation| is_secondary(correlation.relation, &correlation.target))
        {
            let resolved = identity_positions.get(&correlation.target);
            let external = resolved
                .into_iter()
                .flatten()
                .copied()
                .filter(|position| !member_set.contains(position))
                .collect::<Vec<_>>();
            if external.is_empty() {
                if resolved.is_none() && expects_canonical_target(&correlation.target) {
                    missing = true;
                }
                retain_reference(
                    GroupReference {
                        relation: correlation.relation,
                        target: correlation.target.clone(),
                        node_position: None,
                    },
                    limit,
                    &mut values,
                    &mut observed,
                    &mut truncated,
                );
            } else {
                for target_position in external {
                    retain_reference(
                        GroupReference {
                            relation: correlation.relation,
                            target: correlation.target.clone(),
                            node_position: Some(target_position),
                        },
                        limit,
                        &mut values,
                        &mut observed,
                        &mut truncated,
                    );
                }
            }
        }
    }
    ReferenceResult {
        values,
        missing,
        truncated,
    }
}

pub(crate) fn summarize(
    trace: &SessionTrace,
    _kind: GroupKind,
    members: &[usize],
    reference_count: usize,
    text_budget: usize,
    preview_chars: usize,
) -> SummaryResult {
    let mut remaining = text_budget;
    let mut truncated = false;
    let label_source = members
        .first()
        .and_then(|position| trace.nodes.get(*position))
        .map(|node| node.label.as_str());
    let label = label_source.map(|label| {
        let (label, was_truncated) = truncate_utf8(label, remaining);
        remaining = remaining.saturating_sub(label.len());
        truncated |= was_truncated;
        label
    });
    let preview_source = members.iter().find_map(|position| {
        trace
            .nodes
            .get(*position)
            .and_then(|node| node.presentation.preview.as_deref())
    });
    let preview = preview_source.map(|preview| {
        let (preview, char_truncated) = truncate_chars(preview, preview_chars);
        let (preview, byte_truncated) = truncate_utf8(&preview, remaining);
        truncated |= char_truncated || byte_truncated;
        preview
    });
    let mut evidence = GroupEvidenceCounts::default();
    let mut statuses = Vec::new();
    let mut durations = Vec::new();
    for position in members {
        let Some(node) = trace.nodes.get(*position) else {
            continue;
        };
        evidence.record(node.evidence);
        if let Some(status) = node.presentation.status {
            statuses.push(status);
        }
        let facts = facts_at(trace, *position);
        if let (Some(start), Some(end)) = (
            facts.order.wall_clock_start_ms,
            facts.order.wall_clock_end_ms,
        ) && let Ok(duration) = u64::try_from(end.saturating_sub(start))
        {
            durations.push(duration);
        }
    }
    SummaryResult {
        label_bytes: label.as_ref().map_or(0, String::len),
        preview_bytes: preview.as_ref().map_or(0, String::len),
        metadata: GroupMetadata {
            label,
            preview,
            status: aggregate(statuses),
            duration_ms: aggregate(durations),
            member_count: members.len(),
            reference_count,
            evidence,
        },
        truncated,
    }
}

pub(crate) fn completeness(
    trace: &SessionTrace,
    kind: GroupKind,
    members: &[usize],
    missing_reference: bool,
) -> GroupCompleteness {
    let conflicting = members.iter().any(|position| {
        trace.nodes.get(*position).is_some_and(|node| {
            node.evidence == EvidenceGrade::Conflicting
                || facts_at(trace, *position).availability == TraceFactAvailability::Conflicting
        })
    });
    if conflicting || missing_reference {
        return GroupCompleteness::Partial;
    }
    if members
        .iter()
        .any(|position| facts_at(trace, *position).availability == TraceFactAvailability::Partial)
    {
        return GroupCompleteness::Partial;
    }
    if kind != GroupKind::DirectTool {
        return if members.iter().any(|position| {
            facts_at(trace, *position).availability == TraceFactAvailability::Unavailable
        }) {
            GroupCompleteness::Indeterminate
        } else {
            GroupCompleteness::Complete
        };
    }
    let has_input = members.iter().any(|position| {
        trace.nodes.get(*position).is_some_and(|node| {
            matches!(
                node.presentation.class,
                TraceRecordClass::ToolInput | TraceRecordClass::Code
            )
        })
    });
    let has_result = members.iter().any(|position| {
        trace.nodes.get(*position).is_some_and(|node| {
            node.presentation.class == TraceRecordClass::ToolOutput
                || facts_at(trace, *position).order.end.is_some()
                || matches!(
                    node.presentation.status,
                    Some(TraceStatus::Completed | TraceStatus::Failed | TraceStatus::Aborted)
                )
        })
    });
    match (has_input, has_result) {
        (true, true) => GroupCompleteness::Complete,
        (true, false) | (false, true) => GroupCompleteness::Partial,
        (false, false) => GroupCompleteness::Indeterminate,
    }
}

fn aggregate<T: Copy + Eq>(values: Vec<T>) -> GroupAggregate<T> {
    let Some(first) = values.first().copied() else {
        return GroupAggregate::Unavailable;
    };
    if values.iter().all(|value| *value == first) {
        GroupAggregate::Value(first)
    } else {
        GroupAggregate::Conflicting
    }
}

fn is_secondary(relation: TraceRelation, target: &TraceObjectRef) -> bool {
    match (relation, target) {
        (TraceRelation::SourceIdentity | TraceRelation::ModelVisibleCall, _) => false,
        (TraceRelation::Producer | TraceRelation::OwningTool, TraceObjectRef::ToolCall(_)) => false,
        (
            TraceRelation::ModelVisibleCallItem
            | TraceRelation::ModelVisibleOutputItem
            | TraceRelation::CodeSource
            | TraceRelation::CodeOutput
            | TraceRelation::ToolTerminalOperation
            | TraceRelation::CreatedByTerminalOperation,
            _,
        ) => false,
        (
            TraceRelation::ParentThread
            | TraceRelation::SpawnEdge
            | TraceRelation::TurnInput
            | TraceRelation::Producer
            | TraceRelation::OwningTool
            | TraceRelation::InferenceInput
            | TraceRelation::InferenceOutput
            | TraceRelation::InferenceStartedTool
            | TraceRelation::RequestingCodeCell
            | TraceRelation::NestedTool
            | TraceRelation::WaitTool
            | TraceRelation::TerminalSessionOperation
            | TraceRelation::TerminalSession
            | TraceRelation::TerminalObservationCall
            | TraceRelation::TerminalObservationOutput
            | TraceRelation::CompactionRequest
            | TraceRelation::CompactionMarker
            | TraceRelation::CompactionInput
            | TraceRelation::CompactionReplacement
            | TraceRelation::OwningCompaction
            | TraceRelation::InteractionSource
            | TraceRelation::InteractionTarget
            | TraceRelation::InteractionCarriedItem
            | TraceRelation::RawPayload,
            _,
        ) => true,
    }
}

fn expects_canonical_target(target: &TraceObjectRef) -> bool {
    !matches!(
        target,
        TraceObjectRef::UserInput
            | TraceObjectRef::Harness
            | TraceObjectRef::ModelVisibleCall(_)
            | TraceObjectRef::McpCall(_)
            | TraceObjectRef::CodeModeRuntimeTool(_)
    )
}

fn retain_reference(
    reference: GroupReference,
    limit: usize,
    values: &mut Vec<GroupReference>,
    observed: &mut HashSet<(TraceRelation, TraceObjectRef, Option<usize>)>,
    truncated: &mut bool,
) {
    let key = (
        reference.relation,
        reference.target.clone(),
        reference.node_position,
    );
    if !observed.insert(key) {
        return;
    }
    if values.len() < limit {
        values.push(reference);
    } else {
        *truncated = true;
    }
}

fn truncate_utf8(text: &str, byte_limit: usize) -> (String, bool) {
    if text.len() <= byte_limit {
        return (text.to_string(), false);
    }
    let mut end = byte_limit.min(text.len());
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (text[..end].to_string(), true)
}

fn truncate_chars(text: &str, char_limit: usize) -> (String, bool) {
    if text.chars().count() <= char_limit {
        (text.to_string(), false)
    } else {
        (text.chars().take(char_limit).collect(), true)
    }
}

fn facts_at(trace: &SessionTrace, position: usize) -> TraceNodeFacts {
    trace
        .nodes
        .get(position)
        .and_then(|node| trace.facts.get(&node.locator))
        .cloned()
        .unwrap_or_default()
}
