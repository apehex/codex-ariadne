//! Source-derived activities consumed by presentation policies.

/// Whether retained command facts support Codex exploration batching.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceExplorationEligibility {
    /// Every parsed command is a supported read, list, or search operation.
    Eligible,
    /// The command is known not to satisfy the exploration policy.
    Ineligible,
    /// The source did not retain enough information to classify the command.
    #[default]
    Unavailable,
}

/// Immediate caller of a retained runtime tool.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceToolRequester {
    /// The model directly requested the tool.
    Model,
    /// A model-authored code cell requested the nested tool.
    CodeCell,
    /// The source did not retain the requester.
    #[default]
    Unknown,
}

/// Typed operation family used by presentation grouping policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceToolActivity {
    /// A new terminal command, with its exploration classification.
    ExecCommand(TraceExplorationEligibility),
    /// Bytes written to a running terminal.
    WriteStdin,
    /// A terminal poll that writes no bytes.
    PollTerminal,
    /// A file patch operation.
    ApplyPatch,
    /// A Model Context Protocol tool.
    Mcp,
    /// A web operation.
    Web,
    /// An image generation or lookup operation.
    ImageGeneration,
    /// A dynamic or unsupported tool family.
    Other,
}

/// Typed agent/control-plane action used by grouping policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceAgentActivity {
    /// Create a child agent.
    Spawn,
    /// Assign or follow up on agent work.
    Assign,
    /// Send an agent message.
    Send,
    /// Wait for one or more agents.
    Wait,
    /// Deliver a child-agent result.
    Result,
    /// Resume a stopped agent.
    Resume,
    /// Close or interrupt an agent.
    Close,
}

/// Typed compaction observation used by grouping policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceCompactionActivity {
    /// Installed history-replacement checkpoint.
    Checkpoint,
    /// Upstream request contributing to a checkpoint.
    Request,
    /// Model-visible or ordinary compaction marker.
    Marker,
}

/// Renderer-neutral activity retained at the source adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceActivity {
    /// A direct runtime tool operation.
    Tool {
        /// Operation family.
        kind: TraceToolActivity,
        /// Immediate requester.
        requester: TraceToolRequester,
    },
    /// A multi-agent/control-plane operation.
    Agent(TraceAgentActivity),
    /// A model-authored code cell.
    CodeCell,
    /// A context compaction observation.
    Compaction(TraceCompactionActivity),
}
