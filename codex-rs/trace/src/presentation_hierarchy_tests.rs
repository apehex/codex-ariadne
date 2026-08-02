use pretty_assertions::assert_eq;

use super::*;
use crate::GroupEvidenceCounts;
use crate::OrderBandId;
use crate::TraceStatus;

#[test]
fn exploration_reducer_is_chunk_boundary_independent() {
    let groups = [
        tool_group("a", /*band*/ 0),
        tool_group("b", /*band*/ 1),
        tool_group("c", /*band*/ 2),
    ];
    let scope = PresentationScope::Thread("root".to_string());
    let mut once = ExplorationReducer::default();
    once.start_scope(scope.clone());
    once.extend(groups.iter().map(|group| (group, true)));
    once.finish_scope();

    let mut chunked = ExplorationReducer::default();
    chunked.start_scope(scope);
    chunked.extend(groups[..1].iter().map(|group| (group, true)));
    chunked.extend(groups[1..].iter().map(|group| (group, true)));
    chunked.finish_scope();

    assert_eq!(once.finish(), chunked.finish());
}

fn tool_group(id: &str, band: usize) -> PresentationGroup {
    PresentationGroup {
        id: GroupId::Correlated {
            thread_id: "root".to_string(),
            kind: GroupKind::DirectTool,
            correlation: TraceObjectRef::ToolCall(id.to_string()),
            occurrence: 0,
        },
        kind: GroupKind::DirectTool,
        scope: PresentationScope::Thread("root".to_string()),
        anchor_band: OrderBandId(band),
        members: Vec::new(),
        child_groups: Vec::new(),
        references: Vec::new(),
        metadata: GroupMetadata {
            activity: GroupAggregate::Value(GroupActivity::ExecCommand),
            label: Some(id.to_string()),
            preview: None,
            status: GroupAggregate::Value(TraceStatus::Completed),
            duration_ms: GroupAggregate::Unavailable,
            member_count: 0,
            child_count: 0,
            reference_count: 0,
            evidence: GroupEvidenceCounts::default(),
        },
        completeness: GroupCompleteness::Complete,
        origin: GroupOrigin::Persisted,
        default_visibility: GroupVisibility::Shown,
    }
}
