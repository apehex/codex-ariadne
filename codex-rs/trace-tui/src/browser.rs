use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::Result;
use codex_trace::RawPayloadHandle;
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
use crate::jobs::DetailRenderResult;
use crate::jobs::SearchJob;
use crate::jobs::SearchResult;

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

    pub(crate) fn open_detail(&mut self) {
        if self.selected_node().is_some() {
            self.detail_open = true;
            self.detail_scroll = 0;
            self.content_mode = ContentMode::Rendered;
            self.invalidate_detail();
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

    pub(crate) fn cycle_content_mode(&mut self) {
        self.content_mode = self.content_mode.next();
        self.detail_scroll = 0;
        self.invalidate_detail();
    }

    pub(crate) fn scroll_detail(&mut self, delta: isize) {
        self.detail_scroll = self.detail_scroll.saturating_add_signed(delta);
    }

    pub(crate) fn detail_lines(&mut self, width: usize) -> &[Line<'static>] {
        let Some(node) = self.selected_node().cloned() else {
            return &[];
        };
        let key = DetailRenderKey {
            locator: node.locator.clone(),
            width,
            mode: self.content_mode,
            payload_generation: self.payload_generation,
        };
        if self
            .detail_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key)
        {
            return self
                .detail_cache
                .as_ref()
                .map(|cache| cache.lines.as_slice())
                .unwrap_or_default();
        }
        if self
            .pending_detail
            .as_ref()
            .is_none_or(|pending| pending.key != key)
        {
            self.detail_cache = None;
            self.detail_generation = self.detail_generation.wrapping_add(1);
            self.pending_detail = Some(PendingDetail {
                key: key.clone(),
                generation: self.detail_generation,
            });
            self.queued_detail_job = Some(DetailRenderJob {
                key,
                generation: self.detail_generation,
                trace: Arc::clone(&self.trace),
                index: Arc::clone(&self.index),
                payload: self.payloads.get(&node.locator.id).and_then(|state| {
                    if let PayloadState::Loaded(payload) = state {
                        Some(Arc::clone(payload))
                    } else {
                        None
                    }
                }),
                cwd: self.trace.summary.cwd.clone(),
                renderer: Arc::clone(&self.renderer),
            });
        }
        &[]
    }

    pub(crate) fn selected_is_raw_payload(&self) -> bool {
        self.selected_node()
            .is_some_and(|node| node.locator.kind == TraceNodeKind::RawPayload)
    }

    pub(crate) fn detail_truncated(&self) -> bool {
        self.detail_cache
            .as_ref()
            .is_some_and(|cache| cache.truncated)
    }

    pub(crate) fn take_detail_render_job(&mut self) -> Option<DetailRenderJob> {
        self.queued_detail_job.take()
    }

    pub(crate) fn install_detail_render(&mut self, result: DetailRenderResult) {
        let matches_pending = self.pending_detail.as_ref().is_some_and(|pending| {
            pending.generation == result.generation && pending.key == result.key
        });
        if !matches_pending {
            return;
        }
        self.pending_detail = None;
        let is_current = self
            .selected_node()
            .is_some_and(|node| node.locator == result.key.locator)
            && self.content_mode == result.key.mode
            && self.payload_generation == result.key.payload_generation;
        if !is_current {
            return;
        }
        self.detail_cache = Some(DetailCache {
            key: result.key,
            truncated: result.truncated,
            lines: result.lines,
        });
    }

    pub(crate) fn selected_payload_request(&mut self) -> Option<(String, RawPayloadHandle)> {
        let node = self.selected_node()?;
        if node.locator.kind != TraceNodeKind::RawPayload {
            return None;
        }
        let id = node.locator.id.clone();
        if self.payloads.contains_key(&id) {
            return None;
        }
        let handle = self.trace.raw_payload(&id)?;
        self.payloads.insert(id.clone(), PayloadState::Loading);
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.invalidate_detail();
        Some((id, handle))
    }

    pub(crate) fn install_payload(&mut self, id: String, result: Result<SanitizedPayload>) {
        let state = match result {
            Ok(payload) => PayloadState::Loaded(Arc::new(payload)),
            Err(error) => PayloadState::Failed(format!("{error:#}")),
        };
        self.payloads.insert(id, state);
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.invalidate_detail();
    }

    pub(crate) fn payload_notice(&self) -> Option<&str> {
        let node = self.selected_node()?;
        match self.payloads.get(&node.locator.id) {
            Some(PayloadState::Loading) => Some("loading exact raw artifact…"),
            Some(PayloadState::Failed(message)) => Some(message),
            Some(PayloadState::Loaded(_)) | None => None,
        }
    }

    pub(crate) fn begin_search(&mut self, scope: SearchScope) {
        self.invalidate_search_job();
        self.search = Some(SearchState {
            query: String::new(),
            hits: Vec::new(),
            selected: 0,
            scope,
            loading: false,
            completed_query: None,
        });
    }

    pub(crate) fn cancel_search(&mut self) {
        self.search = None;
        self.invalidate_search_job();
    }

    pub(crate) fn search_push(&mut self, ch: char) {
        if let Some(search) = &mut self.search {
            search.query.push(ch);
            search.hits.clear();
            search.selected = 0;
            search.loading = false;
            search.completed_query = None;
        }
        self.invalidate_search_job();
    }

    pub(crate) fn search_pop(&mut self) {
        if let Some(search) = &mut self.search {
            search.query.pop();
            search.hits.clear();
            search.selected = 0;
            search.loading = false;
            search.completed_query = None;
        }
        self.invalidate_search_job();
    }

    pub(crate) fn move_search(&mut self, delta: isize) {
        if let Some(search) = &mut self.search {
            search.selected = move_index(search.selected, delta, search.hits.len());
        }
    }

    pub(crate) fn accept_search(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        if search.loading {
            return;
        }
        let query = search.query.clone();
        let scope = search.scope;
        let needs_search = search.completed_query.as_deref() != Some(query.as_str());
        if needs_search {
            if query.trim().is_empty() {
                return;
            }
            self.search_generation = self.search_generation.wrapping_add(1);
            let generation = self.search_generation;
            if let Some(search) = &mut self.search {
                search.loading = true;
                search.hits.clear();
            }
            self.pending_search_generation = Some(generation);
            self.queued_search_job = Some(SearchJob {
                generation,
                query,
                scope,
                trace: Arc::clone(&self.trace),
                index: Arc::clone(&self.index),
                visible_classes: self.visible_classes.clone(),
            });
            return;
        }
        let Some(search) = self.search.take() else {
            return;
        };
        let locator = search
            .hits
            .get(search.selected)
            .map(|hit| hit.locator.clone());
        self.last_search = Some(search);
        if let Some(locator) = locator {
            self.reveal(locator);
        }
    }

    pub(crate) fn take_search_job(&mut self) -> Option<SearchJob> {
        self.queued_search_job.take()
    }

    pub(crate) fn install_search(&mut self, result: SearchResult) {
        if self.pending_search_generation != Some(result.generation) {
            return;
        }
        self.pending_search_generation = None;
        let Some(search) = &mut self.search else {
            return;
        };
        if search.query != result.query || search.scope != result.scope {
            return;
        }
        search.hits = result.hits;
        search.selected = 0;
        search.loading = false;
        search.completed_query = Some(result.query);
    }

    pub(crate) fn jump_search(&mut self, delta: isize) {
        let locator = self.last_search.as_mut().and_then(|search| {
            search.selected = move_wrapped(search.selected, delta, search.hits.len());
            search
                .hits
                .get(search.selected)
                .map(|hit| hit.locator.clone())
        });
        if let Some(locator) = locator {
            self.reveal(locator);
        }
    }

    pub(crate) fn filter_classes() -> &'static [TraceRecordClass] {
        &TraceRecordClass::ALL
    }

    pub(crate) fn class_visible(&self, class: TraceRecordClass) -> bool {
        self.visible_classes.contains(&class)
    }

    pub(crate) fn toggle_filter_class(&mut self) {
        let Some(class) = Self::filter_classes().get(self.filter_index).copied() else {
            return;
        };
        let preferred = self.selected_node().map(|node| node.locator.clone());
        if !self.visible_classes.remove(&class) {
            self.visible_classes.insert(class);
        }
        self.temporary_reveal = None;
        self.invalidate_visible_search();
        self.rebuild_rows(preferred.as_ref());
    }

    pub(crate) fn move_filter(&mut self, delta: isize) {
        self.filter_index = move_index(self.filter_index, delta, Self::filter_classes().len());
    }

    pub(crate) fn apply_filter(&mut self) {
        let preferred = self.selected_node().map(|node| node.locator.clone());
        self.rebuild_rows(preferred.as_ref());
        self.filter_open = false;
    }

    pub(crate) fn reset_filter(&mut self) {
        self.visible_classes = TraceRecordClass::ALL.into_iter().collect();
        self.temporary_reveal = None;
        self.invalidate_visible_search();
        self.apply_filter();
    }

    pub(crate) fn hidden_count(&self) -> usize {
        self.hidden_rows
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
        self.rows.retain(|position| {
            let visible = self.trace.nodes.get(*position).is_some_and(|node| {
                self.visible_classes.contains(&node.presentation.class)
                    || self.temporary_reveal.as_ref() == Some(&node.locator)
            });
            hidden_rows += usize::from(!visible);
            visible
        });
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

    fn invalidate_detail(&mut self) {
        self.detail_cache = None;
        self.pending_detail = None;
        self.queued_detail_job = None;
    }

    fn invalidate_search_job(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        self.pending_search_generation = None;
        self.queued_search_job = None;
    }

    fn invalidate_visible_search(&mut self) {
        if self
            .last_search
            .as_ref()
            .is_some_and(|search| search.scope == SearchScope::Visible)
        {
            self.last_search = None;
        }
        let invalidate_active = self
            .search
            .as_ref()
            .is_some_and(|search| search.scope == SearchScope::Visible);
        if invalidate_active && let Some(search) = &mut self.search {
            search.hits.clear();
            search.loading = false;
            search.completed_query = None;
        }
        if invalidate_active {
            self.invalidate_search_job();
        }
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
