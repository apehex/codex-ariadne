use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_trace::RawPayloadHandle;
use codex_trace::SanitizedPayload;
use codex_trace::SearchHit;
use codex_trace::SessionSummary;
use codex_trace::SessionTrace;
use codex_trace::TraceDiagnostic;
use codex_trace::TraceNode;
use codex_trace::TraceNodeKind;
use codex_trace::TraceNodeLocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserPane {
    Tree,
    Children,
    Inspector,
}

impl BrowserPane {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Tree => Self::Children,
            Self::Children => Self::Inspector,
            Self::Inspector => Self::Tree,
        }
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::Tree => Self::Inspector,
            Self::Children => Self::Tree,
            Self::Inspector => Self::Children,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BrowserState {
    trace: SessionTrace,
    expanded: BTreeSet<TraceNodeLocator>,
    selected: TraceNodeLocator,
    pub(crate) pane: BrowserPane,
    pub(crate) child_index: usize,
    pub(crate) inspector_scroll: usize,
    pub(crate) search: Option<SearchState>,
    last_search: Option<SearchState>,
    pub(crate) diagnostics_open: bool,
    pub(crate) diagnostic_index: usize,
    payloads: BTreeMap<String, PayloadState>,
}

#[derive(Debug)]
enum PayloadState {
    Loading,
    Loaded(SanitizedPayload),
    Failed(String),
}

#[derive(Debug)]
pub(crate) struct SearchState {
    pub(crate) query: String,
    pub(crate) hits: Vec<SearchHit>,
    pub(crate) selected: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TreeRow<'a> {
    pub(crate) node: &'a TraceNode,
    pub(crate) depth: usize,
    pub(crate) has_children: bool,
    pub(crate) expanded: bool,
}

impl BrowserState {
    pub(crate) fn new(trace: SessionTrace) -> Self {
        let selected = trace
            .nodes
            .iter()
            .find(|node| node.parent.is_none())
            .or_else(|| trace.nodes.first())
            .map(|node| node.locator.clone())
            .unwrap_or_else(|| {
                TraceNodeLocator::new(
                    &trace.summary.session_id,
                    TraceNodeKind::Session,
                    &trace.summary.session_id,
                )
            });
        let mut expanded = BTreeSet::new();
        expanded.insert(selected.clone());
        Self {
            trace,
            expanded,
            selected,
            pane: BrowserPane::Tree,
            child_index: 0,
            inspector_scroll: 0,
            search: None,
            last_search: None,
            diagnostics_open: false,
            diagnostic_index: 0,
            payloads: BTreeMap::new(),
        }
    }

    pub(crate) fn summary(&self) -> &SessionSummary {
        &self.trace.summary
    }

