use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use codex_rollout::ARCHIVED_SESSIONS_SUBDIR;
use codex_rollout::SESSIONS_SUBDIR;

use crate::EvidenceGrade;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::SiblingOrder;
use crate::TraceCapabilities;
use crate::TraceCatalog;
use crate::TraceDiagnostic;
use crate::TraceGraphBuilder;
use crate::TraceLimits;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceSourceKind;
use crate::TraceStatus;
use crate::model::CatalogEntry;
use crate::model::OrdinaryThread;
use crate::model::RichBundle;

/// Read-only entry point for discovering ordinary and rich local traces.
#[derive(Debug, Clone)]
pub struct TraceRepository {
    ordinary_roots: Vec<(PathBuf, bool)>,
    rich_roots: Vec<PathBuf>,
    rich_bundles: Vec<PathBuf>,
    limits: TraceLimits,
}

impl TraceRepository {
    /// Creates a repository over the normal and archived stores below a Codex home.
    pub fn new(codex_home: PathBuf) -> Self {
        Self {
            ordinary_roots: vec![
                (codex_home.join(SESSIONS_SUBDIR), false),
                (codex_home.join(ARCHIVED_SESSIONS_SUBDIR), true),
            ],
            rich_roots: Vec::new(),
            rich_bundles: Vec::new(),
            limits: TraceLimits::default(),
        }
    }

    /// Adds an opt-in rollout-trace bundle root.
    pub fn with_rich_root(mut self, root: PathBuf) -> Self {
        self.rich_roots.push(root);
        self
    }

    /// Adds one exact rollout-trace bundle directory.
    pub fn with_rich_bundle(mut self, bundle: PathBuf) -> Self {
        self.rich_bundles.push(bundle);
        self
    }

