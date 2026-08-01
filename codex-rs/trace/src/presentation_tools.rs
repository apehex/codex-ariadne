//! Conservative correlation of direct model and runtime tool observations.

use std::collections::BTreeSet;
use std::collections::HashMap;

use crate::GroupId;
use crate::GroupKind;
use crate::PresentationDiagnosticCode;
use crate::PresentationDisposition;
use crate::PresentationScope;
use crate::SessionTrace;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceObjectRef;
use crate::TraceOrderDomain;
use crate::TraceRecordClass;
use crate::TraceRelation;
use crate::presentation_build::DiagnosticSink;
use crate::presentation_policy;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallOccurrence {
    scope: PresentationScope,
    call_id: String,
    occurrence: u32,
}

pub(crate) fn assign_primary_groups(
    trace: &SessionTrace,
    dispositions: &[PresentationDisposition],
    diagnostics: &mut DiagnosticSink,
) -> Vec<Option<GroupId>> {
    let mut ids = vec![None; trace.nodes.len()];
    let primary = dispositions
        .iter()
        .enumerate()
        .filter_map(|(position, disposition)| {
            (*disposition == PresentationDisposition::Primary).then_some(position)
        })
        .collect::<Vec<_>>();
    let invocation_occurrences = invocation_occurrences(trace, &primary);
    let unique_call_occurrences = unique_call_occurrences(trace, &invocation_occurrences);
    let item_occurrences = invocation_item_occurrences(trace, &invocation_occurrences);
    let (item_tools, occurrence_tools, ambiguous_tools, delegation_calls) = tool_bridges(
        trace,
        &primary,
        &item_occurrences,
        &unique_call_occurrences,
        diagnostics,
    );
    let operation_tools = operation_tools(trace, &primary);

    for position in primary {
        let node = &trace.nodes[position];
        let facts = facts_at(trace, position);
        let scope = scope(&facts);
        let singleton = || GroupId::Singleton(node.locator.clone());
        if !presentation_policy::direct_tool_candidate(node) {
            ids[position] = Some(singleton());
            continue;
        }
        let exact_tool = exact_tool_id(&facts)
            .or_else(|| {
                source_identity(&facts).and_then(|identity| item_tools.get(&identity).cloned())
            })
            .or_else(|| {
                created_operation_id(&facts)
                    .and_then(|operation| operation_tools.get(&operation).cloned())
            });
        if let Some(tool_id) = exact_tool {
            if ambiguous_tools.contains(&tool_id) {
                diagnostics.push(
                    PresentationDiagnosticCode::CrossThreadCorrelation,
                    None,
                    Some(position),
                    "runtime tool identity was observed in more than one thread",
                );
                ids[position] = Some(singleton());
            } else {
                ids[position] = correlated_tool_id(
                    &scope,
                    TraceObjectRef::ToolCall(tool_id),
                    /*occurrence*/ 0,
                )
                .or_else(|| Some(singleton()));
            }
            continue;
        }
        let Some(call_id) = model_call_id(&facts) else {
            ids[position] = Some(singleton());
            continue;
        };
        if delegation_calls.contains(&(scope.clone(), call_id.clone())) {
            ids[position] = Some(singleton());
            continue;
        }
        let occurrence = invocation_occurrences.get(&position).copied().or_else(|| {
            unique_call_occurrences
                .get(&(scope.clone(), call_id.clone()))
                .copied()
                .flatten()
        });
        let Some(occurrence) = occurrence else {
            diagnostics.push(
                PresentationDiagnosticCode::AmbiguousCorrelation,
                None,
                Some(position),
                "reused model-visible call identity could not be assigned without guessing",
            );
            ids[position] = Some(singleton());
            continue;
        };
        let call = CallOccurrence {
            scope: scope.clone(),
            call_id: call_id.clone(),
            occurrence,
        };
        let correlation = occurrence_tools
            .get(&call)
            .cloned()
            .map(TraceObjectRef::ToolCall)
            .unwrap_or(TraceObjectRef::ModelVisibleCall(call_id));
        ids[position] =
            correlated_tool_id(&scope, correlation, occurrence).or_else(|| Some(singleton()));
    }
    ids
}

