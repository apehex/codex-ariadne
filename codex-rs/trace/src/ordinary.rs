//! Bounded materialization of ordinary Codex rollout files.

use std::path::Path;

use chrono::DateTime;
use codex_protocol::protocol::RolloutItem;

use crate::Admission;
use crate::EvidenceGrade;
use crate::TraceCorrelation;
use crate::TraceDiagnostic;
use crate::TraceFactAvailability;
use crate::TraceGraphBuilder;
use crate::TraceLimits;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TraceRelation;
use crate::TraceSourceKind;
use crate::catalog::OrdinaryThread;
mod facts;
mod reader;
mod topology;

use self::reader::BoundedRolloutReader;
pub(crate) use self::topology::OrdinaryTopology;

/// Projects one ordinary thread and its bounded JSONL records.
pub(crate) async fn load_thread(
    session_id: &str,
    thread: &OrdinaryThread,
    session_locator: &TraceNodeLocator,
    topology: &OrdinaryTopology<'_>,
    limits: TraceLimits,
    graph: &mut TraceGraphBuilder,
) {
    let base_thread_locator = thread_locator(session_id, &thread.thread_id);
    let (parent, parent_id) = match topology.parent_id(thread) {
        Ok(Some(parent_id)) => (thread_locator(session_id, parent_id), Some(parent_id)),
        Ok(None) => (session_locator.clone(), None),
        Err(message) => {
            graph.record_diagnostic(path_diagnostic(&thread.path, message));
            (session_locator.clone(), None)
        }
    };
    let thread_detail = serde_json::json!({
        "session_id": thread.session_id,
        "thread_id": thread.thread_id,
        "path": thread.path,
        "cwd": thread.cwd,
        "model_provider": thread.model_provider,
        "archived": thread.archived,
        "original_parent_thread_id": thread.parent_thread_id,
        "forked_from_thread_id": thread.forked_from_thread_id,
        "history_base": thread.history_base,
    });
    let admission = graph.admit(
        TraceNode::projected(
            base_thread_locator,
            Some(parent),
            TraceSourceKind::Ordinary,
            EvidenceGrade::Semantic,
            Some(thread.timestamp.clone()),
            format!("thread {}", thread.thread_id),
            thread_detail,
        ),
        TraceNodeFacts::new(
            parse_timestamp_millis(&thread.timestamp).map_or_else(
                TraceOrder::default,
                |timestamp| {
                    TraceOrder::structural(timestamp, /*ended_at_unix_ms*/ None)
                },
            ),
            TraceOwnership {
                thread_id: Some(thread.thread_id.clone()),
                turn_id: None,
            },
            TraceFactAvailability::Partial,
            std::iter::once(TraceCorrelation::new(
                TraceRelation::SourceIdentity,
                TraceObjectRef::Thread(thread.thread_id.clone()),
            ))
            .chain(parent_id.map(|parent_id| {
                TraceCorrelation::new(
                    TraceRelation::ParentThread,
                    TraceObjectRef::Thread(parent_id.to_string()),
                )
            })),
        ),
        Some(&thread.path),
    );
    let Admission::Retained(thread_locator) = admission else {
        return;
    };

    let mut reader = match BoundedRolloutReader::open(&thread.path).await {
        Ok(reader) => reader,
        Err(error) => {
            graph.record_diagnostic(path_diagnostic(
                &thread.path,
                format!("cannot open rollout: {error}"),
            ));
            return;
        }
    };
    let mut line_index = 0_u64;
    let mut current_turn_id = None;
    loop {
        let line = match reader.next_line(limits.max_ordinary_record_bytes).await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                graph.record_diagnostic(path_diagnostic(
                    &thread.path,
                    format!("cannot read rollout line: {error}"),
                ));
                break;
            }
        };
        line_index += 1;
        if line.bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if line.oversized {
            graph.record_diagnostic(path_diagnostic(
                &thread.path,
                format!(
                    "rollout record at line {line_index} exceeds {} byte display limit",
                    limits.max_ordinary_record_bytes
                ),
            ));
            continue;
        }
        if graph.len() >= limits.max_nodes_per_session {
            graph.note_limit(Some(&thread.path));
            break;
        }
        let text = match std::str::from_utf8(&line.bytes) {
            Ok(text) => text,
            Err(error) => {
                graph.record_diagnostic(path_diagnostic(
                    &thread.path,
                    format!("rollout record at line {line_index} is not valid UTF-8: {error}"),
                ));
                continue;
            }
        };
        let value = match serde_json::from_str::<serde_json::Value>(text) {
            Ok(value) => value,
            Err(error) => {
                graph.record_diagnostic(path_diagnostic(
                    &thread.path,
                    format!("malformed JSON at line {line_index}: {error}"),
                ));
                continue;
            }
        };
        let typed = serde_json::from_value::<codex_protocol::protocol::RolloutLine>(value.clone());
        let (kind, label, facts) = match typed {
            Ok(line) => {
                if let RolloutItem::TurnContext(context) = &line.item {
                    current_turn_id.clone_from(&context.turn_id);
                }
                let facts = facts::record(
                    &line.item,
                    &thread.thread_id,
                    ordinal(&value, line_index),
                    value
                        .get("timestamp")
                        .and_then(serde_json::Value::as_str)
                        .and_then(parse_timestamp_millis),
                    current_turn_id.as_deref(),
                );
                let (kind, label) = record_kind(&line.item);
                (kind, label, facts)
            }
            Err(error) => {
                graph.record_diagnostic(path_diagnostic(
                    &thread.path,
                    format!("unknown rollout record at line {line_index}: {error}"),
                ));
                (
                    TraceNodeKind::RolloutRecord,
                    "unknown record",
                    TraceNodeFacts::new(
                        TraceOrder::ordinary(
                            ordinal(&value, line_index),
                            value
                                .get("timestamp")
                                .and_then(serde_json::Value::as_str)
                                .and_then(parse_timestamp_millis),
                        ),
                        TraceOwnership {
                            thread_id: Some(thread.thread_id.clone()),
                            turn_id: current_turn_id.clone(),
                        },
                        TraceFactAvailability::Unavailable,
                        [],
                    ),
                )
            }
        };
        let ordinal = ordinal(&value, line_index);
        graph.admit(
            TraceNode::projected(
                TraceNodeLocator::new(
                    session_id,
                    kind,
                    format!("ordinary:{}:{ordinal}", thread.thread_id),
                ),
                Some(thread_locator.clone()),
                TraceSourceKind::Ordinary,
                EvidenceGrade::Semantic,
                value
                    .get("timestamp")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                label.to_string(),
                value,
            ),
            facts,
            Some(&thread.path),
        );
    }
}

