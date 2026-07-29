use codex_trace::EvidenceGrade;
use codex_trace::TraceNode;
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
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::ContentMode;
use crate::HeaderMode;
use crate::PreviewMode;
use crate::TraceColumn;
use crate::TraceRowStyleRequest;
use crate::app::App;
use crate::app::PickerState;
use crate::app::Screen;
use crate::app::picker_matches;
use crate::browser::BrowserState;
use crate::browser::SearchScope;

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
            sanitize(&subtitle).into(),
        ])),
        area,
    );
}

fn render_picker(
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
        items.push(ListItem::new(format!("! {}", sanitize(notice)).cyan()));
    }
    if !catalog.diagnostics.is_empty() {
        items.push(ListItem::new(
            format!("{} catalog diagnostic(s)", catalog.diagnostics.len()).cyan(),
        ));
        for diagnostic in catalog.diagnostics.iter().take(3) {
            items.push(ListItem::new(Line::from(vec![
                "  ! ".cyan(),
                format!("[{}] ", evidence_name(diagnostic.evidence)).dim(),
                sanitize(&diagnostic.message).into(),
            ])));
        }
        if catalog.diagnostics.len() > 3 {
            items.push(ListItem::new(
                format!("  … {} more", catalog.diagnostics.len() - 3).dim(),
            ));
        }
    }
    if let Some(query) = &picker.search {
        items.push(ListItem::new(format!("/ {}", sanitize(query)).cyan()));
    }
    for index in picker_matches(Some(catalog), picker.search.as_deref()) {
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
            sanitize(&cwd)
        )));
    }
    if items.is_empty() {
        items.push(ListItem::new("No local trace sessions were found."));
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
    let columns = visible_columns(browser, inner_width);
    browser.omitted_columns = browser
        .options()
        .columns
        .len()
        .saturating_sub(columns.len());
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
            ListItem::new(format_row(
                /*selected*/ false,
                "NAME",
                &columns,
                /*node*/ None,
                Some("PREVIEW"),
                browser.options().preview,
                inner_width,
            ))
            .style(ratatui::style::Style::new().bold().dim()),
        );
    }
    for (offset, node) in rows.iter().enumerate() {
        let selected = range.start + offset == browser.selected_index();
        let line = format_row(
            selected,
            &sanitize(&node.label),
            &columns,
            Some(node),
            node.presentation.preview.as_deref(),
            browser.options().preview,
            inner_width,
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

fn visible_columns(browser: &BrowserState, width: usize) -> Vec<TraceColumn> {
    let mut columns = browser.options().columns.clone();
    let drop_order = [
        TraceColumn::Identifier,
        TraceColumn::Timestamp,
        TraceColumn::Status,
        TraceColumn::Evidence,
        TraceColumn::Source,
        TraceColumn::Kind,
        TraceColumn::Class,
    ];
    for candidate in drop_order {
        let required = 14
            + columns
                .iter()
                .map(|column| column_width(*column) + 1)
                .sum::<usize>();
        if required <= width {
            break;
        }
        if let Some(index) = columns.iter().position(|column| *column == candidate) {
            columns.remove(index);
        }
    }
    columns
}

fn format_row(
    selected: bool,
    label: &str,
    columns: &[TraceColumn],
    node: Option<&TraceNode>,
    preview: Option<&str>,
    preview_mode: PreviewMode,
    width: usize,
) -> Line<'static> {
    let fixed = columns
        .iter()
        .map(|column| column_width(*column) + 1)
        .sum::<usize>();
    let marker = if selected { "▶ " } else { "  " };
    let allow_preview = match preview_mode {
        PreviewMode::Auto => width.saturating_sub(fixed + 26) >= 32,
        PreviewMode::Always => width.saturating_sub(fixed + 18) >= 12,
        PreviewMode::Never => false,
    };
    let preview_width = allow_preview.then_some(width.saturating_sub(fixed + 26));
    let name_width = width
        .saturating_sub(fixed + preview_width.unwrap_or_default() + 2)
        .max(8);
    let mut text = format!("{marker}{}", pad_fit(label, name_width.saturating_sub(2)));
    for column in columns {
        let value = node
            .map(|node| column_value(*column, node))
            .unwrap_or_else(|| column_name(*column).to_string());
        text.push(' ');
        text.push_str(&pad_fit(&value, column_width(*column)));
    }
    if let Some(preview_width) = preview_width {
        text.push(' ');
        text.push_str(&fit(&sanitize(preview.unwrap_or_default()), preview_width));
    }
    Line::from(text)
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, browser: &mut BrowserState) {
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
        format!("id: {}", sanitize(&node.locator.id)),
    ];
    let mode = mode_name(browser.content_mode);
    let inner_width = usize::from(area.width.saturating_sub(4).max(1));
    let inner_height = usize::from(area.height.saturating_sub(2).max(1));
    browser.detail_page_size = inner_height;
    let content = browser.detail_lines(inner_width).to_vec();
    let mut lines = metadata.into_iter().map(Line::from).collect::<Vec<_>>();
    if let Some(notice) = browser.payload_notice() {
        lines.push(Line::from(format!("raw: {}", sanitize(notice))).red());
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

fn render_search(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
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
        Paragraph::new(format!("/{}", sanitize(&search.query))).block(
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
                    sanitize(&hit.snippet)
                ))
            })
            .collect()
    };
    let mut state =
        ListState::default().with_selected((!search.hits.is_empty()).then_some(search.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Results ").borders(Borders::ALL))
            .highlight_symbol("▶ "),
        results,
        &mut state,
    );
}

