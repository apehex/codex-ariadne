use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

use codex_trace::RawPayloadHandle;
use codex_trace::SanitizedPayload;
use codex_trace::SearchHit;
use codex_trace::SessionTrace;
use codex_trace::TraceDiagnostic;
use codex_trace::TraceIndex;
use codex_trace::TraceNode;
use codex_trace::TraceNodeKind;
use codex_trace::TraceNodeLocator;

const STRUCTURED_DETAIL_LIMIT: usize = 64 * 1024;

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
    index: TraceIndex,
    ordered_roots: Vec<usize>,
    ordered_children: HashMap<TraceNodeLocator, Vec<usize>>,
    has_children: Vec<bool>,
    expanded: BTreeSet<TraceNodeLocator>,
    selected: TraceNodeLocator,
    selected_row: usize,
    visible: Vec<VisibleRow>,
    selected_children: Vec<usize>,
    pub(crate) pane: BrowserPane,
    pub(crate) child_index: usize,
    pub(crate) inspector_scroll: usize,
    pub(crate) search: Option<SearchState>,
    last_search: Option<SearchState>,
    pub(crate) diagnostics_open: bool,
    pub(crate) diagnostic_index: usize,
    payloads: BTreeMap<String, PayloadState>,
    payload_generation: u64,
    inspector: Option<InspectorCache>,
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

#[derive(Debug, Clone)]
struct VisibleRow {
    position: usize,
    depth: usize,
    has_children: bool,
    expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectorKey {
    locator: TraceNodeLocator,
    width: usize,
    payload_generation: u64,
}

#[derive(Debug)]
struct InspectorCache {
    key: InspectorKey,
    prefix: Vec<String>,
    payload_id: Option<String>,
    window_start: usize,
    window_height: usize,
    window: Vec<String>,
}

impl BrowserState {
    pub(crate) fn new(trace: SessionTrace) -> Self {
        let index = TraceIndex::new(&trace);
        let (ordered_roots, ordered_children, has_children) = ordered_structure(&trace);
        let selected = ordered_roots
            .first()
            .and_then(|position| trace.nodes.get(*position))
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
        let mut state = Self {
            trace,
            index,
            ordered_roots,
            ordered_children,
            has_children,
            expanded,
            selected,
            selected_row: 0,
            visible: Vec::new(),
            selected_children: Vec::new(),
            pane: BrowserPane::Tree,
            child_index: 0,
            inspector_scroll: 0,
            search: None,
            last_search: None,
            diagnostics_open: false,
            diagnostic_index: 0,
            payloads: BTreeMap::new(),
            payload_generation: 0,
            inspector: None,
        };
        state.rebuild_visible();
        state.rebuild_selected_children();
        state
    }

    #[cfg(test)]
    pub(crate) fn visible_rows(&self) -> Vec<TreeRow<'_>> {
        self.tree_rows(0, self.visible.len())
    }

    pub(crate) fn visible_row_count(&self) -> usize {
        self.visible.len()
    }

