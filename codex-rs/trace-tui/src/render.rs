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

use crate::ContentMode;
use crate::HeaderMode;
use crate::TraceRowStyleRequest;
use crate::app::App;
use crate::app::Screen;
use crate::browser::BrowserState;

mod overlay;
mod picker;
mod table;
mod text;

use self::overlay::render_detail;
use self::overlay::render_filter;
use self::overlay::render_help;
use self::overlay::render_search;
use self::picker::render_picker;
use self::table::ColumnPlan;
use self::text::multiline;
use self::text::single_line;

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
    if browser.detail_open {
        render_detail(frame, area, browser);
    } else {
        render_level(frame, area, browser);
    }
    if browser.help_open {
        render_help(frame, centered(area, 72, 72));
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
    let inner_width = usize::from(area.width.saturating_sub(2).max(1));
    let columns = ColumnPlan::new(
        &browser.options().columns,
        browser.options().preview,
        inner_width,
    );
    browser.omitted_columns = columns.omitted();
    let header_rows = usize::from(show_headers);
    let capacity = usize::from(area.height.saturating_sub(2))
        .saturating_sub(header_rows)
        .max(1);
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
        items.push(
            ListItem::new(columns.row(
                /*selected*/ false,
                "NAME",
                /*node*/ None,
                Some("PREVIEW"),
            ))
            .style(ratatui::style::Style::new().bold().dim()),
        );
    }
    for (offset, node) in rows.iter().enumerate() {
        let selected = range.start + offset == browser.selected_index();
        let line = columns.row(
            selected,
            &node.label,
            Some(node),
            node.presentation.preview.as_deref(),
        );
        let style = browser.renderer().row_style(TraceRowStyleRequest {
            class: node.presentation.class,
            status: node.presentation.status,
            evidence: node.evidence,
            selected,
        });
        items.push(ListItem::new(line).style(style));
    }
    if rows.is_empty() {
        items.push(ListItem::new("No visible child records".dim()));
    }
    let selected = (!rows.is_empty())
        .then_some(browser.selected_index().saturating_sub(range.start) + header_rows);
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Records ").borders(Borders::ALL))
            .highlight_style(ratatui::style::Style::new().bold()),
        area,
        &mut state,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = match &app.screen {
        Screen::Loading(_) => " Esc cancel  q quit ",
        Screen::Picker(picker) if picker.search.is_some() => {
            " type to filter  ↑↓ select  Enter open  Esc close search "
        }
        Screen::Picker(_) => " ↑↓/jk select  / filter  Enter open  q quit ",
        Screen::Browser(browser) if browser.help_open => " ?/Esc close help  q quit ",
        Screen::Browser(browser) if browser.search.is_some() => {
            " type query  Enter search/open  ↑↓ choose  Esc close  q quit "
        }
        Screen::Browser(browser) if browser.filter_open => {
            " Space toggle  ↑↓ choose  Enter/Esc close  r reset  q quit "
        }
        Screen::Browser(browser) if browser.detail_open => {
            " jk scroll  PgUp/PgDn page  v rendered/text/raw  Esc back  q quit "
        }
        Screen::Browser(browser) => {
            return frame.render_widget(
                Paragraph::new(format!(
                    " {}/{} visible · {} hidden · {} columns omitted  jk move  Enter descend  i detail  Esc parent  / search  f filter  ? help  q quit ",
                    browser.row_count(),
                    browser.row_count() + browser.hidden_count(),
                    browser.hidden_count(),
                    browser.omitted_columns,
                ))
                .dim(),
                area,
            );
        }
        Screen::Error(_) => " Enter/q close ",
    };
    frame.render_widget(Paragraph::new(help).dim(), area);
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

fn mode_name(mode: ContentMode) -> &'static str {
    match mode {
        ContentMode::Rendered => "rendered",
        ContentMode::Text => "text",
        ContentMode::Raw => "raw",
    }
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
