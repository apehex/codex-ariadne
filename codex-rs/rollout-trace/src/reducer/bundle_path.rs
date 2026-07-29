//! Shared containment checks for manifest and payload file references.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

/// Resolves a bundle-relative regular file without following an escape outside the root.
pub(super) fn resolve_contained_file(
    bundle_dir: &Path,
    relative: &Path,
    subject: &str,
) -> Result<PathBuf> {
    let bundle_root = std::fs::canonicalize(bundle_dir)
        .with_context(|| format!("canonicalize trace bundle {}", bundle_dir.display()))?;
    if relative.is_absolute() {
        bail!("{subject} path must be bundle-relative");
    }
    let candidate = std::fs::canonicalize(bundle_root.join(relative))
        .with_context(|| format!("resolve {subject} {}", relative.display()))?;
    if !candidate.starts_with(&bundle_root) {
        bail!("{subject} path escapes trace bundle");
    }
    if !std::fs::metadata(&candidate)
        .with_context(|| format!("inspect {subject} {}", candidate.display()))?
        .is_file()
    {
        bail!("{subject} path is not a regular file");
    }
    Ok(candidate)
}
