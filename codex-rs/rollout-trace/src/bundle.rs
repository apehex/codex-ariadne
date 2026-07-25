//! Trace bundle manifest and local layout constants.

use std::fs::File;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::model::AgentThreadId;

pub(crate) const MANIFEST_FILE_NAME: &str = "manifest.json";
pub(crate) const RAW_EVENT_LOG_FILE_NAME: &str = "trace.jsonl";
pub(crate) const PAYLOADS_DIR_NAME: &str = "payloads";
/// Conventional file name for a reducer-written `RolloutTrace` cache.
pub const REDUCED_STATE_FILE_NAME: &str = "state.json";
pub const TRACE_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub(crate) const REDUCED_TRACE_SCHEMA_VERSION: u32 = 1;

/// Manifest stored at the root of a trace bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceBundleManifest {
    /// Trace manifest schema version.
    pub schema_version: u32,
    /// Stable identifier for this trace bundle.
    pub trace_id: String,
    /// Codex rollout/session identifier captured by this bundle.
    pub rollout_id: String,
    /// Root thread for the recorded rollout. Replay should fail rather than
    /// inventing a placeholder, because every reduced object is scoped back to
    /// this thread tree.
    pub root_thread_id: AgentThreadId,
    /// Bundle creation time as Unix milliseconds.
    pub started_at_unix_ms: i64,
    /// Manifest-relative path to the append-only raw event log.
    pub raw_event_log: String,
    /// Manifest-relative path to the raw payload directory.
    pub payloads_dir: String,
}

impl TraceBundleManifest {
    /// Builds a manifest that uses the standard local bundle layout.
    pub(crate) fn new(
        trace_id: String,
        rollout_id: String,
        root_thread_id: AgentThreadId,
        started_at_unix_ms: i64,
    ) -> Self {
        Self {
            schema_version: TRACE_MANIFEST_SCHEMA_VERSION,
            trace_id,
            rollout_id,
            root_thread_id,
            started_at_unix_ms,
            raw_event_log: RAW_EVENT_LOG_FILE_NAME.to_string(),
            payloads_dir: PAYLOADS_DIR_NAME.to_string(),
        }
    }
}

/// Reads a trace bundle manifest without replaying its event log.
pub fn read_bundle_manifest(bundle_dir: impl AsRef<Path>) -> Result<TraceBundleManifest> {
    let manifest_path = bundle_dir.as_ref().join(MANIFEST_FILE_NAME);
    let file = File::open(&manifest_path)
        .with_context(|| format!("open trace bundle manifest {}", manifest_path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("parse trace bundle manifest {}", manifest_path.display()))
}

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
