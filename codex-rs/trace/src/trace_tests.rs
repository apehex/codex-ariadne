use std::fs;
use std::path::Path;
use std::path::PathBuf;

use codex_rollout_trace::RawPayloadKind;
use codex_rollout_trace::RawPayloadRef;
use codex_rollout_trace::RawToolCallRequester;
use codex_rollout_trace::RawTraceEventContext;
use codex_rollout_trace::RawTraceEventPayload;
use codex_rollout_trace::ToolCallKind;
use codex_rollout_trace::ToolCallSummary;
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
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "hello from root",
        /*extra_line*/ None,
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        CHILD_ID,
        Some(ROOT_ID),
        "hello from child",
        Some("{\"unknown\":true}"),
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        GRANDCHILD_ID,
        Some(CHILD_ID),
        "hello from grandchild",
        /*extra_line*/ None,
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
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        Some(CHILD_ID),
        "cycle a",
        /*extra_line*/ None,
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        CHILD_ID,
        Some(ROOT_ID),
        "cycle b",
        /*extra_line*/ None,
    );
    let orphan_id = "019d0000-0000-7000-8000-000000000003";
    let missing_id = "019d0000-0000-7000-8000-000000000004";
    write_rollout(
        temp.path(),
        ROOT_ID,
        orphan_id,
        Some(missing_id),
        "orphan",
        /*extra_line*/ None,
    );

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

    let orphan_locator = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{orphan_id}"),
    );
    let orphan_session = TraceNodeLocator::new(ROOT_ID, TraceNodeKind::Session, ROOT_ID);
    assert_eq!(
        cycle.node(&orphan_locator).unwrap().parent,
        Some(orphan_session)
    );
    assert!(cycle.nodes.iter().any(|node| {
        node.locator.kind == TraceNodeKind::Diagnostic && node.label.contains("parent rollout")
    }));
}

#[tokio::test]
async fn upstream_session_topology_keeps_lineage_distinct_from_containment() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "root",
        /*extra_line*/ None,
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        CHILD_ID,
        Some(ROOT_ID),
        "child",
        /*extra_line*/ None,
    );
    let child_path = temp.path().join(format!(
        "sessions/2026/07/25/rollout-2026-07-25T00-00-00-{CHILD_ID}.jsonl"
    ));
    let mut child_lines = fs::read_to_string(&child_path)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut child_meta = serde_json::from_str::<serde_json::Value>(&child_lines[0]).unwrap();
    child_meta["payload"]["forked_from_id"] = json!(GRANDCHILD_ID);
    child_meta["payload"]["history_base"] = json!({
        "thread_id": GRANDCHILD_ID,
        "end_ordinal_exclusive": 7,
        "end_byte_offset": 512,
    });
    child_lines[0] = child_meta.to_string();
    fs::write(&child_path, format!("{}\n", child_lines.join("\n"))).unwrap();

    write_rollout(
        temp.path(),
        GRANDCHILD_ID,
        GRANDCHILD_ID,
        /*parent*/ None,
        "independent fork",
        /*extra_line*/ None,
    );

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    assert_eq!(
        catalog
            .sessions
            .iter()
            .map(|summary| summary.session_id.as_str())
            .collect::<Vec<_>>(),
        vec![ROOT_ID, GRANDCHILD_ID]
    );

    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let child = trace
        .node(&TraceNodeLocator::new(
            ROOT_ID,
            TraceNodeKind::Thread,
            format!("ordinary:{CHILD_ID}"),
        ))
        .unwrap();
    assert_eq!(
        child.parent,
        Some(TraceNodeLocator::new(
            ROOT_ID,
            TraceNodeKind::Thread,
            format!("ordinary:{ROOT_ID}"),
        ))
    );
    assert_eq!(child.detail["forked_from_thread_id"], json!(GRANDCHILD_ID));
    assert_eq!(
        child.detail["history_base"]["thread_id"],
        json!(GRANDCHILD_ID)
    );
}

