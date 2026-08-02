//! Deterministic child-group and higher-order exploration policies.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::GroupActivity;
use crate::GroupAggregate;
use crate::GroupCompleteness;
use crate::GroupId;
use crate::GroupKind;
use crate::GroupMetadata;
use crate::GroupOrigin;
use crate::GroupVisibility;
use crate::PresentationDiagnosticCode;
use crate::PresentationGroup;
use crate::PresentationScope;
use crate::SessionTrace;
use crate::TraceActivity;
use crate::TraceExplorationEligibility;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceRelation;
use crate::TraceToolActivity;
use crate::TraceToolRequester;
use crate::presentation_build::DiagnosticSink;
use crate::presentation_model::PRESENTATION_POLICY_VERSION;

/// Attaches deterministic child relationships and optional batch containers.
pub(crate) fn add_hierarchy(
    trace: &SessionTrace,
    groups: &mut Vec<PresentationGroup>,
    diagnostics: &mut DiagnosticSink,
) {
    let positions = group_positions(groups);
    let mut parented = attach_code_cell_children(trace, groups, &positions, diagnostics);
    let batches = exploration_batches(trace, groups, &parented);
    for batch in batches {
        parented.extend(batch.child_groups.iter().cloned());
        groups.push(batch);
    }
    for group in groups {
        group.metadata.child_count = group.child_groups.len();
    }
}

/// Gives each nested runtime tool at most one containing code-cell group.
fn attach_code_cell_children(
    trace: &SessionTrace,
    groups: &mut [PresentationGroup],
    positions: &HashMap<GroupId, usize>,
    diagnostics: &mut DiagnosticSink,
) -> HashSet<GroupId> {
    let mut parented = HashSet::new();
    let children = groups
        .iter()
        .filter(|group| group.kind == GroupKind::DirectTool)
        .filter_map(|group| {
            let owners = group
                .members
                .iter()
                .filter_map(|member| facts_at(trace, member.node_position))
                .flat_map(|facts| facts.correlations.into_iter())
                .filter_map(
                    |correlation| match (correlation.relation, correlation.target) {
                        (TraceRelation::RequestingCodeCell, TraceObjectRef::CodeCell(id)) => {
                            Some(id)
                        }
                        _ => None,
                    },
                )
                .collect::<HashSet<_>>();
            match owners.len() {
                0 => None,
                1 => Some((group.id.clone(), owners.into_iter().next()?)),
                _ => {
                    diagnostics.push(
                        PresentationDiagnosticCode::AmbiguousCorrelation,
                        Some(group.id.clone()),
                        group.members.first().map(|member| member.node_position),
                        "nested tool referenced more than one requesting code cell",
                    );
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    for (child, code_cell_id) in children {
        let parent = GroupId::Correlated {
            thread_id: match positions
                .get(&child)
                .and_then(|position| groups.get(*position))
                .map(|group| &group.scope)
            {
                Some(PresentationScope::Thread(thread_id)) => thread_id.clone(),
                Some(PresentationScope::Session) | None => continue,
            },
            kind: GroupKind::Code,
            correlation: TraceObjectRef::CodeCell(code_cell_id),
            occurrence: 0,
        };
        let Some(parent_position) = positions.get(&parent).copied() else {
            diagnostics.push(
                PresentationDiagnosticCode::MissingReference,
                Some(child),
                /*node_position*/ None,
                "nested tool has no retained requesting code-cell group",
            );
            continue;
        };
        groups[parent_position].child_groups.push(child.clone());
        parented.insert(child);
    }
    parented
}

/// Reduces top-level groups into thread-local exploration batches.
fn exploration_batches(
    trace: &SessionTrace,
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
                .map(|group| (group, is_exploration_group(trace, group))),
        );
        reducer.finish_scope();
    }
    reducer.finish()
}

/// Chunk-tolerant reducer for one ordered stream of top-level groups.
#[derive(Default)]
struct ExplorationReducer<'a> {
    completed: Vec<PresentationGroup>,
    pending: Vec<&'a PresentationGroup>,
    scope: Option<PresentationScope>,
}

impl<'a> ExplorationReducer<'a> {
    /// Opens one thread or session scope after flushing prior state.
    fn start_scope(&mut self, scope: PresentationScope) {
        self.flush();
        self.scope = Some(scope);
    }

    /// Adds one arbitrary chunk without treating its boundary as a flush.
    fn extend(&mut self, groups: impl IntoIterator<Item = (&'a PresentationGroup, bool)>) {
        for (group, eligible) in groups {
            if eligible {
                self.pending.push(group);
            } else {
                self.flush();
            }
        }
    }

    /// Closes the current scope and materializes any valid pending batch.
    fn finish_scope(&mut self) {
        self.flush();
        self.scope = None;
    }

    /// Flushes an eligible run, omitting single-child containers.
    fn flush(&mut self) {
        if self.pending.len() >= 2
            && let Some(scope) = self.scope.clone()
        {
            self.completed.push(batch_group(scope, &self.pending));
        }
        self.pending.clear();
    }

    /// Returns every completed container after flushing pending state.
    fn finish(mut self) -> Vec<PresentationGroup> {
        self.flush();
        self.completed
    }
}

/// Returns whether a top-level lifecycle satisfies policy version 1.
fn is_exploration_group(trace: &SessionTrace, group: &PresentationGroup) -> bool {
    if group.kind != GroupKind::DirectTool {
        return false;
    }
    let activities = group
        .members
        .iter()
        .filter_map(|member| facts_at(trace, member.node_position))
        .filter_map(|facts| facts.policy.activity)
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

/// Materializes a higher-order group without copying child members.
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

/// Aggregates bounded typed metadata from batch children.
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

/// Conservatively combines one aggregate field across child groups.
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

/// Reduces child completeness with partial and running precedence.
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

/// Reduces child origin without hiding mixed evidence.
fn batch_origin(children: &[&PresentationGroup]) -> GroupOrigin {
    let first = children[0].origin;
    if children.iter().all(|group| group.origin == first) {
        first
    } else {
        GroupOrigin::Mixed
    }
}

/// Indexes snapshot-local group identities for child attachment.
fn group_positions(groups: &[PresentationGroup]) -> HashMap<GroupId, usize> {
    groups
        .iter()
        .enumerate()
        .map(|(position, group)| (group.id.clone(), position))
        .collect()
}

/// Returns retained typed facts for one canonical member position.
fn facts_at(trace: &SessionTrace, position: usize) -> Option<TraceNodeFacts> {
    trace
        .nodes
        .get(position)
        .and_then(|node| trace.facts.get(&node.locator))
        .cloned()
}

#[cfg(test)]
#[path = "presentation_hierarchy_tests.rs"]
mod tests;
