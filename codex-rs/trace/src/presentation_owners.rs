//! Durable owner and reverse-member lookups for presentation grouping.

use std::collections::HashMap;

use crate::GroupKind;
use crate::SessionTrace;
use crate::TraceActivity;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceObjectRef;
use crate::TraceRelation;

/// Indexes retained runtime tool identities by presentation family.
pub(crate) fn tool_kinds(trace: &SessionTrace, primary: &[usize]) -> HashMap<String, GroupKind> {
    primary
        .iter()
        .filter_map(|position| {
            let facts = facts_at(trace, *position);
            let TraceObjectRef::ToolCall(tool_id) = source_identity(&facts)? else {
                return None;
            };
            let kind = match facts.policy.activity {
                Some(TraceActivity::Agent(_)) => GroupKind::Delegation,
                Some(
                    TraceActivity::Tool { .. }
                    | TraceActivity::CodeCell
                    | TraceActivity::Compaction(_),
                )
                | None => GroupKind::DirectTool,
            };
            Some((tool_id, kind))
        })
        .collect()
}

/// Maps code source and output items back to their owning code cell.
pub(crate) fn code_items(
    trace: &SessionTrace,
    primary: &[usize],
) -> HashMap<TraceObjectRef, String> {
    linked_items(
        trace,
        primary,
        TraceNodeKind::CodeCell,
        &[TraceRelation::CodeSource, TraceRelation::CodeOutput],
    )
}

/// Maps compaction markers back to their installed checkpoint.
pub(crate) fn compaction_items(
    trace: &SessionTrace,
    primary: &[usize],
) -> HashMap<TraceObjectRef, String> {
    linked_items(
        trace,
        primary,
        TraceNodeKind::Compaction,
        &[TraceRelation::CompactionMarker],
    )
}

/// Maps terminal operations to their owning runtime tool.
pub(crate) fn operation_tools(trace: &SessionTrace, primary: &[usize]) -> HashMap<String, String> {
    primary
        .iter()
        .filter_map(|position| {
            let facts = facts_at(trace, *position);
            let TraceObjectRef::TerminalOperation(operation) = source_identity(&facts)? else {
                return None;
            };
            Some((operation, exact_tool_id(&facts)?))
        })
        .collect()
}

/// Returns a durable runtime-tool identity directly supported by one node.
pub(crate) fn exact_tool_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (
                TraceRelation::SourceIdentity
                | TraceRelation::Producer
                | TraceRelation::OwningTool
                | TraceRelation::InteractionSource
                | TraceRelation::InteractionTarget,
                TraceObjectRef::ToolCall(id),
            ) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Returns a durable code-cell owner directly supported by one node.
pub(crate) fn code_cell_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (
                TraceRelation::SourceIdentity | TraceRelation::Producer,
                TraceObjectRef::CodeCell(id),
            ) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Returns a durable compaction owner directly supported by one node.
pub(crate) fn compaction_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (
                TraceRelation::SourceIdentity
                | TraceRelation::Producer
                | TraceRelation::OwningCompaction,
                TraceObjectRef::Compaction(id),
            ) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Builds a conflict-rejecting reverse map for selected owner relations.
fn linked_items(
    trace: &SessionTrace,
    primary: &[usize],
    owner_kind: TraceNodeKind,
    relations: &[TraceRelation],
) -> HashMap<TraceObjectRef, String> {
    let mut values = HashMap::<TraceObjectRef, Option<String>>::new();
    for position in primary {
        let node = &trace.nodes[*position];
        if node.locator.kind != owner_kind {
            continue;
        }
        let facts = facts_at(trace, *position);
        let Some(owner) = source_identity(&facts).and_then(object_id) else {
            continue;
        };
        for target in facts
            .correlations
            .iter()
            .filter(|correlation| relations.contains(&correlation.relation))
            .map(|correlation| correlation.target.clone())
        {
            values
                .entry(target)
                .and_modify(|retained| {
                    if retained.as_ref() != Some(&owner) {
                        *retained = None;
                    }
                })
                .or_insert_with(|| Some(owner.clone()));
        }
    }
    values
        .into_iter()
        .filter_map(|(target, owner)| Some((target, owner?)))
        .collect()
}

/// Extracts identifiers belonging to supported reverse-map owners.
fn object_id(object: TraceObjectRef) -> Option<String> {
    match object {
        TraceObjectRef::CodeCell(id) | TraceObjectRef::Compaction(id) => Some(id),
        TraceObjectRef::Thread(_)
        | TraceObjectRef::Turn(_)
        | TraceObjectRef::ConversationItem(_)
        | TraceObjectRef::Inference(_)
        | TraceObjectRef::ToolCall(_)
        | TraceObjectRef::ModelVisibleCall(_)
        | TraceObjectRef::McpCall(_)
        | TraceObjectRef::CodeModeRuntimeTool(_)
        | TraceObjectRef::Terminal(_)
        | TraceObjectRef::TerminalOperation(_)
        | TraceObjectRef::CompactionRequest(_)
        | TraceObjectRef::InteractionEdge(_)
        | TraceObjectRef::RawPayload(_)
        | TraceObjectRef::UserInput
        | TraceObjectRef::Harness => None,
    }
}

/// Returns the first retained canonical source identity.
fn source_identity(facts: &TraceNodeFacts) -> Option<TraceObjectRef> {
    facts.correlations.iter().find_map(|correlation| {
        (correlation.relation == TraceRelation::SourceIdentity).then(|| correlation.target.clone())
    })
}

/// Returns typed facts for a canonical position with explicit unavailable fallback.
fn facts_at(trace: &SessionTrace, position: usize) -> TraceNodeFacts {
    trace
        .nodes
        .get(position)
        .and_then(|node| trace.facts.get(&node.locator))
        .cloned()
        .unwrap_or_default()
}
