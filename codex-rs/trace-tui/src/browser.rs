use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

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
use crate::TraceViewOptions;
use crate::TraceVisualRenderer;
use crate::jobs::DetailRenderJob;
use crate::jobs::DetailRenderKey;
use crate::jobs::SearchJob;

mod detail;
mod search;

#[derive(Debug, Clone)]
struct NavigationFrame {
    container: Option<TraceNodeLocator>,
    selected: Option<TraceNodeLocator>,
    viewport: usize,
}

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
    pub(crate) selected: usize,
    pub(crate) scope: SearchScope,
    pub(crate) loading: bool,
    pub(crate) completed_query: Option<String>,
}

struct DetailCache {
    key: DetailRenderKey,
    truncated: bool,
    lines: Vec<Line<'static>>,
}

struct PendingDetail {
    key: DetailRenderKey,
    generation: u64,
}

/// Locator-based state for one single-depth trace list or full-screen record.
pub(crate) struct BrowserState {
    trace: Arc<SessionTrace>,
    index: Arc<TraceIndex>,
    container: Option<TraceNodeLocator>,
    stack: Vec<NavigationFrame>,
    rows: Vec<usize>,
    hidden_rows: usize,
    selected: usize,
    pub(crate) viewport: usize,
    pub(crate) page_size: usize,
    pub(crate) detail_open: bool,
    pub(crate) detail_scroll: usize,
    pub(crate) detail_page_size: usize,
    pub(crate) content_mode: ContentMode,
    pub(crate) search: Option<SearchState>,
    last_search: Option<SearchState>,
    search_generation: u64,
    pending_search_generation: Option<u64>,
    queued_search_job: Option<SearchJob>,
    pub(crate) filter_open: bool,
    pub(crate) filter_index: usize,
    pub(crate) help_open: bool,
    pub(crate) omitted_columns: usize,
    visible_classes: BTreeSet<TraceRecordClass>,
    temporary_reveal: Option<TraceNodeLocator>,
    pub(crate) pending_g: bool,
    payloads: BTreeMap<String, PayloadState>,
    payload_generation: u64,
    detail_cache: Option<DetailCache>,
    pending_detail: Option<PendingDetail>,
    queued_detail_job: Option<DetailRenderJob>,
    detail_generation: u64,
    renderer: Arc<dyn TraceVisualRenderer>,
    options: TraceViewOptions,
}

