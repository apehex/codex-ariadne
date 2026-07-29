use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use codex_protocol::protocol::RolloutItem;
use codex_rollout::ARCHIVED_SESSIONS_SUBDIR;
use codex_rollout::SESSIONS_SUBDIR;

use crate::CatalogEntry;
use crate::EvidenceGrade;
use crate::OrdinaryThread;
use crate::RichBundle;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::TraceCapabilities;
use crate::TraceCatalog;
use crate::TraceDiagnostic;
use crate::TraceLimits;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceSourceKind;
use crate::TraceStatus;

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
        let mut threads = BTreeMap::<String, OrdinaryThread>::new();
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
                        let thread_id = line.meta.id.to_string();
                        let thread = OrdinaryThread {
                            path: path.clone(),
                            thread_id: thread_id.clone(),
                            parent_thread_id: line.meta.parent_thread_id.map(|id| id.to_string()),
                            timestamp: line.meta.timestamp,
                            cwd: line.meta.cwd,
                            model_provider: line.meta.model_provider,
                            archived: *archived,
                        };
                        if let Some(previous) = threads.insert(thread_id.clone(), thread) {
                            diagnostics.push(TraceDiagnostic {
                                locator: None,
                                path: Some(path),
                                evidence: EvidenceGrade::Unavailable,
                                message: format!(
                                    "duplicate ordinary rollout for thread {thread_id}; replaced {}",
                                    previous.path.display()
                                ),
                            });
                        }
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
        for thread in threads.values() {
            let root_id = resolve_root(thread, &threads, &mut diagnostics);
            entries
                .entry(root_id)
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
            match codex_rollout_trace::read_bundle_manifest(&path) {
                Ok(manifest) => {
                    if manifest.schema_version != codex_rollout_trace::TRACE_MANIFEST_SCHEMA_VERSION
                    {
                        diagnostics.push(TraceDiagnostic {
                            locator: None,
                            path: Some(path.clone()),
                            evidence: EvidenceGrade::Unavailable,
                            message: format!(
                                "rich trace manifest schema {} differs from supported schema {}",
                                manifest.schema_version,
                                codex_rollout_trace::TRACE_MANIFEST_SCHEMA_VERSION
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
                    if let Some(previous) = entry.rich.replace(RichBundle {
                        path: path.clone(),
                        manifest,
                    }) {
                        diagnostics.push(TraceDiagnostic {
                            locator: None,
                            path: Some(path),
                            evidence: EvidenceGrade::Conflicting,
                            message: format!(
                                "multiple rich bundles for rollout; replaced {}",
                                previous.path.display()
                            ),
                        });
                    }
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
        let mut nodes = BTreeMap::new();
        let mut diagnostics = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic_belongs_to(diagnostic, entry))
            .cloned()
            .collect::<Vec<_>>();
        let session_locator = TraceNodeLocator::new(session_id, TraceNodeKind::Session, session_id);
        let session_detail = serde_json::to_value(&summary)?;
        nodes.insert(
            session_locator.clone(),
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
        );
        for thread in &entry.ordinary {
            load_ordinary_thread(
                session_id,
                thread,
                &session_locator,
                &entry.ordinary,
                self.limits,
                &mut nodes,
                &mut diagnostics,
            )
            .await;
        }
        let mut payloads = BTreeMap::new();
        if let Some(bundle) = &entry.rich {
            let replay = codex_rollout_trace::replay_bundle_resilient_with_limits(
                &bundle.path,
                codex_rollout_trace::ReplayLimits {
                    max_events: self.limits.max_rich_events,
                    max_event_bytes: self.limits.max_rich_event_bytes,
                },
            )
            .with_context(|| format!("replay rich trace {}", bundle.path.display()))?;
            summary.thread_count = Some(replay.trace.threads.len());
            summary.status = rich_status(&replay.trace.status);
            diagnostics.extend(
                replay
                    .diagnostics
                    .into_iter()
                    .map(|diagnostic| TraceDiagnostic {
                        locator: None,
                        path: Some(bundle.path.join("trace.jsonl")),
                        evidence: EvidenceGrade::Unavailable,
                        message: diagnostic.line.map_or_else(
                            || diagnostic.message.clone(),
                            |line| format!("rich trace line {line}: {}", diagnostic.message),
                        ),
                    }),
            );
            crate::rich::project_rich(
                session_id,
                &bundle.path,
                &replay.trace,
                replay.semantically_complete,
                self.limits.max_nodes_per_session,
                &session_locator,
                &mut nodes,
                &mut payloads,
                &mut diagnostics,
            )?;
            if let Some(session) = nodes.get_mut(&session_locator) {
                session.detail = serde_json::to_value(&summary)?;
                session.presentation = crate::TraceRecordPresentation::from_detail(
                    TraceNodeKind::Session,
                    &session.detail,
                );
            }
        }
        let remaining_diagnostic_nodes = self
            .limits
            .max_nodes_per_session
            .saturating_sub(nodes.len());
        for (index, diagnostic) in diagnostics
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
            nodes.insert(
                locator.clone(),
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
            );
        }
        Ok(SessionTrace {
            summary,
            nodes: nodes.into_values().collect(),
            diagnostics,
            payloads,
        })
    }
}

async fn load_ordinary_thread(
    session_id: &str,
    thread: &OrdinaryThread,
    session_locator: &TraceNodeLocator,
    ordinary_threads: &[OrdinaryThread],
    limits: TraceLimits,
    nodes: &mut BTreeMap<TraceNodeLocator, TraceNode>,
    diagnostics: &mut Vec<TraceDiagnostic>,
) {
    let thread_locator = ordinary_thread_locator(session_id, &thread.thread_id);
    let parent = thread
        .parent_thread_id
        .as_ref()
        .filter(|_| parent_is_reachable(thread, ordinary_threads))
        .map(|id| ordinary_thread_locator(session_id, id))
        .unwrap_or_else(|| session_locator.clone());
    let thread_label = format!("thread {}", thread.thread_id);
    let thread_detail = serde_json::json!({
        "thread_id": thread.thread_id,
        "path": thread.path,
        "cwd": thread.cwd,
        "model_provider": thread.model_provider,
        "archived": thread.archived,
        "original_parent_thread_id": thread.parent_thread_id,
    });
    nodes
        .entry(thread_locator.clone())
        .or_insert_with(|| TraceNode {
            locator: thread_locator.clone(),
            parent: Some(parent),
            provenance: TraceSourceKind::Ordinary,
            evidence: EvidenceGrade::Semantic,
            timestamp: Some(thread.timestamp.clone()),
            label: thread_label.clone(),
            presentation: crate::TraceRecordPresentation::from_detail(
                TraceNodeKind::Thread,
                &thread_detail,
            ),
            detail: thread_detail,
        });

    let mut reader = match codex_rollout::open_rollout_line_reader(&thread.path).await {
        Ok(reader) => reader,
        Err(error) => {
            diagnostics.push(path_diagnostic(
                &thread.path,
                format!("cannot open rollout: {error}"),
            ));
            return;
        }
    };
    let mut line_index = 0_u64;
    loop {
        let line = match reader.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                diagnostics.push(path_diagnostic(
                    &thread.path,
                    format!("cannot read rollout line: {error}"),
                ));
                break;
            }
        };
        line_index += 1;
        if line.trim().is_empty() {
            continue;
        }
        if diagnostics.len() >= limits.max_nodes_per_session {
            break;
        }
        if line.len() > limits.max_ordinary_record_bytes {
            diagnostics.push(path_diagnostic(
                &thread.path,
                format!(
                    "rollout record at line {line_index} exceeds {} byte display limit",
                    limits.max_ordinary_record_bytes
                ),
            ));
            continue;
        }
        if nodes.len() >= limits.max_nodes_per_session {
            diagnostics.push(path_diagnostic(
                &thread.path,
                format!(
                    "session node materialization stopped at {} nodes",
                    limits.max_nodes_per_session
                ),
            ));
            break;
        }
        let value = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(path_diagnostic(
                    &thread.path,
                    format!("malformed JSON at line {line_index}: {error}"),
                ));
                continue;
            }
        };
        let typed = serde_json::from_value::<codex_protocol::protocol::RolloutLine>(value.clone());
        let (kind, label) = match typed {
            Ok(line) => ordinary_kind(&line.item),
            Err(error) => {
                diagnostics.push(path_diagnostic(
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
        let locator = TraceNodeLocator::new(
            session_id,
            kind,
            format!("ordinary:{}:{ordinal}", thread.thread_id),
        );
        nodes.insert(
            locator.clone(),
            TraceNode {
                locator,
                parent: Some(thread_locator.clone()),
                provenance: TraceSourceKind::Ordinary,
                evidence: EvidenceGrade::Semantic,
                timestamp: value
                    .get("timestamp")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                label: label.to_string(),
                presentation: crate::TraceRecordPresentation::from_detail(kind, &value),
                detail: value,
            },
        );
    }
}

fn parent_is_reachable(thread: &OrdinaryThread, threads: &[OrdinaryThread]) -> bool {
    let by_id = threads
        .iter()
        .map(|thread| (thread.thread_id.as_str(), thread))
        .collect::<BTreeMap<_, _>>();
    let mut next = thread.parent_thread_id.as_deref();
    let mut seen = BTreeSet::new();
    while let Some(id) = next {
        if !seen.insert(id) {
            return false;
        }
        let Some(parent) = by_id.get(id) else {
            return false;
        };
        next = parent.parent_thread_id.as_deref();
    }
    true
}

fn ordinary_kind(item: &RolloutItem) -> (TraceNodeKind, &'static str) {
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

fn summarize(session_id: &str, entry: &CatalogEntry) -> SessionSummary {
    let ordinary_root = entry
        .ordinary
        .iter()
        .find(|thread| thread.parent_thread_id.is_none())
        .or_else(|| entry.ordinary.first());
    let source = match (entry.ordinary.is_empty(), entry.rich.is_some()) {
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
    let rich = entry.rich.as_ref().map(|bundle| &bundle.manifest);
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

fn resolve_root(
    thread: &OrdinaryThread,
    threads: &BTreeMap<String, OrdinaryThread>,
    diagnostics: &mut Vec<TraceDiagnostic>,
) -> String {
    let mut current = thread;
    let mut seen = BTreeSet::new();
    while let Some(parent_id) = &current.parent_thread_id {
        if !seen.insert(current.thread_id.clone()) {
            let root = seen
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| thread.thread_id.clone());
            diagnostics.push(path_diagnostic(
                &thread.path,
                format!("parent cycle detected; grouped under {root}"),
            ));
            return root;
        }
        let Some(parent) = threads.get(parent_id) else {
            diagnostics.push(path_diagnostic(
                &thread.path,
                format!("parent rollout {parent_id} is missing"),
            ));
            return parent_id.clone();
        };
        current = parent;
    }
    current.thread_id.clone()
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

fn ordinary_thread_locator(session_id: &str, thread_id: &str) -> TraceNodeLocator {
    TraceNodeLocator::new(
        session_id,
        TraceNodeKind::Thread,
        format!("ordinary:{thread_id}"),
    )
}

fn diagnostic_belongs_to(diagnostic: &TraceDiagnostic, entry: &CatalogEntry) -> bool {
    let Some(path) = diagnostic.path.as_ref() else {
        return false;
    };
    entry.ordinary.iter().any(|thread| thread.path == *path)
        || entry
            .rich
            .as_ref()
            .is_some_and(|bundle| path.starts_with(&bundle.path))
}

fn diagnostic_provenance(diagnostic: &TraceDiagnostic, entry: &CatalogEntry) -> TraceSourceKind {
    if diagnostic.path.as_ref().is_some_and(|path| {
        entry
            .rich
            .as_ref()
            .is_some_and(|bundle| path.starts_with(&bundle.path))
    }) {
        TraceSourceKind::Rich
    } else {
        TraceSourceKind::Ordinary
    }
}
