//! Bounded, deterministic filesystem discovery for trace sources.

use std::path::Path;
use std::path::PathBuf;

use crate::TraceDiagnostic;

pub(super) async fn files(
    root: &Path,
    predicate: fn(&Path) -> bool,
    max_files: usize,
    diagnostics: &mut Vec<TraceDiagnostic>,
) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    let max_entries = max_files.saturating_mul(16).max(1_024);
    let mut inspected_entries = 0_usize;
    while let Some(directory) = pending.pop() {
        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(path_diagnostic(
                    &directory,
                    format!("cannot read directory: {error}"),
                ));
                continue;
            }
        };
        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    inspected_entries += 1;
                    if inspected_entries > max_entries {
                        diagnostics.push(path_diagnostic(
                            root,
                            format!(
                                "file discovery stopped after inspecting {max_entries} entries"
                            ),
                        ));
                        files.sort();
                        return files;
                    }
                    match entry.file_type().await {
                        Ok(file_type) if file_type.is_dir() => pending.push(entry.path()),
                        Ok(file_type) if file_type.is_file() && predicate(&entry.path()) => {
                            files.push(entry.path());
                            if files.len() >= max_files {
                                diagnostics.push(path_diagnostic(
                                    root,
                                    format!("file discovery stopped at {max_files} matches"),
                                ));
                                files.sort();
                                return files;
                            }
                        }
                        Ok(_) => {}
                        Err(error) => diagnostics.push(path_diagnostic(
                            &entry.path(),
                            format!("cannot inspect directory entry: {error}"),
                        )),
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    diagnostics.push(path_diagnostic(
                        &directory,
                        format!("cannot enumerate directory: {error}"),
                    ));
                    break;
                }
            }
        }
    }
    files.sort();
    files
}

pub(super) fn is_rollout_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name.starts_with("rollout-") && (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst"))
}

pub(super) fn is_bundle_manifest(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "manifest.json")
}

fn path_diagnostic(path: &Path, message: String) -> TraceDiagnostic {
    TraceDiagnostic::unavailable_at(path, message)
}