impl std::fmt::Debug for BrowserState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BrowserState")
            .field("container", &self.container)
            .field("rows", &self.rows.len())
            .field("selected", &self.selected)
            .field("detail_open", &self.detail_open)
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
        )
    }

    pub(crate) fn with_visuals(
        trace: SessionTrace,
        options: TraceViewOptions,
        renderer: Arc<dyn TraceVisualRenderer>,
    ) -> Self {
        let index = Arc::new(TraceIndex::new(&trace));
        let trace = Arc::new(trace);
        let container = index
            .root_nodes(&trace)
            .find(|node| node.locator.kind == TraceNodeKind::Session)
            .or_else(|| index.root_nodes(&trace).next())
            .map(|node| node.locator.clone());
        let mut state = Self {
            trace,
            index,
            container,
            stack: Vec::new(),
            rows: Vec::new(),
            hidden_rows: 0,
            selected: 0,
            viewport: 0,
            page_size: 20,
            detail_open: false,
            detail_scroll: 0,
            detail_page_size: 20,
            content_mode: ContentMode::Rendered,
            search: None,
            last_search: None,
            search_generation: 0,
            pending_search_generation: None,
            queued_search_job: None,
            filter_open: false,
            filter_index: 0,
            help_open: false,
            omitted_columns: 0,
            visible_classes: TraceRecordClass::ALL.into_iter().collect(),
            temporary_reveal: None,
            pending_g: false,
            payloads: BTreeMap::new(),
            payload_generation: 0,
            detail_cache: None,
            pending_detail: None,
            queued_detail_job: None,
            detail_generation: 0,
            renderer,
            options,
        };
        state.rebuild_rows(/*preferred*/ None);
        state
    }

    pub(crate) fn options(&self) -> &TraceViewOptions {
        &self.options
    }

    /// Returns the stable root-session identity owned by this browser.
    pub(crate) fn session_id(&self) -> &str {
        &self.trace.summary.session_id
    }

    pub(crate) fn renderer(&self) -> &dyn TraceVisualRenderer {
        self.renderer.as_ref()
    }

    pub(crate) fn breadcrumb(&self) -> Vec<&str> {
        let mut labels = Vec::new();
        let mut current = self.container.as_ref();
        let mut visited = BTreeSet::new();
        while let Some(locator) = current {
            if !visited.insert(locator.clone()) {
                break;
            }
            let Some(node) = self.index.node(&self.trace, locator) else {
                break;
            };
            labels.push(node.label.as_str());
            current = node.parent.as_ref();
        }
        labels.reverse();
        labels
    }

    pub(crate) fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn rows_window(&self, start: usize, len: usize) -> Vec<&TraceNode> {
        self.rows
            .iter()
            .skip(start)
            .take(len)
            .filter_map(|position| self.trace.nodes.get(*position))
            .collect()
    }

    pub(crate) fn selected_node(&self) -> Option<&TraceNode> {
        self.rows
            .get(self.selected)
            .and_then(|position| self.trace.nodes.get(*position))
    }

    pub(crate) fn move_vertical(&mut self, delta: isize) {
        self.selected = move_index(self.selected, delta, self.rows.len());
        self.finish_list_move();
    }

    pub(crate) fn page(&mut self, delta: isize, page_size: usize) {
        let amount = isize::try_from(page_size.max(1)).unwrap_or(isize::MAX);
        self.move_vertical(delta.saturating_mul(amount));
    }

    pub(crate) fn first(&mut self) {
        self.selected = 0;
        self.finish_list_move();
    }

    pub(crate) fn last(&mut self) {
        self.selected = self.rows.len().saturating_sub(1);
        self.finish_list_move();
    }

    pub(crate) fn enter_selected(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        let locator = node.locator.clone();
        if self.index.children(&self.trace, &locator).next().is_some() {
            if self.ancestor_contains(&locator) {
                return;
            }
            let selected = self.selected_node().map(|node| node.locator.clone());
            self.stack.push(NavigationFrame {
                container: self.container.clone(),
                selected,
                viewport: self.viewport,
            });
            self.container = Some(locator);
            self.viewport = 0;
            self.rebuild_rows(/*preferred*/ None);
        } else {
            self.open_detail();
        }
    }

    pub(crate) fn back(&mut self) -> bool {
        if self.detail_open {
            self.detail_open = false;
            self.detail_scroll = 0;
            return true;
        }
        let Some(frame) = self.stack.pop() else {
            return false;
        };
        self.container = frame.container;
        self.viewport = frame.viewport;
        self.rebuild_rows(frame.selected.as_ref());
        true
    }

    fn reveal(&mut self, locator: TraceNodeLocator) {
        let Some(node) = self.index.node(&self.trace, &locator) else {
            return;
        };
        let parent = node.parent.clone();
        self.temporary_reveal = None;
        if !self.visible_classes.contains(&node.presentation.class) {
            self.temporary_reveal = Some(locator.clone());
        }
        let ancestors = self.ancestor_path(parent.as_ref());
        self.stack = ancestors
            .windows(2)
            .map(|window| NavigationFrame {
                container: Some(window[0].clone()),
                selected: Some(window[1].clone()),
                viewport: 0,
            })
            .collect();
        self.container = parent;
        self.viewport = 0;
        self.rebuild_rows(Some(&locator));
    }

    fn rebuild_rows(&mut self, preferred: Option<&TraceNodeLocator>) {
        self.rows = match &self.container {
            Some(container) => self.index.child_positions(&self.trace, container).collect(),
            None => self.index.root_positions(&self.trace).collect(),
        };
        let mut hidden_rows = 0;
        if self.visible_classes.len() != TraceRecordClass::ALL.len()
            || self.temporary_reveal.is_some()
        {
            self.rows.retain(|position| {
                let visible = self.trace.nodes.get(*position).is_some_and(|node| {
                    self.visible_classes.contains(&node.presentation.class)
                        || self.temporary_reveal.as_ref() == Some(&node.locator)
                });
                hidden_rows += usize::from(!visible);
                visible
            });
        }
        self.hidden_rows = hidden_rows;
        self.selected = preferred
            .and_then(|locator| {
                self.rows.iter().position(|position| {
                    self.trace
                        .nodes
                        .get(*position)
                        .is_some_and(|node| node.locator == *locator)
                })
            })
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
        self.invalidate_detail();
    }

    fn ancestor_contains(&self, locator: &TraceNodeLocator) -> bool {
        self.container.as_ref() == Some(locator)
            || self
                .stack
                .iter()
                .any(|frame| frame.container.as_ref() == Some(locator))
    }

    fn ancestor_path(&self, locator: Option<&TraceNodeLocator>) -> Vec<TraceNodeLocator> {
        let mut path = Vec::new();
        let mut current = locator;
        let mut visited = BTreeSet::new();
        while let Some(locator) = current {
            if !visited.insert(locator.clone()) {
                break;
            }
            path.push(locator.clone());
            current = self
                .index
                .node(&self.trace, locator)
                .and_then(|node| node.parent.as_ref());
        }
        path.reverse();
        path
    }

    fn finish_list_move(&mut self) {
        let preferred = self.selected_node().map(|node| node.locator.clone());
        if self.temporary_reveal.take().is_some() {
            self.rebuild_rows(preferred.as_ref());
        }
        self.detail_scroll = 0;
    }
}

fn move_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn move_wrapped(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    current.wrapping_add_signed(delta) % len
}
