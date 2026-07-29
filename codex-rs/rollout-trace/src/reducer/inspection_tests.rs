use std::io::Write;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::ReplayLimits;
use super::inspect_bundle;
use super::replay_bundle_resilient;
use super::replay_bundle_resilient_with_limits;
use crate::RawPayloadKind;
use crate::RawTraceEventPayload;
use crate::TraceWriter;
use crate::replay_bundle;

#[test]
fn inspects_bundle_without_replaying() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-1".to_string(),
        "rollout-1".to_string(),
        "thread-root".to_string(),
    )?;
    drop(writer);

    let metadata = inspect_bundle(temp.path())?;
    assert_eq!(metadata.trace_id, "trace-1");
    assert_eq!(metadata.rollout_id, "rollout-1");
    assert_eq!(metadata.root_thread_id, "thread-root");
    assert_eq!(metadata.raw_event_log, "trace.jsonl");
    Ok(())
}

#[test]
fn resilient_replay_preserves_valid_events_around_malformed_input() -> anyhow::Result<()> {
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
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-child".to_string(),
        agent_path: "/root/thread-child".to_string(),
        metadata_payload: None,
    })?;
    drop(writer);
    let event_log = temp.path().join("trace.jsonl");
    let events = std::fs::read_to_string(&event_log)?;
    let mut lines = events.lines();
    let root = lines.next().expect("root event");
    let child = lines.next().expect("child event");
    let mut file = std::fs::File::create(&event_log)?;
    writeln!(file, "{root}")?;
    writeln!(file, "{{not json}}")?;
    file.write_all(b"\xff\n")?;
    writeln!(file, "{child}")?;

    assert!(replay_bundle(temp.path()).is_err());
    let replay = replay_bundle_resilient(temp.path())?;
    assert!(replay.trace.threads.contains_key("thread-root"));
    assert!(replay.trace.threads.contains_key("thread-child"));
    assert!(!replay.semantically_complete);
    assert_eq!(replay.diagnostics.len(), 2);
    Ok(())
}

#[test]
fn resilient_replay_discards_oversized_records_and_continues() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-limited".to_string(),
        "rollout-limited".to_string(),
        "thread-root".to_string(),
    )?;
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    drop(writer);
    let event_log = temp.path().join("trace.jsonl");
    let valid = std::fs::read_to_string(&event_log)?;
    let event_limit = valid.trim_end_matches(['\r', '\n']).len();
    let mut file = std::fs::File::create(&event_log)?;
    writeln!(file, "{}", "x".repeat(event_limit + 1))?;
    write!(file, "{valid}")?;

    let replay = replay_bundle_resilient_with_limits(
        temp.path(),
        ReplayLimits::default().max_event_bytes(event_limit),
    )?;
    assert!(replay.trace.threads.contains_key("thread-root"));
    assert!(!replay.semantically_complete);
    assert_eq!(replay.diagnostics.len(), 1);
    Ok(())
}

#[test]
fn replay_uses_the_contained_manifest_event_log_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-custom".to_string(),
        "rollout-custom".to_string(),
        "thread-root".to_string(),
    )?;
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    drop(writer);
    std::fs::rename(
        temp.path().join("trace.jsonl"),
        temp.path().join("events.jsonl"),
    )?;
    let manifest_path = temp.path().join("manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["raw_event_log"] = serde_json::Value::String("events.jsonl".to_string());
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    let strict = replay_bundle(temp.path())?;
    let resilient = replay_bundle_resilient(temp.path())?;

    assert!(strict.threads.contains_key("thread-root"));
    assert!(resilient.trace.threads.contains_key("thread-root"));
    Ok(())
}

#[test]
fn replay_rejects_manifest_event_log_escape() -> anyhow::Result<()> {
    let parent = TempDir::new()?;
    let bundle = parent.path().join("bundle");
    std::fs::create_dir(&bundle)?;
    let writer = TraceWriter::create(
        &bundle,
        "trace-escape".to_string(),
        "rollout-escape".to_string(),
        "thread-root".to_string(),
    )?;
    drop(writer);
    std::fs::write(parent.path().join("outside.jsonl"), b"{}\n")?;
    let manifest_path = bundle.join("manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["raw_event_log"] = serde_json::Value::String("../outside.jsonl".to_string());
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    let strict_error = replay_bundle(&bundle).expect_err("strict replay must reject escape");
    let resilient_error =
        replay_bundle_resilient(&bundle).expect_err("resilient replay must reject escape");

    assert!(strict_error.to_string().contains("escapes trace bundle"));
    assert!(resilient_error.to_string().contains("escapes trace bundle"));
    Ok(())
}

#[test]
fn resilient_replay_stops_at_the_event_limit() -> anyhow::Result<()> {
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

    let replay =
        replay_bundle_resilient_with_limits(temp.path(), ReplayLimits::default().max_events(1))?;
    assert!(replay.trace.threads.contains_key("thread-root"));
    assert!(!replay.trace.threads.contains_key("thread-child"));
    assert!(!replay.semantically_complete);
    Ok(())
}

#[test]
fn semantic_payload_limit_is_applied_to_resilient_replay() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let writer = TraceWriter::create(
        temp.path(),
        "trace-payload".to_string(),
        "rollout-payload".to_string(),
        "thread-root".to_string(),
    )?;
    writer.append(RawTraceEventPayload::ThreadStarted {
        thread_id: "thread-root".to_string(),
        agent_path: "/root".to_string(),
        metadata_payload: None,
    })?;
    writer.append(RawTraceEventPayload::CodexTurnStarted {
        codex_turn_id: "turn-1".to_string(),
        thread_id: "thread-root".to_string(),
    })?;
    let request = writer.write_json_payload(
        RawPayloadKind::InferenceRequest,
        &serde_json::json!({"input": [{"type": "message", "role": "user", "content": "large"}]}),
    )?;
    writer.append(RawTraceEventPayload::InferenceStarted {
        inference_call_id: "inference-1".to_string(),
        thread_id: "thread-root".to_string(),
        codex_turn_id: "turn-1".to_string(),
        model: "test".to_string(),
        provider_name: "test".to_string(),
        request_payload: request,
    })?;
    drop(writer);

    let replay = replay_bundle_resilient_with_limits(
        temp.path(),
        ReplayLimits::default().max_semantic_payload_bytes(1),
    )?;
    assert!(!replay.semantically_complete);
    assert!(
        replay
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("semantic replay limit"))
    );
    Ok(())
}
