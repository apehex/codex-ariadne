//! Deterministic child-group attachment policies.

use std::collections::HashMap;
use std::collections::HashSet;

use super::exploration;
use crate::GroupId;
use crate::GroupKind;
use crate::PresentationDiagnosticCode;
use crate::PresentationGroup;
use crate::PresentationScope;
use crate::TraceObjectRef;
use crate::TraceRelation;
use crate::presentation::BuildContext;
use crate::presentation::DiagnosticSink;

/// Attaches deterministic child relationships and optional batch containers.
pub(crate) fn add_hierarchy(
    context: &BuildContext<'_>,
    groups: &mut Vec<PresentationGroup>,
    diagnostics: &mut DiagnosticSink,
) {
    if context.limits().max_group_depth < 2 {
        return;
    }
    let positions = group_positions(groups);
    let mut parented = attach_code_cell_children(context, groups, &positions, diagnostics);
    let batches = exploration::build_batches(context, groups, &parented);
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
    context: &BuildContext<'_>,
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
                .flat_map(|member| context.facts_at(member.node_position).correlations.iter())
                .filter_map(
                    |correlation| match (correlation.relation, &correlation.target) {
                        (TraceRelation::RequestingCodeCell, TraceObjectRef::CodeCell(id)) => {
                            Some(id.clone())
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

/// Indexes snapshot-local group identities for child attachment.
fn group_positions(groups: &[PresentationGroup]) -> HashMap<GroupId, usize> {
    groups
        .iter()
        .enumerate()
        .map(|(position, group)| (group.id.clone(), position))
        .collect()
}
