use codex_trace::EvidenceGrade;
use codex_trace::TraceNodeKind;
use codex_trace::TraceRecordClass;
use codex_trace::TraceSourceKind;
use codex_trace::TraceStatus;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::HeaderMode;
use crate::TraceRowStyleRequest;
use crate::app::App;
use crate::app::Screen;
use crate::browser::BrowserState;

mod footer;
mod gutter;
mod overlay;
mod picker;
mod table;
mod text;
mod viewport;

use self::footer::render_footer;
use self::gutter::row_gutter;
use self::overlay::render_detail;
use self::overlay::render_filter;
use self::overlay::render_help;
use self::overlay::render_search;
use self::overlay::render_structured_scalar;
use self::picker::render_picker;
use self::table::ColumnPlan;
use self::text::multiline;
use self::text::single_line;
use self::viewport::slice_line;

pub(crate) fn render(frame: &mut Frame<'_>, app: &mut App) {
    let header_height = if matches!(&app.screen, Screen::Browser(_)) {
        1
    } else {
        2
    };
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(2),
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
        Screen::Loading(message) => render_message(frame, body, message, "Loading"),
        Screen::Picker(picker) => {
            render_picker(frame, body, catalog.as_ref(), notice.as_deref(), picker)
        }
        Screen::Browser(browser) => render_browser(frame, body, browser),
        Screen::Error(message) => render_message(frame, body, message, "Trace error"),
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
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            " Codex Trace ".bold().cyan(),
            " read-only · offline ".dim(),
            "│ ".dim(),
            single_line(&subtitle).into(),
        ])),
        area,
    );
}

fn render_browser(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
    if browser.detail_open() {
        render_detail(frame, area, browser);
    } else if browser.structured_is_scalar() {
        render_structured_scalar(frame, area, browser);
    } else {
        render_level(frame, area, browser);
    }
    if browser.help_open {
        render_help(frame, centered(area, 72, 90));
    } else if browser.search.is_some() {
        render_search(frame, centered(area, 78, 70), browser);
    } else if browser.filter_open {
        render_filter(frame, centered(area, 52, 75), browser);
    }
}

fn render_level(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
    let show_headers = match browser.options().headers {
        HeaderMode::Always => true,
        HeaderMode::Never => false,
        HeaderMode::Auto => area.width >= 78 && area.height >= 6,
    };
    let inner_width = usize::from(area.width.saturating_sub(2));
    let maximum_width = browser.options().max_content_width.columns();
    let columns = ColumnPlan::new(
        &browser.options().columns,
        browser.options().preview,
        browser.options().column_layout,
        inner_width,
        maximum_width,
    );
    browser.omitted_columns = columns.omitted();
    browser.update_horizontal_extent(columns.canvas_width(), inner_width);
    let horizontal_offset = browser.horizontal_position().0;
    let header_rows = usize::from(show_headers);
    let capacity = usize::from(area.height).saturating_sub(header_rows).max(1);
    browser.page_size = capacity;
    let range = viewport(
        browser.row_count(),
        browser.selected_index(),
        capacity,
        browser.viewport,
    );
    browser.viewport = range.start;
    let rows = browser.rows_window(range.start, range.len());
    let mut items = Vec::with_capacity(rows.len() + header_rows);
    if show_headers {
        let header = columns.row("NAME", /*row*/ None, Some("PREVIEW"));
        let mut spans = vec!["  ".into()];
        spans.extend(slice_line(&header, horizontal_offset, inner_width, maximum_width).spans);
        items.push(
            ListItem::new(Line::from(spans))
                .style(ratatui::style::Style::new().bold().dim().underlined()),
        );
    }
    for (offset, row) in rows.iter().enumerate() {
        let index = range.start + offset;
        let selected = index == browser.selected_index();
        let display = browser.row_display(row);
        let line = columns.row(&display.label, Some(&display), display.preview.as_deref());
        let mut spans = row_gutter(
            row,
            index.checked_sub(1).and_then(|index| browser.row_at(index)),
            browser.row_at(index.saturating_add(1)),
            selected,
        );
        spans.extend(slice_line(&line, horizontal_offset, inner_width, maximum_width).spans);
        let style = browser.renderer().row_style(TraceRowStyleRequest {
            class: display.class,
            status: display.status,
            evidence: display.evidence,
            selected,
        });
        items.push(ListItem::new(Line::from(spans)).style(style));
    }
    if rows.is_empty() {
        items.push(ListItem::new("No visible items at this level".dim()));
    }
    let selected = (!rows.is_empty())
        .then_some(browser.selected_index().saturating_sub(range.start) + header_rows);
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items).highlight_style(ratatui::style::Style::new().bold()),
        area,
        &mut state,
    );
}

fn render_message(frame: &mut Frame<'_>, area: Rect, message: &str, title: &str) {
    frame.render_widget(
        Paragraph::new(multiline(message))
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
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

fn viewport(
    total: usize,
    selected: usize,
    capacity: usize,
    current_start: usize,
) -> std::ops::Range<usize> {
    if total == 0 {
        return 0..0;
    }
    let capacity = capacity.max(1).min(total);
    let selected = selected.min(total - 1);
    let maximum_start = total.saturating_sub(capacity);
    let current_start = current_start.min(maximum_start);
    let start = if selected < current_start {
        selected
    } else if selected >= current_start + capacity {
        selected + 1 - capacity
    } else {
        current_start
    };
    start..start + capacity
}

fn class_name(class: TraceRecordClass) -> &'static str {
    match class {
        TraceRecordClass::Structure => "structure",
        TraceRecordClass::System => "system",
        TraceRecordClass::Developer => "developer",
        TraceRecordClass::User => "user",
        TraceRecordClass::Assistant => "assistant",
        TraceRecordClass::Commentary => "commentary",
        TraceRecordClass::FinalAnswer => "final",
        TraceRecordClass::Reasoning => "reasoning",
        TraceRecordClass::ToolInput => "tool input",
        TraceRecordClass::ToolOutput => "tool output",
        TraceRecordClass::Code => "code",
        TraceRecordClass::Delegation => "delegation",
        TraceRecordClass::Compaction => "compaction",
        TraceRecordClass::Diagnostic => "diagnostic",
        TraceRecordClass::RawArtifact => "raw",
        TraceRecordClass::Other => "other",
    }
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

fn kind_name(kind: TraceNodeKind) -> &'static str {
    match kind {
        TraceNodeKind::Session => "session",
        TraceNodeKind::Thread => "thread",
        TraceNodeKind::Turn => "turn",
        TraceNodeKind::Inference => "inference",
        TraceNodeKind::ConversationItem => "conversation item",
        TraceNodeKind::ToolCall => "tool call",
        TraceNodeKind::CodeCell => "code cell",
        TraceNodeKind::TerminalSession => "terminal session",
        TraceNodeKind::TerminalOperation => "terminal operation",
        TraceNodeKind::Compaction => "compaction",
        TraceNodeKind::CompactionRequest => "compaction request",
        TraceNodeKind::InteractionEdge => "interaction edge",
        TraceNodeKind::RawPayload => "raw payload",
        TraceNodeKind::RolloutRecord => "rollout record",
        TraceNodeKind::Diagnostic => "diagnostic",
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
