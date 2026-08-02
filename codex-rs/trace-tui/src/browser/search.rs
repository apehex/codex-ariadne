//! Asynchronous semantic search and record-class filtering controls.

use std::sync::Arc;

use codex_trace::TraceRecordClass;

use super::BrowserState;
use super::SearchRequestKey;
use super::SearchScope;
use super::SearchState;
use crate::jobs::SearchJob;
use crate::jobs::SearchResult;
use crate::selection::Selection;

impl BrowserState {
    /// Opens an editable semantic search for the selected scope.
    pub(crate) fn begin_search(&mut self, scope: SearchScope) {
        self.invalidate_search_job();
        self.search = Some(SearchState {
            query: String::new(),
            hits: Vec::new(),
            selection: Selection::default(),
            scope,
            loading: false,
            completed_query: None,
        });
    }

    /// Closes search editing and invalidates pending work.
    pub(crate) fn cancel_search(&mut self) {
        self.search = None;
        self.invalidate_search_job();
    }

    /// Appends one character and invalidates results for the old query.
    pub(crate) fn search_push(&mut self, character: char) {
        if let Some(search) = &mut self.search {
            search.query.push(character);
            search.hits.clear();
            search.selection.first();
            search.loading = false;
            search.completed_query = None;
        }
        self.invalidate_search_job();
    }

    /// Removes one character and invalidates results for the old query.
    pub(crate) fn search_pop(&mut self) {
        if let Some(search) = &mut self.search {
            search.query.pop();
            search.hits.clear();
            search.selection.first();
            search.loading = false;
            search.completed_query = None;
        }
        self.invalidate_search_job();
    }

    /// Moves within the completed search result set.
    pub(crate) fn move_search(&mut self, delta: isize) {
        if let Some(search) = &mut self.search {
            search.selection.move_clamped(delta, search.hits.len());
        }
    }

    /// Submits a changed query or reveals the selected completed result.
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
            if let Some(search) = &mut self.search {
                search.loading = true;
                search.hits.clear();
            }
            let key = SearchRequestKey {
                query: query.clone(),
                scope,
                lens: self.active_lens,
                show_hidden_groups: self.show_hidden_groups,
            };
            let trace = Arc::clone(&self.trace);
            let index = Arc::clone(&self.index);
            let presentation = Arc::clone(&self.presentation);
            let visible_classes = self.visible_classes.clone();
            let lens = self.active_lens;
            let show_hidden_groups = self.show_hidden_groups;
            self.search_requests.submit(key, move |token| SearchJob {
                token,
                query,
                scope,
                trace,
                index,
                presentation,
                visible_classes,
                lens,
                show_hidden_groups,
            });
            return;
        }
        let Some(search) = self.search.take() else {
            return;
        };
        let locator = search
            .hits
            .get(search.selection.index())
            .map(|hit| hit.locator.clone());
        self.last_search = Some(search);
        if let Some(locator) = locator {
            self.reveal(locator);
        }
    }

    /// Takes the next queued semantic-search job.
    pub(crate) fn take_search_job(&mut self) -> Option<SearchJob> {
        self.search_requests.take_job()
    }

    /// Installs search results only when their browser, generation, scope, and query are current.
    pub(crate) fn install_search(&mut self, result: SearchResult) {
        let key = SearchRequestKey {
            query: result.query.clone(),
            scope: result.scope,
            lens: result.lens,
            show_hidden_groups: result.show_hidden_groups,
        };
        if !self.search_requests.accept(result.token, &key) {
            return;
        }
        let Some(search) = &mut self.search else {
            return;
        };
        search.hits = result.hits;
        search.selection.first();
        search.loading = false;
        search.completed_query = Some(result.query);
    }

    /// Moves cyclically among results from the last accepted search.
    pub(crate) fn jump_search(&mut self, delta: isize) {
        let locator = self.last_search.as_mut().and_then(|search| {
            search.selection.move_wrapped(delta, search.hits.len());
            search
                .hits
                .get(search.selection.index())
                .map(|hit| hit.locator.clone())
        });
        if let Some(locator) = locator {
            self.reveal(locator);
        }
    }

    /// Returns the stable complete record-class filter order.
    pub(crate) fn filter_classes() -> &'static [TraceRecordClass] {
        &TraceRecordClass::ALL
    }

    /// Returns whether a semantic record class is currently listed.
    pub(crate) fn class_visible(&self, class: TraceRecordClass) -> bool {
        self.visible_classes.contains(&class)
    }

    /// Toggles the highlighted record class while preserving selection when possible.
    pub(crate) fn toggle_filter_class(&mut self) {
        let index = self.filter_selection.index();
        if index == Self::filter_classes().len() {
            self.show_hidden_groups = !self.show_hidden_groups;
            self.visibility_generation = self.visibility_generation.wrapping_add(1);
            let preferred = self.selected_row_id();
            self.temporary_reveal = None;
            self.invalidate_visible_search();
            self.rebuild_rows(preferred.as_ref());
            return;
        }
        let Some(class) = Self::filter_classes().get(index).copied() else {
            return;
        };
        let preferred = self.selected_row_id();
        if !self.visible_classes.remove(&class) {
            self.visible_classes.insert(class);
        }
        self.visibility_generation = self.visibility_generation.wrapping_add(1);
        self.temporary_reveal = None;
        self.invalidate_visible_search();
        self.rebuild_rows(preferred.as_ref());
    }

    /// Moves within the record-class filter menu.
    pub(crate) fn move_filter(&mut self, delta: isize) {
        self.filter_selection
            .move_clamped(delta, Self::filter_classes().len() + 1);
    }

    /// Applies current class visibility and closes the filter menu.
    pub(crate) fn apply_filter(&mut self) {
        let preferred = self.selected_row_id();
        self.rebuild_rows(preferred.as_ref());
        self.filter_open = false;
    }

    /// Restores visibility for every record class.
    pub(crate) fn reset_filter(&mut self) {
        self.visible_classes = TraceRecordClass::ALL.into_iter().collect();
        self.show_hidden_groups = false;
        self.visibility_generation = self.visibility_generation.wrapping_add(1);
        self.temporary_reveal = None;
        self.invalidate_visible_search();
        self.apply_filter();
    }

    /// Returns the number of filtered children at the current depth.
    pub(crate) fn hidden_count(&self) -> usize {
        self.hidden_rows
    }

    /// Returns whether presentation-hidden groups are explicitly visible.
    pub(crate) fn hidden_groups_visible(&self) -> bool {
        self.show_hidden_groups
    }

    /// Invalidates generation-tagged search work.
    fn invalidate_search_job(&mut self) {
        self.search_requests.invalidate();
    }

    /// Invalidates visible-only results after record visibility changes.
    pub(super) fn invalidate_visible_search(&mut self) {
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
