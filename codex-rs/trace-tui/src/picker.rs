//! Cached catalog matching and viewport preparation for the session picker.

use codex_trace::TraceCatalog;

/// Session-picker selection, query, and cached match indexes.
#[derive(Debug)]
pub(crate) struct PickerState {
    /// Selected row within the current match set.
    pub(crate) selected: usize,
    /// Active query editor, when search mode is open.
    pub(crate) search: Option<String>,
    searchable_labels: Vec<String>,
    matches: Vec<usize>,
}

impl PickerState {
    /// Builds normalized labels and the initial complete match list once.
    pub(crate) fn new(catalog: Option<&TraceCatalog>) -> Self {
        let searchable_labels = catalog.map_or_else(Vec::new, |catalog| {
            catalog
                .sessions
                .iter()
                .map(|session| {
                    format!(
                        "{} {} {}",
                        session.session_id,
                        session
                            .cwd
                            .as_ref()
                            .map_or_else(String::new, |path| path.display().to_string()),
                        session.model_provider.as_deref().unwrap_or_default()
                    )
                    .to_lowercase()
                })
                .collect()
        });
        let matches = (0..searchable_labels.len()).collect();
        Self {
            selected: 0,
            search: None,
            searchable_labels,
            matches,
        }
    }

    /// Returns cached session indexes matching the current query.
    pub(crate) fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// Returns the number of cached matches.
    pub(crate) fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Opens an empty query and refreshes the cached match set.
    pub(crate) fn begin_search(&mut self) {
        self.search = Some(String::new());
        self.refresh_matches();
    }

    /// Closes query editing and restores the complete match set.
    pub(crate) fn cancel_search(&mut self) {
        self.search = None;
        self.refresh_matches();
    }

    /// Appends one query character and refreshes cached indexes.
    pub(crate) fn push_search(&mut self, character: char) {
        if let Some(search) = &mut self.search {
            search.push(character);
        }
        self.refresh_matches();
    }

    /// Removes one query character and refreshes cached indexes.
    pub(crate) fn pop_search(&mut self) {
        if let Some(search) = &mut self.search {
            search.pop();
        }
        self.refresh_matches();
    }

    /// Returns the catalog index at the current selection.
    pub(crate) fn selected_catalog_index(&self) -> Option<usize> {
        self.matches.get(self.selected).copied()
    }

    /// Rebuilds matches only after the query changes.
    fn refresh_matches(&mut self) {
        let needle = self
            .search
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        self.matches.clear();
        self.matches.extend(
            self.searchable_labels
                .iter()
                .enumerate()
                .filter(|(_, label)| needle.is_empty() || label.contains(&needle))
                .map(|(index, _)| index),
        );
        self.selected = 0;
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod tests;
