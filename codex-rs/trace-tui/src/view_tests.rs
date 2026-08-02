use pretty_assertions::assert_eq;

use super::HorizontalStep;
use super::LogicalContentWidth;

#[test]
fn numeric_view_options_reject_zero_and_values_above_their_safety_bounds() {
    assert_eq!(HorizontalStep::new(0), None);
    assert_eq!(HorizontalStep::new(HorizontalStep::MAX + 1), None);
    assert_eq!(HorizontalStep::new(7).map(HorizontalStep::columns), Some(7));

    assert_eq!(LogicalContentWidth::new(0), None);
    assert_eq!(LogicalContentWidth::new(LogicalContentWidth::MAX + 1), None);
    assert_eq!(
        LogicalContentWidth::new(2_048).map(LogicalContentWidth::columns),
        Some(2_048)
    );
}
