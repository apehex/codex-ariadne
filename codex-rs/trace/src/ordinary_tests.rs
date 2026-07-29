use std::io::Cursor;

use pretty_assertions::assert_eq;

use super::read_bounded_line;

#[test]
fn bounded_line_retains_exact_limit_and_recovers_after_oversized_line() {
    let mut input = Cursor::new(b"1234\noversized\nok\n");

    let exact = read_bounded_line(&mut input, 4)
        .expect("exact line")
        .expect("line");
    let oversized = read_bounded_line(&mut input, 4)
        .expect("oversized line")
        .expect("line");
    let following = read_bounded_line(&mut input, 4)
        .expect("following line")
        .expect("line");

    assert_eq!((exact.bytes, exact.oversized), (b"1234".to_vec(), false));
    assert_eq!(
        (oversized.bytes, oversized.oversized),
        (b"over".to_vec(), true)
    );
    assert_eq!(
        (following.bytes, following.oversized),
        (b"ok".to_vec(), false)
    );
}

#[test]
fn bounded_line_supports_zero_byte_limit() {
    let mut input = Cursor::new(b"x\n\n");

    let nonempty = read_bounded_line(&mut input, 0)
        .expect("nonempty line")
        .expect("line");
    let empty = read_bounded_line(&mut input, 0)
        .expect("empty line")
        .expect("line");

    assert_eq!((nonempty.bytes, nonempty.oversized), (Vec::new(), true));
    assert_eq!((empty.bytes, empty.oversized), (Vec::new(), false));
}
