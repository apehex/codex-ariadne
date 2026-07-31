//! Read-only discovery and projection of local Codex execution traces.
//!
//! Ordinary rollout JSONL and opt-in rich rollout-trace bundles are normalized
//! into the same stable node model. This crate never repairs, appends, indexes,
//! or otherwise mutates its input stores.

#![warn(missing_docs)]

mod catalog;
mod fact_index;
mod facts;
mod graph;
mod index;
mod model;
mod ordinary;
mod ordinary_facts;
mod payload;
mod presentation;
mod rich;
mod rich_facts;
mod rich_terminal_facts;
mod search;

pub use catalog::TraceRepository;
pub use fact_index::TraceFactIndex;
pub use facts::TraceCorrelation;
pub use facts::TraceFactAvailability;
pub use facts::TraceNodeFacts;
pub use facts::TraceObjectRef;
pub use facts::TraceOrder;
pub use facts::TraceOrderDomain;
pub use facts::TraceOrderPoint;
pub use facts::TraceOwnership;
pub use facts::TraceRelation;
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
