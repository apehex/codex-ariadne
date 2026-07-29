//! Additive, bounded inspection APIs for diagnostic trace viewers.

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use super::SemanticPayloadReadLimit;
use super::bundle_path::resolve_contained_file;
use super::new_reducer;
use super::read_manifest;
use crate::bundle::TRACE_MANIFEST_SCHEMA_VERSION;
use crate::model::AgentThreadId;
use crate::model::RolloutTrace;
use crate::raw_event::RawTraceEvent;

/// Trace bundle manifest schema version understood by the inspection API.
pub const TRACE_BUNDLE_SCHEMA_VERSION: u32 = TRACE_MANIFEST_SCHEMA_VERSION;

/// Immutable manifest metadata needed to catalog a trace bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBundleMetadata {
    /// Trace manifest schema version.
    pub schema_version: u32,
    /// Stable identifier for this trace bundle.
    pub trace_id: String,
    /// Codex rollout/session identifier captured by this bundle.
    pub rollout_id: String,
    /// Root thread for the recorded rollout.
    pub root_thread_id: AgentThreadId,
    /// Bundle creation time as Unix milliseconds.
    pub started_at_unix_ms: i64,
    /// Manifest-relative path to the append-only event log.
    pub raw_event_log: String,
    /// Manifest-relative path to the raw payload directory.
    pub payloads_dir: String,
}

/// Best-effort replay result for diagnostic viewers.
#[derive(Debug)]
pub struct ResilientReplay {
    /// Reduced semantic trace built from usable events.
    pub trace: RolloutTrace,
    /// Bounded failures encountered while reading or reducing events.
    pub diagnostics: Vec<ReplayDiagnostic>,
    /// False when an event was skipped or failed reduction.
    pub semantically_complete: bool,
}

/// Resource limits for best-effort diagnostic replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayLimits {
    max_events: usize,
    max_event_bytes: usize,
    max_semantic_payload_bytes: usize,
}

impl ReplayLimits {
    /// Replaces the maximum number of non-empty events attempted.
    pub fn max_events(mut self, max_events: usize) -> Self {
        self.max_events = max_events;
        self
    }

    /// Replaces the maximum encoded bytes retained for one event.
    pub fn max_event_bytes(mut self, max_event_bytes: usize) -> Self {
        self.max_event_bytes = max_event_bytes;
        self
    }

    /// Replaces the maximum encoded bytes read from one semantic payload.
    pub fn max_semantic_payload_bytes(mut self, max_semantic_payload_bytes: usize) -> Self {
        self.max_semantic_payload_bytes = max_semantic_payload_bytes;
        self
    }
}

impl Default for ReplayLimits {
    fn default() -> Self {
        Self {
            max_events: 100_000,
            max_event_bytes: 1024 * 1024,
            max_semantic_payload_bytes: 1024 * 1024,
        }
    }
}

/// One malformed or inconsistent event skipped by resilient replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDiagnostic {
    /// One-based event-log record number, when applicable.
    pub line: Option<usize>,
    /// Bounded diagnostic message without source payload contents.
    pub message: String,
}

/// Reads trace-bundle metadata without replaying the event log.
pub fn inspect_bundle(bundle_dir: impl AsRef<Path>) -> Result<TraceBundleMetadata> {
    let manifest = read_manifest(bundle_dir.as_ref())?;
    Ok(TraceBundleMetadata {
        schema_version: manifest.schema_version,
        trace_id: manifest.trace_id,
        rollout_id: manifest.rollout_id,
        root_thread_id: manifest.root_thread_id,
        started_at_unix_ms: manifest.started_at_unix_ms,
        raw_event_log: manifest.raw_event_log,
        payloads_dir: manifest.payloads_dir,
    })
}

/// Replays all usable rich events while retaining per-event failures.
pub fn replay_bundle_resilient(bundle_dir: impl AsRef<Path>) -> Result<ResilientReplay> {
    replay_bundle_resilient_with_limits(bundle_dir, ReplayLimits::default())
}

