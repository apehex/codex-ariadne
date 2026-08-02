//! Reconciliation of discovered ordinary and rich source observations.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::CatalogEntry;
use super::OrdinaryThread;
use super::RichBundle;
use crate::EvidenceGrade;
use crate::TraceCatalog;
use crate::TraceDiagnostic;
use crate::TraceLimits;

#[derive(Default)]
pub(super) struct CatalogAccumulator {
    entries: BTreeMap<String, CatalogEntry>,
    ordinary_identities: BTreeMap<(String, String), OrdinaryThread>,
    diagnostics: Vec<TraceDiagnostic>,
}

impl CatalogAccumulator {
    pub(super) fn diagnostics_mut(&mut self) -> &mut Vec<TraceDiagnostic> {
        &mut self.diagnostics
    }

    pub(super) fn observe_ordinary(&mut self, thread: OrdinaryThread) {
        let identity = (thread.session_id.clone(), thread.thread_id.clone());
        if let Some(previous) = self.ordinary_identities.get(&identity)
            && !same_ordinary_observation(previous, &thread)
        {
            self.diagnostics.push(TraceDiagnostic {
                locator: None,
                path: Some(thread.path.clone()),
                evidence: EvidenceGrade::Conflicting,
                message: format!(
                    "conflicting ordinary rollout for thread {}; retained separately from {}",
                    thread.thread_id,
                    previous.path.display()
                ),
            });
        }
        self.ordinary_identities
            .entry(identity)
            .or_insert_with(|| thread.clone());
        self.entries
            .entry(thread.session_id.clone())
            .or_default()
            .ordinary
            .push(thread);
    }

    pub(super) fn observe_rich(
        &mut self,
        path: PathBuf,
        manifest: codex_rollout_trace::TraceBundleMetadata,
    ) {
        if manifest.schema_version != codex_rollout_trace::TRACE_BUNDLE_SCHEMA_VERSION {
            self.diagnostics.push(TraceDiagnostic {
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
        let key = if self.entries.contains_key(&manifest.rollout_id) {
            manifest.rollout_id.clone()
        } else if self.entries.contains_key(&manifest.root_thread_id) {
            manifest.root_thread_id.clone()
        } else {
            manifest.rollout_id.clone()
        };
        let entry = self.entries.entry(key).or_default();
        if !entry.ordinary.is_empty()
            && entry
                .ordinary
                .iter()
                .all(|thread| thread.thread_id != manifest.root_thread_id)
        {
            self.diagnostics.push(TraceDiagnostic {
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
            self.diagnostics.push(TraceDiagnostic {
                locator: None,
                path: Some(path),
                evidence: EvidenceGrade::Conflicting,
                message: format!(
                    "conflicting rich bundle retained separately from {}",
                    previous.path.display()
                ),
            });
        }
        entry.rich.push(bundle);
    }

    pub(super) fn record_diagnostic(&mut self, diagnostic: TraceDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn finish(self, limits: TraceLimits) -> TraceCatalog {
        let mut sessions = self
            .entries
            .iter()
            .map(|(session_id, entry)| super::summarize(session_id, entry))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        TraceCatalog {
            sessions,
            diagnostics: self.diagnostics,
            entries: self.entries,
            limits,
        }
    }
}

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
