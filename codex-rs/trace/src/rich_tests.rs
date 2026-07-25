use std::collections::BTreeMap;

use codex_rollout_trace::AgentOrigin;
use codex_rollout_trace::AgentThread;
use codex_rollout_trace::ExecutionStatus;
use codex_rollout_trace::ExecutionWindow;

use super::rich_parent_is_valid;

#[test]
fn rich_parent_validation_accepts_roots_and_rejects_missing_or_cyclic_chains() {
    let mut threads = BTreeMap::new();
    threads.insert("root".to_string(), thread("root", AgentOrigin::Root));
    threads.insert("child".to_string(), thread("child", spawned_from("root")));
    assert!(rich_parent_is_valid("child", "root", &threads));
    assert!(!rich_parent_is_valid("orphan", "missing", &threads));
    threads.insert(
        "orphan".to_string(),
        thread("orphan", spawned_from("missing")),
    );
    threads.insert(
        "orphan-child".to_string(),
        thread("orphan-child", spawned_from("orphan")),
    );
    assert!(rich_parent_is_valid("orphan-child", "orphan", &threads));

    threads.insert(
        "cycle-a".to_string(),
        thread("cycle-a", spawned_from("cycle-b")),
    );
    threads.insert(
        "cycle-b".to_string(),
        thread("cycle-b", spawned_from("cycle-a")),
    );
    threads.insert(
        "cycle-child".to_string(),
        thread("cycle-child", spawned_from("cycle-a")),
    );
    assert!(!rich_parent_is_valid("cycle-a", "cycle-b", &threads));
    assert!(rich_parent_is_valid("cycle-b", "cycle-a", &threads));
    assert!(rich_parent_is_valid("cycle-child", "cycle-a", &threads));
}

fn spawned_from(parent_thread_id: &str) -> AgentOrigin {
    AgentOrigin::Spawned {
        parent_thread_id: parent_thread_id.to_string(),
        spawn_edge_id: format!("spawn:{parent_thread_id}"),
        task_name: "synthetic".to_string(),
        agent_role: "worker".to_string(),
    }
}

fn thread(thread_id: &str, origin: AgentOrigin) -> AgentThread {
    AgentThread {
        thread_id: thread_id.to_string(),
        agent_path: format!("/root/{thread_id}"),
        nickname: None,
        origin,
        execution: ExecutionWindow {
            started_at_unix_ms: 1,
            started_seq: 1,
            ended_at_unix_ms: None,
            ended_seq: None,
            status: ExecutionStatus::Running,
        },
        default_model: None,
        conversation_item_ids: Vec::new(),
    }
}
