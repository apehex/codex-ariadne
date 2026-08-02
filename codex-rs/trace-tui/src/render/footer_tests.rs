use pretty_assertions::assert_eq;

use super::compose_footer;

#[test]
fn footer_elides_left_hints_before_compacting_right_state() {
    let right = vec![
        "collapsed · rendered · nowrap · x 0/120".to_string(),
        "C · R · NW · x 0/120".to_string(),
        "x 0/120".to_string(),
    ];

    assert_eq!(
        compose_footer("verbose navigation hints", "keys", &right, 48).to_string(),
        "keys     collapsed · rendered · nowrap · x 0/120"
    );
    assert_eq!(
        compose_footer("verbose navigation hints", "keys", &right, 24).to_string(),
        "    C · R · NW · x 0/120"
    );
    assert_eq!(
        compose_footer("verbose navigation hints", "keys", &right, 10).to_string(),
        "   x 0/120"
    );
}
