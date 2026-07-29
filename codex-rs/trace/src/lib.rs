//! Read-only discovery and projection of local Codex execution traces.
//!
//! Ordinary rollout JSONL and opt-in rich rollout-trace bundles are normalized
//! into the same stable node model. This crate never repairs, appends, indexes,
//! or otherwise mutates its input stores.

#![warn(missing_docs)]

mod catalog;
mod graph;
mod index;
mod model;
mod ordinary;
mod payload;
mod presentation;
mod rich;
mod search;

pub use catalog::TraceRepository;
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

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
