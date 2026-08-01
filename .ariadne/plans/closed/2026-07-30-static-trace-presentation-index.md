# Static Trace Presentation Index

Updated: 2026-07-31

Status: closed

Closed: 2026-07-31

## Objective

Build and validate one immutable presentation index per loaded session so renderers and navigation controllers can query groups, members, summaries, and view orderings without reconstructing or verifying groups during rendering.

## Context

The presentation index is a derived projection over immutable canonical trace nodes. It must support non-contiguous membership, stable group identity, reversible evidence navigation, and conservative aggregate metadata while leaving `SessionTrace` and `TraceIndex` authoritative for evidence and topology.

This is Phase 2 and depends on [closed Phase 1](2026-07-30-trace-order-and-correlation.md). Its model and exact structural bounds are governed by the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

## Scope

Owned implementation paths are new focused modules under `codex-rs/trace/`, their tests and design, and selected-session construction at the Ariadne crate boundary.

This phase excludes advanced group families, Ratatui rendering, browser navigation changes, JSON navigation, live incremental updates, parent-TUI changes, recorder changes, and persisted presentation schemas.

## Movements

### Movement 1: Define The Renderer-Neutral Model

- Add stable `GroupId`, `GroupKind`, `GroupMember`, `GroupMemberRole`, `GroupMetadata`, `GroupCompleteness`, and `PresentationIndex` types.
- Store compact node positions or locators with explicit snapshot invalidation semantics.
- Keep summaries typed and renderer-neutral; do not store Ratatui lines, colors, widths, wrapped text, or key bindings.

### Movement 2: Build The Minimal Partition

- Create singleton groups for messages, reasoning, system/developer items, structural records, and unknown presentable records.
- Create one correlated tool lifecycle group for each supported direct tool identity.
- Assign exactly one primary group to every presentable node while retaining secondary references separately.
- Precompute group order by thread, canonical event order by thread, group members in source order, and stable identifier lookup.

### Movement 3: Validate And Bound Construction

- Detect duplicate primary membership, missing members, cross-thread primary groups, cycles, invalid children, and nondeterministic order.
- Degrade incomplete evidence to partial groups and diagnostics instead of dropping nodes.
- Bound groups, members, references, summary text, and construction memory as functions of retained trace limits.
- Prove that rendering-facing queries do not scan the complete trace.

## Acceptance Evidence

- Whole-index equality tests cover complete, interleaved, partial, conflicting, repeated, ordinary-only, rich-only, and merged traces.
- Every presentable node is reachable through exactly one primary group or an explicit bounded diagnostic.
- Secondary references do not steal primary membership.
- Group IDs and order remain stable across repeated construction of the same immutable trace.
- Index construction is linear in retained nodes and references and occurs once per loaded snapshot.
- No user-visible snapshot changes are required in this phase.

## Closure

Close this plan after recording the public and private API surface, invariants, resource accounting, exact validation commands, module sizes, and deferred group families owned by Phase 3.

## Implementation Record

Phase 2 completed on 2026-07-31.

The public `codex-trace` boundary now exposes `PresentationIndex`, renderer-neutral group, member, metadata, completeness, reference, order-band, event, disposition, visibility, origin, diagnostic, and build-status types. `SessionTrace::presentation_index` explicitly builds a snapshot whose node positions and locators are valid for that immutable trace version. Indexed queries cover group identity, canonical-node ownership, disposition, groups, events, and bands by scope, individual bands, diagnostics, and build status without reading arbitrary detail JSON or opening raw payloads.

Private focused modules own construction and resource accounting, the immutable query tables, public model types, partial-order alignment, minimal disposition policy, summary and reference reduction, direct-tool correlation, and structural validation. `BrowserState` constructs and retains the presentation snapshot beside `TraceIndex` during background selected-session installation; no frame or input path constructs it.

The minimal partition assigns sessions, threads, turns, and inference containers to structural-only disposition and raw payloads to reference-only disposition. Every other retained event-like node receives one singleton group unless typed direct-tool facts safely establish a model-visible call occurrence or runtime tool identity. Tool members may be non-contiguous in expanded order. Missing, reused, conflicting, delegation, and cross-thread facts remain singleton or partial rather than being joined by timestamp, adjacency, labels, or arbitrary JSON fields.

Ordinary and rich source positions retain independent internally ordered streams. Stable source identity or correlated group occurrence creates aligned bands; unmatched intervals remain explicitly unordered. Collapsed group order uses the earliest member band plus typed deterministic tie-breakers, while expanded events and entered-group members retain precomputed canonical order.

Construction enforces `G <= 2N`, total membership at most `3N`, nesting depth at most four, direct membership at most `N` per group, secondary references at most 4,096 per group and `4N` globally, diagnostics at most `min(N + 1, 1,024)`, 4 KiB of valid UTF-8 summary text per group, at most 16 MiB of summary text globally, and 256 Unicode scalar values per preview. Saturating arithmetic precedes allocations. Exhausted optional references, summaries, or diagnostics produce bounded saturation evidence; a failed required primary invariant marks the index incomplete.

Deep-equality and focused invariant tests cover empty, complete, interleaved, partial, conflicting, repeated, ordinary-only, rich-only, and merged evidence; structural, reference-only, and primary dispositions; stable rebuilds; ambiguous reused call IDs; non-contiguous tools; partial-order bands; reference bounds; UTF-8 and preview bounds; and snapshot invalidation. Existing trace and trace-TUI suites show no user-visible snapshot change.

## Module And Validation Record

Production presentation modules after formatting are: `presentation_build.rs` 318 lines, `presentation_index.rs` 149, `presentation_model.rs` 336, `presentation_order.rs` 342, `presentation_policy.rs` 121, `presentation_summaries.rs` 336, `presentation_tools.rs` 430, and `presentation_validate.rs` 362. The dedicated presentation-index test file is 825 lines. Every production module remains below the 500-line target and owns one orthogonal responsibility.

Validation passed with `just test -p codex-trace` at 60 tests and `just test -p codex-trace-tui` at 24 passed with one intentionally skipped profile test. Final scoped lint repair passed with `just fix -p codex-trace` and `just fix -p codex-trace-tui`; repository formatting completed with `just fmt`; `git diff --check` passed before closure documentation was recorded.

The optional targeted argument-comment Dylint could not execute because its pinned `nightly-2025-09-18` toolchain reports Rust 1.92 while the resolved `sqlx` 0.9 packages require Rust 1.94. The no-argument Bazel route was cancelled after it expanded into unrelated workspace and V8 compilation. Scoped Clippy completed without argument-comment warnings, and the new opaque positional literals use the repository's exact parameter-comment convention.

## Deferred Families

Phase 3 retains ownership of richer direct-tool policies, exploration batches, agent lifecycle and wait groups, compaction lifecycle reduction, terminal and code-cell specialization, higher-order child groups, policy versions, live-origin semantics needed by those policies, and parity metadata derived from current Codex transcript behavior. Phase 4 owns rendering and navigation through the already materialized groups. No Ratatui group rendering, structured-value navigation, parent `HistoryCell` dependency, recorder change, persisted presentation schema, or live incremental reducer was added here.