    /// Overrides resource limits, primarily for constrained hosts and tests.
    pub fn with_limits(mut self, limits: TraceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Discovers trace metadata without writing indexes or repairing source files.
    pub async fn discover(&self) -> TraceCatalog {
        let mut diagnostics = Vec::new();
        let mut threads = BTreeMap::<(String, String), OrdinaryThread>::new();
        let mut ordinary_observations = Vec::new();
        for (root, archived) in &self.ordinary_roots {
            for path in discover_files(
                root,
                is_rollout_file,
                self.limits.max_discovered_files_per_root,
                &mut diagnostics,
            )
            .await
            {
                match codex_rollout::read_session_meta_line(&path).await {
                    Ok(line) => {
                        let meta = line.meta;
                        let session_id = meta.session_id.to_string();
                        let thread_id = meta.id.to_string();
                        let thread = OrdinaryThread {
                            path: path.clone(),
                            session_id: session_id.clone(),
                            thread_id: thread_id.clone(),
                            parent_thread_id: meta.parent_thread_id.map(|id| id.to_string()),
                            forked_from_thread_id: meta.forked_from_id.map(|id| id.to_string()),
                            history_base: meta.history_base,
                            timestamp: meta.timestamp,
                            cwd: meta.cwd,
                            model_provider: meta.model_provider,
                            archived: *archived,
                        };
                        let identity = (session_id, thread_id.clone());
                        if let Some(previous) = threads.get(&identity)
                            && !same_ordinary_observation(previous, &thread)
                        {
                            diagnostics.push(TraceDiagnostic {
                                locator: None,
                                path: Some(path.clone()),
                                evidence: EvidenceGrade::Conflicting,
                                message: format!(
                                    "conflicting ordinary rollout for thread {thread_id}; retained separately from {}",
                                    previous.path.display()
                                ),
                            });
                        }
                        threads.entry(identity).or_insert_with(|| thread.clone());
                        ordinary_observations.push(thread);
                    }
                    Err(error) => diagnostics.push(TraceDiagnostic {
                        locator: None,
                        path: Some(path),
                        evidence: EvidenceGrade::Unavailable,
                        message: format!("cannot read rollout metadata: {error}"),
                    }),
                }
            }
        }

        let mut entries = BTreeMap::<String, CatalogEntry>::new();
        for thread in &ordinary_observations {
            entries
                .entry(thread.session_id.clone())
                .or_default()
                .ordinary
                .push(thread.clone());
        }

        let mut bundle_paths = self.rich_bundles.clone();
        for rich_root in &self.rich_roots {
            bundle_paths.extend(
                discover_files(
                    rich_root,
                    is_bundle_manifest,
                    self.limits.max_discovered_files_per_root,
                    &mut diagnostics,
                )
                .await
                .into_iter()
                .filter_map(|manifest| manifest.parent().map(Path::to_path_buf)),
            );
        }
        bundle_paths.sort();
        bundle_paths.dedup();
        for path in bundle_paths {
            match codex_rollout_trace::inspect_bundle(&path) {
                Ok(manifest) => {
                    if manifest.schema_version != codex_rollout_trace::TRACE_BUNDLE_SCHEMA_VERSION {
                        diagnostics.push(TraceDiagnostic {
                            locator: None,
                            path: Some(path.clone()),
                            evidence: EvidenceGrade::Unavailable,
                            message: format!(
                                "rich trace manifest schema {} differs from supported schema {}",
                                manifest.schema_version,
                                codex_rollout_trace::TRACE_BUNDLE_SCHEMA_VERSION
                            ),
                        });
                    }
                    let key = if entries.contains_key(&manifest.rollout_id) {
                        manifest.rollout_id.clone()
                    } else if entries.contains_key(&manifest.root_thread_id) {
                        manifest.root_thread_id.clone()
                    } else {
                        manifest.rollout_id.clone()
                    };
                    let entry = entries.entry(key).or_default();
                    if !entry.ordinary.is_empty()
                        && entry
                            .ordinary
                            .iter()
                            .all(|thread| thread.thread_id != manifest.root_thread_id)
                    {
                        diagnostics.push(TraceDiagnostic {
                            locator: None,
                            path: Some(path.clone()),
                            evidence: EvidenceGrade::Conflicting,
                            message: format!(
                                "rich root {} conflicts with ordinary rollout root",
                                manifest.root_thread_id
                            ),
                        });
                    }
                    let bundle = RichBundle {
                        path: path.clone(),
                        manifest,
                    };
                    if let Some(previous) = entry.rich.first()
                        && previous.manifest != bundle.manifest
                    {
                        diagnostics.push(TraceDiagnostic {
                            locator: None,
                            path: Some(path.clone()),
                            evidence: EvidenceGrade::Conflicting,
                            message: format!(
                                "conflicting rich bundle retained separately from {}",
                                previous.path.display()
                            ),
                        });
                    }
                    entry.rich.push(bundle);
                }
                Err(error) => diagnostics.push(TraceDiagnostic {
                    locator: None,
                    path: Some(path),
                    evidence: EvidenceGrade::Unavailable,
                    message: format!("cannot read rich trace manifest: {error:#}"),
                }),
            }
        }

        let mut sessions = entries
            .iter()
            .map(|(session_id, entry)| summarize(session_id, entry))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        TraceCatalog {
            sessions,
            diagnostics,
            entries,
            limits: self.limits,
        }
    }
}

impl TraceCatalog {
    /// Loads one root session into a normalized, searchable node tree.
    pub async fn load_session(&self, session_id: &str) -> Result<SessionTrace> {
        let entry = self
            .entries
            .get(session_id)
            .with_context(|| format!("unknown trace session {session_id}"))?;
        let mut summary = summarize(session_id, entry);
        let diagnostics = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic_belongs_to(diagnostic, entry))
            .cloned()
            .collect::<Vec<_>>();
        let mut graph = TraceGraphBuilder::new(self.limits.max_nodes_per_session, diagnostics);
        let session_locator = TraceNodeLocator::new(session_id, TraceNodeKind::Session, session_id);
        let session_detail = serde_json::to_value(&summary)?;
        graph.admit(
            TraceNode {
                locator: session_locator.clone(),
                parent: None,
                provenance: summary.source,
                evidence: EvidenceGrade::Reconstructed,
                timestamp: summary.created_at.clone(),
                label: format!("session {session_id}"),
                presentation: crate::TraceRecordPresentation::from_detail(
                    TraceNodeKind::Session,
                    &session_detail,
                ),
                detail: session_detail,
            },
            SiblingOrder::Unspecified,
            /*source_path*/ None,
        );
        if graph.is_empty() {
            let (nodes, diagnostics) = graph.finish();
            return Ok(SessionTrace {
                summary,
                nodes,
                diagnostics,
                payloads: BTreeMap::new(),
            });
        }
        for thread in &entry.ordinary {
            crate::ordinary::load_thread(
                session_id,
                thread,
                &session_locator,
                &entry.ordinary,
                self.limits,
                &mut graph,
            )
            .await;
        }
        let mut payloads = BTreeMap::new();
        for (bundle_index, bundle) in entry.rich.iter().enumerate() {
            let replay = codex_rollout_trace::replay_bundle_resilient_with_limits(
                &bundle.path,
                codex_rollout_trace::ReplayLimits::default()
                    .max_events(self.limits.max_rich_events)
                    .max_event_bytes(self.limits.max_rich_event_bytes)
                    .max_semantic_payload_bytes(self.limits.max_rich_semantic_payload_bytes),
            )
            .with_context(|| format!("replay rich trace {}", bundle.path.display()))?;
            if bundle_index == 0 {
                summary.thread_count = Some(replay.trace.threads.len());
                summary.status = rich_status(&replay.trace.status);
            }
            for diagnostic in replay.diagnostics {
                graph.record_diagnostic(TraceDiagnostic {
                    locator: None,
                    path: Some(bundle.path.join(&bundle.manifest.raw_event_log)),
                    evidence: EvidenceGrade::Unavailable,
                    message: diagnostic.line.map_or_else(
                        || diagnostic.message.clone(),
                        |line| format!("rich trace line {line}: {}", diagnostic.message),
                    ),
                });
            }
            crate::rich::project_rich(
                session_id,
                &bundle.path,
                &replay.trace,
                replay.semantically_complete,
                &session_locator,
                &mut graph,
                &mut payloads,
            )?;
        }
        if let Some(session) = graph.node_mut(&session_locator) {
            session.detail = serde_json::to_value(&summary)?;
            session.presentation = crate::TraceRecordPresentation::from_detail(
                TraceNodeKind::Session,
                &session.detail,
            );
        }
        let remaining_diagnostic_nodes = self
            .limits
            .max_nodes_per_session
            .saturating_sub(graph.len());
        let diagnostics_for_nodes = graph.diagnostics().to_vec();
        for (index, diagnostic) in diagnostics_for_nodes
            .iter()
            .take(remaining_diagnostic_nodes)
            .enumerate()
        {
            let locator = TraceNodeLocator::new(
                session_id,
                TraceNodeKind::Diagnostic,
                format!("diagnostic:{index}"),
            );
            let detail = serde_json::to_value(diagnostic)?;
            graph.admit(
                TraceNode {
                    locator,
                    parent: Some(session_locator.clone()),
                    provenance: diagnostic_provenance(diagnostic, entry),
                    evidence: diagnostic.evidence,
                    timestamp: None,
                    label: diagnostic.message.clone(),
                    presentation: crate::TraceRecordPresentation::from_detail(
                        TraceNodeKind::Diagnostic,
                        &detail,
                    ),
                    detail,
                },
                SiblingOrder::Unspecified,
                diagnostic.path.as_deref(),
            );
        }
        let (nodes, diagnostics) = graph.finish();
        Ok(SessionTrace {
            summary,
            nodes,
            diagnostics,
            payloads,
        })
    }
}

