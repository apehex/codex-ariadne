use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::RolloutItem;
use pretty_assertions::assert_eq;

use super::record;
use crate::TraceActivity;
use crate::TraceCorrelation;
use crate::TraceExplorationEligibility;
use crate::TraceFactAvailability;
use crate::TraceNodeFacts;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOwnership;
use crate::TracePolicyFacts;
use crate::TraceRelation;
use crate::TraceToolActivity;
use crate::TraceToolRequester;

#[test]
fn response_items_retain_ordinary_order_ownership_and_call_identity() {
    let item = RolloutItem::ResponseItem(ResponseItem::FunctionCall {
        id: None,
        name: "shell".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        encrypted_function_args: None,
        call_id: "call-1".to_string(),
        internal_chat_message_metadata_passthrough: None,
    });

    assert_eq!(
        record(
            &item,
            "thread",
            /*ordinal*/ 7,
            /*wall_clock_start_ms*/ Some(99),
            Some("turn"),
        ),
        TraceNodeFacts {
            order: TraceOrder::ordinary(/*ordinal*/ 7, Some(99)),
            ownership: TraceOwnership {
                thread_id: Some("thread".to_string()),
                turn_id: Some("turn".to_string()),
            },
            availability: TraceFactAvailability::Partial,
            correlations: vec![TraceCorrelation {
                relation: TraceRelation::ModelVisibleCall,
                target: TraceObjectRef::ModelVisibleCall("call-1".to_string()),
            }],
            policy: TracePolicyFacts {
                activity: Some(TraceActivity::Tool {
                    kind: TraceToolActivity::ExecCommand(TraceExplorationEligibility::Unavailable,),
                    requester: TraceToolRequester::Model,
                }),
            },
        }
    );
}
