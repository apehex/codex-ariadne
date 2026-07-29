//! Full-screen details and modal browser overlays.

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;

use super::class_name;
use super::evidence_name;
use super::kind_name;
use super::kind_tag;
use super::mode_name;
use super::render_message;
use super::source_name;
use super::status_name;
use super::text::single_line;
use crate::browser::BrowserState;
use crate::browser::SearchScope;

/// Renders the selected leaf as a scrollable, semantic full-screen document.
pub(super) fn render_detail(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
    let Some(node) = browser.selected_node().cloned() else {
        render_message(frame, area, "No selected record", "Detail");
        return;
    };
    let metadata = vec![
        format!("kind: {}", kind_name(node.locator.kind)),
        format!("class: {}", class_name(node.presentation.class)),
        format!(
            "status: {}",
            node.presentation.status.map_or("—", status_name)
        ),
        format!("timestamp: {}", node.timestamp.as_deref().unwrap_or("—")),
        format!("source: {}", source_name(node.provenance)),
        format!("evidence: {}", evidence_name(node.evidence)),
        format!("id: {}", single_line(&node.locator.id)),
    ];
    let mode = mode_name(browser.content_mode);
    let inner_width = usize::from(area.width.saturating_sub(4).max(1));
    let inner_height = usize::from(area.height.saturating_sub(2).max(1));
    browser.detail_page_size = inner_height;
    let content = browser.detail_lines(inner_width).to_vec();
    let mut lines = metadata.into_iter().map(Line::from).collect::<Vec<_>>();
    if let Some(notice) = browser.payload_notice() {
        lines.push(Line::from(format!("raw: {}", single_line(notice))).red());
    }
    if browser.detail_truncated() {
        lines.push(Line::from("content truncated at the safe display bound").magenta());
    }
    lines.push(Line::from(""));
    if content.is_empty() {
        lines.push(Line::from("preparing content…").dim());
    } else {
        lines.extend(content.iter().cloned());
    }
    let max_scroll = lines.len().saturating_sub(inner_height);
    browser.detail_scroll = browser.detail_scroll.min(max_scroll);
    let visible = lines
        .into_iter()
        .skip(browser.detail_scroll)
        .take(inner_height)
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(visible).block(
            Block::default()
                .title(format!(" Detail · {mode} · v change view "))
                .borders(Borders::ALL),
        ),
        area,
    );
}

/// Renders the generation-tagged semantic search editor and result list.
pub(super) fn render_search(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let Some(search) = &browser.search else {
        return;
    };
    frame.render_widget(Clear, area);
    let scope = match search.scope {
        SearchScope::Visible => "visible",
        SearchScope::All => "all records",
    };
    let [input, results] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(2)]).areas(area);
    frame.render_widget(
        Paragraph::new(format!("/{}", single_line(&search.query))).block(
            Block::default()
                .title(format!(" Search {scope} "))
                .borders(Borders::ALL),
        ),
        input,
    );
    let items = if search.loading {
        vec![ListItem::new("Searching…".cyan())]
    } else if search.hits.is_empty() {
        let message = if search.completed_query.is_some() {
            "No matches"
        } else {
            "Press Enter to search"
        };
        vec![ListItem::new(message)]
    } else {
        search
            .hits
            .iter()
            .map(|hit| {
                ListItem::new(format!(
                    "{:<4} [{} {}] {}: {}",
                    kind_tag(hit.locator.kind),
                    source_name(hit.provenance),
                    evidence_name(hit.evidence),
                    hit.field,
                    single_line(&hit.snippet)
                ))
            })
            .collect()
    };
    let mut state = ListState::default()
        .with_selected((!search.hits.is_empty()).then_some(search.selection.index()));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Results ").borders(Borders::ALL))
            .highlight_symbol("▶ "),
        results,
        &mut state,
    );
}

/// Renders the semantic record-class visibility editor.
pub(super) fn render_filter(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    frame.render_widget(Clear, area);
    let items = BrowserState::filter_classes()
        .iter()
        .map(|class| {
            let checked = if browser.class_visible(*class) {
                "x"
            } else {
                " "
            };
            ListItem::new(format!("[{checked}] {}", class_name(*class)))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(browser.filter_selection.index()));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Filter · Space toggle · Enter apply ")
                    .borders(Borders::ALL),
            )
            .highlight_symbol("▶ "),
        area,
        &mut state,
    );
}

/// Renders the complete keyboard help overlay.
pub(super) fn render_help(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("j/k, arrows     move"),
            Line::from("PgUp/PgDn       page"),
            Line::from("Ctrl-U/Ctrl-D   half-page"),
            Line::from("gg / G          first / last"),
            Line::from("Enter           descend or open leaf"),
            Line::from("i / Esc         inspect / return"),
            Line::from("/ / g/          visible / all-record search"),
            Line::from("n / N           next / previous result"),
            Line::from("f / F           edit / reset filters"),
            Line::from("v               rendered / text / raw"),
            Line::from("? / q           help / quit"),
        ])
        .block(
            Block::default()
                .title(" Trace navigation ")
                .borders(Borders::ALL),
        ),
        area,
    );
}
