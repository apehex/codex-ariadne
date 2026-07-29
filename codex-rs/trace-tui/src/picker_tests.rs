use pretty_assertions::assert_eq;

use super::PickerState;

#[test]
fn picker_rebuilds_matches_only_when_the_query_changes() {
    let mut picker = PickerState {
        selected: 0,
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
