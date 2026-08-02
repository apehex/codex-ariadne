use pretty_assertions::assert_eq;

use super::*;

#[test]
fn exploration_requires_only_read_list_or_search_commands() {
    assert_eq!(
        exploration(&[
            "bash".to_string(),
            "-lc".to_string(),
            "rg TODO && ls".to_string()
        ]),
        TraceExplorationEligibility::Eligible,
    );
    assert_eq!(
        exploration(&[
            "bash".to_string(),
            "-lc".to_string(),
            "rg TODO && cargo check".to_string()
        ]),
        TraceExplorationEligibility::Ineligible,
    );
}

#[test]
fn named_agent_tools_retain_resume_separately() {
    assert_eq!(
        named_tool("resume_agent"),
        TraceActivity::Agent(TraceAgentActivity::Resume),
    );
    assert_eq!(
        named_tool("future_tool"),
        TraceActivity::Tool {
            kind: TraceToolActivity::Other,
            requester: TraceToolRequester::Model,
        },
    );
}

#[test]
fn named_tool_families_have_exhaustive_stable_classification() {
    let cases = [
        (
            "spawn_agent",
            TraceActivity::Agent(TraceAgentActivity::Spawn),
        ),
        (
            "followup_task",
            TraceActivity::Agent(TraceAgentActivity::Assign),
        ),
        (
            "send_message",
            TraceActivity::Agent(TraceAgentActivity::Send),
        ),
        ("wait_agent", TraceActivity::Agent(TraceAgentActivity::Wait)),
        (
            "close_agent",
            TraceActivity::Agent(TraceAgentActivity::Close),
        ),
        (
            "apply_patch",
            TraceActivity::Tool {
                kind: TraceToolActivity::ApplyPatch,
                requester: TraceToolRequester::Model,
            },
        ),
        (
            "web_search",
            TraceActivity::Tool {
                kind: TraceToolActivity::Web,
                requester: TraceToolRequester::Model,
            },
        ),
        (
            "imagegen",
            TraceActivity::Tool {
                kind: TraceToolActivity::ImageGeneration,
                requester: TraceToolRequester::Model,
            },
        ),
    ];
    assert_eq!(
        cases
            .into_iter()
            .map(|(name, expected)| (named_tool(name), expected))
            .collect::<Vec<_>>(),
        cases
            .into_iter()
            .map(|(_, expected)| (expected, expected))
            .collect::<Vec<_>>(),
    );
}
