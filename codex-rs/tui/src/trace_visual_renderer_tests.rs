use codex_trace_tui::EvidenceGrade;
use codex_trace_tui::TraceContentDocument;
use codex_trace_tui::TraceContentFormat;
use codex_trace_tui::TraceRecordClass;
use codex_trace_tui::TraceRenderRequest;
use codex_trace_tui::TraceRowStyleRequest;
use codex_trace_tui::TraceVisualRenderer;
use insta::assert_debug_snapshot;
use pretty_assertions::assert_eq;
use ratatui::style::Color;
use ratatui::style::Modifier;

use super::CodexTraceVisualRenderer;
use super::row_background_rgb;

#[test]
fn markdown_content_uses_the_parent_renderer_at_the_requested_width() {
    let renderer = CodexTraceVisualRenderer::new();
    let document = TraceContentDocument {
        format: TraceContentFormat::Markdown,
        text: "# Heading\n\nA **bold** value.".to_string(),
        truncated: false,
    };
    let lines = renderer.render_content(TraceRenderRequest {
        document: &document,
        width: 40,
        cwd: None,
    });
    assert_eq!(
        lines.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec!["# Heading", "", "A bold value."]
    );
}

#[test]
fn selected_rows_remain_distinguishable_without_background_color() {
    let style = CodexTraceVisualRenderer::new().row_style(TraceRowStyleRequest {
        class: TraceRecordClass::User,
        status: None,
        evidence: EvidenceGrade::Semantic,
        selected: true,
    });
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn user_assistant_and_tool_rows_have_distinct_subtle_backgrounds() {
    let terminal_background = (0, 0, 0);
    assert_eq!(
        [
            row_background_rgb(TraceRecordClass::User, terminal_background),
            row_background_rgb(TraceRecordClass::Assistant, terminal_background),
            row_background_rgb(TraceRecordClass::ToolOutput, terminal_background),
        ],
        [Some((30, 30, 30)), Some((17, 9, 22)), Some((3, 15, 18))]
    );
}

#[test]
fn oversized_json_line_does_not_disable_highlighting_for_safe_lines() {
    let oversized = "x".repeat(crate::render::highlight::MAX_HIGHLIGHT_LINE_BYTES + 1);
    let document = TraceContentDocument {
        format: TraceContentFormat::Json,
        text: format!("{{\n  \"safe\": true,\n  \"oversized\": \"{oversized}\"\n}}"),
        truncated: false,
    };

    let lines = CodexTraceVisualRenderer::new().render_content(TraceRenderRequest {
        document: &document,
        width: 8_192,
        cwd: None,
    });

    assert!(lines[1].spans.iter().any(|span| span.style.fg.is_some()));
    assert!(
        lines[2]
            .spans
            .iter()
            .all(|span| span.style.fg.is_none() || span.style.fg == Some(Color::Reset))
    );
    assert!(lines[3].spans.iter().any(|span| span.style.fg.is_some()));
    assert_debug_snapshot!(
        lines
            .iter()
            .map(|line| line.spans.iter().any(|span| span.style.fg.is_some()))
            .collect::<Vec<_>>(),
        @r###"
    [
        true,
        true,
        false,
        true,
    ]
    "###
    );
}
