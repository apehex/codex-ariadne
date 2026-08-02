//! Higher-order batching of consecutive top-level exploration groups.

use std::collections::BTreeMap;
use std::collections::HashSet;

use crate::GroupActivity;
use crate::GroupAggregate;
use crate::GroupCompleteness;
use crate::GroupId;
use crate::GroupKind;
use crate::GroupMetadata;
use crate::GroupOrigin;
use crate::GroupVisibility;
use crate::PRESENTATION_POLICY_VERSION;
use crate::PresentationGroup;
use crate::PresentationScope;
use crate::TraceActivity;
use crate::TraceExplorationEligibility;
use crate::TraceToolActivity;
use crate::TraceToolRequester;
use crate::presentation::BuildContext;

pub(crate) fn build_batches(
    context: &BuildContext<'_>,
    groups: &[PresentationGroup],
    parented: &HashSet<GroupId>,
) -> Vec<PresentationGroup> {
    let mut scopes = BTreeMap::<PresentationScope, Vec<&PresentationGroup>>::new();
    for group in groups {
        if !parented.contains(&group.id) {
            scopes.entry(group.scope.clone()).or_default().push(group);
        }
    }
    let mut reducer = ExplorationReducer::default();
    for (scope, mut scoped) in scopes {
        scoped.sort_by(|left, right| {
            left.anchor_band
                .cmp(&right.anchor_band)
                .then(left.kind.cmp(&right.kind))
                .then(left.id.cmp(&right.id))
        });
        reducer.start_scope(scope);
        reducer.extend(
            scoped
                .into_iter()
                .map(|group| (group, is_exploration_group(context, group))),
        );
        reducer.finish_scope();
    }
    reducer.finish()
}

#[derive(Default)]
struct ExplorationReducer<'a> {
    completed: Vec<PresentationGroup>,
    pending: Vec<&'a PresentationGroup>,
    scope: Option<PresentationScope>,
}

impl<'a> ExplorationReducer<'a> {
    fn start_scope(&mut self, scope: PresentationScope) {
        self.flush();
        self.scope = Some(scope);
    }

    fn extend(&mut self, groups: impl IntoIterator<Item = (&'a PresentationGroup, bool)>) {
        for (group, eligible) in groups {
            if eligible {
                self.pending.push(group);
            } else {
                self.flush();
            }
        }
    }

    fn finish_scope(&mut self) {
        self.flush();
        self.scope = None;
    }

    fn flush(&mut self) {
        if self.pending.len() >= 2
            && let Some(scope) = self.scope.clone()
        {
            self.completed.push(batch_group(scope, &self.pending));
        }
        self.pending.clear();
    }

    fn finish(mut self) -> Vec<PresentationGroup> {
        self.flush();
        self.completed
    }
}

fn is_exploration_group(context: &BuildContext<'_>, group: &PresentationGroup) -> bool {
    if group.kind != GroupKind::DirectTool {
        return false;
    }
    let activities = group
        .members
        .iter()
        .filter_map(|member| context.facts_at(member.node_position).policy.activity)
        .collect::<Vec<_>>();
    if activities.iter().any(|activity| {
        matches!(
            activity,
            TraceActivity::Tool {
                requester: TraceToolRequester::CodeCell,
                ..
            }
        )
    }) {
        return false;
    }
    activities.iter().any(|activity| {
        matches!(
            activity,
            TraceActivity::Tool {
                kind: TraceToolActivity::ExecCommand(TraceExplorationEligibility::Eligible),
                requester: TraceToolRequester::Model | TraceToolRequester::Unknown,
            }
        )
    }) && !activities.iter().any(|activity| {
        matches!(
            activity,
            TraceActivity::Tool {
                kind: TraceToolActivity::ExecCommand(TraceExplorationEligibility::Ineligible),
                ..
            }
        )
    })
}

fn batch_group(scope: PresentationScope, children: &[&PresentationGroup]) -> PresentationGroup {
    let first = children[0];
    let thread_id = match &scope {
        PresentationScope::Thread(thread_id) => thread_id.clone(),
        PresentationScope::Session => String::new(),
    };
    PresentationGroup {
        id: GroupId::Batch {
            thread_id,
            kind: GroupKind::ExplorationBatch,
            policy_version: PRESENTATION_POLICY_VERSION,
            first_child: Box::new(first.id.clone()),
        },
        kind: GroupKind::ExplorationBatch,
        scope,
        anchor_band: first.anchor_band,
        members: Vec::new(),
        child_groups: children.iter().map(|group| group.id.clone()).collect(),
        references: Vec::new(),
        metadata: batch_metadata(children),
        completeness: batch_completeness(children),
        origin: batch_origin(children),
        default_visibility: GroupVisibility::Shown,
    }
}

fn batch_metadata(children: &[&PresentationGroup]) -> GroupMetadata {
    let mut evidence = crate::GroupEvidenceCounts::default();
    for child in children {
        evidence.exact = evidence.exact.saturating_add(child.metadata.evidence.exact);
        evidence.semantic = evidence
            .semantic
            .saturating_add(child.metadata.evidence.semantic);
        evidence.reconstructed = evidence
            .reconstructed
            .saturating_add(child.metadata.evidence.reconstructed);
        evidence.unavailable = evidence
            .unavailable
            .saturating_add(child.metadata.evidence.unavailable);
        evidence.conflicting = evidence
            .conflicting
            .saturating_add(child.metadata.evidence.conflicting);
    }
    GroupMetadata {
        activity: GroupAggregate::Value(GroupActivity::Exploration),
        label: None,
        preview: None,
        status: aggregate_children(children, |child| &child.metadata.status),
        duration_ms: aggregate_children(children, |child| &child.metadata.duration_ms),
        member_count: 0,
        child_count: children.len(),
        reference_count: 0,
        evidence,
    }
}

fn aggregate_children<T: Copy + Eq>(
    children: &[&PresentationGroup],
    value: impl Fn(&PresentationGroup) -> &GroupAggregate<T>,
) -> GroupAggregate<T> {
    let mut retained = None;
    for child in children {
        match value(child) {
            GroupAggregate::Unavailable => {}
            GroupAggregate::Conflicting => return GroupAggregate::Conflicting,
            GroupAggregate::Value(current) => match retained {
                Some(previous) if previous != *current => return GroupAggregate::Conflicting,
                Some(_) => {}
                None => retained = Some(*current),
            },
        }
    }
    retained.map_or(GroupAggregate::Unavailable, GroupAggregate::Value)
}

fn batch_completeness(children: &[&PresentationGroup]) -> GroupCompleteness {
    if children
        .iter()
        .any(|group| group.completeness == GroupCompleteness::Partial)
    {
        GroupCompleteness::Partial
    } else if children
        .iter()
        .any(|group| group.completeness == GroupCompleteness::Running)
    {
        GroupCompleteness::Running
    } else if children
        .iter()
        .any(|group| group.completeness == GroupCompleteness::Indeterminate)
    {
        GroupCompleteness::Indeterminate
    } else {
        GroupCompleteness::Complete
    }
}

fn batch_origin(children: &[&PresentationGroup]) -> GroupOrigin {
    let first = children[0].origin;
    if children.iter().all(|group| group.origin == first) {
        first
    } else {
        GroupOrigin::Mixed
    }
}

#[cfg(test)]
#[path = "exploration_tests.rs"]
mod tests;
