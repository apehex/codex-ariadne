use std::collections::BTreeMap;

use anyhow::Result;
use codex_rollout_trace::AgentOrigin;
use serde::Serialize;

use crate::Admission;
use crate::EvidenceGrade;
use crate::SiblingOrder;
use crate::TraceGraphBuilder;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::model::BundlePayload;

// Projection keeps source identity, evidence policy, and bounded output stores
// explicit at this one source-boundary call.
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_rich(
    session_id: &str,
    bundle_path: &std::path::Path,
    trace: &codex_rollout_trace::RolloutTrace,
    semantically_complete: bool,
    session_locator: &TraceNodeLocator,
    graph: &mut TraceGraphBuilder,
    payloads: &mut BTreeMap<String, BundlePayload>,
) -> Result<()> {
    let semantic_evidence = if semantically_complete {
        EvidenceGrade::Semantic
    } else {
        EvidenceGrade::Reconstructed
    };
    for (id, thread) in &trace.threads {
        let parent = match &thread.origin {
            AgentOrigin::Root => session_locator.clone(),
            AgentOrigin::Spawned {
                parent_thread_id, ..
            } if rich_parent_is_valid(id, parent_thread_id, &trace.threads) => {
                locator(session_id, TraceNodeKind::Thread, parent_thread_id)
            }
            AgentOrigin::Spawned {
                parent_thread_id, ..
            } => {
                graph.record_diagnostic(crate::TraceDiagnostic {
                    locator: Some(locator(session_id, TraceNodeKind::Thread, id)),
                    path: Some(bundle_path.to_path_buf()),
                    evidence: EvidenceGrade::Unavailable,
                    message: format!(
                        "rich thread {id} has missing or cyclic parent {parent_thread_id}; attached to session"
                    ),
                });
                session_locator.clone()
            }
        };
        insert(
            graph,
            locator(session_id, TraceNodeKind::Thread, id),
            Some(parent),
            Some(thread.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Timestamp(thread.execution.started_at_unix_ms),
            format!("thread {}", thread.agent_path),
            thread,
            semantic_evidence,
            ParentAdmission::Deferred,
        )?;
    }
    for (id, turn) in &trace.codex_turns {
        insert(
            graph,
            locator(session_id, TraceNodeKind::Turn, id),
            Some(locator(session_id, TraceNodeKind::Thread, &turn.thread_id)),
            Some(turn.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(turn.execution.started_seq),
            format!("turn {id}"),
            turn,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, item) in &trace.conversation_items {
        let parent = item.codex_turn_id.as_ref().map_or_else(
            || locator(session_id, TraceNodeKind::Thread, &item.thread_id),
            |turn| locator(session_id, TraceNodeKind::Turn, turn),
        );
        insert(
            graph,
            locator(session_id, TraceNodeKind::ConversationItem, id),
            Some(parent),
            Some(item.first_seen_at_unix_ms.to_string()),
            SiblingOrder::Sequence(item.first_seen_seq),
            format!("conversation {:?}", item.kind),
            item,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, inference) in &trace.inference_calls {
        insert(
            graph,
            locator(session_id, TraceNodeKind::Inference, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &inference.codex_turn_id,
            )),
            Some(inference.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(inference.execution.started_seq),
            format!("inference {}", inference.model),
            inference,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, tool) in &trace.tool_calls {
        let parent = tool.started_by_codex_turn_id.as_ref().map_or_else(
            || locator(session_id, TraceNodeKind::Thread, &tool.thread_id),
            |turn| locator(session_id, TraceNodeKind::Turn, turn),
        );
        insert(
            graph,
            locator(session_id, TraceNodeKind::ToolCall, id),
            Some(parent),
            Some(tool.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(tool.execution.started_seq),
            format!("tool {:?}", tool.kind),
            tool,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, cell) in &trace.code_cells {
        insert(
            graph,
            locator(session_id, TraceNodeKind::CodeCell, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &cell.codex_turn_id,
            )),
            Some(cell.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(cell.execution.started_seq),
            format!("code cell {id}"),
            cell,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    project_runtime_maps(session_id, trace, session_locator, graph, semantic_evidence)?;
    for (id, reference) in &trace.raw_payloads {
        let inserted = insert(
            graph,
            locator(session_id, TraceNodeKind::RawPayload, id),
            Some(session_locator.clone()),
            None,
            SiblingOrder::Unspecified,
            format!("payload {:?}", reference.kind),
            reference,
            EvidenceGrade::Exact,
            ParentAdmission::Required,
        )?;
        if let Admission::Retained(locator) = inserted {
            payloads.insert(
                locator.id,
                BundlePayload {
                    bundle_root: bundle_path.to_path_buf(),
                    reference: reference.clone(),
                },
            );
        }
    }
    Ok(())
}

fn project_runtime_maps(
    session_id: &str,
    trace: &codex_rollout_trace::RolloutTrace,
    session_locator: &TraceNodeLocator,
    graph: &mut TraceGraphBuilder,
    semantic_evidence: EvidenceGrade,
) -> Result<()> {
    for (id, compaction) in &trace.compactions {
        insert(
            graph,
            locator(session_id, TraceNodeKind::Compaction, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &compaction.codex_turn_id,
            )),
            Some(compaction.installed_at_unix_ms.to_string()),
            SiblingOrder::Sequence(compaction.installed_seq),
            format!("compaction {id}"),
            compaction,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, request) in &trace.compaction_requests {
        insert(
            graph,
            locator(session_id, TraceNodeKind::CompactionRequest, id),
            Some(locator(
                session_id,
                TraceNodeKind::Compaction,
                &request.compaction_id,
            )),
            Some(request.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(request.execution.started_seq),
            format!("compaction request {}", request.model),
            request,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, operation) in &trace.terminal_operations {
        insert(
            graph,
            locator(session_id, TraceNodeKind::TerminalOperation, id),
            Some(locator(
                session_id,
                TraceNodeKind::ToolCall,
                &operation.tool_call_id,
            )),
            Some(operation.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(operation.execution.started_seq),
            format!("terminal operation {:?}", operation.kind),
            operation,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, terminal) in &trace.terminal_sessions {
        insert(
            graph,
            locator(session_id, TraceNodeKind::TerminalSession, id),
            Some(locator(
                session_id,
                TraceNodeKind::TerminalOperation,
                &terminal.created_by_operation_id,
            )),
            Some(terminal.execution.started_at_unix_ms.to_string()),
            SiblingOrder::Sequence(terminal.execution.started_seq),
            format!("terminal {id}"),
            terminal,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    for (id, edge) in &trace.interaction_edges {
        insert(
            graph,
            locator(session_id, TraceNodeKind::InteractionEdge, id),
            Some(session_locator.clone()),
            Some(edge.started_at_unix_ms.to_string()),
            SiblingOrder::Timestamp(edge.started_at_unix_ms),
            format!("interaction {:?}", edge.kind),
            edge,
            semantic_evidence,
            ParentAdmission::Required,
        )?;
    }
    Ok(())
}

// Keeping every rendered field explicit avoids a second lossy intermediate
// node representation at the projection boundary.
#[allow(clippy::too_many_arguments)]
fn insert<T: Serialize>(
    graph: &mut TraceGraphBuilder,
    locator: TraceNodeLocator,
    parent: Option<TraceNodeLocator>,
    timestamp: Option<String>,
    sibling_order: SiblingOrder,
    label: String,
    value: &T,
    evidence: EvidenceGrade,
    parent_admission: ParentAdmission,
) -> Result<Admission> {
    let detail = serde_json::to_value(value)?;
    let presentation = crate::TraceRecordPresentation::from_detail(locator.kind, &detail);
    let node = TraceNode {
        locator,
        parent,
        provenance: crate::TraceSourceKind::Rich,
        evidence,
        timestamp,
        label,
        presentation,
        detail,
    };
    Ok(match parent_admission {
        ParentAdmission::Deferred => graph.admit(node, sibling_order, /*source_path*/ None),
        ParentAdmission::Required => {
            graph.admit_with_required_parent(node, sibling_order, /*source_path*/ None)
        }
    })
}

/// Whether the source guarantees that a parent was projected earlier.
#[derive(Debug, Clone, Copy)]
enum ParentAdmission {
    Deferred,
    Required,
}

fn locator(session_id: &str, kind: TraceNodeKind, id: &str) -> TraceNodeLocator {
    TraceNodeLocator::new(session_id, kind, id)
}

fn rich_parent_is_valid(
    thread_id: &str,
    parent_thread_id: &str,
    threads: &BTreeMap<String, codex_rollout_trace::AgentThread>,
) -> bool {
    if !threads.contains_key(parent_thread_id) {
        return false;
    }
    let mut current = parent_thread_id;
    let mut path = vec![thread_id];
    loop {
        if let Some(cycle_start) = path.iter().position(|id| *id == current) {
            let cycle = &path[cycle_start..];
            let cut = cycle.iter().copied().min();
            return !cycle.contains(&thread_id) || cut != Some(thread_id);
        }
        path.push(current);
        let Some(parent) = threads.get(current) else {
            return true;
        };
        match &parent.origin {
            AgentOrigin::Root => return true,
            AgentOrigin::Spawned {
                parent_thread_id, ..
            } => current = parent_thread_id,
        }
    }
}

#[cfg(test)]
#[path = "rich_tests.rs"]
mod tests;
