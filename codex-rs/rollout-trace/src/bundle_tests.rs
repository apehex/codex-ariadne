use pretty_assertions::assert_eq;
use tempfile::TempDir;

use crate::RawPayloadKind;
use crate::RawTraceEventPayload;
use crate::ReplayLimits;
use crate::TraceWriter;
use crate::replay_bundle;
use crate::replay_bundle_resilient;
use crate::replay_bundle_resilient_with_limits;

use super::MANIFEST_FILE_NAME;
use super::RAW_EVENT_LOG_FILE_NAME;
use super::TraceBundleManifest;
use super::read_bundle_manifest;

#[test]
fn reads_bundle_manifest_without_replay() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let expected = TraceBundleManifest::new(
        "trace-1".to_string(),
        "rollout-1".to_string(),
        "thread-root".to_string(),
        1_234,
    );
    std::fs::write(
        temp.path().join(MANIFEST_FILE_NAME),
        serde_json::to_vec(&expected)?,
    )?;

    assert_eq!(read_bundle_manifest(temp.path())?, expected);
    Ok(())
}

#[test]
fn malformed_bundle_manifest_returns_path_context() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let manifest_path = temp.path().join(MANIFEST_FILE_NAME);
    std::fs::write(&manifest_path, "{")?;

    let error = read_bundle_manifest(temp.path()).expect_err("malformed manifest should fail");
    assert!(
        format!("{error:#}").contains(&format!(
            "parse trace bundle manifest {}",
            manifest_path.display()
        )),
        "unexpected error: {error:#}"
    );
    Ok(())
}

#[test]
fn resilient_replay_preserves_valid_events_around_a_malformed_line() -> anyhow::Result<()> {
    use std::io::Write;

    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-1".to_string(),
        "rollout-1".to_string(),
        "thread-root".to_string(),
    )?;
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    drop(writer);
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(temp.path().join(RAW_EVENT_LOG_FILE_NAME))?,
        "{{not json}}"
    )?;

    assert!(replay_bundle(temp.path()).is_err());
    let replay = replay_bundle_resilient(temp.path())?;
    assert!(replay.trace.threads.contains_key("thread-root"));
    assert!(!replay.semantically_complete);
    assert_eq!(replay.diagnostics.len(), 1);
    assert_eq!(replay.diagnostics[0].line, Some(2));
    assert!(replay.diagnostics[0].message.contains("parse trace event"));
    Ok(())
}

#[test]
fn resilient_replay_stops_at_the_configured_event_limit() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-limited".to_string(),
        "rollout-limited".to_string(),
        "thread-root".to_string(),
    )?;
    for id in ["thread-root", "thread-child"] {
        writer.append(RawTraceEventPayload::ThreadStarted {
            thread_id: id.to_string(),
            agent_path: format!("/root/{id}"),
            metadata_payload: None,
        })?;
    }
    drop(writer);

    let replay = replay_bundle_resilient_with_limits(
        temp.path(),
        ReplayLimits {
            max_events: 1,
            max_event_bytes: 1024,
        },
    )?;
    assert!(!replay.semantically_complete);
    assert!(replay.trace.threads.contains_key("thread-root"));
    assert!(!replay.trace.threads.contains_key("thread-child"));
    assert!(
        replay
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("stopped after 1 events"))
    );
    Ok(())
}

#[test]
fn reducer_failure_stops_downstream_replay_and_marks_the_graph_incomplete() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-failure".to_string(),
        "rollout-failure".to_string(),
        "thread-root".to_string(),
    )?;
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    for (turn, inference, command) in [
        ("turn-1", "inference-1", "cargo test"),
        ("turn-2", "inference-2", "cargo check"),
    ] {
        writer.append(RawTraceEventPayload::CodexTurnStarted {
            codex_turn_id: turn.to_string(),
            thread_id: "thread-root".to_string(),
        })?;
        let request = writer.write_json_payload(
            RawPayloadKind::InferenceRequest,
            &serde_json::json!({
                "input": [{
                    "type": "function_call",
                    "name": "shell",
                    "arguments": format!("{{\"cmd\":\"{command}\"}}"),
                    "call_id": "reused-call"
                }]
            }),
        )?;
        writer.append(RawTraceEventPayload::InferenceStarted {
            inference_call_id: inference.to_string(),
            thread_id: "thread-root".to_string(),
            codex_turn_id: turn.to_string(),
            model: "gpt-test".to_string(),
            provider_name: "test".to_string(),
            request_payload: request,
        })?;
    }
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "downstream-child".to_string(),
        agent_path: "/root/downstream".to_string(),
        metadata_payload: None,
    })?;
    drop(writer);

    let replay = replay_bundle_resilient(temp.path())?;
    assert!(!replay.semantically_complete);
    assert!(replay.trace.inference_calls.contains_key("inference-1"));
    assert!(!replay.trace.threads.contains_key("downstream-child"));
    assert!(
        replay
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("reused with different content"))
    );
    Ok(())
}
