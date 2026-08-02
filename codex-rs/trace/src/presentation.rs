//! Renderer-neutral trace presentation and its bounded construction pipeline.

pub(crate) mod activity;
mod bounded_text;
mod build;
mod content;
mod context;
mod diagnostics;
mod grouping;
mod index;
mod limits;
mod model;
mod order;
mod policy;
mod record;
mod references;
mod summary;
mod validate;

pub use index::PresentationIndex;
pub use limits::PresentationLimits;
pub use model::GroupActivity;
pub use model::GroupAggregate;
pub use model::GroupCompleteness;
pub use model::GroupEvidenceCounts;
pub use model::GroupId;
pub use model::GroupKind;
pub use model::GroupMember;
pub use model::GroupMemberRole;
pub use model::GroupMetadata;
pub use model::GroupOrigin;
pub use model::GroupReference;
pub use model::GroupVisibility;
pub use model::OrderBandId;
pub use model::OrderBandKind;
pub use model::OrderSegment;
pub use model::PRESENTATION_POLICY_VERSION;
pub use model::PresentationBuildStatus;
pub use model::PresentationDiagnostic;
pub use model::PresentationDiagnosticCode;
pub use model::PresentationDisposition;
pub use model::PresentationEvent;
pub use model::PresentationGroup;
pub use model::PresentationOrderBand;
pub use model::PresentationScope;

pub(crate) use context::BuildContext;
pub(crate) use diagnostics::DiagnosticSink;

#[cfg(test)]
#[path = "presentation/limits_tests.rs"]
mod limits_tests;
