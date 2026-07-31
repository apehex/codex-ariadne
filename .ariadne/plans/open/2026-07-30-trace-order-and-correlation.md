# Trace Order And Correlation

Updated: 2026-07-31

Status: open

## Objective

Retain typed order and correlation facts in the loaded Ariadne trace model so presentation groups can be constructed without parsing arbitrary detail JSON, comparing incompatible clocks, or inferring relationships from timestamp proximity.

## Context

Ordinary rollout records have per-thread ordinals or line order. Rich traces have global raw-event sequences, execution windows, call identifiers, producer references, and interaction edges. `TraceGraphBuilder` currently uses source order to sort siblings and then discards its private `SiblingOrder`; the final node vector is parent-bucketed and is not a global event timeline.

This is Phase 1 and depends on the closed decisions from [Phase 0](../closed/2026-07-30-trace-presentation-architecture-contract.md). It implements the order and correlation boundary of the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md). The open [quality-remediation plan](2026-07-28-trace-browser-quality-remediation.md) remains the baseline correctness and bounds gate.

## Scope

Owned implementation paths are `codex-rs/trace/`, its tests and design, and focused synthetic fixtures.

Changes to `codex-rollout-trace` are permitted only when a missing typed fact cannot be adapted from its existing reduced model and the Phase 0 contract records the necessity. Persisted schemas, recorder behavior, `codex-core`, app-server APIs, trace-TUI behavior, and parent-TUI behavior are excluded.

## Movements

### Movement 1: Introduce Typed Order Domains

- Retain ordinary ordinal, rich sequence, optional wall-clock time, and deterministic tie-break information through graph finalization.
- Keep incompatible sequence domains incomparable rather than coercing them into one integer timeline.
- Preserve existing sibling order and public traversal compatibility.
- Keep the order representation bounded, serializable only if explicitly required, and independent of terminal presentation.

### Movement 2: Expose Typed Correlation Facts

- Provide typed access to model-visible call IDs, runtime tool IDs, thread and turn ownership, producer references, compaction membership, terminal observations, and interaction edges needed by later grouping.
- Prefer source model fields and stable locators over JSON-pointer extraction.
- Represent absent or conflicting correlations explicitly and preserve every observation.

### Movement 3: Build And Validate Order Access

- Provide an additive snapshot index or immutable loaded-session companion that resolves node positions to order and correlation facts.
- Define rebuild semantics after public node mutation consistently with `TraceIndex`.
- Test ordinary-only, rich-only, merged, repeated, malformed, partial, and limit-truncated sources.

## Acceptance Evidence

- A consumer can order one thread's presentable events without reading `TraceNode.detail`.
- Background completion after unrelated records retains its later event position and stable correlation to the earlier start.
- Ordinary records never claim a cross-thread causal order that the source does not provide.
- Rich raw-event sequence remains the causal tie-breaker even when wall-clock timestamps regress.
- Existing trace traversal, evidence preservation, source bounds, and upstream persisted formats remain unchanged.
- Focused `codex-trace` tests, scoped lint fixes, formatting, and relevant Bazel data checks pass.

## Closure

Close this plan after recording the final types, source-to-order mapping, correlation inventory, exact tests, limits, module sizes, and any unavailable fact deferred to a later observability proposal.
