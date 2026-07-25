use std::fs;
use std::path::Path;
use std::path::PathBuf;

use codex_rollout_trace::RawPayloadKind;
use codex_rollout_trace::RawPayloadRef;
use codex_rollout_trace::RawTraceEventPayload;
use codex_rollout_trace::TraceWriter;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;

use super::*;

const ROOT_ID: &str = "019d0000-0000-7000-8000-000000000001";
const CHILD_ID: &str = "019d0000-0000-7000-8000-000000000002";
const GRANDCHILD_ID: &str = "019d0000-0000-7000-8000-000000000003";

#[tokio::test]
async fn ordinary_threads_form_a_searchable_tree_and_preserve_diagnostics() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, None, "hello from root", None);
    write_rollout(
        temp.path(),
        CHILD_ID,
        Some(ROOT_ID),
        "hello from child",
        Some("{\"unknown\":true}"),
    );
    write_rollout(
        temp.path(),
        GRANDCHILD_ID,
        Some(CHILD_ID),
        "hello from grandchild",
        None,
    );

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    assert_eq!(
        catalog.sessions,
        vec![SessionSummary {
            session_id: ROOT_ID.to_string(),
            root_thread_id: ROOT_ID.to_string(),
            source: TraceSourceKind::Ordinary,
            capabilities: TraceCapabilities::ORDINARY,
            created_at: Some("2026-07-25T00:00:00Z".to_string()),
            cwd: Some(PathBuf::from("/workspace")),
            model_provider: Some("openai".to_string()),
            status: TraceStatus::Unknown,
            archived: false,
            thread_count: Some(3),
        }]
    );

    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let root_thread = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{ROOT_ID}"),
    );
    let child_thread = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{CHILD_ID}"),
    );
    assert_eq!(trace.node(&child_thread).unwrap().parent, Some(root_thread));
    let grandchild_thread = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{GRANDCHILD_ID}"),
    );
    assert_eq!(
        trace.node(&grandchild_thread).unwrap().parent,
        Some(child_thread)
    );
    assert_eq!(trace.search("hello from child").len(), 1);
    assert_eq!(
        trace.search("hello from child")[0].evidence,
        EvidenceGrade::Semantic
    );
    assert!(
        trace.diagnostics[0]
            .message
            .contains("unknown rollout record")
    );
    assert!(
        trace
            .nodes
            .iter()
            .any(|node| node.locator.kind == TraceNodeKind::Diagnostic)
    );
}

#[tokio::test]
async fn orphan_and_cycle_threads_remain_reachable_from_the_session() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, Some(CHILD_ID), "cycle a", None);
    write_rollout(temp.path(), CHILD_ID, Some(ROOT_ID), "cycle b", None);
    let orphan_id = "019d0000-0000-7000-8000-000000000003";
    let missing_id = "019d0000-0000-7000-8000-000000000004";
    write_rollout(temp.path(), orphan_id, Some(missing_id), "orphan", None);

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let cycle = catalog.load_session(ROOT_ID).await.unwrap();
    let cycle_session = TraceNodeLocator::new(ROOT_ID, TraceNodeKind::Session, ROOT_ID);
    for id in [ROOT_ID, CHILD_ID] {
        let locator =
            TraceNodeLocator::new(ROOT_ID, TraceNodeKind::Thread, format!("ordinary:{id}"));
        assert_eq!(
            cycle.node(&locator).unwrap().parent,
            Some(cycle_session.clone())
        );
    }

    let orphan = catalog.load_session(missing_id).await.unwrap();
    let orphan_locator = TraceNodeLocator::new(
        missing_id,
        TraceNodeKind::Thread,
        format!("ordinary:{orphan_id}"),
    );
    let orphan_session = TraceNodeLocator::new(missing_id, TraceNodeKind::Session, missing_id);
    assert_eq!(
        orphan.node(&orphan_locator).unwrap().parent,
        Some(orphan_session)
    );
    assert!(orphan.nodes.iter().any(|node| {
        node.locator.kind == TraceNodeKind::Diagnostic && node.label.contains("parent rollout")
    }));
}