fn invocation_occurrences(trace: &SessionTrace, primary: &[usize]) -> HashMap<usize, u32> {
    let mut calls = HashMap::<(PresentationScope, TraceOrderDomain, String), Vec<usize>>::new();
    for position in primary {
        let node = &trace.nodes[*position];
        let facts = facts_at(trace, *position);
        if node.presentation.class == TraceRecordClass::ToolInput
            && let Some(call_id) = model_call_id(&facts)
        {
            calls
                .entry((scope(&facts), facts.order.start.domain(), call_id))
                .or_default()
                .push(*position);
        }
    }
    let mut occurrences = HashMap::new();
    for positions in calls.values_mut() {
        positions.sort_by(|left, right| {
            let left_facts = facts_at(trace, *left);
            let right_facts = facts_at(trace, *right);
            left_facts
                .source_cmp(&right_facts)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(left_facts.order.tie_break.cmp(&right_facts.order.tie_break))
                .then(left.cmp(right))
        });
        let mut last_order = None;
        let mut occurrence = 0_u32;
        for position in positions {
            let order = facts_at(trace, *position).order.start;
            if last_order.is_some_and(|last| last != order) {
                occurrence = occurrence.saturating_add(1);
            }
            occurrences.insert(*position, occurrence);
            last_order = Some(order);
        }
    }
    occurrences
}

fn invocation_item_occurrences(
    trace: &SessionTrace,
    occurrences: &HashMap<usize, u32>,
) -> HashMap<TraceObjectRef, CallOccurrence> {
    let mut candidates = HashMap::<TraceObjectRef, BTreeSet<CallOccurrence>>::new();
    for position in 0..trace.nodes.len() {
        let Some(occurrence) = occurrences.get(&position) else {
            continue;
        };
        let facts = facts_at(trace, position);
        let Some(identity) = source_identity(&facts) else {
            continue;
        };
        let Some(call_id) = model_call_id(&facts) else {
            continue;
        };
        candidates
            .entry(identity)
            .or_default()
            .insert(CallOccurrence {
                scope: scope(&facts),
                call_id,
                occurrence: *occurrence,
            });
    }
    candidates
        .into_iter()
        .filter_map(|(identity, occurrences)| {
            if occurrences.len() != 1 {
                return None;
            }
            Some((identity, occurrences.into_iter().next()?))
        })
        .collect()
}

fn unique_call_occurrences(
    trace: &SessionTrace,
    occurrences: &HashMap<usize, u32>,
) -> HashMap<(PresentationScope, String), Option<u32>> {
    let mut unique = HashMap::new();
    for (position, occurrence) in occurrences {
        let facts = facts_at(trace, *position);
        let Some(call_id) = model_call_id(&facts) else {
            continue;
        };
        unique
            .entry((scope(&facts), call_id))
            .and_modify(|retained| {
                if *retained != Some(*occurrence) {
                    *retained = None;
                }
            })
            .or_insert(Some(*occurrence));
    }
    unique
}

