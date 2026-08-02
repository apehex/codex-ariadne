//! Reversible single-depth navigation over structural and presentation indexes.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use codex_trace::PresentationIndex;
use codex_trace::PresentationScope;
use codex_trace::SanitizedPayload;
use codex_trace::SearchHit;
use codex_trace::SessionTrace;
use codex_trace::TraceIndex;
use codex_trace::TraceNode;
use codex_trace::TraceNodeKind;
use codex_trace::TraceNodeLocator;
use codex_trace::TraceRecordClass;
use ratatui::text::Line;

use crate::ContentMode;
#[cfg(test)]
use crate::PlainTraceVisualRenderer;
use crate::TraceLens;
use crate::TraceViewOptions;
use crate::TraceVisualRenderer;
use crate::jobs::DetailRenderJob;
use crate::jobs::DetailRenderKey;
use crate::jobs::SearchJob;
use crate::request::BrowserEpoch;
use crate::request::LatestRequest;
use crate::selection::Selection;

use self::horizontal::HorizontalState;
use self::location::BrowserLocation;
use self::location::NavigationFrame;
use self::rows::BrowserRow;

mod detail;
mod display;
pub(crate) mod horizontal;
mod location;
mod navigation;
mod projection;
pub(crate) mod rows;
mod search;
mod structured;

#[derive(Debug)]
enum PayloadState {
    Loading,
    Loaded(Arc<SanitizedPayload>),
    Failed(String),
}

/// Set of records considered by a submitted semantic search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchScope {
    Visible,
    All,
}

/// Editable search overlay state and the most recently completed result set.
#[derive(Debug)]
pub(crate) struct SearchState {
    pub(crate) query: String,
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) selection: Selection,
    pub(crate) scope: SearchScope,
    pub(crate) loading: bool,
    pub(crate) completed_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchRequestKey {
    query: String,
    scope: SearchScope,
    lens: TraceLens,
    show_hidden_groups: bool,
}

struct DetailCache {
    key: DetailRenderKey,
    truncated: bool,
    lines: Vec<Line<'static>>,
    maximum_width: usize,
}

/// Typed state for one single-depth trace location or full-screen record.
pub(crate) struct BrowserState {
    epoch: BrowserEpoch,
    trace: Arc<SessionTrace>,
    index: Arc<TraceIndex>,
    presentation: Arc<PresentationIndex>,
    location: BrowserLocation,
    active_scope: PresentationScope,
    active_lens: TraceLens,
    stack: Vec<NavigationFrame>,
    rows: Vec<BrowserRow>,
    hidden_rows: usize,
    selection: Selection,
    pub(crate) viewport: usize,
    pub(crate) page_size: usize,
    pub(crate) detail_scroll: usize,
    pub(crate) detail_page_size: usize,
    horizontal: HorizontalState,
    pub(crate) content_mode: ContentMode,
    pub(crate) search: Option<SearchState>,
    last_search: Option<SearchState>,
    search_requests: LatestRequest<SearchRequestKey, SearchJob>,
    pub(crate) filter_open: bool,
    pub(crate) filter_selection: Selection,
    pub(crate) help_open: bool,
    pub(crate) omitted_columns: usize,
    visible_classes: BTreeSet<TraceRecordClass>,
    show_hidden_groups: bool,
    visibility_generation: u64,
    temporary_reveal: Option<TraceNodeLocator>,
    pub(crate) pending_g: bool,
    payloads: BTreeMap<String, PayloadState>,
    payload_generation: u64,
    detail_cache: Option<DetailCache>,
    detail_requests: LatestRequest<DetailRenderKey, DetailRenderJob>,
    structured_cache: Option<structured::StructuredLayoutCache>,
    renderer: Arc<dyn TraceVisualRenderer>,
    options: TraceViewOptions,
}

impl std::fmt::Debug for BrowserState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BrowserState")
            .field("location", &self.location)
            .field("rows", &self.rows.len())
            .field("presentation_groups", &self.presentation.groups().len())
            .field("selection", &self.selection)
            .field("content_mode", &self.content_mode)
            .finish()
    }
}

