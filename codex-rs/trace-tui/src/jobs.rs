//! Generation-tagged whole-trace search and semantic detail rendering jobs.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use codex_trace::PresentationIndex;
use codex_trace::SanitizedPayload;
use codex_trace::SearchHit;
use codex_trace::SessionTrace;
use codex_trace::TraceContentDocument;
use codex_trace::TraceContentFormat;
use codex_trace::TraceIndex;
use codex_trace::TraceNode;
use codex_trace::TraceNodeKind;
use codex_trace::TraceNodeLocator;
use codex_trace::TraceRecordClass;
use ratatui::text::Line;

use crate::ContentMode;
use crate::TraceLens;
use crate::TraceRenderRequest;
use crate::TraceVisualRenderer;
use crate::browser::SearchScope;
use crate::request::RequestToken;

const STRUCTURED_DETAIL_LIMIT: usize = 64 * 1024;

/// Immutable identity of one width- and mode-dependent detail render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetailRenderKey {
    pub(crate) locator: TraceNodeLocator,
    pub(crate) width: usize,
    pub(crate) mode: ContentMode,
    pub(crate) payload_generation: u64,
}

/// Owned background work needed to prepare one detail surface.
pub(crate) struct DetailRenderJob {
    pub(crate) key: DetailRenderKey,
    pub(crate) token: RequestToken,
    pub(crate) trace: Arc<SessionTrace>,
    pub(crate) index: Arc<TraceIndex>,
    pub(crate) payload: Option<Arc<SanitizedPayload>>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) renderer: Arc<dyn TraceVisualRenderer>,
}

/// Completed detail output that can be rejected when its generation is stale.
pub(crate) struct DetailRenderResult {
    pub(crate) key: DetailRenderKey,
    pub(crate) token: RequestToken,
    pub(crate) truncated: bool,
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) maximum_width: usize,
}

impl DetailRenderJob {
    pub(crate) fn run(self) -> DetailRenderResult {
        let document = self.index.node(&self.trace, &self.key.locator).map_or_else(
            || TraceContentDocument {
                format: TraceContentFormat::Text,
                text: "record is no longer available".to_string(),
                truncated: false,
            },
            |node| detail_document(self.key.mode, node, self.payload.as_deref()),
        );
        let lines = self.renderer.render_content(TraceRenderRequest {
            document: &document,
            width: self.key.width,
            cwd: self.cwd.as_deref(),
        });
        let maximum_width = lines
            .iter()
            .map(Line::width)
            .max()
            .unwrap_or_default()
            .min(self.key.width);
        DetailRenderResult {
            key: self.key,
            token: self.token,
            truncated: document.truncated,
            lines,
            maximum_width,
        }
    }
}

/// Owned background search over one immutable trace and filter snapshot.
pub(crate) struct SearchJob {
    pub(crate) token: RequestToken,
    pub(crate) query: String,
    pub(crate) scope: SearchScope,
    pub(crate) trace: Arc<SessionTrace>,
    pub(crate) index: Arc<TraceIndex>,
    pub(crate) presentation: Arc<PresentationIndex>,
    pub(crate) visible_classes: BTreeSet<TraceRecordClass>,
    pub(crate) lens: TraceLens,
    pub(crate) show_hidden_groups: bool,
}

/// Attributed hits tagged with the submitted query generation.
pub(crate) struct SearchResult {
    pub(crate) token: RequestToken,
    pub(crate) query: String,
    pub(crate) scope: SearchScope,
    pub(crate) lens: TraceLens,
    pub(crate) show_hidden_groups: bool,
    pub(crate) hits: Vec<SearchHit>,
}

impl SearchJob {
    pub(crate) fn run(self) -> SearchResult {
        let hits = self
            .trace
            .search(&self.query)
            .into_iter()
            .filter(|hit| {
                self.scope == SearchScope::All
                    || self
                        .index
                        .node(&self.trace, &hit.locator)
                        .is_some_and(|node| {
                            if !self.visible_classes.contains(&node.presentation.class) {
                                return false;
                            }
                            if self.lens == TraceLens::Structural || self.show_hidden_groups {
                                return true;
                            }
                            self.index
                                .node_position(&self.trace, &hit.locator)
                                .and_then(|position| self.presentation.group_for_node(position))
                                .is_none_or(|group| {
                                    group.default_visibility == codex_trace::GroupVisibility::Shown
                                })
                        })
            })
            .collect();
        SearchResult {
            token: self.token,
            query: self.query,
            scope: self.scope,
            lens: self.lens,
            show_hidden_groups: self.show_hidden_groups,
            hits,
        }
    }
}

fn detail_document(
    mode: ContentMode,
    node: &TraceNode,
    payload: Option<&SanitizedPayload>,
) -> TraceContentDocument {
    match mode {
        ContentMode::Rendered => node.content_document(STRUCTURED_DETAIL_LIMIT),
        ContentMode::Text => {
            let mut document = node.content_document(STRUCTURED_DETAIL_LIMIT);
            document.format = TraceContentFormat::Text;
            document
        }
        ContentMode::Raw => {
            if node.locator.kind == TraceNodeKind::RawPayload
                && let Some(payload) = payload
            {
                let (text, display_truncated) =
                    bounded_text(payload.text.clone(), STRUCTURED_DETAIL_LIMIT);
                return TraceContentDocument {
                    format: TraceContentFormat::Text,
                    text,
                    truncated: payload.truncated || display_truncated,
                };
            }
            let text = serde_json::to_string_pretty(&sorted_json(&node.detail))
                .unwrap_or_else(|_| node.detail.to_string());
            let (text, truncated) = bounded_text(text, STRUCTURED_DETAIL_LIMIT);
            TraceContentDocument {
                format: TraceContentFormat::Json,
                text: format!("semantic normalized JSON — not an exact source artifact\n\n{text}"),
                truncated,
            }
        }
    }
}

/// Clones normalized JSON with recursively sorted object keys for cross-build display stability.
fn sorted_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            let mut sorted = serde_json::Map::new();
            for (key, value) in entries {
                sorted.insert(key.clone(), sorted_json(value));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(sorted_json).collect())
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => value.clone(),
    }
}

fn bounded_text(mut text: String, byte_limit: usize) -> (String, bool) {
    if text.len() <= byte_limit {
        return (text, false);
    }
    let mut boundary = byte_limit.min(text.len());
    while !text.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    text.truncate(boundary);
    (text, true)
}

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod tests;