#[tokio::test]
async fn ordinary_records_follow_numeric_ordinals_despite_timestamp_regression() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "placeholder",
        /*extra_line*/ None,
    );
    let path = temp.path().join(format!(
        "sessions/2026/07/25/rollout-2026-07-25T00-00-00-{ROOT_ID}.jsonl"
    ));
    let records = [
        json!({
            "timestamp": "2026-07-20T00:00:00Z",
            "ordinal": 1,
            "type": "session_meta",
            "payload": {
                "session_id": ROOT_ID,
                "id": ROOT_ID,
                "timestamp": "2026-07-20T00:00:00Z",
                "cwd": "/workspace",
                "originator": "codex-trace-test",
                "cli_version": "0.0.0",
                "source": "cli",
                "model_provider": "openai",
                "base_instructions": null
            }
        }),
        json!({
            "timestamp": "2026-07-30T00:00:00Z",
            "ordinal": 2,
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": "newer wall time",
                "kind": "plain"
            }
        }),
        json!({
            "timestamp": "2026-07-20T00:00:01Z",
            "ordinal": 10,
            "type": "event_msg",
            "payload": {
                "type": "agent_message",
                "message": "later causal event with old wall time"
            }
        }),
    ];
    fs::write(
        path,
        format!(
            "{}\n",
            records
                .iter()
                .map(serde_json::Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
    .unwrap();

    let trace = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await
        .load_session(ROOT_ID)
        .await
        .unwrap();
    let thread = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{ROOT_ID}"),
    );

    assert_eq!(
        trace
            .children(&thread)
            .map(|node| (node.locator.id.as_str(), node.timestamp.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            (
                "ordinary:019d0000-0000-7000-8000-000000000001:1",
                Some("2026-07-20T00:00:00Z"),
            ),
            (
                "ordinary:019d0000-0000-7000-8000-000000000001:2",
                Some("2026-07-30T00:00:00Z"),
            ),
            (
                "ordinary:019d0000-0000-7000-8000-000000000001:10",
                Some("2026-07-20T00:00:01Z"),
            ),
        ]
    );
}

#[tokio::test]
async fn rich_turn_children_follow_raw_event_sequence_across_node_kinds() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("rich/bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-causal".to_string(),
        ROOT_ID.to_string(),
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
    writer
        .append(RawTraceEventPayload::CodexTurnStarted {
            codex_turn_id: "turn-z".to_string(),
            thread_id: ROOT_ID.to_string(),
        })
        .unwrap();
    let request = writer
        .write_json_payload(
            RawPayloadKind::InferenceRequest,
            &json!({
                "input": [{
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "causal first"}]
                }]
            }),
        )
        .unwrap();
    writer
        .append(RawTraceEventPayload::InferenceStarted {
            inference_call_id: "inference-z".to_string(),
            thread_id: ROOT_ID.to_string(),
            codex_turn_id: "turn-z".to_string(),
            model: "gpt-test".to_string(),
            provider_name: "synthetic".to_string(),
            request_payload: request,
        })
        .unwrap();
    writer
        .append_with_context(
            RawTraceEventContext {
                thread_id: Some(ROOT_ID.to_string()),
                codex_turn_id: Some("turn-z".to_string()),
            },
            RawTraceEventPayload::ToolCallStarted {
                tool_call_id: "tool-a".to_string(),
                model_visible_call_id: None,
                code_mode_runtime_tool_id: None,
                requester: RawToolCallRequester::Model,
                kind: ToolCallKind::Other {
                    name: "synthetic".to_string(),
                },
                summary: ToolCallSummary::Generic {
                    label: "synthetic tool".to_string(),
                    input_preview: None,
                    output_preview: None,
                },
                invocation_payload: None,
            },
        )
        .unwrap();
    drop(writer);

    let trace = TraceRepository::new(temp.path().to_path_buf())
        .with_rich_bundle(bundle)
        .discover()
        .await
        .load_session(ROOT_ID)
        .await
        .unwrap();
    let turn = TraceNodeLocator::new(ROOT_ID, TraceNodeKind::Turn, "turn-z");

    assert_eq!(
        trace
            .children(&turn)
            .map(|node| (node.locator.kind, node.locator.id.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (TraceNodeKind::ConversationItem, "conversation_item:1",),
            (TraceNodeKind::Inference, "inference-z"),
            (TraceNodeKind::ToolCall, "tool-a"),
        ],
    );
}

#[tokio::test]
async fn cross_session_parent_ids_do_not_merge_independent_sessions() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "first session",
        /*extra_line*/ None,
    );
    write_rollout(
        temp.path(),
        GRANDCHILD_ID,
        CHILD_ID,
        Some(ROOT_ID),
        "second session",
        /*extra_line*/ None,
    );

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    assert_eq!(catalog.sessions.len(), 2);
    let trace = catalog.load_session(GRANDCHILD_ID).await.unwrap();
    let session = TraceNodeLocator::new(GRANDCHILD_ID, TraceNodeKind::Session, GRANDCHILD_ID);
    let thread = TraceNodeLocator::new(
        GRANDCHILD_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{CHILD_ID}"),
    );

    assert_eq!(trace.node(&thread).unwrap().parent, Some(session));
    assert!(trace.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("missing or belongs to another session")
    }));
}

