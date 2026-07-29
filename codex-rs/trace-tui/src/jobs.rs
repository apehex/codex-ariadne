//! Generation-tagged whole-trace search and semantic detail rendering jobs.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

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
use crate::TraceRenderRequest;
use crate::TraceVisualRenderer;
use crate::browser::SearchScope;

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
    pub(crate) generation: u64,
    pub(crate) trace: Arc<SessionTrace>,
    pub(crate) index: Arc<TraceIndex>,
    pub(crate) payload: Option<Arc<SanitizedPayload>>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) renderer: Arc<dyn TraceVisualRenderer>,
}

/// Completed detail output that can be rejected when its generation is stale.
pub(crate) struct DetailRenderResult {
    pub(crate) key: DetailRenderKey,
    pub(crate) generation: u64,
    pub(crate) truncated: bool,
    pub(crate) lines: Vec<Line<'static>>,
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
        DetailRenderResult {
            key: self.key,
            generation: self.generation,
            truncated: document.truncated,
            lines,
        }
    }
}

/// Owned background search over one immutable trace and filter snapshot.
pub(crate) struct SearchJob {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) scope: SearchScope,
    pub(crate) trace: Arc<SessionTrace>,
    pub(crate) index: Arc<TraceIndex>,
    pub(crate) visible_classes: BTreeSet<TraceRecordClass>,
}

/// Attributed hits tagged with the submitted query generation.
pub(crate) struct SearchResult {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) scope: SearchScope,
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
                        .is_some_and(|node| self.visible_classes.contains(&node.presentation.class))
            })
            .collect();
        SearchResult {
            generation: self.generation,
            query: self.query,
            scope: self.scope,
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
            let text = serde_json::to_string_pretty(&node.detail)
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
