//! Renderer-neutral presentation groups, order bands, and diagnostics.

use crate::EvidenceGrade;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrderDomain;
use crate::TraceRelation;
use crate::TraceStatus;

/// Version of the deterministic presentation grouping policy.
pub const PRESENTATION_POLICY_VERSION: u16 = 1;

/// Snapshot-local scope that owns presentation order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PresentationScope {
    /// Events whose source does not record a thread owner.
    Session,
    /// Events owned by one source-authored thread.
    Thread(String),
}

/// Stable identity for one group within an immutable presentation snapshot.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupId {
    /// A fallback group owned by exactly one canonical node.
    Singleton(TraceNodeLocator),
    /// A lifecycle joined through durable source correlation.
    Correlated {
        /// Owning source thread.
        thread_id: String,
        /// Presentation family selected by the policy.
        kind: GroupKind,
        /// Durable identity anchoring the lifecycle.
        correlation: TraceObjectRef,
        /// Source-order occurrence when an identity is reused.
        occurrence: u32,
    },
    /// A versioned higher-order group containing existing child groups.
    Batch {
        /// Owning source thread.
        thread_id: String,
        /// Higher-order presentation family.
        kind: GroupKind,
        /// Policy version that selected the children.
        policy_version: u16,
        /// Stable identity of the first child that anchored the batch.
        first_child: Box<GroupId>,
    },
}

/// Minimal renderer-neutral presentation family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupKind {
    /// User-authored message.
    UserMessage,
    /// Assistant message without a narrower channel.
    AssistantMessage,
    /// Assistant commentary.
    Commentary,
    /// Assistant final answer.
    FinalAnswer,
    /// Model reasoning or a reasoning summary.
    Reasoning,
    /// System-authored context.
    SystemContext,
    /// Developer-authored context.
    DeveloperContext,
    /// Direct model/runtime tool lifecycle.
    DirectTool,
    /// Consecutive read, list, and search tool lifecycles.
    ExplorationBatch,
    /// Code that lacks a supported direct-tool correlation.
    Code,
    /// Agent/control-plane event deferred to agent-specific policy.
    Delegation,
    /// Compaction event deferred to compaction-specific policy.
    Compaction,
    /// Diagnostic or conflicting evidence.
    Diagnostic,
    /// Event-like structural record that is not a canonical container.
    StructuralRecord,
    /// Visible fallback for an unsupported event family.
    Unknown,
}

/// Typed operation summarized by a presentation group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupActivity {
    /// A terminal command.
    ExecCommand,
    /// Bytes written to a terminal.
    WriteStdin,
    /// A terminal poll.
    PollTerminal,
    /// A file patch.
    ApplyPatch,
    /// A Model Context Protocol tool.
    Mcp,
    /// A web operation.
    Web,
    /// Image generation or lookup.
    ImageGeneration,
    /// A model-authored code cell.
    CodeCell,
    /// Child-agent creation.
    AgentSpawn,
    /// Agent assignment or follow-up.
    AgentAssign,
    /// Agent messaging.
    AgentSend,
    /// Agent waiting.
    AgentWait,
    /// Child-agent result delivery.
    AgentResult,
    /// Agent resumption.
    AgentResume,
    /// Agent close or interruption.
    AgentClose,
    /// Context compaction.
    Compaction,
    /// Higher-order exploration batch.
    Exploration,
    /// Unsupported or dynamic tool operation.
    OtherTool,
}

/// Disposition of one canonical node in presentation views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationDisposition {
    /// The node belongs to exactly one primary group.
    Primary,
    /// The node is reachable as evidence but has no primary group.
    ReferenceOnly,
    /// The node remains exclusively in canonical structural navigation.
    StructuralOnly,
}

/// Semantic role of a canonical node inside one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMemberRole {
    /// Model or caller invocation.
    Invocation,
    /// Model-visible input item.
    ModelVisibleInput,
    /// Runtime lifecycle start or duration-bearing runtime object.
    RuntimeStart,
    /// Runtime-produced intermediate output.
    RuntimeOutput,
    /// Runtime lifecycle end.
    RuntimeEnd,
    /// Result returned to the model.
    ModelVisibleResult,
    /// User- or assistant-visible message content.
    MessageContent,
    /// Reasoning content.
    ReasoningContent,
    /// Summary content.
    Summary,
    /// Supporting source evidence.
    AuxiliaryEvidence,
    /// Observation that conflicts with another member.
    ConflictEvidence,
    /// Non-owning reference.
    Reference,
}

/// One primary canonical member of a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMember {
    /// Position in the immutable `SessionTrace::nodes` snapshot.
    pub node_position: usize,
    /// Presentation role of this observation.
    pub role: GroupMemberRole,
}

/// One bounded secondary relationship from a group to canonical evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupReference {
    /// Semantic meaning of the source relationship.
    pub relation: TraceRelation,
    /// Stable referenced source object.
    pub target: TraceObjectRef,
    /// Resolved canonical position, or `None` when retained evidence is absent.
    pub node_position: Option<usize>,
}

/// Conservative aggregate over member values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupAggregate<T> {
    /// No member exposed the value.
    Unavailable,
    /// Every retained value agrees.
    Value(T),
    /// Retained values disagree.
    Conflicting,
}

/// Counts of canonical evidence grades represented by a group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupEvidenceCounts {
    /// Exact observations.
    pub exact: usize,
    /// Semantic observations.
    pub semantic: usize,
    /// Reconstructed observations.
    pub reconstructed: usize,
    /// Unavailable observations.
    pub unavailable: usize,
    /// Conflicting observations.
    pub conflicting: usize,
}