    pub(crate) fn visible_rows_window(&self, start: usize, len: usize) -> Vec<TreeRow<'_>> {
        self.tree_rows(start, len)
    }

    fn tree_rows(&self, start: usize, len: usize) -> Vec<TreeRow<'_>> {
        self.visible
            .iter()
            .skip(start)
            .take(len)
            .filter_map(|row| {
                self.trace.nodes.get(row.position).map(|node| TreeRow {
                    node,
                    depth: row.depth,
                    has_children: row.has_children,
                    expanded: row.expanded,
                })
            })
            .collect()
    }

    fn rebuild_visible(&mut self) {
        let mut visible = Vec::with_capacity(self.visible.len().max(16));
        let mut visited = HashSet::new();
        let mut pending = self
            .ordered_roots
            .iter()
            .copied()
            .rev()
            .map(|position| (position, 0usize))
            .collect::<Vec<_>>();
        while let Some((position, depth)) = pending.pop() {
            if !visited.insert(position) {
                continue;
            }
            let Some(node) = self.trace.nodes.get(position) else {
                continue;
            };
            let children = self.ordered_children.get(&node.locator);
            let has_children = children.is_some_and(|children| !children.is_empty());
            let expanded = self.expanded.contains(&node.locator);
            visible.push(VisibleRow {
                position,
                depth,
                has_children,
                expanded,
            });
            if expanded {
                let child_depth = depth.saturating_add(1);
                if let Some(children) = children {
                    pending.extend(children.iter().rev().map(|child| (*child, child_depth)));
                }
            }
        }
        self.visible = visible;
        self.selected_row = self
            .visible
            .iter()
            .position(|row| {
                self.trace
                    .nodes
                    .get(row.position)
                    .is_some_and(|node| node.locator == self.selected)
            })
            .unwrap_or(0);
    }

    fn rebuild_selected_children(&mut self) {
        self.selected_children = self
            .ordered_children
            .get(&self.selected)
            .cloned()
            .unwrap_or_default();
        self.child_index = self
            .child_index
            .min(self.selected_children.len().saturating_sub(1));
    }

    fn select(&mut self, locator: TraceNodeLocator, visible_index: Option<usize>) {
        self.selected = locator;
        self.selected_row = visible_index.unwrap_or_else(|| {
            self.visible
                .iter()
                .position(|row| {
                    self.trace
                        .nodes
                        .get(row.position)
                        .is_some_and(|node| node.locator == self.selected)
                })
                .unwrap_or(0)
        });
        self.child_index = 0;
        self.inspector_scroll = 0;
        self.inspector = None;
        self.rebuild_selected_children();
    }

    pub(crate) fn selected_node(&self) -> Option<&TraceNode> {
        self.index.node(&self.trace, &self.selected)
    }

    pub(crate) fn breadcrumb(&self) -> Vec<&str> {
        let mut labels = Vec::new();
        let mut current = Some(&self.selected);
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

    pub(crate) fn selected_index(&self) -> usize {
        self.selected_row
    }

    pub(crate) fn child_count(&self) -> usize {
        self.selected_children.len()
    }

    pub(crate) fn children_window(&self, start: usize, len: usize) -> Vec<&TraceNode> {
        self.selected_children
            .iter()
            .skip(start)
            .take(len)
            .filter_map(|position| self.trace.nodes.get(*position))
            .collect()
    }

    pub(crate) fn move_vertical(&mut self, delta: isize) {
        match self.pane {
            BrowserPane::Tree => {
                if self.visible.is_empty() {
                    return;
                }
                let next = move_index(self.selected_row, delta, self.visible.len());
                if let Some(locator) = self
                    .trace
                    .nodes
                    .get(self.visible[next].position)
                    .map(|node| node.locator.clone())
                {
                    self.select(locator, Some(next));
                }
            }
            BrowserPane::Children => {
                self.child_index =
                    move_index(self.child_index, delta, self.selected_children.len());
            }
            BrowserPane::Inspector => {
                self.inspector_scroll = move_index(self.inspector_scroll, delta, usize::MAX);
            }
        }
    }

    pub(crate) fn first(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if let Some(locator) = self
                    .visible
                    .first()
                    .and_then(|row| self.trace.nodes.get(row.position))
                    .map(|node| node.locator.clone())
                {
                    self.select(locator, Some(0));
                }
            }
            BrowserPane::Children => self.child_index = 0,
            BrowserPane::Inspector => self.inspector_scroll = 0,
        }
    }

    pub(crate) fn last(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if let Some((index, locator)) = self
                    .visible
                    .iter()
                    .enumerate()
                    .next_back()
                    .and_then(|(index, row)| {
                        self.trace
                            .nodes
                            .get(row.position)
                            .map(|node| (index, node.locator.clone()))
                    })
                {
                    self.select(locator, Some(index));
                }
            }
            BrowserPane::Children => {
                self.child_index = self.selected_children.len().saturating_sub(1);
            }
            BrowserPane::Inspector => {}
        }
    }

    pub(crate) fn expand_or_enter(&mut self) {
        match self.pane {
            BrowserPane::Tree => {
                if self.expanded.contains(&self.selected) {
                    self.collapse_selected();
                } else {
                    self.expand_selected();
                }
            }
            BrowserPane::Children => {
                if let Some(locator) = self
                    .selected_children
                    .get(self.child_index)
                    .and_then(|position| self.trace.nodes.get(*position))
                    .map(|node| node.locator.clone())
                {
                    self.select(locator, None);
                    self.pane = BrowserPane::Tree;
                }
            }
            BrowserPane::Inspector => {}
        }
    }

    pub(crate) fn expand(&mut self) {
        self.expand_selected();
    }

    pub(crate) fn collapse_or_parent(&mut self) {
        if self.expanded.contains(&self.selected) {
            self.collapse_selected();
            return;
        }
        if let Some(parent) = self.selected_node().and_then(|node| node.parent.clone()) {
            self.select(parent, None);
        }
    }

    fn expand_selected(&mut self) {
        if !self.expanded.insert(self.selected.clone()) {
            return;
        }
        let Some(row) = self.visible.get_mut(self.selected_row) else {
            self.rebuild_visible();
            return;
        };
        row.expanded = true;
        let parent_position = row.position;
        let child_depth = row.depth.saturating_add(1);
        let descendants = self.expanded_descendants(parent_position, child_depth);
        let insert_at = self.selected_row.saturating_add(1);
        self.visible.splice(insert_at..insert_at, descendants);
    }

    fn collapse_selected(&mut self) {
        if !self.expanded.remove(&self.selected) {
            return;
        }
        let Some(row) = self.visible.get_mut(self.selected_row) else {
            self.rebuild_visible();
            return;
        };
        row.expanded = false;
        let depth = row.depth;
        let start = self.selected_row.saturating_add(1);
        let end = self.visible[start..]
            .iter()
            .position(|candidate| candidate.depth <= depth)
            .map_or(self.visible.len(), |offset| start + offset);
        self.visible.drain(start..end);
    }

    fn expanded_descendants(&self, parent_position: usize, child_depth: usize) -> Vec<VisibleRow> {
        let Some(parent) = self.trace.nodes.get(parent_position) else {
            return Vec::new();
        };
        let Some(children) = self.ordered_children.get(&parent.locator) else {
            return Vec::new();
        };
        let mut rows = Vec::with_capacity(children.len());
        let mut visited = HashSet::with_capacity(children.len());
        visited.insert(parent_position);
        let mut pending = children
            .iter()
            .rev()
            .map(|position| (*position, child_depth))
            .collect::<Vec<_>>();
        while let Some((position, depth)) = pending.pop() {
            if !visited.insert(position) {
                continue;
            }
            let Some(node) = self.trace.nodes.get(position) else {
                continue;
            };
            let has_children = self.has_children.get(position).copied().unwrap_or(false);
            let expanded = has_children && self.expanded.contains(&node.locator);
            rows.push(VisibleRow {
                position,
                depth,
                has_children,
                expanded,
            });
            if expanded && let Some(children) = self.ordered_children.get(&node.locator) {
                let child_depth = depth.saturating_add(1);
                pending.extend(children.iter().rev().map(|child| (*child, child_depth)));
            }
        }
        rows
    }

    pub(crate) fn prepare_inspector(&mut self, width: usize, height: usize) -> &[String] {
        let width = width.max(1);
        let height = height.max(1);
        let key = InspectorKey {
            locator: self.selected.clone(),
            width,
            payload_generation: self.payload_generation,
        };
        if self.inspector.as_ref().map(|cache| &cache.key) != Some(&key) {
            let (prefix, payload_id) = build_inspector_prefix(
                &self.trace,
                &self.index,
                &self.selected,
                &self.payloads,
                width,
            );
            self.inspector = Some(InspectorCache {
                key,
                prefix,
                payload_id,
                window_start: usize::MAX,
                window_height: 0,
                window: Vec::new(),
            });
        }

        let needs_window = self.inspector.as_ref().is_some_and(|cache| {
            cache.window_start != self.inspector_scroll || cache.window_height != height
        });
        if needs_window {
            let Some(cache) = self.inspector.as_ref() else {
                return &[];
            };
            let payload = cache
                .payload_id
                .as_deref()
                .and_then(|id| self.payloads.get(id))
                .and_then(|state| match state {
                    PayloadState::Loaded(payload) => Some(payload.text.as_str()),
                    PayloadState::Loading | PayloadState::Failed(_) => None,
                });
            let window =
                inspector_window(&cache.prefix, payload, width, self.inspector_scroll, height);
            let Some(cache) = self.inspector.as_mut() else {
                return &[];
            };
            cache.window_start = self.inspector_scroll;
            cache.window_height = height;
            cache.window = window;
        }
        self.inspector
            .as_ref()
            .map(|cache| cache.window.as_slice())
            .unwrap_or(&[])
    }

    #[cfg(test)]
    pub(crate) fn cached_visible_row_count(&self) -> usize {
        self.visible.len()
    }

    #[cfg(test)]
    pub(crate) fn inspector_cached_line_count(&self) -> usize {
        self.inspector
            .as_ref()
            .map_or(0, |cache| cache.window.len())
    }

    /*
     * Search remains explicit and bounded in codex-trace. It is intentionally
     * not maintained incrementally during ordinary cursor movement.
     */

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
                .index
                .node(&self.trace, &item)
                .and_then(|node| node.parent.clone());
            if let Some(parent) = &parent {
                self.expanded.insert(parent.clone());
            }
            current = parent;
        }
        self.rebuild_visible();
        self.select(locator, None);
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
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.inspector = None;
        Some((id, handle))
    }

    pub(crate) fn install_payload(&mut self, id: String, result: anyhow::Result<SanitizedPayload>) {
        let state = match result {
            Ok(payload) => PayloadState::Loaded(payload),
            Err(error) => PayloadState::Failed(format!("{error:#}")),
        };
        self.payloads.insert(id, state);
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.inspector_scroll = 0;
        self.inspector = None;
    }

    pub(crate) fn diagnostics(&self) -> &[TraceDiagnostic] {
        &self.trace.diagnostics
    }

    pub(crate) fn move_diagnostic(&mut self, delta: isize) {
        self.diagnostic_index =
            move_index(self.diagnostic_index, delta, self.trace.diagnostics.len());
    }
}

