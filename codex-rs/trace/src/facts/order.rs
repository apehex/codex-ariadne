//! Typed source order that never compares incompatible clocks implicitly.

use std::cmp::Ordering;

/// Source domain that gives an order point its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceOrderDomain {
    /// Per-thread ordinary rollout ordinal or line order.
    Ordinary,
    /// Global rich raw-event sequence.
    Rich,
    /// Wall-clock placement used only for structural containers.
    Structural,
    /// No source position is available.
    Unspecified,
}

/// Typed source position that never compares incompatible clocks implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceOrderPoint {
    /// Per-thread ordinary rollout position.
    Ordinary {
        /// Persisted ordinal, or bounded reader line order when absent.
        ordinal: u64,
    },
    /// Rich raw-event-spine position.
    Rich {
        /// Global raw event sequence.
        sequence: u64,
    },
    /// Structural wall-clock placement with no causal meaning.
    Structural {
        /// Milliseconds since the Unix epoch.
        unix_ms: i64,
    },
    /// No source position is available.
    Unspecified,
}

impl TraceOrderPoint {
    /// Returns the source domain represented by this point.
    pub fn domain(self) -> TraceOrderDomain {
        match self {
            Self::Ordinary { .. } => TraceOrderDomain::Ordinary,
            Self::Rich { .. } => TraceOrderDomain::Rich,
            Self::Structural { .. } => TraceOrderDomain::Structural,
            Self::Unspecified => TraceOrderDomain::Unspecified,
        }
    }

    pub(crate) fn same_domain_cmp(self, other: Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Ordinary { ordinal: left }, Self::Ordinary { ordinal: right }) => {
                Some(left.cmp(&right))
            }
            (Self::Rich { sequence: left }, Self::Rich { sequence: right }) => {
                Some(left.cmp(&right))
            }
            (Self::Structural { unix_ms: left }, Self::Structural { unix_ms: right }) => {
                Some(left.cmp(&right))
            }
            (Self::Unspecified, Self::Unspecified) => Some(Ordering::Equal),
            (
                Self::Ordinary { .. }
                | Self::Rich { .. }
                | Self::Structural { .. }
                | Self::Unspecified,
                Self::Ordinary { .. }
                | Self::Rich { .. }
                | Self::Structural { .. }
                | Self::Unspecified,
            ) => None,
        }
    }
}

/// Retained start, end, wall-clock, and deterministic tie-break positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceOrder {
    /// Source position at which the object first became observable.
    pub start: TraceOrderPoint,
    /// Source position at which the object ended, when recorded.
    pub end: Option<TraceOrderPoint>,
    /// Display-only start time in Unix milliseconds.
    pub wall_clock_start_ms: Option<i64>,
    /// Display-only end time in Unix milliseconds.
    pub wall_clock_end_ms: Option<i64>,
    /// Stable admission order used only after equal or incomparable positions.
    pub tie_break: usize,
}

impl Default for TraceOrder {
    fn default() -> Self {
        Self {
            start: TraceOrderPoint::Unspecified,
            end: None,
            wall_clock_start_ms: None,
            wall_clock_end_ms: None,
            tie_break: 0,
        }
    }
}

impl TraceOrder {
    pub(crate) fn ordinary(ordinal: u64, wall_clock_start_ms: Option<i64>) -> Self {
        Self {
            start: TraceOrderPoint::Ordinary { ordinal },
            wall_clock_start_ms,
            ..Self::default()
        }
    }

    pub(crate) fn rich(
        started_seq: u64,
        ended_seq: Option<u64>,
        started_at_unix_ms: i64,
        ended_at_unix_ms: Option<i64>,
    ) -> Self {
        Self {
            start: TraceOrderPoint::Rich {
                sequence: started_seq,
            },
            end: ended_seq.map(|sequence| TraceOrderPoint::Rich { sequence }),
            wall_clock_start_ms: Some(started_at_unix_ms),
            wall_clock_end_ms: ended_at_unix_ms,
            tie_break: 0,
        }
    }

    pub(crate) fn structural(started_at_unix_ms: i64, ended_at_unix_ms: Option<i64>) -> Self {
        Self {
            start: TraceOrderPoint::Structural {
                unix_ms: started_at_unix_ms,
            },
            end: ended_at_unix_ms.map(|unix_ms| TraceOrderPoint::Structural { unix_ms }),
            wall_clock_start_ms: Some(started_at_unix_ms),
            wall_clock_end_ms: ended_at_unix_ms,
            tie_break: 0,
        }
    }

    pub(crate) fn stable_cmp(&self, other: &Self) -> Ordering {
        self.start
            .same_domain_cmp(other.start)
            .unwrap_or(Ordering::Equal)
            .then(self.tie_break.cmp(&other.tie_break))
    }
}

/// Source ownership needed to partition order and grouping facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceOwnership {
    /// Source-authored thread identity, when the record belongs to a thread.
    pub thread_id: Option<String>,
    /// Source-authored Codex turn identity, when recorded.
    pub turn_id: Option<String>,
}

#[cfg(test)]
#[path = "order_tests.rs"]
mod tests;
