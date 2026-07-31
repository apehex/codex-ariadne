# Trace Order And Correlation

Updated: 2026-07-31

Status: closed

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

## Completion Record

Phase 1 closed on 2026-07-31 with an additive typed-fact companion in `codex-trace`. Persisted ordinary rollouts, rich bundle schemas, recorder behavior, traversal APIs, and TUI behavior were not changed.

### Final types and access

- `TraceOrderPoint` keeps ordinary ordinals, rich raw-event sequences, structural wall-clock placement, and unspecified positions in distinct domains.
- `TraceOrder` retains start and optional end positions, display-only start and end wall-clock values, and deterministic admission order.
- `TraceOwnership` retains recorded thread and turn IDs.
- `TraceNodeFacts` retains order, ownership, bounded correlations, and explicit complete, partial, unavailable, or conflicting availability.
- `TraceObjectRef`, `TraceRelation`, and `TraceCorrelation` provide typed stable identities and relationship roles.
- `SessionTrace::fact_index()` builds an immutable `TraceFactIndex` snapshot with locator and position lookup, compatible-domain per-thread order, and exact reverse-correlation lookup. Its invalidation and rebuild contract matches the public-node mutation boundary of `TraceIndex`.

### Source-to-order mapping

- Ordinary thread containers use their parsed RFC 3339 start only as structural placement. Ordinary records use persisted numeric `ordinal`, falling back to bounded reader line order, and retain a timestamp only as display data.
- Ordinary positions compare only when both observations name the same nonempty recorded thread. Ordinary observations in different threads and ordinary/rich observations remain incomparable.
- Rich threads use structural execution placement. Rich turns, conversation items, inferences, tools, code cells, compactions, compaction requests, terminal operations, terminal sessions, and interaction edges retain their raw-event start sequence. Duration-bearing objects also retain their end sequence when the reduced model exposes one.
- Rich interaction edges now use `started_seq`; their wall-clock start is no longer the sibling-order key. The reduced model exposes only an interaction end time, so an interaction end sequence remains unavailable.
- Equal or incomparable sibling positions retain existing stable admission order. The tie-break is excluded from semantic duplicate comparison.

### Correlation inventory

Rich projection retains source identities; thread parent and spawn edges; turn inputs; conversation producers; model-visible call IDs; inference inputs, outputs, and started tools; runtime tool, MCP, code-mode tool, and code-cell IDs; model-visible call/output items; nested and wait tools; compaction requests, markers, inputs, and replacements; terminal ownership, sessions, operations, and model-visible observations; interaction endpoints and carried items; and raw payload references.

Ordinary projection retains every durable source identity available from response items, agent communication, turn contexts, and compaction windows, plus model-visible protocol call IDs and compaction replacement item IDs. It retains current or item-authored turn ownership but remains explicitly partial because ordinary records do not expose the full reduced runtime graph.

Missing typed records are unavailable. Incomplete rich replay and ordinary projections are partial. When repeated observations disagree, both retained sides are marked conflicting and no observation is deleted.

### Bounds and complexity

- One node retains at most 4,096 correlations.
- One loaded graph retains at most four correlations per configured node slot across the session. Saturation truncates correlations, downgrades complete facts to partial, and emits one diagnostic.
- Fact-index construction is linear in retained nodes and correlations plus ordering within each thread/domain. It clones bounded facts but never clones node detail or opens raw payloads.
- No new production module exceeds 500 lines: `facts.rs` 335, `fact_index.rs` 182, `ordinary_facts.rs` 140, `rich_facts.rs` 449, `rich_terminal_facts.rs` 105, `graph.rs` 331, `ordinary.rs` 432, and `rich.rs` 375. The pre-existing `model.rs` and `catalog.rs` remain 538 and 606 lines; Phase 1 added only the private fact store/accessor and graph-finalization plumbing there.

### Acceptance evidence

`just test -p codex-trace` passed all 49 tests after the final semantic change. New focused tests cover compatible and incompatible source domains, per-node and session correlation limits, immutable-index rebuild semantics, rich sequence order despite regressing wall time, merged ordinary/rich identity without invented order, ordinary call and ownership extraction, later rich tool completion with stable runtime/call identity, interaction sequence and endpoints, terminal model observations, duplicate conflict availability, and admission-order exclusion from duplicate equality.

Existing synthetic tests continue to cover ordinary ordinal order despite timestamp regression, rich cross-kind sequence order, merged source retention, repeated observations, malformed and partial rich replay, topology failures, exact node limits, payload containment, and unchanged traversal behavior.

`just fix -p codex-trace`, `just fmt`, and `git diff --check` passed. The crate already uses the Bazel macro's source discovery and Phase 1 added no compile-time file reads or data assets, so no `BUILD.bazel` data entry changed. The targeted prebuilt `just argument-comment-lint -p codex-trace` could not run because its pinned Rust 1.92 nightly is older than `sqlx 0.9`'s Rust 1.94 requirement. The default unscoped recipe was stopped after it expanded to 830 unrelated workspace targets and began building V8; no lint result is claimed. Scoped Clippy completed without findings.

### Deferred evidence

The reduced rich `InteractionEdge` has no end sequence, so Phase 1 retains its end wall-clock value but does not invent a causal end position. Ordinary sources do not expose runtime tool/MCP/code-mode IDs, inference request membership, terminal observation links, or complete producer edges. Those absences are represented as partial facts; no recorder or persisted-schema change is justified by this phase.