#[tokio::test]
async fn matching_ordinary_and_rich_sources_merge_without_node_loss() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, None, "ordinary evidence", None);
    let bundle = temp.path().join("rich/bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-merged".to_string(),
        "different-rollout-id".to_string(),
        ROOT_ID.to_string(),
    )
    .unwrap();
    writer
        .append(RawTraceEventPayload::ThreadStarted {
            thread_id: ROOT_ID.to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: None,
        })
        .unwrap();
    drop(writer);

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .with_rich_bundle(bundle)
        .discover()
        .await;
    let summary = &catalog.sessions[0];
    assert_eq!(summary.source, TraceSourceKind::Merged);
    assert_eq!(
        summary.capabilities,
        TraceCapabilities::ORDINARY.union(TraceCapabilities::RICH)
    );
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    assert!(
        trace
            .nodes
            .iter()
            .any(|node| node.provenance == TraceSourceKind::Ordinary)
    );
    assert!(
        trace
            .nodes
            .iter()
            .any(|node| node.provenance == TraceSourceKind::Rich)
    );
}

#[tokio::test]
async fn selected_session_materialization_is_bounded_and_visible_as_a_diagnostic() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, None, "bounded record", None);
    let limits = TraceLimits {
        max_nodes_per_session: 3,
        ..TraceLimits::default()
    };
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .with_limits(limits)
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    assert!(trace.nodes.len() <= limits.max_nodes_per_session);
    assert!(
        trace
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("materialization stopped"))
    );
}

#[tokio::test]
async fn malformed_rich_spines_downgrade_surviving_semantic_nodes() {
    use std::io::Write;

    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-degraded".to_string(),
        "rollout-degraded".to_string(),
        ROOT_ID.to_string(),
    )
    .unwrap();
    writer
        .append(RawTraceEventPayload::ThreadStarted {
            thread_id: ROOT_ID.to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: None,
        })
        .unwrap();
    drop(writer);
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(bundle.join("trace.jsonl"))
            .unwrap(),
        "{{malformed}}"
    )
    .unwrap();

    let catalog = TraceRepository::new(temp.path().join("empty"))
        .with_rich_bundle(bundle)
        .discover()
        .await;
    let trace = catalog.load_session("rollout-degraded").await.unwrap();
    let thread = trace
        .nodes
        .iter()
        .find(|node| node.locator.kind == TraceNodeKind::Thread)
        .unwrap();
    assert_eq!(thread.evidence, EvidenceGrade::Reconstructed);
}

#[tokio::test]
async fn mismatched_rich_root_is_a_conflicting_diagnostic() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, None, "ordinary evidence", None);
    let bundle = temp.path().join("rich/bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-conflict".to_string(),
        ROOT_ID.to_string(),
        CHILD_ID.to_string(),
    )
    .unwrap();
    writer
        .append(RawTraceEventPayload::ThreadStarted {
            thread_id: CHILD_ID.to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: None,
        })
        .unwrap();
    drop(writer);

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .with_rich_root(bundle)
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    assert!(trace.nodes.iter().any(|node| {
        node.locator.kind == TraceNodeKind::Diagnostic
            && node.evidence == EvidenceGrade::Conflicting
    }));
}

#[tokio::test]
async fn exact_bundle_directory_discovers_rich_semantic_nodes() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-1".to_string(),
        "rollout-1".to_string(),
        ROOT_ID.to_string(),
    )
    .unwrap();
    let metadata_payload = writer
        .write_json_payload(
            RawPayloadKind::SessionMetadata,
            &json!({"name": "exact rich metadata"}),
        )
        .unwrap();
    writer
        .append(RawTraceEventPayload::ThreadStarted {
            thread_id: ROOT_ID.to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: Some(metadata_payload),
        })
        .unwrap();
    drop(writer);
    let nested = TraceWriter::create(
        bundle.join("nested"),
        "trace-nested".to_string(),
        "rollout-nested".to_string(),
        CHILD_ID.to_string(),
    )
    .unwrap();
    drop(nested);

    let catalog = TraceRepository::new(temp.path().join("empty"))
        .with_rich_bundle(bundle)
        .discover()
        .await;
    assert_eq!(catalog.sessions.len(), 1);
    assert_eq!(catalog.sessions[0].source, TraceSourceKind::Rich);
    let trace = catalog.load_session("rollout-1").await.unwrap();
    let thread = trace
        .nodes
        .iter()
        .find(|node| node.locator.kind == TraceNodeKind::Thread)
        .unwrap();
    assert_eq!(
        (thread.provenance, thread.evidence),
        (TraceSourceKind::Rich, EvidenceGrade::Semantic)
    );
    let payload = trace
        .nodes
        .iter()
        .find(|node| node.locator.kind == TraceNodeKind::RawPayload)
        .unwrap();
    assert_eq!(
        (payload.provenance, payload.evidence),
        (TraceSourceKind::Rich, EvidenceGrade::Exact)
    );
}