impl BrowserState {
    #[cfg(test)]
    pub(crate) fn new(trace: SessionTrace) -> Self {
        Self::with_visuals(
            trace,
            TraceViewOptions::default(),
            Arc::new(PlainTraceVisualRenderer),
            BrowserEpoch::TEST,
        )
    }

    pub(crate) fn with_visuals(
        trace: SessionTrace,
        options: TraceViewOptions,
        renderer: Arc<dyn TraceVisualRenderer>,
        epoch: BrowserEpoch,
    ) -> Self {
        let index = Arc::new(TraceIndex::new(&trace));
        let presentation = Arc::new(PresentationIndex::new(&trace));
        let trace = Arc::new(trace);
        let session_root = index
            .root_nodes(&trace)
            .find(|node| node.locator.kind == TraceNodeKind::Session)
            .or_else(|| index.root_nodes(&trace).next())
            .map(|node| node.locator.clone());
        let active_scope = projection::initial_scope(
            &trace,
            &index,
            &presentation,
            session_root.as_ref(),
            &options.initial_scope,
        );
        let active_lens = options.initial_lens;
        let structural_container = (active_lens == TraceLens::Structural)
            .then(|| session_root.clone())
            .flatten();
        let location = BrowserLocation::Lens {
            lens: active_lens,
            scope: active_scope.clone(),
            structural_container,
        };
        let mut state = Self {
            epoch,
            trace,
            index,
            presentation,
            location,
            active_scope,
            active_lens,
            stack: Vec::new(),
            rows: Vec::new(),
            hidden_rows: 0,
            selection: Selection::default(),
            viewport: 0,
            page_size: 20,
            detail_scroll: 0,
            detail_page_size: 20,
            horizontal: HorizontalState::default(),
            content_mode: ContentMode::Rendered,
            search: None,
            last_search: None,
            search_requests: LatestRequest::new(epoch),
            filter_open: false,
            filter_selection: Selection::default(),
            help_open: false,
            omitted_columns: 0,
            visible_classes: TraceRecordClass::ALL.into_iter().collect(),
            show_hidden_groups: false,
            visibility_generation: 0,
            temporary_reveal: None,
            pending_g: false,
            payloads: BTreeMap::new(),
            payload_generation: 0,
            detail_cache: None,
            detail_requests: LatestRequest::new(epoch),
            structured_cache: None,
            renderer,
            options,
        };
        state.rebuild_rows(/*preferred*/ None);
        state
    }

    pub(crate) fn options(&self) -> &TraceViewOptions {
        &self.options
    }

    pub(crate) fn epoch(&self) -> BrowserEpoch {
        self.epoch
    }

    pub(crate) fn renderer(&self) -> &dyn TraceVisualRenderer {
        self.renderer.as_ref()
    }

    pub(crate) fn lens(&self) -> TraceLens {
        self.active_lens
    }

    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selection.index()
    }

    pub(super) fn rows_window(&self, start: usize, len: usize) -> &[BrowserRow] {
        let end = start.saturating_add(len).min(self.rows.len());
        self.rows.get(start..end).unwrap_or_default()
    }

    pub(super) fn row_at(&self, index: usize) -> Option<&BrowserRow> {
        self.rows.get(index)
    }

    pub(crate) fn selected_node(&self) -> Option<&TraceNode> {
        self.detail_locator()
            .and_then(|locator| self.index.node(&self.trace, locator))
            .or_else(|| {
                self.rows
                    .get(self.selection.index())
                    .and_then(|row| self.node_for_row(row))
            })
    }

    pub(super) fn detail_locator(&self) -> Option<&TraceNodeLocator> {
        match &self.location {
            BrowserLocation::Detail(locator) | BrowserLocation::Structured { locator, .. } => {
                Some(locator)
            }
            BrowserLocation::Lens { .. } | BrowserLocation::Group(_) => None,
        }
    }
}