/// Returns the persisted ordinal or the reader's stable line position.
fn ordinal(value: &serde_json::Value, line_index: u64) -> u64 {
    value
        .get("ordinal")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(line_index)
}

/// Parses an upstream RFC 3339 timestamp for display without making it causal.
fn parse_timestamp_millis(timestamp: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

/// Maps a typed ordinary record to its normalized kind and label.
fn record_kind(item: &RolloutItem) -> (TraceNodeKind, &'static str) {
    match item {
        RolloutItem::SessionMeta(_) => (TraceNodeKind::RolloutRecord, "session metadata"),
        RolloutItem::ResponseItem(_) | RolloutItem::InterAgentCommunication(_) => {
            (TraceNodeKind::ConversationItem, "conversation item")
        }
        RolloutItem::InterAgentCommunicationMetadata { .. } => {
            (TraceNodeKind::RolloutRecord, "agent communication metadata")
        }
        RolloutItem::Compacted(_) => (TraceNodeKind::Compaction, "context compaction"),
        RolloutItem::TurnContext(_) => (TraceNodeKind::Turn, "turn context"),
        RolloutItem::WorldState(_) => (TraceNodeKind::RolloutRecord, "world state"),
        RolloutItem::EventMsg(_) => (TraceNodeKind::RolloutRecord, "event"),
    }
}

/// Builds the stable locator for an ordinary rollout thread.
fn thread_locator(session_id: &str, thread_id: &str) -> TraceNodeLocator {
    TraceNodeLocator::new(
        session_id,
        TraceNodeKind::Thread,
        format!("ordinary:{thread_id}"),
    )
}

/// Builds a source-local non-fatal diagnostic.
fn path_diagnostic(path: &Path, message: String) -> TraceDiagnostic {
    TraceDiagnostic::unavailable_at(path, message)
}

#[cfg(test)]
#[path = "ordinary_tests.rs"]
mod tests;
