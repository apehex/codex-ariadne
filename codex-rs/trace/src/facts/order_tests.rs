use std::cmp::Ordering;

use pretty_assertions::assert_eq;

use super::TraceOrder;
use super::TraceOrderPoint;

#[test]
fn stable_comparison_orders_structural_points_before_using_tie_breaks() {
    let earlier = TraceOrder {
        start: TraceOrderPoint::Structural { unix_ms: 10 },
        tie_break: 9,
        ..TraceOrder::default()
    };
    let later = TraceOrder {
        start: TraceOrderPoint::Structural { unix_ms: 20 },
        tie_break: 1,
        ..TraceOrder::default()
    };

    assert_eq!(earlier.stable_cmp(&later), Ordering::Less);
}

#[test]
fn stable_comparison_uses_tie_break_for_incompatible_or_unspecified_points() {
    let ordinary = TraceOrder {
        start: TraceOrderPoint::Ordinary { ordinal: 99 },
        tie_break: 1,
        ..TraceOrder::default()
    };
    let rich = TraceOrder {
        start: TraceOrderPoint::Rich { sequence: 1 },
        tie_break: 2,
        ..TraceOrder::default()
    };
    let unspecified_first = TraceOrder {
        tie_break: 3,
        ..TraceOrder::default()
    };
    let unspecified_second = TraceOrder {
        tie_break: 4,
        ..TraceOrder::default()
    };

    assert_eq!(ordinary.stable_cmp(&rich), Ordering::Less);
    assert_eq!(
        unspecified_first.stable_cmp(&unspecified_second),
        Ordering::Less
    );
}
