use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_rollout_trace::RawPayloadRef;
use tokio::io::AsyncReadExt;

/// Bundle root and reference needed for an on-demand payload read.
#[derive(Debug, Clone)]
pub(crate) struct BundlePayload {
    pub(crate) bundle_root: PathBuf,
    pub(crate) reference: RawPayloadRef,
}

/// Maximum raw bytes to read before returning a truncated display payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadReadLimit(usize);

impl PayloadReadLimit {
    /// Default one-megabyte raw display window.
    pub const DEFAULT: Self = Self(1024 * 1024);

    /// Creates an explicit maximum byte window.
    pub fn new(bytes: usize) -> Self {
        Self(bytes)
    }

    /// Returns the configured maximum byte count.
    pub fn bytes(self) -> usize {
        self.0
    }
}

/// Terminal-safe, lossy UTF-8 view of a raw payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedPayload {
    /// UTF-8 lossy text with unsafe terminal controls removed.
    pub text: String,
    /// Whether the source exceeded the requested byte window.
    pub truncated: bool,
    /// Number of source bytes retained before text sanitization.
    pub original_bytes_read: usize,
}

/// Containment-enforcing lazy reader for one rich trace bundle.
#[derive(Debug, Clone)]
pub struct SafePayloadReader {
    bundle_root: PathBuf,
}

impl SafePayloadReader {
    /// Creates a reader contained beneath one bundle root.
    pub fn new(bundle_root: PathBuf) -> Self {
        Self { bundle_root }
    }

    /// Reads one referenced regular file through a bounded display window.
    pub async fn read(
        &self,
        reference: &RawPayloadRef,
        limit: PayloadReadLimit,
    ) -> Result<SanitizedPayload> {
        let root = tokio::fs::canonicalize(&self.bundle_root)
            .await
            .with_context(|| format!("canonicalize trace bundle {}", self.bundle_root.display()))?;
        let relative = Path::new(&reference.path);
        if relative.is_absolute() {
            bail!("raw payload path must be bundle-relative");
        }
        let candidate = tokio::fs::canonicalize(root.join(relative))
            .await
            .with_context(|| format!("open raw payload {}", reference.path))?;
        if !candidate.starts_with(&root) {
            bail!("raw payload path escapes trace bundle");
        }
        let metadata = tokio::fs::metadata(&candidate).await?;
        if !metadata.file_type().is_file() {
            bail!("raw payload path is not a regular file");
        }

        let file = tokio::fs::File::open(&candidate).await?;
        let read_cap = limit.bytes().saturating_add(1);
        let mut bytes = Vec::with_capacity(read_cap.min(64 * 1024));
        file.take(read_cap as u64).read_to_end(&mut bytes).await?;
        let truncated = bytes.len() > limit.bytes();
        if truncated {
            bytes.truncate(limit.bytes());
        }
        let original_bytes_read = bytes.len() + usize::from(truncated);
        let text = sanitize_terminal_text(&String::from_utf8_lossy(&bytes));
        Ok(SanitizedPayload {
            text,
            truncated,
            original_bytes_read,
        })
    }
}

fn sanitize_terminal_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.next_if_eq(&'[').is_some() {
            let _ = chars.find(|ch| ('@'..='~').contains(ch));
        } else if matches!(ch, '\n' | '\t') || !ch.is_control() {
            sanitized.push(ch);
        }
    }
    sanitized
}
