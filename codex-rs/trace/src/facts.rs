//! Typed source order and correlation facts for retained trace nodes.

use std::cmp::Ordering;

pub(crate) const MAX_CORRELATIONS_PER_NODE: usize = 4_096;

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
}

/// Source ownership needed to partition order and grouping facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceOwnership {
    /// Source-authored thread identity, when the record belongs to a thread.
    pub thread_id: Option<String>,
    /// Source-authored Codex turn identity, when recorded.
    pub turn_id: Option<String>,
}

/// Completeness of the typed order and correlation facts for one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceFactAvailability {
    /// The contributing source exposed every supported fact.
    Complete,
    /// Some supported facts were absent or bounded away.
    Partial,
    /// The source record could not be interpreted into typed facts.
    #[default]
    Unavailable,
    /// Retained observations disagree about the same source identity.
    Conflicting,
}

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

/// Small typed facts consumed only by presentation policies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TracePolicyFacts {
    /// Source-derived activity, or `None` when the record has no supported policy role.
    pub activity: Option<TraceActivity>,
}

/// Typed identity referenced by a correlation relation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceObjectRef {
    /// Agent thread identity.
    Thread(String),
    /// Codex runtime turn identity.
    Turn(String),
    /// Model-visible conversation item identity.
    ConversationItem(String),
    /// Upstream inference-call identity.
    Inference(String),
    /// Runtime tool-call identity.
    ToolCall(String),
    /// Model-visible protocol call identity.
    ModelVisibleCall(String),
    /// MCP backend call identity.
    McpCall(String),
    /// Code-mode runtime tool identity.
    CodeModeRuntimeTool(String),
    /// Model-authored code-cell identity.
    CodeCell(String),
    /// Terminal process or session identity.
    Terminal(String),
    /// Terminal operation identity.
    TerminalOperation(String),
    /// Installed compaction identity.
    Compaction(String),
    /// Compaction request identity.
    CompactionRequest(String),
    /// Inter-agent interaction edge identity.
    InteractionEdge(String),
    /// Raw payload identity.
    RawPayload(String),
    /// A user-input producer without a separate runtime object.
    UserInput,
    /// A harness producer without a separate runtime object.
    Harness,
}

/// Semantic role connecting one node to a referenced trace object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraceRelation {
    /// Stable source identity represented by this node.
    SourceIdentity,
    /// Recorded parent thread.
    ParentThread,
    /// Interaction edge that spawned a thread.
    SpawnEdge,
    /// Conversation item that activated a turn.
    TurnInput,
    /// Runtime or control-plane producer of a conversation item.
    Producer,
    /// Model-visible call identity carried by an item or runtime tool.
    ModelVisibleCall,
    /// Conversation item in an inference request.
    InferenceInput,
    /// Conversation item produced by an inference response.
    InferenceOutput,
    /// Runtime tool started by an inference response.
    InferenceStartedTool,
    /// Model-visible call item associated with a runtime tool.
    ModelVisibleCallItem,
    /// Model-visible output item associated with a runtime tool.
    ModelVisibleOutputItem,
    /// Code cell that requested a nested runtime tool.
    RequestingCodeCell,
    /// Source item containing model-authored code.
    CodeSource,
    /// Output item produced by a code cell.
    CodeOutput,
    /// Nested runtime tool invoked by a code cell.
    NestedTool,
    /// Wait tool associated with a code cell.
    WaitTool,
    /// Terminal operation associated with a runtime tool.
    ToolTerminalOperation,
    /// Terminal operation that created a terminal session.
    CreatedByTerminalOperation,
    /// Operation belonging to a terminal session.
    TerminalSessionOperation,
    /// Terminal session observed by an operation.
    TerminalSession,
    /// Model-visible call item that observed terminal activity.
    TerminalObservationCall,
    /// Model-visible output item that observed terminal activity.
    TerminalObservationOutput,
    /// Request contributing to a compaction.
    CompactionRequest,
    /// Structural marker installed by a compaction.
    CompactionMarker,
    /// Item present before compaction replacement.
    CompactionInput,
    /// Item installed by compaction replacement.
    CompactionReplacement,
    /// Compaction owning one request.
    OwningCompaction,
    /// Runtime tool owning one nested runtime object.
    OwningTool,
    /// Source endpoint of an interaction edge.
    InteractionSource,
    /// Target endpoint of an interaction edge.
    InteractionTarget,
    /// Conversation item carried by an interaction edge.
    InteractionCarriedItem,
    /// Raw payload associated with a typed object.
    RawPayload,
}

/// One typed relationship retained for grouping or evidence navigation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceCorrelation {
    /// Meaning of the relationship.
    pub relation: TraceRelation,
    /// Referenced source object.
    pub target: TraceObjectRef,
}

impl TraceCorrelation {
    pub(crate) fn new(relation: TraceRelation, target: TraceObjectRef) -> Self {
        Self { relation, target }
    }
}

/// Typed order, ownership, and correlation facts for one retained node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceNodeFacts {
    /// Source order and display time.
    pub order: TraceOrder,
    /// Thread and turn ownership.
    pub ownership: TraceOwnership,
    /// Whether supported facts were completely retained.
    pub availability: TraceFactAvailability,
    /// Bounded typed relations to other source objects.
    pub correlations: Vec<TraceCorrelation>,
    /// Typed renderer-neutral facts needed by grouping policies.
    pub policy: TracePolicyFacts,
}

impl TraceNodeFacts {
    pub(crate) fn new(
        order: TraceOrder,
        ownership: TraceOwnership,
        availability: TraceFactAvailability,
        correlations: impl IntoIterator<Item = TraceCorrelation>,
    ) -> Self {
        let mut correlations = correlations.into_iter();
        let retained = correlations
            .by_ref()
            .take(MAX_CORRELATIONS_PER_NODE)
            .collect::<Vec<_>>();
        let truncated = correlations.next().is_some();
        Self {
            order,
            ownership,
            availability: if truncated {
                TraceFactAvailability::Partial
            } else {
                availability
            },
            correlations: retained,
            policy: TracePolicyFacts::default(),
        }
    }

    /// Attaches one typed activity while preserving order and correlation facts.
    pub(crate) fn with_activity(mut self, activity: TraceActivity) -> Self {
        self.policy.activity = Some(activity);
        self
    }

    /// Compares causal source positions only when their domains are compatible.
    pub fn source_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.order.start, other.order.start) {
            (
                TraceOrderPoint::Ordinary { ordinal: left },
                TraceOrderPoint::Ordinary { ordinal: right },
            ) if self.ownership.thread_id == other.ownership.thread_id
                && self.ownership.thread_id.is_some() =>
            {
                Some(left.cmp(&right))
            }
            (
                TraceOrderPoint::Rich { sequence: left },
                TraceOrderPoint::Rich { sequence: right },
            ) => Some(left.cmp(&right)),
            (
                TraceOrderPoint::Ordinary { .. }
                | TraceOrderPoint::Rich { .. }
                | TraceOrderPoint::Structural { .. }
                | TraceOrderPoint::Unspecified,
                TraceOrderPoint::Ordinary { .. }
                | TraceOrderPoint::Rich { .. }
                | TraceOrderPoint::Structural { .. }
                | TraceOrderPoint::Unspecified,
            ) => None,
        }
    }
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod tests;
