use codex_trace::GroupId;
use codex_trace::GroupKind;
use codex_trace::OrderBandId;
use codex_trace::TraceObjectRef;
use pretty_assertions::assert_eq;

use super::row_gutter;
use crate::browser::rows::BrowserRow;

#[test]
fn event_gutters_mark_complete_and_interrupted_group_spans() {
    let group = GroupId::Correlated {
        thread_id: "thread".to_string(),
        kind: GroupKind::DirectTool,
        correlation: TraceObjectRef::ToolCall("call".to_string()),
        occurrence: 0,
    };
    let first = event(group.clone(), 0);
    let middle = event(group.clone(), 1);
    let last = event(group, 2);
    let other = event(
        GroupId::Correlated {
            thread_id: "thread".to_string(),
            kind: GroupKind::DirectTool,
            correlation: TraceObjectRef::ToolCall("other".to_string()),
            occurrence: 0,
        },
        3,
    );

    assert_eq!(
        row_gutter(&first, None, Some(&middle), false)[1].content,
        "╭"
    );
    assert_eq!(
        row_gutter(&middle, Some(&first), Some(&last), true)[1].content,
        "│"
    );
    assert_eq!(
        row_gutter(&last, Some(&middle), None, false)[1].content,
        "╰"
    );
    assert_eq!(
        row_gutter(&first, Some(&other), Some(&other), false)[1].content,
        "•"
    );
}

fn event(group: GroupId, position: usize) -> BrowserRow {
    BrowserRow::Event {
        node_position: position,
        group,
        band: OrderBandId(position),
    }
}