/// Replays usable rich events within explicit resource limits.
pub fn replay_bundle_resilient_with_limits(
    bundle_dir: impl AsRef<Path>,
    limits: ReplayLimits,
) -> Result<ResilientReplay> {
    let bundle_dir = bundle_dir.as_ref();
    let manifest = read_manifest(bundle_dir)?;
    let event_log_path = resolve_event_log(bundle_dir, &manifest.raw_event_log)?;
    let mut reducer = new_reducer(
        bundle_dir,
        manifest,
        SemanticPayloadReadLimit::Bytes(limits.max_semantic_payload_bytes),
    );
    let event_log = File::open(&event_log_path)
        .with_context(|| format!("open trace event log {}", event_log_path.display()))?;
    let mut reader = BufReader::new(event_log);
    let mut diagnostics = Vec::new();
    let mut event_count = 0_usize;
    let mut line_number = 0_usize;
    let mut semantically_complete = true;
    let mut can_finalize = true;
    while let Some(line) = read_bounded_line(&mut reader, limits.max_event_bytes)? {
        line_number += 1;
        if line.bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if line.oversized {
            diagnostics.push(ReplayDiagnostic {
                line: Some(line_number),
                message: format!(
                    "trace event exceeds {} byte replay limit",
                    limits.max_event_bytes
                ),
            });
            semantically_complete = false;
            continue;
        }
        if event_count >= limits.max_events {
            diagnostics.push(ReplayDiagnostic {
                line: Some(line_number),
                message: format!("trace replay stopped after {} events", limits.max_events),
            });
            semantically_complete = false;
            break;
        }
        event_count += 1;
        let text = match std::str::from_utf8(&line.bytes) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(ReplayDiagnostic {
                    line: Some(line_number),
                    message: format!("trace event is not valid UTF-8: {error}"),
                });
                semantically_complete = false;
                continue;
            }
        };
        let event = match serde_json::from_str::<RawTraceEvent>(text) {
            Ok(event) => event,
            Err(error) => {
                diagnostics.push(ReplayDiagnostic {
                    line: Some(line_number),
                    message: format!("parse trace event: {error}"),
                });
                semantically_complete = false;
                continue;
            }
        };
        if let Err(error) = reducer.apply_event(event) {
            diagnostics.push(ReplayDiagnostic {
                line: Some(line_number),
                message: format!("reduce trace event: {error:#}"),
            });
            semantically_complete = false;
            can_finalize = false;
            break;
        }
    }
    if can_finalize && let Err(error) = reducer.resolve_pending_spawn_edge_fallbacks() {
        diagnostics.push(ReplayDiagnostic {
            line: None,
            message: format!("finalize trace replay: {error:#}"),
        });
        semantically_complete = false;
    }
    Ok(ResilientReplay {
        trace: reducer.rollout,
        diagnostics,
        semantically_complete,
    })
}

/// One event-log record retained up to its configured byte limit.
struct BoundedLine {
    bytes: Vec<u8>,
    oversized: bool,
}

/// Reads one line while discarding, rather than retaining, bytes over the cap.
fn read_bounded_line(reader: &mut impl BufRead, limit: usize) -> Result<Option<BoundedLine>> {
    let mut bytes = Vec::with_capacity(limit.saturating_add(1).min(64 * 1024));
    let mut oversized = false;
    let mut consumed_any = false;
    loop {
        let available = reader.fill_buf().context("read trace event")?;
        if available.is_empty() {
            if !consumed_any {
                return Ok(None);
            }
            break;
        }
        consumed_any = true;
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let chunk = &available[..take];
        let content = chunk.strip_suffix(b"\n").unwrap_or(chunk);
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&content[..content.len().min(remaining)]);
        oversized |= content.len() > remaining;
        let ended = chunk.ends_with(b"\n");
        reader.consume(take);
        if ended {
            break;
        }
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    Ok(Some(BoundedLine { bytes, oversized }))
}

/// Resolves a manifest-relative regular file contained by the bundle root.
pub(super) fn resolve_event_log(bundle_dir: &Path, relative: &str) -> Result<PathBuf> {
    resolve_contained_file(bundle_dir, Path::new(relative), "trace event log")
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