fn ordered_structure(
    trace: &SessionTrace,
) -> (Vec<usize>, HashMap<TraceNodeLocator, Vec<usize>>, Vec<bool>) {
    let mut roots = Vec::new();
    let mut children = HashMap::<TraceNodeLocator, Vec<usize>>::new();
    for (position, node) in trace.nodes.iter().enumerate() {
        if let Some(parent) = &node.parent {
            children.entry(parent.clone()).or_default().push(position);
        } else {
            roots.push(position);
        }
    }
    roots.sort_by(|left, right| compare_positions(trace, *left, *right));
    for positions in children.values_mut() {
        positions.sort_by(|left, right| compare_positions(trace, *left, *right));
    }
    let has_children = trace
        .nodes
        .iter()
        .map(|node| children.contains_key(&node.locator))
        .collect();
    (roots, children, has_children)
}

fn compare_positions(trace: &SessionTrace, left: usize, right: usize) -> std::cmp::Ordering {
    let left = &trace.nodes[left];
    let right = &trace.nodes[right];
    left.timestamp
        .cmp(&right.timestamp)
        .then_with(|| left.locator.cmp(&right.locator))
}

fn build_inspector_prefix(
    trace: &SessionTrace,
    index: &TraceIndex,
    selected: &TraceNodeLocator,
    payloads: &BTreeMap<String, PayloadState>,
    width: usize,
) -> (Vec<String>, Option<String>) {
    let Some(node) = index.node(trace, selected) else {
        return (vec!["No inspectable node".to_string()], None);
    };
    let mut lines = wrap_bounded_lines(
        &format!("{} — {}", kind_name(node.locator.kind), node.label),
        width,
    );
    lines.push(format!("id: {}", sanitize_display(&node.locator.id)));
    lines.push(format!(
        "timestamp: {}",
        sanitize_display(node.timestamp.as_deref().unwrap_or("unavailable"))
    ));
    lines.push(format!(
        "evidence: {} · {}",
        source_name(node.provenance),
        evidence_name(node.evidence)
    ));
    if node.locator.kind == TraceNodeKind::Session {
        let capabilities = trace.summary.capabilities;
        lines.push("capabilities:".to_string());
        lines.extend([
            capability_text("ordinary transcript", capabilities.ordinary_transcript),
            capability_text("raw rollout records", capabilities.raw_rollout_records),
            capability_text(
                "exact inference context",
                capabilities.exact_inference_context,
            ),
            capability_text("per-generation usage", capabilities.per_generation_usage),
            capability_text("runtime graph", capabilities.runtime_graph),
            capability_text("raw payloads", capabilities.raw_payloads),
            capability_text("compaction detail", capabilities.compaction_detail),
        ]);
    }
    lines.push(String::new());
    let detail = bounded_pretty_json(&node.detail, STRUCTURED_DETAIL_LIMIT);
    lines.extend(wrap_bounded_lines(&detail.text, width));
    if detail.truncated {
        lines.push(format!(
            "… structured detail truncated after {STRUCTURED_DETAIL_LIMIT} bytes"
        ));
    }

    if node.locator.kind != TraceNodeKind::RawPayload {
        return (lines, None);
    }
    lines.push(String::new());
    match payloads.get(&node.locator.id) {
        None => {
            lines.push("Raw payload collapsed. Press Enter (or r) to load up to 1 MiB.".to_string())
        }
        Some(PayloadState::Loading) => lines.push("Loading raw payload…".to_string()),
        Some(PayloadState::Failed(message)) => lines.push(format!(
            "Raw payload unavailable: {}",
            sanitize_display(message)
        )),
        Some(PayloadState::Loaded(payload)) => {
            let suffix = if payload.truncated {
                " [truncated]"
            } else {
                ""
            };
            lines.push(format!(
                "Raw payload · {} bytes observed{suffix}",
                payload.original_bytes_read
            ));
            return (lines, Some(node.locator.id.clone()));
        }
    }
    (lines, None)
}

