//! Parent-TUI visual language adapter for the historical trace browser.

use codex_trace_tui::TraceContentFormat;
use codex_trace_tui::TraceRecordClass;
use codex_trace_tui::TraceRenderRequest;
use codex_trace_tui::TraceRowStyleRequest;
use codex_trace_tui::TraceStatus;
use codex_trace_tui::TraceVisualRenderer;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;

use crate::color::blend;
use crate::markdown_render::render_markdown_text_with_width_and_cwd;
use crate::render::highlight::MAX_HIGHLIGHT_LINE_BYTES;
use crate::render::highlight::exceeds_highlight_limits;
use crate::render::highlight::highlight_code_to_lines;
use crate::style::user_message_bg_rgb;
use crate::terminal_palette::best_color;
use crate::terminal_palette::default_bg;
use crate::wrapping::word_wrap_lines;

/// Reuses Codex's Markdown, syntax-highlighting, and terminal-aware color rules.
#[derive(Debug, Default)]
pub struct CodexTraceVisualRenderer;

impl CodexTraceVisualRenderer {
    /// Creates the stateless trace visual adapter.
    pub fn new() -> Self {
        Self
    }
}

impl TraceVisualRenderer for CodexTraceVisualRenderer {
    fn render_content(&self, request: TraceRenderRequest<'_>) -> Vec<Line<'static>> {
        match &request.document.format {
            TraceContentFormat::Markdown => {
                render_markdown_text_with_width_and_cwd(
                    &request.document.text,
                    Some(request.width),
                    request.cwd,
                )
                .lines
            }
            TraceContentFormat::Json => word_wrap_lines(
                highlight_safe_line_runs(&request.document.text, "json"),
                request.width,
            ),
            TraceContentFormat::Code { language } => word_wrap_lines(
                highlight_safe_line_runs(&request.document.text, language),
                request.width,
            ),
            TraceContentFormat::Text => wrap_plain(&request.document.text, request.width),
        }
    }

    fn row_style(&self, request: TraceRowStyleRequest) -> Style {
        let mut style = default_bg()
            .and_then(|background| row_background_rgb(request.class, background))
            .map_or_else(Style::default, |background| {
                Style::default().bg(best_color(background))
            });
        if request.status == Some(TraceStatus::Failed) {
            style = style.red();
        }
        style = match request.evidence {
            codex_trace_tui::EvidenceGrade::Unavailable => style.dim(),
            codex_trace_tui::EvidenceGrade::Conflicting => style.magenta(),
            codex_trace_tui::EvidenceGrade::Exact
            | codex_trace_tui::EvidenceGrade::Semantic
            | codex_trace_tui::EvidenceGrade::Reconstructed => style,
        };
        if request.selected {
            style.bold()
        } else {
            style
        }
    }
}

fn highlight_safe_line_runs(code: &str, language: &str) -> Vec<Line<'static>> {
    if exceeds_highlight_limits(code.len(), code.lines().count()) {
        return highlight_code_to_lines(code, language);
    }
    let mut rendered = Vec::new();
    let mut safe_run = String::new();
    for line in code.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        if content.len() <= MAX_HIGHLIGHT_LINE_BYTES {
            safe_run.push_str(line);
            continue;
        }
        flush_highlight_run(&mut rendered, &mut safe_run, language);
        rendered.push(Line::from(content.to_string()));
    }
    flush_highlight_run(&mut rendered, &mut safe_run, language);
    if rendered.is_empty() {
        rendered.push(Line::from(""));
    }
    rendered
}

fn flush_highlight_run(rendered: &mut Vec<Line<'static>>, safe_run: &mut String, language: &str) {
    if safe_run.is_empty() {
        return;
    }
    rendered.extend(highlight_code_to_lines(safe_run, language));
    safe_run.clear();
}

fn row_background_rgb(class: TraceRecordClass, background: (u8, u8, u8)) -> Option<(u8, u8, u8)> {
    match class {
        TraceRecordClass::User => Some(user_message_bg_rgb(background)),
        TraceRecordClass::Assistant
        | TraceRecordClass::Commentary
        | TraceRecordClass::FinalAnswer => Some(blend((170, 90, 220), background, 0.10)),
        TraceRecordClass::ToolInput | TraceRecordClass::ToolOutput | TraceRecordClass::Code => {
            Some(blend((30, 150, 180), background, 0.10))
        }
        TraceRecordClass::Reasoning => Some(blend((190, 145, 30), background, 0.10)),
        TraceRecordClass::System | TraceRecordClass::Developer => {
            Some(blend((110, 120, 135), background, 0.10))
        }
        TraceRecordClass::Structure
        | TraceRecordClass::Delegation
        | TraceRecordClass::Compaction
        | TraceRecordClass::Diagnostic
        | TraceRecordClass::RawArtifact
        | TraceRecordClass::Other => None,
    }
}

fn wrap_plain(text: &str, width: usize) -> Vec<Line<'static>> {
    text.split('\n')
        .flat_map(|line| {
            let wrapped = textwrap::wrap(line, width.max(1));
            if wrapped.is_empty() {
                vec![Line::from("")]
            } else {
                wrapped
                    .into_iter()
                    .map(|line| Line::from(line.into_owned()))
                    .collect()
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "trace_visual_renderer_tests.rs"]
mod tests;
