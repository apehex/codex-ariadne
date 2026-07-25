//! Read-only discovery and projection of local Codex execution traces.
//!
//! Ordinary rollout JSONL and opt-in rich rollout-trace bundles are normalized
//! into the same stable node model. This crate never repairs, appends, indexes,
//! or otherwise mutates its input stores.

mod catalog;
mod model;
mod payload;
mod rich;
mod search;

pub use catalog::TraceRepository;
pub use model::*;
pub use payload::PayloadReadLimit;
pub use payload::SafePayloadReader;
pub use payload::SanitizedPayload;

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
