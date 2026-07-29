use pretty_assertions::assert_eq;

use super::Selection;

#[test]
fn selection_clamps_empty_shortened_and_signed_movement() {
    let mut selection = Selection::default();
    selection.set(8, 10);
    selection.move_clamped(/*delta*/ 8, 10);
    assert_eq!(selection.index(), 9);
    selection.move_clamped(/*delta*/ -20, 10);
    assert_eq!(selection.index(), 0);
    selection.set(8, 3);
    assert_eq!(selection.index(), 2);
    selection.move_clamped(/*delta*/ 1, 0);
    assert_eq!(selection.index(), 0);
}

#[test]
fn selection_supports_first_last_and_wrapped_movement() {
    let mut selection = Selection::default();
    selection.last(4);
    assert_eq!(selection.index(), 3);
    selection.move_wrapped(/*delta*/ 1, 4);
    assert_eq!(selection.index(), 0);
    selection.move_wrapped(/*delta*/ -1, 4);
    assert_eq!(selection.index(), 3);
    selection.first();
    assert_eq!(selection.index(), 0);
}
