//! Windowed session-picker rendering.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;

use super::evidence_name;
use super::render_message;
use super::source_name;
use super::status_name;
use super::text::single_line;
use crate::picker::PickerState;

/// Renders fixed diagnostics and only the visible window of matching sessions.
pub(super) fn render_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    catalog: Option<&codex_trace::TraceCatalog>,
    notice: Option<&str>,
    picker: &PickerState,
) {
    let Some(catalog) = catalog else {
        render_message(frame, area, "catalog unavailable", "Root sessions");
        return;
    };
    let mut items = Vec::new();
    if let Some(notice) = notice {
        items.push(ListItem::new(format!("! {}", single_line(notice)).cyan()));
    }
    if !catalog.diagnostics.is_empty() {
        items.push(ListItem::new(
            format!("{} catalog diagnostic(s)", catalog.diagnostics.len()).cyan(),
        ));
        for diagnostic in catalog.diagnostics.iter().take(3) {
            items.push(ListItem::new(Line::from(vec![
                "  ! ".cyan(),
                format!("[{}] ", evidence_name(diagnostic.evidence)).dim(),
                single_line(&diagnostic.message).into(),
            ])));
        }
        if catalog.diagnostics.len() > 3 {
            items.push(ListItem::new(
                format!("  … {} more", catalog.diagnostics.len() - 3).dim(),
            ));
        }
    }
    if let Some(query) = &picker.search {
        items.push(ListItem::new(format!("/ {}", single_line(query)).cyan()));
    }
    let fixed_rows = items.len();
    let visible_session_rows = usize::from(area.height)
        .saturating_sub(2)
        .saturating_sub(fixed_rows)
        .max(1);
    let match_start = picker
        .selection
        .index()
        .saturating_sub(visible_session_rows.saturating_sub(1));
    for index in picker
        .matches()
        .iter()
        .skip(match_start)
        .take(visible_session_rows)
        .copied()
    {
        let session = &catalog.sessions[index];
        let created = session.created_at.as_deref().unwrap_or("time unavailable");
        let model = session
            .model_provider
            .as_deref()
            .unwrap_or("model unavailable");
        let thread_count = session.thread_count.map_or_else(
            || "? threads".to_string(),
            |count| format!("{count:>3} threads"),
        );
        let cwd = session.cwd.as_ref().map_or_else(
            || "cwd unavailable".to_string(),
            |path| path.display().to_string(),
        );
        items.push(ListItem::new(format!(
            "{created:<24} {:<8} {:<9} {thread_count}  {model:<18} {}",
            source_name(session.source),
            status_name(session.status),
            single_line(&cwd)
        )));
    }
    if picker.matches().is_empty() {
        items.push(ListItem::new(
            "No matching local trace sessions were found.",
        ));
    }
    let selected = (!picker.matches().is_empty())
        .then_some(fixed_rows + picker.selection.index().saturating_sub(match_start));
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Root sessions ")
                    .borders(Borders::ALL),
            )
            .highlight_symbol("▶ ")
            .highlight_style(ratatui::style::Style::new().bold()),
        area,
        &mut state,
    );
}
