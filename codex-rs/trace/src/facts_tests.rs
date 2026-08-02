use std::cmp::Ordering;

use pretty_assertions::assert_eq;

use super::MAX_CORRELATIONS_PER_NODE;
use super::TraceFactAvailability;
use super::TraceNodeFacts;
use super::TraceObjectRef;
use super::TraceOrder;
use super::TraceOwnership;
use super::TraceRelation;
use crate::TraceCorrelation;

#[test]
fn source_comparison_respects_order_domains_and_thread_ownership() {
    let ordinary_first = ordinary("thread-a", /*ordinal*/ 1);
    let ordinary_second = ordinary("thread-a", /*ordinal*/ 2);
    let other_thread = ordinary("thread-b", /*ordinal*/ 2);
    let rich_first = rich(/*sequence*/ 3);

    assert_eq!(
        ordinary_first.source_cmp(&ordinary_second),
        Some(Ordering::Less)
    );
    assert_eq!(ordinary_first.source_cmp(&other_thread), None);
    assert_eq!(ordinary_first.source_cmp(&rich_first), None);
    assert_eq!(
        rich_first.source_cmp(&rich(/*sequence*/ 4)),
        Some(Ordering::Less)
    );
}

#[test]
fn per_node_correlation_limit_marks_truncated_facts_partial() {
    let facts = TraceNodeFacts::new(
        TraceOrder::default(),
        TraceOwnership::default(),
        TraceFactAvailability::Complete,
        (0..=MAX_CORRELATIONS_PER_NODE).map(|index| {
            TraceCorrelation::new(
                TraceRelation::RawPayload,
                TraceObjectRef::RawPayload(index.to_string()),
            )
        }),
    );

    assert_eq!(facts.correlations.len(), MAX_CORRELATIONS_PER_NODE);
    assert_eq!(facts.availability, TraceFactAvailability::Partial);
}

fn ordinary(thread_id: &str, ordinal: u64) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::ordinary(ordinal, /*wall_clock_start_ms*/ None),
        TraceOwnership {
            thread_id: Some(thread_id.to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Partial,
        [],
    )
}

fn rich(sequence: u64) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::rich(
            sequence, /*ended_seq*/ None, /*started_at_unix_ms*/ 0,
            /*ended_at_unix_ms*/ None,
        ),
        TraceOwnership::default(),
        TraceFactAvailability::Complete,
        [],
    )
}