#[allow(clippy::type_complexity)]
fn tool_bridges(
    trace: &SessionTrace,
    primary: &[usize],
    item_occurrences: &HashMap<TraceObjectRef, CallOccurrence>,
    unique_call_occurrences: &HashMap<(PresentationScope, String), Option<u32>>,
    diagnostics: &mut DiagnosticSink,
) -> (
    HashMap<TraceObjectRef, String>,
    HashMap<CallOccurrence, String>,
    BTreeSet<String>,
    BTreeSet<(PresentationScope, String)>,
) {
    let mut item_tools = HashMap::new();
    let mut ambiguous_items = BTreeSet::new();
    let mut occurrence_tools = HashMap::new();
    let mut ambiguous_occurrences = BTreeSet::new();
    let mut tool_scopes = HashMap::<String, BTreeSet<PresentationScope>>::new();
    let mut delegation_calls = BTreeSet::new();
    for position in primary {
        let node = &trace.nodes[*position];
        if node.locator.kind != TraceNodeKind::ToolCall {
            continue;
        }
        let facts = facts_at(trace, *position);
        let Some(tool_id) = source_tool_id(&facts) else {
            continue;
        };
        let scope = scope(&facts);
        tool_scopes
            .entry(tool_id.clone())
            .or_default()
            .insert(scope.clone());
        if node.presentation.class == TraceRecordClass::Delegation {
            if let Some(call_id) = model_call_id(&facts) {
                delegation_calls.insert((scope, call_id));
            }
            continue;
        }
        let mut occurrence = None;
        for item in exact_tool_items(&facts) {
            if !ambiguous_items.contains(&item)
                && let Some(previous) = item_tools.insert(item.clone(), tool_id.clone())
                && previous != tool_id
            {
                item_tools.remove(&item);
                ambiguous_items.insert(item.clone());
                diagnostics.push(
                    PresentationDiagnosticCode::AmbiguousCorrelation,
                    None,
                    Some(*position),
                    "one model-visible item referenced multiple runtime tools",
                );
            }
            if let Some(item_occurrence) = item_occurrences.get(&item) {
                occurrence = Some(item_occurrence.clone());
            }
        }
        if occurrence.is_none()
            && let Some(call_id) = model_call_id(&facts)
            && let Some(Some(unique_occurrence)) =
                unique_call_occurrences.get(&(scope.clone(), call_id.clone()))
        {
            occurrence = Some(CallOccurrence {
                scope: scope.clone(),
                call_id,
                occurrence: *unique_occurrence,
            });
        }
        if let Some(occurrence) = occurrence
            && !ambiguous_occurrences.contains(&occurrence)
            && let Some(previous) = occurrence_tools.insert(occurrence.clone(), tool_id.clone())
            && previous != tool_id
        {
            occurrence_tools.remove(&occurrence);
            ambiguous_occurrences.insert(occurrence);
            diagnostics.push(
                PresentationDiagnosticCode::AmbiguousCorrelation,
                None,
                Some(*position),
                "one call occurrence referenced multiple runtime tool identities",
            );
        }
    }
    let ambiguous_tools = tool_scopes
        .into_iter()
        .filter_map(|(tool, scopes)| (scopes.len() > 1).then_some(tool))
        .collect();
    (
        item_tools,
        occurrence_tools,
        ambiguous_tools,
        delegation_calls,
    )
}

fn operation_tools(trace: &SessionTrace, primary: &[usize]) -> HashMap<String, String> {
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

fn correlated_tool_id(
    scope: &PresentationScope,
    correlation: TraceObjectRef,
    occurrence: u32,
) -> Option<GroupId> {
    let PresentationScope::Thread(thread_id) = scope else {
        return None;
    };
    Some(GroupId::Correlated {
        thread_id: thread_id.clone(),
        kind: GroupKind::DirectTool,
        correlation,
        occurrence,
    })
}

fn exact_tool_items(facts: &TraceNodeFacts) -> impl Iterator<Item = TraceObjectRef> + '_ {
    facts
        .correlations
        .iter()
        .filter(|&correlation| {
            matches!(
                correlation.relation,
                TraceRelation::ModelVisibleCallItem | TraceRelation::ModelVisibleOutputItem
            )
        })
        .map(|correlation| correlation.target.clone())
}

fn exact_tool_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (
                TraceRelation::SourceIdentity | TraceRelation::Producer | TraceRelation::OwningTool,
                TraceObjectRef::ToolCall(id),
            ) => Some(id.clone()),
            _ => None,
        }
    })
}

fn source_tool_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (TraceRelation::SourceIdentity, TraceObjectRef::ToolCall(id)) => Some(id.clone()),
            _ => None,
        }
    })
}

fn model_call_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (TraceRelation::ModelVisibleCall, TraceObjectRef::ModelVisibleCall(id)) => {
                Some(id.clone())
            }
            _ => None,
        }
    })
}

fn source_identity(facts: &TraceNodeFacts) -> Option<TraceObjectRef> {
    facts.correlations.iter().find_map(|correlation| {
        (correlation.relation == TraceRelation::SourceIdentity).then(|| correlation.target.clone())
    })
}

fn created_operation_id(facts: &TraceNodeFacts) -> Option<String> {
    facts.correlations.iter().find_map(|correlation| {
        match (correlation.relation, &correlation.target) {
            (TraceRelation::CreatedByTerminalOperation, TraceObjectRef::TerminalOperation(id)) => {
                Some(id.clone())
            }
            _ => None,
        }
    })
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
