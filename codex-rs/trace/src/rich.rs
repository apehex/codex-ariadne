//! Projection of reduced rollout-trace bundles into normalized trace nodes.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use codex_rollout_trace::AgentOrigin;
use serde::Serialize;

use crate::Admission;
use crate::EvidenceGrade;
use crate::TraceFactAvailability;
use crate::TraceGraphBuilder;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::payload::BundlePayload;
use crate::rich_facts;
use crate::rich_terminal_facts;

/// Projects one reduced rich trace through a source-specific projection context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_rich(
    session_id: &str,
    bundle_path: &Path,
    trace: &codex_rollout_trace::RolloutTrace,
    semantically_complete: bool,
    session_locator: &TraceNodeLocator,
    graph: &mut TraceGraphBuilder,
    payloads: &mut BTreeMap<String, BundlePayload>,
) -> Result<()> {
    RichProjector::new(
        session_id,
        bundle_path,
        session_locator,
        graph,
        payloads,
        semantically_complete,
    )
    .project(trace)
}

/// Shared rich-source identity, evidence, graph, and payload state.
struct RichProjector<'a> {
    session_id: &'a str,
    bundle_path: &'a Path,
    session_locator: &'a TraceNodeLocator,
    graph: &'a mut TraceGraphBuilder,
    payloads: &'a mut BTreeMap<String, BundlePayload>,
    semantic_evidence: EvidenceGrade,
    fact_availability: TraceFactAvailability,
}

impl<'a> RichProjector<'a> {
    /// Creates one projection context for a selected bundle observation.
    fn new(
        session_id: &'a str,
        bundle_path: &'a Path,
        session_locator: &'a TraceNodeLocator,
        graph: &'a mut TraceGraphBuilder,
        payloads: &'a mut BTreeMap<String, BundlePayload>,
        semantically_complete: bool,
    ) -> Self {
        let semantic_evidence = if semantically_complete {
            EvidenceGrade::Semantic
        } else {
            EvidenceGrade::Reconstructed
        };
        let fact_availability = if semantically_complete {
            TraceFactAvailability::Complete
        } else {
            TraceFactAvailability::Partial
        };
        Self {
            session_id,
            bundle_path,
            session_locator,
            graph,
            payloads,
            semantic_evidence,
            fact_availability,
        }
    }

