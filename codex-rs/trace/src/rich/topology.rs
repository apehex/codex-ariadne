//! Rich-trace parent topology and interaction ownership.

use std::collections::BTreeMap;

use codex_rollout_trace::AgentOrigin;

pub(super) fn interaction_ownership(
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

pub(super) fn parent_is_valid(
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
