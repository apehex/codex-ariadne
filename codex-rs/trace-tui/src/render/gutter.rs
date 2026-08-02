//! Pinned non-color row identity and group-boundary glyphs.

use codex_trace::GroupId;
use ratatui::style::Stylize;
use ratatui::text::Span;

use crate::browser::rows::BrowserRow;

/// Returns the two-cell pinned gutter for one selectable row.
pub(super) fn row_gutter(
    row: &BrowserRow,
    previous: Option<&BrowserRow>,
    next: Option<&BrowserRow>,
    selected: bool,
) -> Vec<Span<'static>> {
    let selection = if selected { "▶".cyan() } else { " ".into() };
    let marker = match row {
        BrowserRow::Group(_) => "◆",
        BrowserRow::ChildGroup(_) => "◇",
        BrowserRow::Event { group, .. } => boundary_marker(
            previous.and_then(event_group) == Some(group),
            next.and_then(event_group) == Some(group),
        ),
        BrowserRow::Member { .. } => boundary_marker(
            previous.is_some_and(|row| matches!(row, BrowserRow::Member { .. })),
            next.is_some_and(|row| matches!(row, BrowserRow::Member { .. })),
        ),
        BrowserRow::Structural(_) => "·",
        BrowserRow::Reference { .. } => "↗",
        BrowserRow::Structured { .. } => "›",
        BrowserRow::Notice(_) => "!",
    };
    vec![selection, marker.dim()]
}

fn event_group(row: &BrowserRow) -> Option<&GroupId> {
    match row {
        BrowserRow::Event { group, .. } => Some(group),
        BrowserRow::Group(_)
        | BrowserRow::Structural(_)
        | BrowserRow::Member { .. }
        | BrowserRow::ChildGroup(_)
        | BrowserRow::Reference { .. }
        | BrowserRow::Structured { .. }
        | BrowserRow::Notice(_) => None,
    }
}

fn boundary_marker(previous_matches: bool, next_matches: bool) -> &'static str {
    match (previous_matches, next_matches) {
        (false, false) => "•",
        (false, true) => "╭",
        (true, true) => "│",
        (true, false) => "╰",
    }
}

#[cfg(test)]
#[path = "gutter_tests.rs"]
mod tests;