#[tokio::test]
async fn rich_catalog_discovery_is_lazy_about_event_replay() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    fs::create_dir_all(bundle.join("payloads")).unwrap();
    fs::write(
        bundle.join("manifest.json"),
        json!({
            "schema_version": 999,
            "trace_id": "trace-lazy",
            "rollout_id": "rollout-lazy",
            "root_thread_id": ROOT_ID,
            "started_at_unix_ms": 1,
            "raw_event_log": "trace.jsonl",
            "payloads_dir": "payloads"
        })
        .to_string(),
    )
    .unwrap();
    fs::write(bundle.join("trace.jsonl"), "malformed event").unwrap();

    let catalog = TraceRepository::new(temp.path().join("empty"))
        .with_rich_root(bundle)
        .discover()
        .await;
    assert_eq!(catalog.sessions[0].session_id, "rollout-lazy");
    assert!(
        catalog
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("manifest schema 999"))
    );
    let trace = catalog.load_session("rollout-lazy").await.unwrap();
    assert!(
        trace
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("parse trace event"))
    );
    assert!(trace.nodes.iter().any(|node| {
        node.locator.kind == TraceNodeKind::Diagnostic
            && node.evidence == EvidenceGrade::Unavailable
    }));
}

#[tokio::test]
async fn payload_reader_contains_paths_sanitizes_controls_and_caps_reads() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    fs::create_dir_all(bundle.join("payloads")).unwrap();
    fs::create_dir_all(bundle.join("payloads/directory")).unwrap();
    fs::write(bundle.join("payloads/1.json"), "safe\u{1b}[31m red\u{0}\n").unwrap();
    fs::write(temp.path().join("outside.json"), "outside").unwrap();
    let reader = SafePayloadReader::new(bundle);
    let payload = RawPayloadRef {
        raw_payload_id: "raw:1".to_string(),
        kind: RawPayloadKind::ToolResult,
        path: "payloads/1.json".to_string(),
    };
    assert_eq!(
        reader
            .read(&payload, PayloadReadLimit::DEFAULT)
            .await
            .unwrap(),
        SanitizedPayload {
            text: "safe red\n".to_string(),
            truncated: false,
            original_bytes_read: 15,
        }
    );

    let escaped = RawPayloadRef {
        raw_payload_id: "raw:2".to_string(),
        kind: RawPayloadKind::ToolResult,
        path: "../outside.json".to_string(),
    };
    assert!(
        reader
            .read(&escaped, PayloadReadLimit::new(3))
            .await
            .unwrap_err()
            .to_string()
            .contains("escapes")
    );

    let directory = RawPayloadRef {
        raw_payload_id: "raw:3".to_string(),
        kind: RawPayloadKind::ToolResult,
        path: "payloads/directory".to_string(),
    };
    assert!(
        reader
            .read(&directory, PayloadReadLimit::DEFAULT)
            .await
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
}

fn write_rollout(
    codex_home: &Path,
    id: &str,
    parent: Option<&str>,
    message: &str,
    extra_line: Option<&str>,
) {
    let directory = codex_home.join("sessions/2026/07/25");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("rollout-2026-07-25T00-00-00-{id}.jsonl"));
    let mut lines = vec![
        json!({
            "timestamp": "2026-07-25T00:00:00Z",
            "type": "session_meta",
            "payload": {
                "session_id": id,
                "id": id,
                "parent_thread_id": parent,
                "timestamp": "2026-07-25T00:00:00Z",
                "cwd": "/workspace",
                "originator": "codex-trace-test",
                "cli_version": "0.0.0",
                "source": "cli",
                "model_provider": "openai",
                "base_instructions": null
            }
        })
        .to_string(),
        json!({
            "timestamp": "2026-07-25T00:00:01Z",
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": message,
                "kind": "plain"
            }
        })
        .to_string(),
    ];
    lines.extend(extra_line.map(str::to_string));
    fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}
