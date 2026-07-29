use crossterm::event::KeyCode;
use pretty_assertions::assert_eq;

use super::PickerAction;
use super::PickerState;
use crate::selection::Selection;

#[test]
fn picker_rebuilds_matches_only_when_the_query_changes() {
    let mut picker = PickerState {
        selection: Selection::default(),
        search: None,
        searchable_labels: vec!["alpha /work/one".to_string(), "beta /work/two".to_string()],
        matches: vec![0, 1],
    };

    assert_eq!(picker.matches(), &[0, 1]);
    picker.begin_search();
    picker.push_search('t');
    picker.push_search('w');
    picker.push_search('o');
    assert_eq!(picker.matches(), &[1]);
    picker.cancel_search();
    assert_eq!(picker.matches(), &[0, 1]);
}

#[test]
fn picker_keys_return_semantic_actions_and_keep_selection_bounded() {
    let mut picker = PickerState {
        selection: Selection::default(),
        search: None,
        searchable_labels: vec!["alpha".to_string(), "beta".to_string()],
        matches: vec![0, 1],
    };

    assert_eq!(picker.handle_key(KeyCode::Down), PickerAction::Stay);
    assert_eq!(picker.handle_key(KeyCode::Enter), PickerAction::Open(1));
    assert_eq!(picker.handle_key(KeyCode::Down), PickerAction::Stay);
    assert_eq!(picker.selection.index(), 1);
    assert_eq!(picker.handle_key(KeyCode::Char('/')), PickerAction::Stay);
    assert_eq!(picker.handle_key(KeyCode::Char('a')), PickerAction::Stay);
    assert_eq!(picker.matches(), &[0, 1]);
    assert_eq!(picker.handle_key(KeyCode::Esc), PickerAction::Stay);
    assert_eq!(picker.handle_key(KeyCode::Char('q')), PickerAction::Quit);
}