    pub(crate) fn visible_rows(&self) -> Vec<TreeRow<'_>> {
        let mut rows = Vec::new();
        let mut visited = BTreeSet::new();
        let mut roots = self
            .trace
            .nodes
            .iter()
            .filter(|node| node.parent.is_none())
            .collect::<Vec<_>>();
        sort_nodes(&mut roots);
        for root in roots {
            self.append_visible(root, 0, &mut visited, &mut rows);
        }
        rows
    }

    fn append_visible<'a>(
        &'a self,
        node: &'a TraceNode,
        depth: usize,
        visited: &mut BTreeSet<TraceNodeLocator>,
        rows: &mut Vec<TreeRow<'a>>,
    ) {
        if !visited.insert(node.locator.clone()) {
            return;
        }
        let mut children = self.children_of(&node.locator);
        let has_children = !children.is_empty();
        let expanded = self.expanded.contains(&node.locator);
        rows.push(TreeRow {
            node,
            depth,
            has_children,
            expanded,
        });
        if expanded {
            sort_nodes(&mut children);
            for child in children {
                self.append_visible(child, depth.saturating_add(1), visited, rows);
            }
        }
    }

    pub(crate) fn selected_node(&self) -> Option<&TraceNode> {
        self.trace
            .nodes
            .iter()
            .find(|node| node.locator == self.selected)
    }

    pub(crate) fn breadcrumb(&self) -> Vec<&str> {
        let mut labels = Vec::new();
        let mut current = Some(&self.selected);
        let mut visited = BTreeSet::new();
        while let Some(locator) = current {
            if !visited.insert(locator.clone()) {
                break;
            }
            let Some(node) = self
                .trace
                .nodes
                .iter()
                .find(|node| node.locator == *locator)
            else {
                break;
            };
            labels.push(node.label.as_str());
            current = node.parent.as_ref();
        }
        labels.reverse();
        labels
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.visible_rows()
            .iter()
            .position(|row| row.node.locator == self.selected)
            .unwrap_or(0)
    }

    pub(crate) fn children(&self) -> Vec<&TraceNode> {
        let mut children = self.children_of(&self.selected);
        sort_nodes(&mut children);
        children
    }

    fn children_of(&self, parent: &TraceNodeLocator) -> Vec<&TraceNode> {
        self.trace
            .nodes
            .iter()
            .filter(|node| node.parent.as_ref() == Some(parent))
            .collect()
    }

    pub(crate) fn move_vertical(&mut self, delta: isize) {
        match self.pane {
            BrowserPane::Tree => {
                let rows = self.visible_rows();
                if rows.is_empty() {
                    return;
                }
                let current = self.selected_index();
                let next = move_index(current, delta, rows.len());
                self.selected = rows[next].node.locator.clone();
                self.child_index = 0;
                self.inspector_scroll = 0;
            }
            BrowserPane::Children => {
                self.child_index = move_index(self.child_index, delta, self.children().len());
            }
            BrowserPane::Inspector => {
                self.inspector_scroll = move_index(self.inspector_scroll, delta, usize::MAX);
            }
        }
    }

    pub(crate) fn first(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if let Some(row) = self.visible_rows().first() {
                    self.selected = row.node.locator.clone();
                }
            }
            BrowserPane::Children => self.child_index = 0,
            BrowserPane::Inspector => self.inspector_scroll = 0,
        }
    }

    pub(crate) fn last(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if let Some(row) = self.visible_rows().last() {
                    self.selected = row.node.locator.clone();
                }
            }
            BrowserPane::Children => {
                self.child_index = self.children().len().saturating_sub(1);
            }
            BrowserPane::Inspector => {}
        }
    }

    pub(crate) fn expand_or_enter(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if !self.expanded.insert(self.selected.clone()) {
                    self.expanded.remove(&self.selected);
                }
            }
            BrowserPane::Children => {
                if let Some(child) = self.children().get(self.child_index) {
                    self.selected = child.locator.clone();
                    self.child_index = 0;
                    self.inspector_scroll = 0;
                    self.pane = BrowserPane::Tree;
                }
            }
            BrowserPane::Inspector => {}
        }
    }

    pub(crate) fn expand(&mut self) {
        self.expanded.insert(self.selected.clone());
    }

    pub(crate) fn collapse_or_parent(&mut self) {
        if self.expanded.remove(&self.selected) {
            return;
        }
        if let Some(parent) = self.selected_node().and_then(|node| node.parent.clone()) {
            self.selected = parent;
            self.child_index = 0;
            self.inspector_scroll = 0;
        }
    }

    pub(crate) fn begin_search(&mut self) {
        self.search = Some(SearchState {
            query: String::new(),
            hits: Vec::new(),
            selected: 0,
        });
    }

    pub(crate) fn search_push(&mut self, ch: char) {
        if let Some(search) = &mut self.search {
            search.query.push(ch);
            search.hits.clear();
            search.selected = 0;
        }
    }

    pub(crate) fn search_pop(&mut self) {
        if let Some(search) = &mut self.search {
            search.query.pop();
            search.hits.clear();
            search.selected = 0;
        }
    }

    fn refresh_search(&mut self) {
        let Some(query) = self.search.as_ref().map(|search| search.query.clone()) else {
            return;
        };
        let hits = self.trace.search(&query);
        if let Some(search) = &mut self.search {
            search.hits = hits;
            search.selected = search.selected.min(search.hits.len().saturating_sub(1));
        }
    }

    pub(crate) fn move_search(&mut self, delta: isize) {
        if let Some(search) = &mut self.search {
            search.selected = move_index(search.selected, delta, search.hits.len());
        }
    }

    pub(crate) fn accept_search(&mut self) {
        self.refresh_search();
        let locator = self
            .search
            .as_ref()
            .and_then(|search| search.hits.get(search.selected))
            .map(|hit| hit.locator.clone());
        self.last_search = self.search.take();
        if let Some(locator) = locator {
            self.reveal(locator);
        }
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

    pub(crate) fn selected_is_raw_payload(&self) -> bool {
        self.selected_node()
            .is_some_and(|node| node.locator.kind == TraceNodeKind::RawPayload)
    }

    fn reveal(&mut self, locator: TraceNodeLocator) {
        let mut current = Some(locator.clone());
        let mut visited = BTreeSet::new();
        while let Some(item) = current {
            if !visited.insert(item.clone()) {
                break;
            }
            let parent = self
                .trace
                .nodes
                .iter()
                .find(|node| node.locator == item)
                .and_then(|node| node.parent.clone());
            if let Some(parent) = &parent {
                self.expanded.insert(parent.clone());
            }
            current = parent;
        }
        self.selected = locator;
        self.child_index = 0;
        self.inspector_scroll = 0;
        self.pane = BrowserPane::Tree;
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
        Some((id, handle))
    }

    pub(crate) fn install_payload(&mut self, id: String, result: anyhow::Result<SanitizedPayload>) {
        let state = match result {
            Ok(payload) => PayloadState::Loaded(payload),
            Err(error) => PayloadState::Failed(format!("{error:#}")),
        };
        self.payloads.insert(id, state);
    }

    pub(crate) fn payload_display(&self, id: &str) -> Option<PayloadDisplay<'_>> {
        self.payloads.get(id).map(|state| match state {
            PayloadState::Loading => PayloadDisplay::Loading,
            PayloadState::Loaded(payload) => PayloadDisplay::Loaded(payload),
            PayloadState::Failed(message) => PayloadDisplay::Failed(message),
        })
    }

    pub(crate) fn diagnostics(&self) -> &[TraceDiagnostic] {
        &self.trace.diagnostics
    }

    pub(crate) fn move_diagnostic(&mut self, delta: isize) {
        self.diagnostic_index =
            move_index(self.diagnostic_index, delta, self.trace.diagnostics.len());
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PayloadDisplay<'a> {
    Loading,
    Loaded(&'a SanitizedPayload),
    Failed(&'a str),
}

fn sort_nodes(nodes: &mut Vec<&TraceNode>) {
    nodes.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.locator.cmp(&right.locator))
    });
}

fn move_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current
            .saturating_add(delta as usize)
            .min(len.saturating_sub(1))
    }
}

fn move_wrapped(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta.is_negative() {
        (current + len - (delta.unsigned_abs() % len)) % len
    } else {
        (current + delta as usize) % len
    }
}
