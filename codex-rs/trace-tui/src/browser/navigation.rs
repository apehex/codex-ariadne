//! Reversible descent, lens switching, and stable selection movement.

use codex_trace::PresentationScope;
use codex_trace::TraceObjectRef;

use crate::ContentMode;
use crate::TraceLens;

use super::BrowserState;
use super::location::BrowserLocation;
use super::location::NavigationFrame;
use super::rows::BrowserRow;
use super::rows::BrowserRowId;
use super::structured::JsonPath;

impl BrowserState {
    pub(crate) fn move_vertical(&mut self, delta: isize) {
        self.selection.move_clamped(delta, self.rows.len());
        self.finish_list_move();
    }

    pub(crate) fn page(&mut self, delta: isize, page_size: usize) {
        let amount = isize::try_from(page_size.max(1)).unwrap_or(isize::MAX);
        self.move_vertical(delta.saturating_mul(amount));
    }

    pub(crate) fn first(&mut self) {
        self.selection.first();
        self.finish_list_move();
    }

    pub(crate) fn last(&mut self) {
        self.selection.last(self.rows.len());
        self.finish_list_move();
    }

    pub(crate) fn cycle_lens(&mut self, reverse: bool) {
        let preferred_node = self.selected_node().map(|node| node.locator.clone());
        self.active_lens = if reverse {
            self.active_lens.previous()
        } else {
            self.active_lens.next()
        };
        self.invalidate_visible_search();
        let structural_container = if self.active_lens == TraceLens::Structural {
            preferred_node.as_ref().and_then(|locator| {
                self.index
                    .node(&self.trace, locator)
                    .and_then(|node| node.parent.clone())
            })
        } else {
            None
        };
        self.location = BrowserLocation::Lens {
            lens: self.active_lens,
            scope: self.active_scope.clone(),
            structural_container,
        };
        self.stack.clear();
        self.viewport = 0;
        self.reset_horizontal();
        let preferred = preferred_node.map(BrowserRowId::Node);
        self.rebuild_rows(preferred.as_ref());
    }

    pub(crate) fn enter_selected(&mut self) {
        let Some(row) = self.rows.get(self.selection.index()).cloned() else {
            return;
        };
        match row {
            BrowserRow::Group(id) | BrowserRow::ChildGroup(id) => {
                self.descend(BrowserLocation::Group(id));
            }
            BrowserRow::Structural(position) => {
                let Some(node) = self.trace.nodes.get(position) else {
                    return;
                };
                let locator = node.locator.clone();
                if self.index.children(&self.trace, &locator).next().is_some() {
                    self.descend(BrowserLocation::Lens {
                        lens: TraceLens::Structural,
                        scope: self.active_scope.clone(),
                        structural_container: Some(locator),
                    });
                } else {
                    self.descend(BrowserLocation::Detail(locator));
                }
            }
            BrowserRow::Event { node_position, .. } | BrowserRow::Member { node_position, .. } => {
                if let Some(node) = self.trace.nodes.get(node_position) {
                    self.descend(BrowserLocation::Detail(node.locator.clone()));
                }
            }
            BrowserRow::Reference {
                target: TraceObjectRef::Thread(thread_id),
                ..
            } => {
                self.active_scope = PresentationScope::Thread(thread_id);
                self.active_lens = TraceLens::Collapsed;
                self.descend(BrowserLocation::Lens {
                    lens: TraceLens::Collapsed,
                    scope: self.active_scope.clone(),
                    structural_container: None,
                });
            }
            BrowserRow::Reference {
                node_position: Some(position),
                ..
            } => {
                if let Some(node) = self.trace.nodes.get(position) {
                    self.descend(BrowserLocation::Detail(node.locator.clone()));
                }
            }
            BrowserRow::Reference {
                node_position: None,
                ..
            } => {}
            BrowserRow::Structured { component, .. } => {
                let BrowserLocation::Structured { locator, path } = &self.location else {
                    return;
                };
                let Some(path) = path.child(component) else {
                    return;
                };
                self.descend(BrowserLocation::Structured {
                    locator: locator.clone(),
                    path,
                });
            }
            BrowserRow::Notice(_) => {}
        }
    }

    pub(crate) fn open_detail(&mut self) {
        if let Some(locator) = self.selected_node().map(|node| node.locator.clone()) {
            self.descend(BrowserLocation::Detail(locator));
        }
    }

    pub(crate) fn open_structured(&mut self) {
        if let Some(locator) = self.selected_node().map(|node| node.locator.clone()) {
            self.descend(BrowserLocation::Structured {
                locator,
                path: JsonPath::root(),
            });
        }
    }

