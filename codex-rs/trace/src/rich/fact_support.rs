//! Small constructors shared by rich semantic fact projectors.

use codex_rollout_trace::ExecutionWindow;

use crate::TraceCorrelation;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceRelation;

pub(super) fn execution_order(execution: &ExecutionWindow) -> TraceOrder {
    TraceOrder::rich(
        execution.started_seq,
        execution.ended_seq,
        execution.started_at_unix_ms,
        execution.ended_at_unix_ms,
    )
}

pub(super) fn identity(target: TraceObjectRef) -> Vec<TraceCorrelation> {
    vec![link(TraceRelation::SourceIdentity, target)]
}

pub(super) fn link(relation: TraceRelation, target: TraceObjectRef) -> TraceCorrelation {
    TraceCorrelation::new(relation, target)
}
