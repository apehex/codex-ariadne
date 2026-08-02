//! Minimal Phase 2 presentation dispositions and singleton policy.

use crate::EvidenceGrade;
use crate::GroupKind;
use crate::GroupMemberRole;
use crate::GroupVisibility;
use crate::PresentationDisposition;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceRecordClass;

pub(crate) fn disposition(node: &TraceNode) -> PresentationDisposition {
    match node.locator.kind {
        TraceNodeKind::Session
        | TraceNodeKind::Thread
        | TraceNodeKind::Turn
        | TraceNodeKind::Inference => PresentationDisposition::StructuralOnly,
        TraceNodeKind::RawPayload | TraceNodeKind::TerminalSession => {
            PresentationDisposition::ReferenceOnly
        }
        TraceNodeKind::ConversationItem
        | TraceNodeKind::ToolCall
        | TraceNodeKind::CodeCell
        | TraceNodeKind::TerminalOperation
        | TraceNodeKind::Compaction
        | TraceNodeKind::CompactionRequest
        | TraceNodeKind::InteractionEdge
        | TraceNodeKind::RolloutRecord
        | TraceNodeKind::Diagnostic => PresentationDisposition::Primary,
    }
}

pub(crate) fn group_kind(node: &TraceNode) -> GroupKind {
    match node.presentation.class {
        TraceRecordClass::System => GroupKind::SystemContext,
        TraceRecordClass::Developer => GroupKind::DeveloperContext,
        TraceRecordClass::User => GroupKind::UserMessage,
        TraceRecordClass::Assistant => GroupKind::AssistantMessage,
        TraceRecordClass::Commentary => GroupKind::Commentary,
        TraceRecordClass::FinalAnswer => GroupKind::FinalAnswer,
        TraceRecordClass::Reasoning => GroupKind::Reasoning,
        TraceRecordClass::ToolInput | TraceRecordClass::ToolOutput => GroupKind::DirectTool,
        TraceRecordClass::Code => GroupKind::Code,
        TraceRecordClass::Delegation => GroupKind::Delegation,
        TraceRecordClass::Compaction => GroupKind::Compaction,
        TraceRecordClass::Diagnostic => GroupKind::Diagnostic,
        TraceRecordClass::Structure => GroupKind::StructuralRecord,
        TraceRecordClass::RawArtifact | TraceRecordClass::Other => GroupKind::Unknown,
    }
}

pub(crate) fn member_role(node: &TraceNode) -> GroupMemberRole {
    if node.evidence == EvidenceGrade::Conflicting {
        return GroupMemberRole::ConflictEvidence;
    }
    match node.presentation.class {
        TraceRecordClass::User
        | TraceRecordClass::Assistant
        | TraceRecordClass::Commentary
        | TraceRecordClass::FinalAnswer
        | TraceRecordClass::System
        | TraceRecordClass::Developer => GroupMemberRole::MessageContent,
        TraceRecordClass::Reasoning => GroupMemberRole::ReasoningContent,
        TraceRecordClass::ToolInput => GroupMemberRole::ModelVisibleInput,
        TraceRecordClass::ToolOutput => GroupMemberRole::ModelVisibleResult,
        TraceRecordClass::Code => GroupMemberRole::Invocation,
        TraceRecordClass::Structure
        | TraceRecordClass::Delegation
        | TraceRecordClass::Compaction
        | TraceRecordClass::Diagnostic
        | TraceRecordClass::RawArtifact
        | TraceRecordClass::Other => match node.locator.kind {
            TraceNodeKind::ToolCall
            | TraceNodeKind::CodeCell
            | TraceNodeKind::TerminalOperation => GroupMemberRole::RuntimeStart,
            TraceNodeKind::TerminalSession
            | TraceNodeKind::Session
            | TraceNodeKind::Thread
            | TraceNodeKind::Turn
            | TraceNodeKind::Inference
            | TraceNodeKind::ConversationItem
            | TraceNodeKind::Compaction
            | TraceNodeKind::CompactionRequest
            | TraceNodeKind::InteractionEdge
            | TraceNodeKind::RawPayload
            | TraceNodeKind::RolloutRecord
            | TraceNodeKind::Diagnostic => GroupMemberRole::AuxiliaryEvidence,
        },
    }
}

pub(crate) fn default_visibility(kind: GroupKind) -> GroupVisibility {
    match kind {
        GroupKind::SystemContext
        | GroupKind::DeveloperContext
        | GroupKind::Compaction
        | GroupKind::StructuralRecord => GroupVisibility::Hidden,
        GroupKind::UserMessage
        | GroupKind::AssistantMessage
        | GroupKind::Commentary
        | GroupKind::FinalAnswer
        | GroupKind::Reasoning
        | GroupKind::DirectTool
        | GroupKind::ExplorationBatch
        | GroupKind::Code
        | GroupKind::Delegation
        | GroupKind::Diagnostic
        | GroupKind::Unknown => GroupVisibility::Shown,
    }
}

pub(crate) fn direct_tool_candidate(node: &TraceNode) -> bool {
    matches!(
        node.presentation.class,
        TraceRecordClass::ToolInput
            | TraceRecordClass::ToolOutput
            | TraceRecordClass::Code
            | TraceRecordClass::Delegation
    ) || matches!(
        node.locator.kind,
        TraceNodeKind::ToolCall
            | TraceNodeKind::CodeCell
            | TraceNodeKind::TerminalOperation
            | TraceNodeKind::InteractionEdge
    )
}
