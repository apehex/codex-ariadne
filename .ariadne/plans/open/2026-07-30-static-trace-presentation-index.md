# Static Trace Presentation Index

Updated: 2026-07-31

Status: open

## Objective

Build and validate one immutable presentation index per loaded session so renderers and navigation controllers can query groups, members, summaries, and view orderings without reconstructing or verifying groups during rendering.

## Context

The presentation index is a derived projection over immutable canonical trace nodes. It must support non-contiguous membership, stable group identity, reversible evidence navigation, and conservative aggregate metadata while leaving `SessionTrace` and `TraceIndex` authoritative for evidence and topology.

This is Phase 2 and depends on [Phase 1](2026-07-30-trace-order-and-correlation.md). Its model and exact structural bounds are governed by the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

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