fn inspector_window(
    prefix: &[String],
    payload: Option<&str>,
    width: usize,
    start: usize,
    height: usize,
) -> Vec<String> {
    let mut lines = prefix
        .iter()
        .skip(start)
        .take(height)
        .cloned()
        .collect::<Vec<_>>();
    if lines.len() < height
        && let Some(payload) = payload
    {
        let payload_start = start.saturating_sub(prefix.len());
        let remaining = height - lines.len();
        lines.extend(wrap_window(payload, width, payload_start, remaining));
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct BoundedJson {
    text: String,
    truncated: bool,
}

fn bounded_pretty_json(value: &serde_json::Value, limit: usize) -> BoundedJson {
    let mut writer = LimitedWriter::new(limit);
    let result = serde_json::to_writer_pretty(&mut writer, value);
    if let Err(error) = result
        && !writer.truncated
    {
        return BoundedJson {
            text: format!("cannot render node detail: {error}"),
            truncated: false,
        };
    }
    BoundedJson {
        text: String::from_utf8_lossy(&writer.bytes).into_owned(),
        truncated: writer.truncated,
    }
}

struct LimitedWriter {
    bytes: Vec<u8>,
    limit: usize,
    truncated: bool,
}

impl LimitedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(4096)),
            limit,
            truncated: false,
        }
    }
}

