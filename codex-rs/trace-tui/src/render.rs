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
use crate::browser::PayloadDisplay;

const WIDE_MIN: u16 = 120;
const MEDIUM_MIN: u16 = 78;

pub(crate) fn render(frame: &mut Frame<'_>, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    render_header(frame, header, app);
    match &app.screen {
        Screen::Loading(message) => render_centered_message(frame, body, message, "Loading"),
        Screen::Picker(picker) => render_picker(frame, body, app, picker),
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

fn render_picker(frame: &mut Frame<'_>, area: Rect, app: &App, picker: &PickerState) {
    let Some(catalog) = &app.catalog else {
        render_centered_message(frame, area, "catalog unavailable", "Root sessions");
        return;
    };
    let mut lines = Vec::new();
    if let Some(notice) = &app.notice {
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
        + usize::from(app.notice.is_some())
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

fn render_browser(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
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
    let rows = browser.visible_rows();
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
    let mut state = ListState::default().with_selected(Some(browser.selected_index()));
    let list = List::new(items)
        .block(pane_block("Thread tree", browser.pane == BrowserPane::Tree))
        .highlight_symbol("▶ ")
        .highlight_style(ratatui::style::Style::new().bold());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_children(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let children = browser.children();
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
    let selected = (!children.is_empty()).then_some(browser.child_index);
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

fn render_inspector(frame: &mut Frame<'_>, area: Rect, browser: &BrowserState) {
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let mut lines = Vec::new();
    if let Some(node) = browser.selected_node() {
        lines.extend(wrapped_lines(
            &format!("{} — {}", kind_name(node.locator.kind), node.label),
            inner_width,
        ));
        lines.push(Line::from(vec![
            "id: ".dim(),
            sanitize_display(&node.locator.id).into(),
        ]));
        lines.push(Line::from(vec![
            "timestamp: ".dim(),
            sanitize_display(node.timestamp.as_deref().unwrap_or("unavailable")).into(),
        ]));
        lines.push(Line::from(vec![
            "evidence: ".dim(),
            format!(
                "{} · {}",
                source_name(node.provenance),
                evidence_name(node.evidence)
            )
            .into(),
        ]));
        if node.locator.kind == TraceNodeKind::Session {
            let capabilities = browser.summary().capabilities;
            lines.push(Line::from("capabilities:").dim());
            lines.extend([
                capability_line("ordinary transcript", capabilities.ordinary_transcript),
                capability_line("raw rollout records", capabilities.raw_rollout_records),
                capability_line(
                    "exact inference context",
                    capabilities.exact_inference_context,
                ),
                capability_line("per-generation usage", capabilities.per_generation_usage),
                capability_line("runtime graph", capabilities.runtime_graph),
                capability_line("raw payloads", capabilities.raw_payloads),
                capability_line("compaction detail", capabilities.compaction_detail),
            ]);
        }
        lines.push(Line::from(""));
        let detail = serde_json::to_string_pretty(&node.detail)
            .unwrap_or_else(|error| format!("cannot render node detail: {error}"));
        lines.extend(wrapped_lines(&detail, inner_width));
        if node.locator.kind == TraceNodeKind::RawPayload {
            lines.push(Line::from(""));
            match browser.payload_display(&node.locator.id) {
                None => lines.push(Line::from(
                    "Raw payload collapsed. Press Enter (or r) to load up to 1 MiB.".cyan(),
                )),
                Some(PayloadDisplay::Loading) => {
                    lines.push(Line::from("Loading raw payload…".cyan()));
                }
                Some(PayloadDisplay::Failed(message)) => {
                    lines.push(Line::from(vec![
                        "Raw payload unavailable: ".red(),
                        sanitize_display(message).into(),
                    ]));
                }
                Some(PayloadDisplay::Loaded(payload)) => {
                    let suffix = if payload.truncated {
                        " [truncated]"
                    } else {
                        ""
                    };
                    lines.push(
                        Line::from(format!(
                            "Raw payload · {} bytes observed{suffix}",
                            payload.original_bytes_read
                        ))
                        .dim(),
                    );
                    lines.extend(wrapped_lines(&payload.text, inner_width));
                }
            }
        }
    } else {
        lines.push("No inspectable node".into());
    }
    let inspector = Paragraph::new(Text::from(lines))
        .block(pane_block(
            "Inspector",
            browser.pane == BrowserPane::Inspector,
        ))
        .scroll((browser.inspector_scroll.min(u16::MAX as usize) as u16, 0));
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
    let items = if search.hits.is_empty() {
        vec![ListItem::new("Press Enter to search")]
    } else {
        search
            .hits
            .iter()
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
    let selected = (!search.hits.is_empty()).then_some(search.selected);
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
    let items = if diagnostics.is_empty() {
        vec![ListItem::new("No diagnostics recorded for this session.")]
    } else {
        diagnostics
            .iter()
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
    let selected = (!diagnostics.is_empty()).then_some(browser.diagnostic_index);
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

fn kind_name(kind: TraceNodeKind) -> &'static str {
    match kind {
        TraceNodeKind::Session => "Session",
        TraceNodeKind::Thread => "Agent thread",
        TraceNodeKind::Turn => "Turn",
        TraceNodeKind::Inference => "Inference call",
        TraceNodeKind::ConversationItem => "Conversation item",
        TraceNodeKind::ToolCall => "Tool call",
        TraceNodeKind::CodeCell => "Code cell",
        TraceNodeKind::TerminalSession => "Terminal session",
        TraceNodeKind::TerminalOperation => "Terminal operation",
        TraceNodeKind::Compaction => "Compaction",
        TraceNodeKind::CompactionRequest => "Compaction request",
        TraceNodeKind::InteractionEdge => "Interaction edge",
        TraceNodeKind::RawPayload => "Raw payload",
        TraceNodeKind::RolloutRecord => "Rollout record",
        TraceNodeKind::Diagnostic => "Diagnostic",
    }
}

fn capability_line(name: &str, available: bool) -> Line<'static> {
    let marker = if available { "yes" } else { "no" };
    Line::from(vec![
        "  ".into(),
        format!("{name}: ").dim(),
        marker.to_string().into(),
    ])
}
use codex_trace::EvidenceGrade;