#[tokio::test]
async fn matching_ordinary_and_rich_sources_merge_without_node_loss() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "ordinary evidence",
        /*extra_line*/ None,
    );
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
async fn duplicate_ordinary_observations_are_retained_with_stable_parentage() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "first observation",
        /*extra_line*/ None,
    );
    let source = temp.path().join(format!(
        "sessions/2026/07/25/rollout-2026-07-25T00-00-00-{ROOT_ID}.jsonl"
    ));
    let archived_directory = temp.path().join("archived_sessions/2026/07/25");
    fs::create_dir_all(&archived_directory).unwrap();
    let archived = archived_directory.join(format!("rollout-2026-07-25T00-00-00-{ROOT_ID}.jsonl"));
    fs::write(
        &archived,
        fs::read_to_string(&source)
            .unwrap()
            .replace("first observation", "conflicting observation"),
    )
    .unwrap();

    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let repeated_thread = TraceNodeLocator::new(
        ROOT_ID,
        TraceNodeKind::Thread,
        format!("ordinary:{ROOT_ID}:observation:2"),
    );
    let repeated_record = trace
        .nodes
        .iter()
        .find(|node| node.locator.id == format!("ordinary:{ROOT_ID}:2:observation:2"))
        .expect("repeated ordinary record");

    assert!(trace.node(&repeated_thread).is_some());
    assert_eq!(repeated_record.parent.as_ref(), Some(&repeated_thread));
    assert!(
        trace
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.evidence == EvidenceGrade::Conflicting)
    );
    assert_eq!(
        trace
            .search("first observation")
            .into_iter()
            .filter(|hit| hit.locator.kind == TraceNodeKind::RolloutRecord)
            .count(),
        1
    );
    assert_eq!(
        trace
            .search("conflicting observation")
            .into_iter()
            .filter(|hit| hit.locator.kind == TraceNodeKind::RolloutRecord)
            .count(),
        1
    );
}

#[tokio::test]
async fn selected_session_materialization_is_bounded_and_visible_as_a_diagnostic() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "bounded record",
        /*extra_line*/ None,
    );
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
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "ordinary evidence",
        /*extra_line*/ None,
    );
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
        (
            thread.provenance,
            thread.evidence,
            thread.presentation.class,
        ),
        (
            TraceSourceKind::Rich,
            EvidenceGrade::Semantic,
            TraceRecordClass::Structure,
        )
    );
    let payload = trace
        .nodes
        .iter()
        .find(|node| node.locator.kind == TraceNodeKind::RawPayload)
        .unwrap();
    assert_eq!(
        (
            payload.provenance,
            payload.evidence,
            payload.presentation.class,
        ),
        (
            TraceSourceKind::Rich,
            EvidenceGrade::Exact,
            TraceRecordClass::RawArtifact,
        )
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
            .read(&escaped, PayloadReadLimit::new(/*bytes*/ 3))
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
    session_id: &str,
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
                "session_id": session_id,
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