fn summarize(session_id: &str, entry: &CatalogEntry) -> SessionSummary {
    let ordinary_root = entry
        .ordinary
        .iter()
        .find(|thread| thread.thread_id == session_id)
        .or_else(|| {
            entry
                .ordinary
                .iter()
                .find(|thread| thread.parent_thread_id.is_none())
        })
        .or_else(|| entry.ordinary.first());
    let source = match (entry.ordinary.is_empty(), !entry.rich.is_empty()) {
        (false, false) => TraceSourceKind::Ordinary,
        (true, true) => TraceSourceKind::Rich,
        (false, true) => TraceSourceKind::Merged,
        (true, false) => TraceSourceKind::Ordinary,
    };
    let capabilities = match source {
        TraceSourceKind::Ordinary => TraceCapabilities::ORDINARY,
        TraceSourceKind::Rich => TraceCapabilities::RICH,
        TraceSourceKind::Merged => TraceCapabilities::ORDINARY.union(TraceCapabilities::RICH),
    };
    let rich = entry.rich.first().map(|bundle| &bundle.manifest);
    SessionSummary {
        session_id: session_id.to_string(),
        root_thread_id: rich
            .map(|trace| trace.root_thread_id.clone())
            .or_else(|| ordinary_root.map(|thread| thread.thread_id.clone()))
            .unwrap_or_else(|| session_id.to_string()),
        source,
        capabilities,
        created_at: ordinary_root
            .map(|thread| thread.timestamp.clone())
            .or_else(|| {
                rich.and_then(|manifest| {
                    DateTime::from_timestamp_millis(manifest.started_at_unix_ms)
                        .map(|timestamp| timestamp.to_rfc3339())
                })
            }),
        cwd: ordinary_root.map(|thread| thread.cwd.clone()),
        model_provider: ordinary_root.and_then(|thread| thread.model_provider.clone()),
        status: TraceStatus::Unknown,
        archived: !entry.ordinary.is_empty() && entry.ordinary.iter().all(|thread| thread.archived),
        thread_count: rich.is_none().then_some(entry.ordinary.len()),
    }
}