    /// Projects every supported rich semantic family and lazy raw artifact.
    fn project(&mut self, trace: &codex_rollout_trace::RolloutTrace) -> Result<()> {
        for (id, thread) in &trace.threads {
            let parent = match &thread.origin {
                AgentOrigin::Root => self.session_locator.clone(),
                AgentOrigin::Spawned {
                    parent_thread_id, ..
                } if rich_parent_is_valid(id, parent_thread_id, &trace.threads) => {
                    self.locator(TraceNodeKind::Thread, parent_thread_id)
                }
                AgentOrigin::Spawned {
                    parent_thread_id, ..
                } => {
                    self.graph.record_diagnostic(crate::TraceDiagnostic {
                        locator: Some(self.locator(TraceNodeKind::Thread, id)),
                        path: Some(self.bundle_path.to_path_buf()),
                        evidence: EvidenceGrade::Unavailable,
                        message: format!(
                            "rich thread {id} has missing or cyclic parent {parent_thread_id}; attached to session"
                        ),
                    });
                    self.session_locator.clone()
                }
            };
            self.admit_deferred(RichNodeSpec {
                kind: TraceNodeKind::Thread,
                id,
                parent,
                timestamp: Some(thread.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::thread(id, thread, self.fact_availability),
                label: format!("thread {}", thread.agent_path),
                value: thread,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, turn) in &trace.codex_turns {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::Turn,
                id,
                parent: self.locator(TraceNodeKind::Thread, &turn.thread_id),
                timestamp: Some(turn.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::turn(id, turn, self.fact_availability),
                label: format!("turn {id}"),
                value: turn,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, item) in &trace.conversation_items {
            let parent = item.codex_turn_id.as_ref().map_or_else(
                || self.locator(TraceNodeKind::Thread, &item.thread_id),
                |turn| self.locator(TraceNodeKind::Turn, turn),
            );
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::ConversationItem,
                id,
                parent,
                timestamp: Some(item.first_seen_at_unix_ms.to_string()),
                facts: rich_facts::conversation_item(id, item, self.fact_availability),
                label: format!("conversation {:?}", item.kind),
                value: item,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, inference) in &trace.inference_calls {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::Inference,
                id,
                parent: self.locator(TraceNodeKind::Turn, &inference.codex_turn_id),
                timestamp: Some(inference.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::inference(id, inference, self.fact_availability),
                label: format!("inference {}", inference.model),
                value: inference,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, tool) in &trace.tool_calls {
            let parent = tool.started_by_codex_turn_id.as_ref().map_or_else(
                || self.locator(TraceNodeKind::Thread, &tool.thread_id),
                |turn| self.locator(TraceNodeKind::Turn, turn),
            );
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::ToolCall,
                id,
                parent,
                timestamp: Some(tool.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::tool(id, tool, self.fact_availability),
                label: format!("tool {:?}", tool.kind),
                value: tool,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, cell) in &trace.code_cells {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::CodeCell,
                id,
                parent: self.locator(TraceNodeKind::Turn, &cell.codex_turn_id),
                timestamp: Some(cell.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::code_cell(id, cell, self.fact_availability),
                label: format!("code cell {id}"),
                value: cell,
                evidence: self.semantic_evidence,
            })?;
        }
        self.project_runtime_maps(trace)?;
        for (id, reference) in &trace.raw_payloads {
            let admission = self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::RawPayload,
                id,
                parent: self.session_locator.clone(),
                timestamp: None,
                facts: rich_facts::raw_payload(id),
                label: format!("payload {:?}", reference.kind),
                value: reference,
                evidence: EvidenceGrade::Exact,
            })?;
            if let Admission::Retained(locator) = admission {
                self.payloads.insert(
                    locator.id,
                    BundlePayload {
                        bundle_root: self.bundle_path.to_path_buf(),
                        reference: reference.clone(),
                    },
                );
            }
        }
        Ok(())
    }

    /// Projects compaction, terminal, and interaction runtime maps.
    fn project_runtime_maps(&mut self, trace: &codex_rollout_trace::RolloutTrace) -> Result<()> {
        for (id, compaction) in &trace.compactions {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::Compaction,
                id,
                parent: self.locator(TraceNodeKind::Turn, &compaction.codex_turn_id),
                timestamp: Some(compaction.installed_at_unix_ms.to_string()),
                facts: rich_facts::compaction(id, compaction, self.fact_availability),
                label: format!("compaction {id}"),
                value: compaction,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, request) in &trace.compaction_requests {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::CompactionRequest,
                id,
                parent: self.locator(TraceNodeKind::Compaction, &request.compaction_id),
                timestamp: Some(request.execution.started_at_unix_ms.to_string()),
                facts: rich_facts::compaction_request(id, request, self.fact_availability),
                label: format!("compaction request {}", request.model),
                value: request,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, operation) in &trace.terminal_operations {
            let ownership = trace.tool_calls.get(&operation.tool_call_id).map_or_else(
                crate::TraceOwnership::default,
                |tool| crate::TraceOwnership {
                    thread_id: Some(tool.thread_id.clone()),
                    turn_id: tool.started_by_codex_turn_id.clone(),
                },
            );
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::TerminalOperation,
                id,
                parent: self.locator(TraceNodeKind::ToolCall, &operation.tool_call_id),
                timestamp: Some(operation.execution.started_at_unix_ms.to_string()),
                facts: rich_terminal_facts::operation(
                    id,
                    operation,
                    ownership,
                    self.fact_availability,
                ),
                label: format!("terminal operation {:?}", operation.kind),
                value: operation,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, terminal) in &trace.terminal_sessions {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::TerminalSession,
                id,
                parent: self.locator(
                    TraceNodeKind::TerminalOperation,
                    &terminal.created_by_operation_id,
                ),
                timestamp: Some(terminal.execution.started_at_unix_ms.to_string()),
                facts: rich_terminal_facts::session(id, terminal, self.fact_availability),
                label: format!("terminal {id}"),
                value: terminal,
                evidence: self.semantic_evidence,
            })?;
        }
        for (id, edge) in &trace.interaction_edges {
            self.admit_required(RichNodeSpec {
                kind: TraceNodeKind::InteractionEdge,
                id,
                parent: self.session_locator.clone(),
                timestamp: Some(edge.started_at_unix_ms.to_string()),
                facts: rich_facts::interaction(
                    id,
                    edge,
                    interaction_ownership(trace, edge),
                    self.fact_availability,
                ),
                label: format!("interaction {:?}", edge.kind),
                value: edge,
                evidence: self.semantic_evidence,
            })?;
        }
        Ok(())
    }

    /// Admits a child whose parent must already be retained.
    fn admit_required<T: Serialize>(&mut self, spec: RichNodeSpec<'_, T>) -> Result<Admission> {
        let (node, facts) = self.node(spec)?;
        Ok(self
            .graph
            .admit_with_required_parent(node, facts, /*source_path*/ None))
    }

    /// Admits a structural container whose parent may be projected later.
    fn admit_deferred<T: Serialize>(&mut self, spec: RichNodeSpec<'_, T>) -> Result<Admission> {
        let (node, facts) = self.node(spec)?;
        Ok(self.graph.admit(node, facts, /*source_path*/ None))
    }

    /// Builds one rich node while deriving presentation from serialized detail.
    fn node<T: Serialize>(&self, spec: RichNodeSpec<'_, T>) -> Result<(TraceNode, TraceNodeFacts)> {
        let detail = serde_json::to_value(spec.value)?;
        Ok((
            TraceNode::projected(
                self.locator(spec.kind, spec.id),
                Some(spec.parent),
                crate::TraceSourceKind::Rich,
                spec.evidence,
                spec.timestamp,
                spec.label,
                detail,
            ),
            spec.facts,
        ))
    }

    /// Builds a stable rich-source locator beneath this selected session.
    fn locator(&self, kind: TraceNodeKind, id: &str) -> TraceNodeLocator {
        TraceNodeLocator::new(self.session_id, kind, id)
    }
}

/// Derives the owning parent thread from typed interaction endpoints.
fn interaction_ownership(
    trace: &codex_rollout_trace::RolloutTrace,
    edge: &codex_rollout_trace::InteractionEdge,
) -> crate::TraceOwnership {
    let tool = [&edge.source, &edge.target].into_iter().find_map(|anchor| {
        let codex_rollout_trace::TraceAnchor::ToolCall { tool_call_id } = anchor else {
            return None;
        };
        trace.tool_calls.get(tool_call_id)
    });
    if let Some(tool) = tool {
        return crate::TraceOwnership {
            thread_id: Some(tool.thread_id.clone()),
            turn_id: tool.started_by_codex_turn_id.clone(),
        };
    }
    let thread_id = match &edge.source {
        codex_rollout_trace::TraceAnchor::Thread { thread_id } => Some(thread_id.clone()),
        codex_rollout_trace::TraceAnchor::ConversationItem { item_id } => trace
            .conversation_items
            .get(item_id)
            .map(|item| item.thread_id.clone()),
        codex_rollout_trace::TraceAnchor::ToolCall { .. } => None,
    };
    crate::TraceOwnership {
        thread_id,
        turn_id: None,
    }
}

/// Complete inputs for one rich-node admission.
struct RichNodeSpec<'a, T> {
    kind: TraceNodeKind,
    id: &'a str,
    parent: TraceNodeLocator,
    timestamp: Option<String>,
    facts: TraceNodeFacts,
    label: String,
    value: &'a T,
    evidence: EvidenceGrade,
}

/// Returns whether a rich parent chain exists and admits this cycle member.
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