fn render_filter(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
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
    let mut state = ListState::default().with_selected(Some(browser.filter_index));
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

fn render_help(frame: &mut Frame<'_>, area: Rect) {
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
        Paragraph::new(sanitize(message))
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn column_name(column: TraceColumn) -> &'static str {
    match column {
        TraceColumn::Kind => "KIND",
        TraceColumn::Class => "CLASS",
        TraceColumn::Status => "STATUS",
        TraceColumn::Timestamp => "TIME",
        TraceColumn::Source => "SOURCE",
        TraceColumn::Evidence => "EVIDENCE",
        TraceColumn::Identifier => "ID",
    }
}

fn column_width(column: TraceColumn) -> usize {
    match column {
        TraceColumn::Kind => 4,
        TraceColumn::Class => 12,
        TraceColumn::Status => 9,
        TraceColumn::Timestamp => 20,
        TraceColumn::Source => 8,
        TraceColumn::Evidence => 13,
        TraceColumn::Identifier => 16,
    }
}

fn column_value(column: TraceColumn, node: &TraceNode) -> String {
    match column {
        TraceColumn::Kind => kind_tag(node.locator.kind).to_string(),
        TraceColumn::Class => class_name(node.presentation.class).to_string(),
        TraceColumn::Status => node
            .presentation
            .status
            .map_or_else(|| "—".to_string(), |status| status_name(status).to_string()),
        TraceColumn::Timestamp => node.timestamp.clone().unwrap_or_else(|| "—".to_string()),
        TraceColumn::Source => source_name(node.provenance).to_string(),
        TraceColumn::Evidence => evidence_name(node.evidence).to_string(),
        TraceColumn::Identifier => node.locator.id.clone(),
    }
}

fn fit(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let target = width - 1;
    let mut used = 0;
    let mut fitted = String::new();
    for character in text.chars() {
        let character_width = character.width().unwrap_or_default();
        if used + character_width > target {
            break;
        }
        fitted.push(character);
        used += character_width;
    }
    fitted.push('…');
    fitted
}

fn pad_fit(text: &str, width: usize) -> String {
    let mut fitted = fit(text, width);
    fitted.push_str(&" ".repeat(width.saturating_sub(UnicodeWidthStr::width(fitted.as_str()))));
    fitted
}

fn sanitize(text: &str) -> String {
    text.chars()
        .map(|character| {
            if matches!(character, '\n' | '\t') || !character.is_control() {
                character
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