fn rich_status(status: &codex_rollout_trace::RolloutStatus) -> TraceStatus {
    match status {
        codex_rollout_trace::RolloutStatus::Running => TraceStatus::Running,
        codex_rollout_trace::RolloutStatus::Completed => TraceStatus::Completed,
        codex_rollout_trace::RolloutStatus::Failed => TraceStatus::Failed,
        codex_rollout_trace::RolloutStatus::Aborted => TraceStatus::Aborted,
    }
}

async fn discover_files(
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

fn is_rollout_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name.starts_with("rollout-") && (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst"))
}

fn is_bundle_manifest(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "manifest.json")
}

fn path_diagnostic(path: &Path, message: String) -> TraceDiagnostic {
    TraceDiagnostic {
        locator: None,
        path: Some(path.to_path_buf()),
        evidence: EvidenceGrade::Unavailable,
        message,
    }
}

fn diagnostic_belongs_to(diagnostic: &TraceDiagnostic, entry: &CatalogEntry) -> bool {
    let Some(path) = diagnostic.path.as_ref() else {
        return false;
    };
    entry.ordinary.iter().any(|thread| thread.path == *path)
        || entry
            .rich
            .iter()
            .any(|bundle| path.starts_with(&bundle.path))
}

fn diagnostic_provenance(diagnostic: &TraceDiagnostic, entry: &CatalogEntry) -> TraceSourceKind {
    if diagnostic.path.as_ref().is_some_and(|path| {
        entry
            .rich
            .iter()
            .any(|bundle| path.starts_with(&bundle.path))
    }) {
        TraceSourceKind::Rich
    } else {
        TraceSourceKind::Ordinary
    }
}

/// Compares ordinary trace metadata while ignoring the duplicate source path.
fn same_ordinary_observation(left: &OrdinaryThread, right: &OrdinaryThread) -> bool {
    left.session_id == right.session_id
        && left.thread_id == right.thread_id
        && left.parent_thread_id == right.parent_thread_id
        && left.forked_from_thread_id == right.forked_from_thread_id
        && left.history_base == right.history_base
        && left.timestamp == right.timestamp
        && left.cwd == right.cwd
        && left.model_provider == right.model_provider
        && left.archived == right.archived
}
