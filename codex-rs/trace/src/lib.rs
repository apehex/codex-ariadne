//! Read-only discovery and projection of local Codex execution traces.
//!
//! Ordinary rollout JSONL and opt-in rich rollout-trace bundles are normalized
//! into the same stable node model. This crate never repairs, appends, indexes,
//! or otherwise mutates its input stores.

#![warn(missing_docs)]

mod catalog;
mod facts;
mod graph;
mod index;
mod model;
mod ordinary;
mod payload;
mod presentation;
mod rich;
mod search;

pub use catalog::TraceRepository;
pub use facts::TraceActivity;
pub use facts::TraceAgentActivity;
pub use facts::TraceCompactionActivity;
pub use facts::TraceCorrelation;
pub use facts::TraceExplorationEligibility;
pub use facts::TraceFactAvailability;
pub use facts::TraceFactIndex;
pub use facts::TraceNodeFacts;
pub use facts::TraceObjectRef;
pub use facts::TraceOrder;
pub use facts::TraceOrderDomain;
pub use facts::TraceOrderPoint;
pub use facts::TraceOwnership;
pub use facts::TracePolicyFacts;
pub use facts::TraceRelation;
pub use facts::TraceToolActivity;
pub use facts::TraceToolRequester;
pub(crate) use graph::Admission;
pub(crate) use graph::TraceGraphBuilder;
pub use index::TraceIndex;
pub use model::EvidenceGrade;
pub use model::RawPayloadHandle;
pub use model::SearchHit;
pub use model::SessionSummary;
pub use model::SessionTrace;
pub use model::TraceCapabilities;
pub use model::TraceCatalog;
pub use model::TraceContentDocument;
pub use model::TraceContentFormat;
pub use model::TraceDiagnostic;
pub use model::TraceLimits;
pub use model::TraceNode;
pub use model::TraceNodeKind;
pub use model::TraceNodeLocator;
pub use model::TraceRecordChannel;
pub use model::TraceRecordClass;
pub use model::TraceRecordPresentation;
pub use model::TraceRecordRole;
pub use model::TraceSourceKind;
pub use model::TraceStatus;
pub use payload::PayloadReadLimit;
pub use payload::SafePayloadReader;
pub use payload::SanitizedPayload;
pub use presentation::GroupActivity;
pub use presentation::GroupAggregate;
pub use presentation::GroupCompleteness;
pub use presentation::GroupEvidenceCounts;
pub use presentation::GroupId;
pub use presentation::GroupKind;
pub use presentation::GroupMember;
pub use presentation::GroupMemberRole;
pub use presentation::GroupMetadata;
pub use presentation::GroupOrigin;
pub use presentation::GroupReference;
pub use presentation::GroupVisibility;
pub use presentation::OrderBandId;
pub use presentation::OrderBandKind;
pub use presentation::OrderSegment;
pub use presentation::PRESENTATION_POLICY_VERSION;
pub use presentation::PresentationBuildStatus;
pub use presentation::PresentationDiagnostic;
pub use presentation::PresentationDiagnosticCode;
pub use presentation::PresentationDisposition;
pub use presentation::PresentationEvent;
pub use presentation::PresentationGroup;
pub use presentation::PresentationIndex;
pub use presentation::PresentationLimits;
pub use presentation::PresentationOrderBand;
pub use presentation::PresentationScope;

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
