//! Display-width-safe slicing of styled Ratatui lines.

use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Returns the bounded display width of one styled line.
#[cfg(test)]
pub(super) fn line_width(line: &Line<'_>, maximum: usize) -> usize {
    line.spans
        .iter()
        .map(|span| sanitized_width(span.content.as_ref()))
        .sum::<usize>()
        .min(maximum)
}

/// Returns a styled viewport over one logical line.
pub(super) fn slice_line(
    line: &Line<'_>,
    offset: usize,
    width: usize,
    maximum: usize,
) -> Line<'static> {
    let logical_end = maximum;
    let viewport_end = offset.saturating_add(width);
    let mut column = 0usize;
    let mut spans = Vec::new();

    for span in &line.spans {
        for grapheme in span.content.graphemes(true) {
            let text = sanitize_grapheme(grapheme);
            let grapheme_width = UnicodeWidthStr::width(text.as_str());
            let start = column;
            let end = start.saturating_add(grapheme_width);
            column = end;

            if start >= logical_end || start >= viewport_end {
                break;
            }
            if grapheme_width == 0 || end <= offset {
                continue;
            }

            let visible_start = start.max(offset);
            let visible_end = end.min(viewport_end).min(logical_end);
            if visible_start >= visible_end {
                continue;
            }
            if visible_start == start && visible_end == end {
                push_span(&mut spans, text, span.style);
            } else {
                push_span(
                    &mut spans,
                    " ".repeat(visible_end - visible_start),
                    span.style,
                );
            }
        }
        if column >= logical_end || column >= viewport_end {
            break;
        }
    }

    let mut sliced = Line::from(spans);
    sliced.style = line.style;
    sliced.alignment = line.alignment;
    sliced
}

#[cfg(test)]
fn sanitized_width(text: &str) -> usize {
    text.graphemes(true)
        .map(sanitize_grapheme)
        .map(|grapheme| UnicodeWidthStr::width(grapheme.as_str()))
        .sum()
}

fn sanitize_grapheme(grapheme: &str) -> String {
    grapheme
        .chars()
        .map(|character| match character {
            '\n' | '\r' | '\t' => ' ',
            character if character.is_control() => '�',
            character => character,
        })
        .collect()
}

fn push_span(spans: &mut Vec<Span<'static>>, text: String, style: Style) {
    if let Some(previous) = spans.last_mut()
        && previous.style == style
    {
        previous.content.to_mut().push_str(&text);
        return;
    }
    spans.push(Span::styled(text, style));
}

#[cfg(test)]
#[path = "viewport_tests.rs"]
mod tests;
