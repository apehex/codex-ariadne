//! Typed source identities and relationships between trace objects.

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
