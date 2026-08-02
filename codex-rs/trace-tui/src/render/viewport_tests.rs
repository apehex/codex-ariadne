use pretty_assertions::assert_eq;
use ratatui::style::Stylize;
use ratatui::text::Line;

use super::line_width;
use super::slice_line;

#[test]
fn slicing_preserves_graphemes_styles_and_display_coordinates() {
    let line = Line::from(vec!["á".red(), "界b".cyan()]).bold();

    let sliced = slice_line(&line, 2, 2, 20);

    assert_eq!(sliced.to_string(), " b");
    assert_eq!(sliced.spans, vec![" b".cyan()]);
    assert_eq!(sliced.style, line.style);
    assert_eq!(line_width(&line, 20), 4);
}

#[test]
fn slicing_neutralizes_controls_and_obeys_zero_and_maximum_widths() {
    let line = Line::from("ab\t\u{1b}cdef");

    assert_eq!(slice_line(&line, 0, 0, 20).to_string(), "");
    assert_eq!(slice_line(&line, 2, 4, 20).to_string(), " �cd");
    assert_eq!(slice_line(&line, 0, 20, 3).to_string(), "ab ");
    assert_eq!(line_width(&line, 3), 3);
}

#[test]
fn right_edge_partial_wide_grapheme_becomes_styled_blank_cells() {
    let line = Line::from(vec!["ab".into(), "界".magenta()]);

    assert_eq!(
        slice_line(&line, 0, 3, 20).spans,
        vec!["ab".into(), " ".magenta()]
    );
}
