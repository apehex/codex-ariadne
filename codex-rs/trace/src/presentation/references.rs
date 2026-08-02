//! Bounded secondary references from presentation groups to canonical evidence.

use std::collections::HashSet;

use super::BuildContext;
use crate::GroupReference;
use crate::TraceObjectRef;
use crate::TraceRelation;

pub(crate) struct ReferenceResult {
    pub(crate) values: Vec<GroupReference>,
    pub(crate) missing: bool,
    pub(crate) truncated: bool,
}

pub(crate) fn collect(
    context: &BuildContext<'_>,
    members: &[usize],
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
        for correlation in context
            .facts_at(*position)
            .correlations
            .iter()
            .filter(|correlation| is_secondary(correlation.relation, &correlation.target))
        {
            let resolved = context.identity_positions().get(&correlation.target);
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
                retain(
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
                    retain(
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

fn retain(
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