impl GroupEvidenceCounts {
    pub(crate) fn record(&mut self, evidence: EvidenceGrade) {
        match evidence {
            EvidenceGrade::Exact => self.exact += 1,
            EvidenceGrade::Semantic => self.semantic += 1,
            EvidenceGrade::Reconstructed => self.reconstructed += 1,
            EvidenceGrade::Unavailable => self.unavailable += 1,
            EvidenceGrade::Conflicting => self.conflicting += 1,
        }
    }
}

/// Whether retained members establish a complete lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupCompleteness {
    /// Retained evidence establishes the expected lifecycle.
    Complete,
    /// A live lifecycle has started but has not ended.
    Running,
    /// Expected evidence is absent, truncated, or conflicting.
    Partial,
    /// Persisted evidence cannot establish whether the lifecycle ended.
    Indeterminate,
}

/// Origin of presentation facts contributing to a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupOrigin {
    /// Derived exclusively from persisted evidence.
    Persisted,
    /// Derived exclusively from transient live facts.
    Live,
    /// Derived from persisted and live facts.
    Mixed,
}

/// Default inclusion of a group in the collapsed presentation lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupVisibility {
    /// Show the group by default.
    Shown,
    /// Retain the group but hide it until its family is enabled.
    Hidden,
}

/// Typed bounded metadata summarized from group members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMetadata {
    /// Conservative aggregate typed operation.
    pub activity: GroupAggregate<GroupActivity>,
    /// Bounded operation or record label.
    pub label: Option<String>,
    /// Bounded first useful member preview.
    pub preview: Option<String>,
    /// Conservative aggregate runtime status.
    pub status: GroupAggregate<TraceStatus>,
    /// Conservative aggregate member duration in milliseconds.
    pub duration_ms: GroupAggregate<u64>,
    /// Number of direct primary members.
    pub member_count: usize,
    /// Number of direct child groups.
    pub child_count: usize,
    /// Number of retained secondary references.
    pub reference_count: usize,
    /// Counts of supporting canonical evidence grades.
    pub evidence: GroupEvidenceCounts,
}

/// Snapshot-local identifier for one order band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderBandId(pub usize);

/// Relationship between the source segments in one order band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderBandKind {
    /// One source segment has ordinary or rich causal order.
    Ordered,
    /// Stable identities align observations from different source domains.
    Aligned,
    /// Source segments are ordered internally but incomparable to each other.
    Unordered,
    /// No causal source position is available.
    Unpositioned,
}

/// One internally ordered source segment inside an order band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSegment {
    /// Source order domain for this segment.
    pub domain: TraceOrderDomain,
    /// Canonical primary positions in source order.
    pub node_positions: Vec<usize>,
}

/// One canonical partial-order band in a thread or session scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationOrderBand {
    /// Snapshot-local band identity.
    pub id: OrderBandId,
    /// Scope owning the events.
    pub scope: PresentationScope,
    /// Relationship between contained source segments.
    pub kind: OrderBandKind,
    /// Internally ordered source segments.
    pub segments: Vec<OrderSegment>,
}

/// One row in canonical expanded-event order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationEvent {
    /// Canonical node position.
    pub node_position: usize,
    /// Exactly one primary group.
    pub primary_group: GroupId,
    /// Partial-order band containing this observation.
    pub band: OrderBandId,
}

/// One immutable renderer-neutral presentation group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationGroup {
    /// Snapshot-stable group identity.
    pub id: GroupId,
    /// Presentation family.
    pub kind: GroupKind,
    /// Thread or session scope owning this group.
    pub scope: PresentationScope,
    /// Earliest canonical order band containing a direct member.
    pub anchor_band: OrderBandId,
    /// Direct canonical primary members in event order.
    pub members: Vec<GroupMember>,
    /// Higher-order child groups; empty in the minimal Phase 2 policy.
    pub child_groups: Vec<GroupId>,
    /// Bounded non-owning links to supporting canonical evidence.
    pub references: Vec<GroupReference>,
    /// Bounded aggregate metadata.
    pub metadata: GroupMetadata,
    /// Lifecycle completeness supported by retained evidence.
    pub completeness: GroupCompleteness,
    /// Source origin of the presentation facts.
    pub origin: GroupOrigin,
    /// Default collapsed-lens visibility.
    pub default_visibility: GroupVisibility,
}

/// Stable category for a derived presentation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationDiagnosticCode {
    /// A required primary group could not be admitted.
    PrimaryMembership,
    /// A reference target was not retained.
    MissingReference,
    /// One correlation crossed thread scopes.
    CrossThreadCorrelation,
    /// A reused identity could not be assigned safely.
    AmbiguousCorrelation,
    /// Source alignment anchors crossed or disagreed.
    OrderAlignment,
    /// A structural resource budget was exhausted.
    ResourceLimit,
    /// Constructed group relationships failed validation.
    InvalidStructure,
}

/// One bounded non-fatal presentation construction problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationDiagnostic {
    /// Stable diagnostic category.
    pub code: PresentationDiagnosticCode,
    /// Affected group when known.
    pub group_id: Option<GroupId>,
    /// Affected canonical position when known.
    pub node_position: Option<usize>,
    /// Bounded explanation without source contents.
    pub message: String,
}

/// Whether required presentation invariants were fully materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationBuildStatus {
    /// Every primary event and required structure was admitted.
    Complete,
    /// Canonical evidence remains available but presentation is incomplete.
    Incomplete,
}
