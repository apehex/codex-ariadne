//! Trace bundle format, writer, and reducer for Codex rollouts.
//!
//! This crate owns the trace schema. Hot-path Codex code should depend on the
//! small writer API here; semantic replay and viewer projections stay outside
//! `codex-core`.
//!
//! See `README.md` for the system diagram and reducer model.

mod bundle;
mod code_cell;
mod compaction;
mod inference;
mod mcp;
mod model;
mod payload;
mod protocol_event;
mod raw_event;
mod reducer;
mod thread;
mod tool_dispatch;
mod writer;

/// Conventional reduced-state cache name written next to a raw trace bundle.
pub use bundle::REDUCED_STATE_FILE_NAME;
/// No-op-capable handle for recording one code-mode runtime cell.
pub use code_cell::CodeCellTraceContext;
/// Raw checkpoint payload for a remote compaction install event.
pub use compaction::CompactionCheckpointTracePayload;
/// No-op-capable handle for recording remote-compaction requests.
pub use compaction::CompactionTraceAttempt;
/// Shared recorder context for a compaction checkpoint.
pub use compaction::CompactionTraceContext;
/// No-op-capable handle for recording one upstream inference attempt.
pub use inference::InferenceTraceAttempt;
/// Shared recorder context for inference attempts within one Codex turn.
pub use inference::InferenceTraceContext;
/// Trace-owned MCP execution correlation propagated to bridge request metadata.
pub use mcp::McpCallTraceContext;
/// Public reduced trace model returned by replay.
pub use model::*;
/// Stable identifier for one raw payload inside a rollout bundle.
pub use payload::RawPayloadId;
/// Coarse role labels for raw payload files.
pub use payload::RawPayloadKind;
/// Reference to a raw request/response/log payload stored in the bundle.
pub use payload::RawPayloadRef;
/// Monotonic sequence number assigned by the raw trace writer.
pub use raw_event::RawEventSeq;
/// Runtime requester observed before semantic reduction.
pub use raw_event::RawToolCallRequester;
/// One append-only raw trace event from `trace.jsonl`.
pub use raw_event::RawTraceEvent;
/// Event-envelope context supplied by hot-path trace producers.
pub use raw_event::RawTraceEventContext;
/// Typed payload for one raw trace event.
pub use raw_event::RawTraceEventPayload;
/// One event-level failure retained by resilient replay.
pub use reducer::ReplayDiagnostic;
/// Resource limits for best-effort diagnostic replay.
pub use reducer::ReplayLimits;
/// Best-effort reduced state plus event-level diagnostics.
pub use reducer::ResilientReplay;
/// Trace bundle manifest schema version understood by the inspection API.
pub use reducer::TRACE_BUNDLE_SCHEMA_VERSION;
/// Public metadata needed to catalog a trace bundle without replaying it.
pub use reducer::TraceBundleMetadata;
/// Read trace-bundle catalog metadata without replaying its raw event log.
pub use reducer::inspect_bundle;
/// Replay a raw trace bundle and write/read its reduced `RolloutTrace`.
pub use reducer::replay_bundle;
/// Replay a raw trace bundle while isolating malformed individual events.
pub use reducer::replay_bundle_resilient;
/// Replay a raw bundle within explicit resource limits.
pub use reducer::replay_bundle_resilient_with_limits;
/// Raw payload captured when a child agent reports completion to its parent.
pub use thread::AgentResultTracePayload;
/// Environment variable that enables local trace-bundle recording.
pub use thread::CODEX_ROLLOUT_TRACE_ROOT_ENV;
/// Raw metadata captured when a thread starts.
pub use thread::ThreadStartedTraceMetadata;
/// No-op-capable handle for recording one thread in a rollout bundle.
pub use thread::ThreadTraceContext;
/// Request data for the canonical Codex tool boundary.
pub use tool_dispatch::ToolDispatchInvocation;
/// Tool input observed at the registry boundary.
pub use tool_dispatch::ToolDispatchPayload;
/// Runtime source that caused a dispatch-level tool call.
pub use tool_dispatch::ToolDispatchRequester;
/// Result data returned from a dispatch-level tool call.
pub use tool_dispatch::ToolDispatchResult;
/// No-op-capable handle for recording one resolved tool dispatch.
pub use tool_dispatch::ToolDispatchTraceContext;
/// Append-only writer used by hot-path Codex instrumentation.
pub use writer::TraceWriter;