impl Write for LimitedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        if buffer.len() <= remaining {
            self.bytes.extend_from_slice(buffer);
            return Ok(buffer.len());
        }
        self.bytes.extend_from_slice(&buffer[..remaining]);
        self.truncated = true;
        Err(std::io::Error::other(
            "structured detail display limit reached",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn wrap_window(text: &str, width: usize, start: usize, count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let width = width.max(1);
    let end = start.saturating_add(count);
    let mut row = 0usize;
    let mut column = 0usize;
    let mut current = String::new();
    let mut lines = Vec::new();
    let push_row = |current: &mut String, row: &mut usize, lines: &mut Vec<String>| {
        if *row >= start && *row < end {
            lines.push(std::mem::take(current));
        } else {
            current.clear();
        }
        *row = row.saturating_add(1);
    };

    for original in text.chars() {
        if row >= end {
            break;
        }
        let ch = if matches!(original, '\n' | '\t') || !original.is_control() {
            original
        } else {
            '�'
        };
        if ch == '\n' {
            push_row(&mut current, &mut row, &mut lines);
            column = 0;
            continue;
        }
        if column == width {
            push_row(&mut current, &mut row, &mut lines);
            column = 0;
            if row >= end {
                break;
            }
        }
        current.push(ch);
        column = column.saturating_add(1);
    }
    if row < end && (!current.is_empty() || text.is_empty()) {
        push_row(&mut current, &mut row, &mut lines);
    }
    lines
}

fn wrap_bounded_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for source_line in sanitize_display(text).lines() {
        for wrapped in textwrap::wrap(source_line, width) {
            lines.push(wrapped.into_owned());
        }
        if source_line.is_empty() {
            lines.push(String::new());
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn sanitize_display(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if matches!(ch, '\n' | '\t') || !ch.is_control() {
                ch
            } else {
                '�'
            }
        })
        .collect()
}

fn source_name(source: codex_trace::TraceSourceKind) -> &'static str {
    match source {
        codex_trace::TraceSourceKind::Ordinary => "ordinary",
        codex_trace::TraceSourceKind::Rich => "rich",
        codex_trace::TraceSourceKind::Merged => "merged",
    }
}

fn evidence_name(evidence: codex_trace::EvidenceGrade) -> &'static str {
    match evidence {
        codex_trace::EvidenceGrade::Exact => "exact",
        codex_trace::EvidenceGrade::Semantic => "semantic",
        codex_trace::EvidenceGrade::Reconstructed => "reconstructed",
        codex_trace::EvidenceGrade::Unavailable => "unavailable",
        codex_trace::EvidenceGrade::Conflicting => "conflicting",
    }
}

fn kind_name(kind: TraceNodeKind) -> &'static str {
    match kind {
        TraceNodeKind::Session => "Session",
        TraceNodeKind::Thread => "Agent thread",
        TraceNodeKind::Turn => "Turn",
        TraceNodeKind::Inference => "Inference call",
        TraceNodeKind::ConversationItem => "Conversation item",
        TraceNodeKind::ToolCall => "Tool call",
        TraceNodeKind::CodeCell => "Code cell",
        TraceNodeKind::TerminalSession => "Terminal session",
        TraceNodeKind::TerminalOperation => "Terminal operation",
        TraceNodeKind::Compaction => "Compaction",
        TraceNodeKind::CompactionRequest => "Compaction request",
        TraceNodeKind::InteractionEdge => "Interaction edge",
        TraceNodeKind::RawPayload => "Raw payload",
        TraceNodeKind::RolloutRecord => "Rollout record",
        TraceNodeKind::Diagnostic => "Diagnostic",
    }
}

fn capability_text(name: &str, available: bool) -> String {
    let marker = if available { "yes" } else { "no" };
    format!("  {name}: {marker}")
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
