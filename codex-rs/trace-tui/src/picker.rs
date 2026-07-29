//! Cached catalog matching and viewport preparation for the session picker.

use codex_trace::TraceCatalog;
use crossterm::event::KeyCode;

use crate::selection::Selection;

/// Semantic outcome of one picker key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerAction {
    Stay,
    Quit,
    Open(usize),
}

/// Session-picker selection, query, and cached match indexes.
#[derive(Debug)]
pub(crate) struct PickerState {
    /// Selected row within the current match set.
    pub(crate) selection: Selection,
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
            selection: Selection::default(),
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
        self.matches.get(self.selection.index()).copied()
    }

    /// Applies one key to picker editing, movement, or selection.
    pub(crate) fn handle_key(&mut self, code: KeyCode) -> PickerAction {
        let match_count = self.match_count();
        if self.search.is_some() {
            match code {
                KeyCode::Esc => {
                    self.cancel_search();
                    return PickerAction::Stay;
                }
                KeyCode::Enter => {}
                KeyCode::Backspace => {
                    self.pop_search();
                    return PickerAction::Stay;
                }
                KeyCode::Down => {
                    self.selection.move_clamped(/*delta*/ 1, match_count);
                    return PickerAction::Stay;
                }
                KeyCode::Up => {
                    self.selection.move_clamped(/*delta*/ -1, match_count);
                    return PickerAction::Stay;
                }
                KeyCode::Char(character) => {
                    self.push_search(character);
                    return PickerAction::Stay;
                }
                _ => return PickerAction::Stay,
            }
        }
        match code {
            KeyCode::Char('q') | KeyCode::Esc => PickerAction::Quit,
            KeyCode::Down | KeyCode::Char('j') => {
                self.selection.move_clamped(/*delta*/ 1, match_count);
                PickerAction::Stay
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selection.move_clamped(/*delta*/ -1, match_count);
                PickerAction::Stay
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selection.first();
                PickerAction::Stay
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.selection.last(match_count);
                PickerAction::Stay
            }
            KeyCode::Char('/') => {
                self.begin_search();
                PickerAction::Stay
            }
            KeyCode::Enter => self
                .selected_catalog_index()
                .map_or(PickerAction::Stay, PickerAction::Open),
            _ => PickerAction::Stay,
        }
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
        self.selection.first();
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod tests;
