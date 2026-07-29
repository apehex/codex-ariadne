//! Contained semantic payload reads used while reducing rich trace events.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;

use super::bundle_path::resolve_contained_file;
use crate::payload::RawPayloadRef;

/// Maximum amount of a semantic payload that the reducer may read.
#[derive(Debug, Clone, Copy)]
pub(super) enum SemanticPayloadReadLimit {
    /// Preserve the historical strict-replay behavior.
    Unbounded,
    /// Reject payloads larger than the supplied byte count.
    Bytes(usize),
}

/// Resolves and reads semantic payloads without allowing bundle escapes.
#[derive(Debug)]
pub(super) struct SemanticPayloadReader {
    bundle_root: PathBuf,
    limit: SemanticPayloadReadLimit,
}

impl SemanticPayloadReader {
    /// Creates a reader rooted at one trace bundle.
    pub(super) fn new(bundle_root: PathBuf, limit: SemanticPayloadReadLimit) -> Self {
        Self { bundle_root, limit }
    }

    /// Reads and parses one contained JSON payload.
    pub(super) fn read_json(&self, payload: &RawPayloadRef) -> Result<Value> {
        let payload_path = resolve_payload_path(&self.bundle_root, payload)?;
        let file = File::open(&payload_path)
            .with_context(|| format!("open payload {}", payload.raw_payload_id))?;
        match self.limit {
            SemanticPayloadReadLimit::Unbounded => serde_json::from_reader(file)
                .with_context(|| format!("parse payload {}", payload.raw_payload_id)),
            SemanticPayloadReadLimit::Bytes(limit) => read_bounded_json(file, payload, limit),
        }
    }
}

/// Resolves a payload path and rejects absolute, escaping, or non-file targets.
fn resolve_payload_path(bundle_dir: &Path, payload: &RawPayloadRef) -> Result<PathBuf> {
    let relative = Path::new(&payload.path);
    resolve_contained_file(
        bundle_dir,
        relative,
        &format!("payload {}", payload.raw_payload_id),
    )
}

/// Reads at most `limit + 1` bytes so oversize detection is allocation-bounded.
fn read_bounded_json(file: File, payload: &RawPayloadRef, limit: usize) -> Result<Value> {
    let read_limit = limit.saturating_add(1);
    let mut bytes = Vec::with_capacity(read_limit.min(64 * 1024));
    file.take(read_limit as u64)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read payload {}", payload.raw_payload_id))?;
    if bytes.len() > limit {
        bail!(
            "payload {} exceeds {limit} byte semantic replay limit",
            payload.raw_payload_id
        );
    }
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse payload {}", payload.raw_payload_id))
}

#[cfg(test)]
#[path = "semantic_payload_tests.rs"]
mod tests;
