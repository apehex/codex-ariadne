use pretty_assertions::assert_eq;

use super::fit;
use super::multiline;
use super::single_line;

#[test]
fn single_line_neutralizes_layout_and_terminal_controls() {
    assert_eq!(
        single_line("alpha\nbeta\tgamma\u{1b}[31m\u{7}"),
        "alpha beta gamma�[31m�"
    );
}

#[test]
fn multiline_preserves_layout_but_neutralizes_other_controls() {
    assert_eq!(multiline("alpha\nbeta\t\u{7}"), "alpha\nbeta\t�");
}

#[test]
fn display_width_fitting_preserves_unicode_boundaries() {
    assert_eq!(fit("éclair", 4), "écl…");
    assert_eq!(fit("界面", 3), "界…");
}
