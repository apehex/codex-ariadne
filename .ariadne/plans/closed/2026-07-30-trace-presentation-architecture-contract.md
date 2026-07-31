# Trace Presentation Architecture Contract

Updated: 2026-07-31

Status: closed

## Objective

Freeze the architectural contract for a materialized presentation index that can support an Ariadne historical browser, a Codex-like grouped transcript, structured trace inspection, and a future parent-TUI integration without changing the canonical trace evidence.

Completion means the project has durable decisions for ordering, identity, group membership, view semantics, resource bounds, missing evidence, and parent integration before implementation changes the model or UI.

## Context

`SessionTrace` and `TraceIndex` currently provide an evidence-preserving structural hierarchy. The parent Codex transcript applies an additional presentation reduction by merging lifecycle records, selecting type-specific fields, and hiding some control-plane events. The agreed direction adds that reduction as a separate materialized index rather than rewriting trace topology or rebuilding groups during rendering.

This is Phase 0 of the phased presentation program recorded in [the project roadmap](../../ROADMAP.md). It is the dependency for every later phase.

## Scope

Owned paths are Ariadne design, decision, and plan documents under `.ariadne/`.

This phase may update `codex-rs/trace/DESIGN.md` and `codex-rs/trace-tui/DESIGN.md` to record accepted boundaries, but it does not authorize Rust implementation, persisted-format changes, recorder changes, app-server changes, parent-TUI integration, or external submission.

## Movements

### Movement 1: Define The Evidence And Projection Boundary

- Define canonical trace nodes, structural indexes, presentation groups, primary membership, secondary references, and derived summaries as separate concepts.
- Require every presentation group and summary to retain reversible links to canonical trace locators.
- Define how exact, semantic, reconstructed, conflicting, unavailable, partial, and live-only evidence affects group presentation.
- Confirm that presentation grouping never rewrites model-visible history, persisted rollouts, rich bundles, or upstream conversation topology.

### Movement 2: Define Ordering And Identity

- Define ordinary per-thread ordinal, rich raw-event sequence, wall-clock display time, and deterministic tie-breaking as distinct order domains.
- Define stable group identities from durable correlation identifiers rather than display positions or timestamps.
- Define collapsed group order, expanded event order, entered-group member order, and cross-thread navigation order.
- Record how repeated observations, merged sources, background completion, missing endpoints, and concurrent threads remain deterministic.

### Movement 3: Define Group And View Contracts

- Enumerate initial group families and the fallback singleton group.
- Define one primary group per presentable item while permitting secondary references from inference, compaction, terminal, interaction, and raw-evidence views.
- Define typed group-member roles, aggregate metadata, completeness, nested groups, summaries, and visibility defaults.
- Define grouped conversation, expanded timeline, structural trace, entered-group, thread, and structured-value lenses without terminal-specific rendering types.

### Movement 4: Define Bounds And Parent Integration

- Set structural caps for groups, members, secondary references, summaries, nested depth, structured-value children, and index construction.
- Require index construction during selected-session loading or explicit live updates, never during frame rendering.
- Define the narrow renderer-neutral boundary through which the parent Codex TUI can consume the same groups.
- Record that Phases 7 and 8 close on a locally demonstrated compatibility candidate; upstream adoption remains an external roadmap outcome.

## Acceptance Evidence

- Accepted decisions assign exactly one owner to canonical topology, ordering, presentation grouping, rendering, and parent integration.
- Every later phase links to the exact contract it implements and names its exclusions.
- The contract includes examples for interleaved background tools, exploration batches, agent waits, compaction, missing endpoints, repeated observations, and concurrent child threads.
- Resource bounds and evidence labels are explicit enough to write exact-boundary tests.
- Documentation contains no claim that upstream acceptance is within Ariadne's control.

## Closure

Close this plan after recording the accepted decisions, updating the cross-crate and module designs, linking every successor phase, and listing unresolved choices that must block implementation rather than be guessed.

No Rust implementation, push, release, external proposal, or upstream submission is authorized by this phase.

## Implementation Record

Phase 0 completed on 2026-07-31 as a documentation-and-decisions phase.

The accepted [Materialized Trace Presentation Index decision](../../decisions/2026-07-31-trace-presentation-index-contract.md) defines:

- exclusive ownership of canonical evidence, structural lookup, typed order and correlation, presentation reduction, browser behavior, and parent integration;
- primary, reference-only, and structural-only presentation dispositions;
- orthogonal evidence grade, completeness, and persisted or live origin;
- ordinary ordinal, rich sequence, structural, timestamp, and deterministic tie-break domains;
- explicit unordered cross-source bands and the prohibition on timestamp-derived causality;
- stable singleton, correlated lifecycle, and higher-order batch identities;
- initial group families, member roles, conservative aggregate status, nesting, summaries, and default visibility;
- collapsed, expanded, entered-group, structural, thread, and structured-value navigation order;
- exact group, membership, reference, nesting, diagnostic, summary, preview, and structured-value bounds;
- batch and incremental equivalence and a renderer-neutral parent boundary;
- degradation and stop rules for facts that later phases cannot support without guessing.

The decision includes worked examples for interleaved background tools, exploration batches, agent waits, compaction, missing endpoints, repeated and conflicting observations, concurrent child threads, and regressing timestamps.

The cross-crate [Ariadne design](../../DESIGN.md), [`codex-trace` design](../../../codex-rs/trace/DESIGN.md), and [`codex-trace-tui` design](../../../codex-rs/trace-tui/DESIGN.md) now record the accepted layering and module boundaries.

Every successor plan links to the accepted decision and names the part of the contract it implements. Phase 1 is the next executable phase.

## Residual And Blocking Conditions

No unresolved architecture choice blocks Phase 1.

Later phases must stop their affected movement rather than infer missing order, correlation, batching, persisted live state, canonical JSON-value nodes, or a new upstream observability surface. The exact stop conditions are recorded in the accepted decision.

Parent transcript family parity, exact exploration flush rules, the live event adapter, and the eventual upstream integration diff remain deliberately owned by Phases 3, 6, 7, and 8 rather than being guessed in Phase 0.

## Validation Record

Documentation validation checked required plan sections, successor links, local Markdown targets, unfolded semantic-line formatting, trailing whitespace, and `git diff --check`.

No Rust source, persisted format, recorder, app-server API, parent-TUI behavior, push, release, external proposal, or upstream submission changed in this phase.
