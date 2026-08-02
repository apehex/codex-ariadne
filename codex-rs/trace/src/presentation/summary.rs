//! Bounded group references, metadata, and completeness reduction.

use super::BuildContext;
use super::bounded_text::truncate_chars;
use super::bounded_text::truncate_utf8;
use crate::EvidenceGrade;
use crate::GroupActivity;
use crate::GroupAggregate;
use crate::GroupCompleteness;
use crate::GroupEvidenceCounts;
use crate::GroupKind;
use crate::GroupMetadata;
use crate::TraceActivity;
use crate::TraceAgentActivity;
use crate::TraceCompactionActivity;
use crate::TraceFactAvailability;
use crate::TraceRecordClass;
use crate::TraceStatus;
use crate::TraceToolActivity;

pub(crate) struct SummaryResult {
    pub(crate) metadata: GroupMetadata,
    pub(crate) label_bytes: usize,
    pub(crate) preview_bytes: usize,
    pub(crate) truncated: bool,
}

pub(crate) fn summarize(
    context: &BuildContext<'_>,
    kind: GroupKind,
    members: &[usize],
    child_count: usize,
    reference_count: usize,
    text_budget: usize,
    preview_chars: usize,
) -> SummaryResult {
    let trace = context.trace();
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
    let mut activities = Vec::new();
    for position in members {
        let Some(node) = trace.nodes.get(*position) else {
            continue;
        };
        evidence.record(node.evidence);
        if let Some(status) = node.presentation.status {
            statuses.push(status);
        }
        let facts = context.facts_at(*position);
        if let Some(activity) = facts.policy.activity.and_then(group_activity) {
            activities.push(activity);
        }
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
            activity: if kind == GroupKind::ExplorationBatch {
                GroupAggregate::Value(GroupActivity::Exploration)
            } else {
                aggregate_activities(activities)
            },
            label,
            preview,
            status: aggregate(statuses),
            duration_ms: aggregate(durations),
            member_count: members.len(),
            child_count,
            reference_count,
            evidence,
        },
        truncated,
    }
}

/// Maps source policy activity into renderer-neutral group metadata.
fn group_activity(activity: TraceActivity) -> Option<GroupActivity> {
    match activity {
        TraceActivity::Tool { kind, .. } => Some(match kind {
            TraceToolActivity::ExecCommand(_) => GroupActivity::ExecCommand,
            TraceToolActivity::WriteStdin => GroupActivity::WriteStdin,
            TraceToolActivity::PollTerminal => GroupActivity::PollTerminal,
            TraceToolActivity::ApplyPatch => GroupActivity::ApplyPatch,
            TraceToolActivity::Mcp => GroupActivity::Mcp,
            TraceToolActivity::Web => GroupActivity::Web,
            TraceToolActivity::ImageGeneration => GroupActivity::ImageGeneration,
            TraceToolActivity::Other => GroupActivity::OtherTool,
        }),
        TraceActivity::Agent(activity) => Some(match activity {
            TraceAgentActivity::Spawn => GroupActivity::AgentSpawn,
            TraceAgentActivity::Assign => GroupActivity::AgentAssign,
            TraceAgentActivity::Send => GroupActivity::AgentSend,
            TraceAgentActivity::Wait => GroupActivity::AgentWait,
            TraceAgentActivity::Result => GroupActivity::AgentResult,
            TraceAgentActivity::Resume => GroupActivity::AgentResume,
            TraceAgentActivity::Close => GroupActivity::AgentClose,
        }),
        TraceActivity::CodeCell => Some(GroupActivity::CodeCell),
        TraceActivity::Compaction(
            TraceCompactionActivity::Checkpoint
            | TraceCompactionActivity::Request
            | TraceCompactionActivity::Marker,
        ) => Some(GroupActivity::Compaction),
    }
}

/// Aggregates activities while allowing terminal poll refinement.
fn aggregate_activities(values: Vec<GroupActivity>) -> GroupAggregate<GroupActivity> {
    if values.contains(&GroupActivity::PollTerminal)
        && values.iter().all(|activity| {
            matches!(
                activity,
                GroupActivity::PollTerminal | GroupActivity::WriteStdin
            )
        })
    {
        GroupAggregate::Value(GroupActivity::PollTerminal)
    } else {
        aggregate(values)
    }
}

pub(crate) fn completeness(
    context: &BuildContext<'_>,
    kind: GroupKind,
    members: &[usize],
    missing_reference: bool,
) -> GroupCompleteness {
    let trace = context.trace();
    let conflicting = members.iter().any(|position| {
        trace.nodes.get(*position).is_some_and(|node| {
            node.evidence == EvidenceGrade::Conflicting
                || context.facts_at(*position).availability == TraceFactAvailability::Conflicting
        })
    });
    if conflicting || missing_reference {
        return GroupCompleteness::Partial;
    }
    if members
        .iter()
        .any(|position| context.facts_at(*position).availability == TraceFactAvailability::Partial)
    {
        return GroupCompleteness::Partial;
    }
    if !matches!(
        kind,
        GroupKind::DirectTool | GroupKind::Code | GroupKind::Delegation
    ) {
        return if members.iter().any(|position| {
            context.facts_at(*position).availability == TraceFactAvailability::Unavailable
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
                || context.facts_at(*position).order.end.is_some()
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