    pub(crate) fn back(&mut self) -> bool {
        let Some(frame) = self.stack.pop() else {
            if let BrowserLocation::Lens {
                lens: TraceLens::Structural,
                scope,
                structural_container: Some(container),
            } = &self.location
                && let Some(parent) = self
                    .index
                    .node(&self.trace, container)
                    .and_then(|node| node.parent.clone())
            {
                let preferred = BrowserRowId::Node(container.clone());
                self.location = BrowserLocation::Lens {
                    lens: TraceLens::Structural,
                    scope: scope.clone(),
                    structural_container: Some(parent),
                };
                self.viewport = 0;
                self.reset_horizontal();
                self.rebuild_rows(Some(&preferred));
                return true;
            }
            return false;
        };
        self.location = frame.location;
        self.viewport = frame.viewport;
        self.detail_scroll = frame.detail_scroll;
        self.content_mode = frame.content_mode;
        self.horizontal = frame.horizontal;
        self.sync_active_location();
        if frame.visibility_generation == self.visibility_generation {
            self.rows = frame.rows;
            self.hidden_rows = frame.hidden_rows;
            self.selection.set(frame.selected_index, self.rows.len());
            self.invalidate_detail();
        } else {
            self.rebuild_rows(frame.selected.as_ref());
        }
        true
    }

    pub(crate) fn detail_open(&self) -> bool {
        matches!(self.location, BrowserLocation::Detail(_))
    }

    pub(crate) fn structured_is_scalar(&self) -> bool {
        self.structured_value().is_some_and(|value| {
            !matches!(
                value,
                serde_json::Value::Array(_) | serde_json::Value::Object(_)
            )
        })
    }

    pub(crate) fn structured_scalar(&self) -> Option<(String, bool)> {
        self.structured_value()
            .map(super::structured::bounded_scalar)
    }

    pub(crate) fn structured_pointer(&self) -> Option<String> {
        match &self.location {
            BrowserLocation::Structured { path, .. } => {
                let pointer = path.pointer();
                Some(if pointer.is_empty() {
                    "/".to_string()
                } else {
                    pointer
                })
            }
            BrowserLocation::Lens { .. }
            | BrowserLocation::Group(_)
            | BrowserLocation::Detail(_) => None,
        }
    }

    pub(super) fn reveal(&mut self, locator: codex_trace::TraceNodeLocator) {
        let Some(position) = self.index.node_position(&self.trace, &locator) else {
            return;
        };
        if let Some(group) = self.presentation.group_for_node(position) {
            self.active_scope = group.scope.clone();
            self.temporary_reveal = Some(locator.clone());
            match self.active_lens {
                TraceLens::Collapsed => {
                    let preferred = BrowserRowId::Group(group.id.clone());
                    self.location = BrowserLocation::Lens {
                        lens: TraceLens::Collapsed,
                        scope: self.active_scope.clone(),
                        structural_container: None,
                    };
                    self.viewport = 0;
                    self.reset_horizontal();
                    self.rebuild_rows(Some(&preferred));
                    return;
                }
                TraceLens::Expanded => {
                    let preferred = BrowserRowId::Node(locator);
                    self.location = BrowserLocation::Lens {
                        lens: TraceLens::Expanded,
                        scope: self.active_scope.clone(),
                        structural_container: None,
                    };
                    self.viewport = 0;
                    self.reset_horizontal();
                    self.rebuild_rows(Some(&preferred));
                    return;
                }
                TraceLens::Structural => {}
            }
        }
        let parent = self
            .index
            .node(&self.trace, &locator)
            .and_then(|node| node.parent.clone());
        self.active_lens = TraceLens::Structural;
        self.location = BrowserLocation::Lens {
            lens: TraceLens::Structural,
            scope: self.active_scope.clone(),
            structural_container: parent,
        };
        self.stack.clear();
        self.viewport = 0;
        self.reset_horizontal();
        self.temporary_reveal = Some(locator.clone());
        self.rebuild_rows(Some(&BrowserRowId::Node(locator)));
    }

    fn descend(&mut self, location: BrowserLocation) {
        let selected = self.selected_row_id();
        let selected_index = self.selection.index();
        let rows = std::mem::take(&mut self.rows);
        self.stack.push(NavigationFrame {
            location: self.location.clone(),
            selected,
            viewport: self.viewport,
            detail_scroll: self.detail_scroll,
            content_mode: self.content_mode,
            horizontal: self.horizontal,
            rows,
            hidden_rows: self.hidden_rows,
            selected_index,
            visibility_generation: self.visibility_generation,
        });
        self.location = location;
        self.viewport = 0;
        self.detail_scroll = 0;
        self.reset_horizontal();
        self.content_mode = ContentMode::Rendered;
        self.sync_active_location();
        self.rebuild_rows(/*preferred*/ None);
    }

    fn sync_active_location(&mut self) {
        match &self.location {
            BrowserLocation::Lens { lens, scope, .. } => {
                self.active_lens = *lens;
                self.active_scope = scope.clone();
            }
            BrowserLocation::Group(id) => {
                if let Some(group) = self.presentation.group(id) {
                    self.active_scope = group.scope.clone();
                }
            }
            BrowserLocation::Detail(_) | BrowserLocation::Structured { .. } => {}
        }
    }

    fn finish_list_move(&mut self) {
        let preferred = self.selected_row_id();
        if self.temporary_reveal.take().is_some() {
            self.rebuild_rows(preferred.as_ref());
        }
        self.detail_scroll = 0;
    }
}
