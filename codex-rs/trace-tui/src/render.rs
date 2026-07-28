use codex_trace::EvidenceGrade;
use codex_trace::TraceNodeKind;
use codex_trace::TraceSourceKind;
use codex_trace::TraceStatus;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::app::App;
use crate::app::PickerState;
use crate::app::Screen;
use crate::app::pane_name;
use crate::app::picker_matches;
use crate::browser::BrowserPane;
use crate::browser::BrowserState;

const WIDE_MIN: u16 = 120;
const MEDIUM_MIN: u16 = 78;

pub(crate) fn render(frame: &mut Frame<'_>, app: &mut App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    render_header(frame, header, app);
    let App {
        screen,
        catalog,
        notice,
        ..
    } = app;
    match screen {
        Screen::Loading(message) => render_centered_message(frame, body, message, "Loading"),
        Screen::Picker(picker) => {
            render_picker(frame, body, catalog.as_ref(), notice.as_deref(), picker)
        }
        Screen::Browser(browser) => render_browser(frame, body, browser),
        Screen::Error(message) => render_centered_message(frame, body, message, "Trace error"),
    }
    render_footer(frame, footer, app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let subtitle = match &app.screen {
        Screen::Loading(_) => "loading".to_string(),
        Screen::Picker(_) => "root sessions".to_string(),
        Screen::Browser(browser) => browser.breadcrumb().join(" / "),
        Screen::Error(_) => "error".to_string(),
    };
    let line = vec![
        " Codex Trace ".bold().cyan(),
        " read-only · offline ".dim(),
        "│ ".dim(),
        sanitize_display(&subtitle).into(),
    ];
    frame.render_widget(Paragraph::new(Line::from(line)), area);
}

fn render_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    catalog: Option<&codex_trace::TraceCatalog>,
    notice: Option<&str>,
    picker: &PickerState,
) {
    let Some(catalog) = catalog else {
        render_centered_message(frame, area, "catalog unavailable", "Root sessions");
        return;
    };
    let mut lines = Vec::new();
    if let Some(notice) = notice {
        lines.push(ListItem::new(Line::from(vec![
            "! ".cyan(),
            sanitize_display(notice).cyan(),
        ])));
    }
    if !catalog.diagnostics.is_empty() {
        lines.push(ListItem::new(
            format!("{} catalog diagnostic(s)", catalog.diagnostics.len()).cyan(),
        ));
        for diagnostic in catalog.diagnostics.iter().take(3) {
            lines.push(ListItem::new(Line::from(vec![
                "  ! ".cyan(),
                format!("[{}] ", evidence_name(diagnostic.evidence)).dim(),
                sanitize_display(&diagnostic.message).into(),
            ])));
        }
        if catalog.diagnostics.len() > 3 {
            lines.push(ListItem::new(
                format!("  … {} more", catalog.diagnostics.len() - 3).dim(),
            ));
        }
    }
    if catalog.sessions.is_empty() {
        lines.push(ListItem::new("No local trace sessions were found."));
    }
    if let Some(query) = &picker.search {
        lines.push(ListItem::new(Line::from(vec![
            "/ ".cyan(),
            sanitize_display(query).into(),
        ])));
    }
    for index in picker_matches(Some(catalog), picker.search.as_deref()) {
        let session = &catalog.sessions[index];
        let created = session.created_at.as_deref().unwrap_or("time unavailable");
        let cwd = session.cwd.as_ref().map_or_else(
            || "cwd unavailable".to_string(),
            |path| path.display().to_string(),
        );
        let model = session
            .model_provider
            .as_deref()
            .unwrap_or("model unavailable");
        let archived = if session.archived { " archived" } else { "" };
        let thread_count = session.thread_count.map_or_else(
            || "? threads".to_string(),
            |count| format!("{count:>3} threads"),
        );
        lines.push(ListItem::new(Line::from(vec![
            format!("{created:<24} ").dim(),
            format!("{:<8} ", source_name(session.source)).cyan(),
            format!("{:<9} ", status_name(session.status)).into(),
            format!("{thread_count}  ").dim(),
            format!("{model}{archived}  ").dim(),
            sanitize_display(&cwd).into(),
        ])));
    }
    let diagnostic_rows = if catalog.diagnostics.is_empty() {
        0
    } else {
        1 + catalog.diagnostics.len().min(3) + usize::from(catalog.diagnostics.len() > 3)
    };
    let selected = picker.selected
        + usize::from(notice.is_some())
        + diagnostic_rows
        + usize::from(picker.search.is_some());
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(lines)
        .block(
            Block::default()
                .title(" Root sessions ")
                .borders(Borders::ALL),
        )
        .highlight_symbol("▶ ")
        .highlight_style(ratatui::style::Style::new().bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_browser(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
    if area.width >= WIDE_MIN {
        let [tree, children, inspector] = Layout::horizontal([
            Constraint::Percentage(34),
            Constraint::Percentage(27),
            Constraint::Percentage(39),
        ])
        .areas(area);
        render_tree(frame, tree, browser);
        render_children(frame, children, browser);
        render_inspector(frame, inspector, browser);
    } else if area.width >= MEDIUM_MIN {
        let [tree, content] =
            Layout::horizontal([Constraint::Percentage(44), Constraint::Percentage(56)])
                .areas(area);
        render_tree(frame, tree, browser);
        if browser.pane == BrowserPane::Children {
            render_children(frame, content, browser);
        } else {
            render_inspector(frame, content, browser);
        }
    } else {
        match browser.pane {
            BrowserPane::Tree => render_tree(frame, area, browser),
            BrowserPane::Children => render_children(frame, area, browser),
            BrowserPane::Inspector => render_inspector(frame, area, browser),
        }
    }
    if browser.search.is_some() {
        render_search(frame, centered(area, 78, 70), browser);
    } else if browser.diagnostics_open {
        render_diagnostics(frame, centered(area, 82, 70), browser);
    }
}

fn render_tree(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let capacity = area.height.saturating_sub(2).max(1) as usize;
    let range = viewport(
        browser.visible_row_count(),
        browser.selected_index(),
        capacity,
    );
    let rows = browser.visible_rows_window(range.start, range.len());
    let items = rows
        .iter()
        .map(|row| {
            let disclosure = match (row.has_children, row.expanded) {
                (true, true) => "▾",
                (true, false) => "▸",
                (false, _) => "·",
            };
            ListItem::new(Line::from(vec![
                "  ".repeat(row.depth).into(),
                format!("{disclosure} ").dim(),
                format!("{:<4} ", kind_tag(row.node.locator.kind)).magenta(),
                sanitize_display(&row.node.label).into(),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default()
        .with_selected(Some(browser.selected_index().saturating_sub(range.start)));
    let list = List::new(items)
        .block(pane_block("Thread tree", browser.pane == BrowserPane::Tree))
        .highlight_symbol("▶ ")
        .highlight_style(ratatui::style::Style::new().bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_children(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let capacity = area.height.saturating_sub(2).max(1) as usize;
    let range = viewport(browser.child_count(), browser.child_index, capacity);
    let children = browser.children_window(range.start, range.len());
    let items = if children.is_empty() {
        vec![ListItem::new("No child nodes")]
    } else {
        children
            .iter()
            .map(|node| {
                ListItem::new(Line::from(vec![
                    format!("{:<4} ", kind_tag(node.locator.kind)).magenta(),
                    sanitize_display(&node.label).into(),
                ]))
            })
            .collect()
    };
    let selected =
        (!children.is_empty()).then_some(browser.child_index.saturating_sub(range.start));
    let mut state = ListState::default().with_selected(selected);
    let list = List::new(items)
        .block(pane_block(
            "Ordered children",
            browser.pane == BrowserPane::Children,
        ))
        .highlight_symbol("▶ ")
        .highlight_style(ratatui::style::Style::new().bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_inspector(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let inner_height = area.height.saturating_sub(2).max(1) as usize;
    let lines = browser
        .prepare_inspector(inner_width, inner_height)
        .iter()
        .cloned()
        .map(Line::from)
        .collect::<Vec<_>>();
    let inspector = Paragraph::new(Text::from(lines)).block(pane_block(
        "Inspector",
        browser.pane == BrowserPane::Inspector,
    ));
    frame.render_widget(inspector, area);
}

fn render_search(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let Some(search) = &browser.search else {
        return;
    };
    frame.render_widget(Clear, area);
    let [input, results] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(2)]).areas(area);
    frame.render_widget(
        Paragraph::new(format!("/{}", sanitize_display(&search.query))).block(
            Block::default()
                .title(" Search this tree ")
                .borders(Borders::ALL),
        ),
        input,
    );
    let capacity = results.height.saturating_sub(2).max(1) as usize;
    let range = viewport(search.hits.len(), search.selected, capacity);
    let items = if search.hits.is_empty() {
        vec![ListItem::new("Press Enter to search")]
    } else {
        search
            .hits
            .iter()
            .skip(range.start)
            .take(range.len())
            .map(|hit| {
                ListItem::new(Line::from(vec![
                    format!("{:<4} ", kind_tag(hit.locator.kind)).magenta(),
                    format!(
                        "[{} {}] ",
                        source_name(hit.provenance),
                        evidence_name(hit.evidence)
                    )
                    .dim(),
                    format!("{}: {}", hit.field, sanitize_display(&hit.snippet)).into(),
                ]))
            })
            .collect()
    };
    let selected = (!search.hits.is_empty()).then_some(search.selected.saturating_sub(range.start));
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Results ").borders(Borders::ALL))
            .highlight_symbol("▶ "),
        results,
        &mut state,
    );
}

fn render_diagnostics(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    frame.render_widget(Clear, area);
    let diagnostics = browser.diagnostics();
    let capacity = area.height.saturating_sub(2).max(1) as usize;
    let range = viewport(diagnostics.len(), browser.diagnostic_index, capacity);
    let items = if diagnostics.is_empty() {
        vec![ListItem::new("No diagnostics recorded for this session.")]
    } else {
        diagnostics
            .iter()
            .skip(range.start)
            .take(range.len())
            .map(|diagnostic| {
                let path = diagnostic
                    .path
                    .as_ref()
                    .map_or_else(String::new, |path| format!(" · {}", path.display()));
                ListItem::new(sanitize_display(&format!(
                    "[{}] {}{path}",
                    evidence_name(diagnostic.evidence),
                    diagnostic.message,
                )))
            })
            .collect()
    };
    let selected =
        (!diagnostics.is_empty()).then_some(browser.diagnostic_index.saturating_sub(range.start));
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Diagnostics · d/Esc close ")
                    .borders(Borders::ALL),
            )
            .highlight_symbol("▶ "),
        area,
        &mut state,
    );
}

fn render_centered_message(frame: &mut Frame<'_>, area: Rect, message: &str, title: &str) {
    frame.render_widget(
        Paragraph::new(Text::from(wrapped_lines(
            &sanitize_display(message),
            area.width.saturating_sub(4).max(1) as usize,
        )))
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = match &app.screen {
        Screen::Loading(_) => " Esc cancel  q quit ",
        Screen::Picker(picker) => {
            if picker.search.is_some() {
                " type to filter  ↑↓ select  Enter open  Esc close search "
            } else {
                " ↑↓/jk select  / filter  Enter open  q quit "
            }
        }
        Screen::Browser(browser) => {
            let pane = pane_name(browser.pane);
            return frame.render_widget(
                Paragraph::new(format!(
                    " {pane}  Tab pane  jk move  h/Backspace parent  Enter open/raw  / search  n/N hits  d diagnostics  q quit "
                ))
                .style(ratatui::style::Style::new().dim()),
                area,
            );
        }
        Screen::Error(_) => " Enter/q close ",
    };
    frame.render_widget(Paragraph::new(help).dim(), area);
}

fn pane_block(title: &str, active: bool) -> Block<'_> {
    let marker = if active { "●" } else { " " };
    let block = Block::default()
        .title(format!(" {marker} {title} "))
        .borders(Borders::ALL);
    if active {
        block.border_style(ratatui::style::Style::new().cyan())
    } else {
        block
    }
}

fn wrapped_lines(text: &str, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for source_line in sanitize_display(text).lines() {
        for wrapped in textwrap::wrap(source_line, width) {
            lines.push(Line::from(wrapped.into_owned()));
        }
        if source_line.is_empty() {
            lines.push(Line::from(""));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
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

fn centered(area: Rect, max_width: u16, percent_height: u16) -> Rect {
    let width = area.width.saturating_sub(4).min(max_width).max(1);
    let height = area
        .height
        .saturating_mul(percent_height)
        .checked_div(100)
        .unwrap_or(1)
        .max(5)
        .min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn source_name(source: TraceSourceKind) -> &'static str {
    match source {
        TraceSourceKind::Ordinary => "ordinary",
        TraceSourceKind::Rich => "rich",
        TraceSourceKind::Merged => "merged",
    }
}

fn status_name(status: TraceStatus) -> &'static str {
    match status {
        TraceStatus::Unknown => "unknown",
        TraceStatus::Running => "running",
        TraceStatus::Completed => "complete",
        TraceStatus::Failed => "failed",
        TraceStatus::Aborted => "aborted",
    }
}

fn evidence_name(evidence: EvidenceGrade) -> &'static str {
    match evidence {
        EvidenceGrade::Exact => "exact",
        EvidenceGrade::Semantic => "semantic",
        EvidenceGrade::Reconstructed => "reconstructed",
        EvidenceGrade::Unavailable => "unavailable",
        EvidenceGrade::Conflicting => "conflicting",
    }
}

fn kind_tag(kind: TraceNodeKind) -> &'static str {
    match kind {
        TraceNodeKind::Session => "SES",
        TraceNodeKind::Thread => "THR",
        TraceNodeKind::Turn => "TRN",
        TraceNodeKind::Inference => "INF",
        TraceNodeKind::ConversationItem => "MSG",
        TraceNodeKind::ToolCall => "TOL",
        TraceNodeKind::CodeCell => "COD",
        TraceNodeKind::TerminalSession => "TTY",
        TraceNodeKind::TerminalOperation => "OP",
        TraceNodeKind::Compaction => "CMP",
        TraceNodeKind::CompactionRequest => "CPR",
        TraceNodeKind::InteractionEdge => "EDG",
        TraceNodeKind::RawPayload => "RAW",
        TraceNodeKind::RolloutRecord => "REC",
        TraceNodeKind::Diagnostic => "DIA",
    }
}

fn viewport(total: usize, selected: usize, capacity: usize) -> std::ops::Range<usize> {
    if total == 0 {
        return 0..0;
    }
    let capacity = capacity.max(1).min(total);
    let selected = selected.min(total - 1);
    let start = selected
        .saturating_sub(capacity / 2)
        .min(total.saturating_sub(capacity));
    start..start + capacity
}
