use std::collections::BTreeMap;

use anyhow::Result;
use codex_rollout_trace::AgentOrigin;
use serde::Serialize;

use crate::BundlePayload;
use crate::EvidenceGrade;
use crate::TraceDiagnostic;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;

// Projection keeps source identity, evidence policy, and bounded output stores
// explicit at this one source-boundary call.
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_rich(
    session_id: &str,
    bundle_path: &std::path::Path,
    trace: &codex_rollout_trace::RolloutTrace,
    semantically_complete: bool,
    max_nodes: usize,
    session_locator: &TraceNodeLocator,
    nodes: &mut BTreeMap<TraceNodeLocator, TraceNode>,
    payloads: &mut BTreeMap<String, BundlePayload>,
    diagnostics: &mut Vec<TraceDiagnostic>,
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
                diagnostics.push(TraceDiagnostic {
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
            nodes,
            locator(session_id, TraceNodeKind::Thread, id),
            Some(parent),
            Some(thread.execution.started_at_unix_ms.to_string()),
            format!("thread {}", thread.agent_path),
            thread,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, turn) in &trace.codex_turns {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::Turn, id),
            Some(locator(session_id, TraceNodeKind::Thread, &turn.thread_id)),
            Some(turn.execution.started_at_unix_ms.to_string()),
            format!("turn {id}"),
            turn,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, item) in &trace.conversation_items {
        let parent = item.codex_turn_id.as_ref().map_or_else(
            || locator(session_id, TraceNodeKind::Thread, &item.thread_id),
            |turn| locator(session_id, TraceNodeKind::Turn, turn),
        );
        insert(
            nodes,
            locator(session_id, TraceNodeKind::ConversationItem, id),
            Some(parent),
            Some(item.first_seen_at_unix_ms.to_string()),
            format!("conversation {:?}", item.kind),
            item,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, inference) in &trace.inference_calls {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::Inference, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &inference.codex_turn_id,
            )),
            Some(inference.execution.started_at_unix_ms.to_string()),
            format!("inference {}", inference.model),
            inference,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, tool) in &trace.tool_calls {
        let parent = tool.started_by_codex_turn_id.as_ref().map_or_else(
            || locator(session_id, TraceNodeKind::Thread, &tool.thread_id),
            |turn| locator(session_id, TraceNodeKind::Turn, turn),
        );
        insert(
            nodes,
            locator(session_id, TraceNodeKind::ToolCall, id),
            Some(parent),
            Some(tool.execution.started_at_unix_ms.to_string()),
            format!("tool {:?}", tool.kind),
            tool,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, cell) in &trace.code_cells {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::CodeCell, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &cell.codex_turn_id,
            )),
            Some(cell.execution.started_at_unix_ms.to_string()),
            format!("code cell {id}"),
            cell,
            semantic_evidence,
            max_nodes,
        )?;
    }
    project_runtime_maps(
        session_id,
        trace,
        session_locator,
        nodes,
        semantic_evidence,
        max_nodes,
    )?;
    for (id, reference) in &trace.raw_payloads {
        let inserted = insert(
            nodes,
            locator(session_id, TraceNodeKind::RawPayload, id),
            Some(session_locator.clone()),
            None,
            format!("payload {:?}", reference.kind),
            reference,
            EvidenceGrade::Exact,
            max_nodes,
        )?;
        if inserted {
            payloads.insert(
                id.clone(),
                BundlePayload {
                    bundle_root: bundle_path.to_path_buf(),
                    reference: reference.clone(),
                },
            );
        }
    }
    if nodes.len() >= max_nodes {
        diagnostics.push(TraceDiagnostic {
            locator: None,
            path: Some(bundle_path.to_path_buf()),
            evidence: EvidenceGrade::Unavailable,
            message: format!("session node materialization stopped at {max_nodes} nodes"),
        });
    }
    Ok(())
}

fn project_runtime_maps(
    session_id: &str,
    trace: &codex_rollout_trace::RolloutTrace,
    session_locator: &TraceNodeLocator,
    nodes: &mut BTreeMap<TraceNodeLocator, TraceNode>,
    semantic_evidence: EvidenceGrade,
    max_nodes: usize,
) -> Result<()> {
    for (id, compaction) in &trace.compactions {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::Compaction, id),
            Some(locator(
                session_id,
                TraceNodeKind::Turn,
                &compaction.codex_turn_id,
            )),
            Some(compaction.installed_at_unix_ms.to_string()),
            format!("compaction {id}"),
            compaction,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, request) in &trace.compaction_requests {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::CompactionRequest, id),
            Some(locator(
                session_id,
                TraceNodeKind::Compaction,
                &request.compaction_id,
            )),
            Some(request.execution.started_at_unix_ms.to_string()),
            format!("compaction request {}", request.model),
            request,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, terminal) in &trace.terminal_sessions {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::TerminalSession, id),
            Some(locator(
                session_id,
                TraceNodeKind::TerminalOperation,
                &terminal.created_by_operation_id,
            )),
            Some(terminal.execution.started_at_unix_ms.to_string()),
            format!("terminal {id}"),
            terminal,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, operation) in &trace.terminal_operations {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::TerminalOperation, id),
            Some(locator(
                session_id,
                TraceNodeKind::ToolCall,
                &operation.tool_call_id,
            )),
            Some(operation.execution.started_at_unix_ms.to_string()),
            format!("terminal operation {:?}", operation.kind),
            operation,
            semantic_evidence,
            max_nodes,
        )?;
    }
    for (id, edge) in &trace.interaction_edges {
        insert(
            nodes,
            locator(session_id, TraceNodeKind::InteractionEdge, id),
            Some(session_locator.clone()),
            Some(edge.started_at_unix_ms.to_string()),
            format!("interaction {:?}", edge.kind),
            edge,
            semantic_evidence,
            max_nodes,
        )?;
    }
    Ok(())
}

// Keeping every rendered field explicit avoids a second lossy intermediate
// node representation at the projection boundary.
#[allow(clippy::too_many_arguments)]
fn insert<T: Serialize>(
    nodes: &mut BTreeMap<TraceNodeLocator, TraceNode>,
    locator: TraceNodeLocator,
    parent: Option<TraceNodeLocator>,
    timestamp: Option<String>,
    label: String,
    value: &T,
    evidence: EvidenceGrade,
    max_nodes: usize,
) -> Result<bool> {
    if nodes.len() >= max_nodes && !nodes.contains_key(&locator) {
        return Ok(false);
    }
    let detail = serde_json::to_value(value)?;
    let presentation = crate::TraceRecordPresentation::from_detail(locator.kind, &detail);
    nodes.insert(
        locator.clone(),
        TraceNode {
            locator,
            parent,
            provenance: crate::TraceSourceKind::Rich,
            evidence,
            timestamp,
            label,
            presentation,
            detail,
        },
    );
    Ok(true)
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
