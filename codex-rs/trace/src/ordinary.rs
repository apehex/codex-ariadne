//! Bounded materialization of ordinary Codex rollout files.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use chrono::DateTime;
use codex_protocol::protocol::RolloutItem;

use crate::Admission;
use crate::EvidenceGrade;
use crate::SiblingOrder;
use crate::TraceDiagnostic;
use crate::TraceGraphBuilder;
use crate::TraceLimits;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceSourceKind;
use crate::catalog::OrdinaryThread;

/// Precomputed ordinary-thread identity and containment relationships.
pub(crate) struct OrdinaryTopology<'a> {
    by_id: BTreeMap<&'a str, Vec<&'a OrdinaryThread>>,
}

impl<'a> OrdinaryTopology<'a> {
    /// Indexes every retained observation without silently choosing a conflicting parent.
    pub(crate) fn new(threads: &'a [OrdinaryThread]) -> Self {
        let mut by_id = BTreeMap::<&str, Vec<&OrdinaryThread>>::new();
        for thread in threads {
            by_id
                .entry(thread.thread_id.as_str())
                .or_default()
                .push(thread);
        }
        Self { by_id }
    }

    /// Resolves a same-session parent while rejecting gaps, conflicts, and cycles.
    fn parent_id(&self, thread: &'a OrdinaryThread) -> Result<Option<&'a str>, String> {
        let Some(parent_id) = thread.parent_thread_id.as_deref() else {
            return Ok(None);
        };
        let mut next = Some(parent_id);
        let mut seen = BTreeSet::new();
        while let Some(id) = next {
            if !seen.insert(id) {
                return Err(format!(
                    "parent cycle at {id}; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            let Some(observations) = self.by_id.get(id) else {
                return Err(format!(
                    "parent rollout {id} is missing or belongs to another session; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            };
            let same_session = observations
                .iter()
                .copied()
                .filter(|parent| parent.session_id == thread.session_id)
                .collect::<Vec<_>>();
            if same_session.is_empty() {
                let observed_session = &observations[0].session_id;
                return Err(format!(
                    "parent rollout {id} belongs to session {observed_session}; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            let parent_parent = same_session[0].parent_thread_id.as_deref();
            if same_session
                .iter()
                .any(|parent| parent.parent_thread_id.as_deref() != parent_parent)
            {
                return Err(format!(
                    "parent rollout {id} has conflicting containment observations; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            next = parent_parent;
        }
        Ok(Some(parent_id))
    }
}

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
    let parent = match topology.parent_id(thread) {
        Ok(Some(parent_id)) => thread_locator(session_id, parent_id),
        Ok(None) => session_locator.clone(),
        Err(message) => {
            graph.record_diagnostic(path_diagnostic(&thread.path, message));
            session_locator.clone()
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
        thread_start_order(&thread.timestamp),
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
        let (kind, label) = match typed {
            Ok(line) => record_kind(&line.item),
            Err(error) => {
                graph.record_diagnostic(path_diagnostic(
                    &thread.path,
                    format!("unknown rollout record at line {line_index}: {error}"),
                ));
                (TraceNodeKind::RolloutRecord, "unknown record")
            }
        };
        let ordinal = value
            .get("ordinal")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(line_index);
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
            SiblingOrder::Sequence(ordinal),
            Some(&thread.path),
        );
    }
}

/// Converts an upstream RFC 3339 thread start into a sortable session-level position.
fn thread_start_order(timestamp: &str) -> SiblingOrder {
    DateTime::parse_from_rfc3339(timestamp).map_or(SiblingOrder::Unspecified, |timestamp| {
        SiblingOrder::Timestamp(timestamp.timestamp_millis())
    })
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

/// One bounded rollout line, excluding its line terminator.
struct BoundedLine {
    bytes: Vec<u8>,
    oversized: bool,
}

/// Blocking plain-or-zstd reader driven from Tokio's blocking pool.
struct BoundedRolloutReader {
    reader: Option<BufReader<Box<dyn Read + Send>>>,
}

impl BoundedRolloutReader {
    /// Opens an ordinary rollout without changing its on-disk representation.
    async fn open(path: &Path) -> std::io::Result<Self> {
        let path = path.to_path_buf();
        let reader = tokio::task::spawn_blocking(move || open_blocking(path))
            .await
            .map_err(std::io::Error::other)??;
        Ok(Self {
            reader: Some(reader),
        })
    }

    /// Reads one record while discarding bytes beyond the configured cap.
    async fn next_line(&mut self, limit: usize) -> std::io::Result<Option<BoundedLine>> {
        let Some(mut reader) = self.reader.take() else {
            return Err(std::io::Error::other("rollout reader is busy"));
        };
        let (result, reader) =
            tokio::task::spawn_blocking(move || (read_bounded_line(&mut reader, limit), reader))
                .await
                .map_err(std::io::Error::other)?;
        self.reader = Some(reader);
        result
    }
}

/// Opens a plain or compressed rollout for blocking buffered reads.
fn open_blocking(path: PathBuf) -> std::io::Result<BufReader<Box<dyn Read + Send>>> {
    let file = File::open(&path)?;
    let reader: Box<dyn Read + Send> =
        if path.extension().is_some_and(|extension| extension == "zst") {
            Box::new(zstd::stream::read::Decoder::new(file)?)
        } else {
            Box::new(file)
        };
    Ok(BufReader::new(reader))
}

/// Reads through one newline without retaining bytes beyond `limit`.
fn read_bounded_line(
    reader: &mut impl BufRead,
    limit: usize,
) -> std::io::Result<Option<BoundedLine>> {
    let mut bytes = Vec::with_capacity(limit.saturating_add(1).min(64 * 1024));
    let mut oversized = false;
    let mut saw_bytes = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        saw_bytes = true;
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let chunk = &available[..consumed];
        let without_newline = chunk.strip_suffix(b"\n").unwrap_or(chunk);
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&without_newline[..without_newline.len().min(remaining)]);
        oversized |= without_newline.len() > remaining;
        let ends_with_newline = chunk.ends_with(b"\n");
        reader.consume(consumed);
        if ends_with_newline {
            break;
        }
    }
    if !saw_bytes {
        return Ok(None);
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    Ok(Some(BoundedLine { bytes, oversized }))
}

#[cfg(test)]
#[path = "ordinary_tests.rs"]
mod tests;
