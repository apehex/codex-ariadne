# Ariadne roadmap

Status: non-executable successor directions

The first historical `codex trace` browser and its ordinary, rich, and merged trace model are implemented. This roadmap distinguishes accepted executable plans from later directions that still require their own authority.

## Phased presentation and parent compatibility

The accepted presentation program is materialized as separate open plans so each data, interaction, visual, and integration boundary can be reviewed independently:

1. [Phase 0: Trace Presentation Architecture Contract](plans/closed/2026-07-30-trace-presentation-architecture-contract.md) froze evidence, ordering, grouping, view, bounds, and integration decisions in the accepted [presentation-index contract](decisions/2026-07-31-trace-presentation-index-contract.md).
2. [Phase 1: Trace Order And Correlation](plans/closed/2026-07-30-trace-order-and-correlation.md) retains typed source-order and correlation facts.
3. [Phase 2: Static Trace Presentation Index](plans/closed/2026-07-30-static-trace-presentation-index.md) materializes the minimal renderer-neutral partition.
4. [Phase 3: Trace Grouping Policies](plans/open/2026-07-30-trace-grouping-policies.md) implements deterministic lifecycle, agent, compaction, and higher-order grouping.
5. [Phase 4: Trace Browser Lenses And Structured Navigation](plans/open/2026-07-30-trace-browser-lenses-and-structured-navigation.md) exposes grouped, expanded, structural, thread, member, and JSON navigation.
6. [Phase 5: Borderless Horizontal Trace Surface](plans/open/2026-07-30-borderless-horizontal-trace-surface.md) applies the full-body visual surface and horizontal viewport.
7. [Phase 6: Codex Transcript Parity](plans/open/2026-07-30-codex-transcript-parity.md) compares Ariadne groups with the current parent transcript.
8. [Phase 7: Live Parent Trace Integration](plans/open/2026-07-30-live-parent-trace-integration.md) adds an opt-in `/trace` and transcript-shortcut integration.
9. [Phase 8: Parent-Compatible Transcript Candidate](plans/open/2026-07-30-parent-compatible-transcript-candidate.md) prepares a locally maintainable replacement candidate.

The open plan for each remaining phase is the execution authority for that phase; this roadmap is not. Closed phases retain their accepted decisions and validation records.

Phases 7 and 8 close on local, reproducible compatibility candidates with an intact upstream fallback. Upstream interest, review, merge, release, default enablement, and eventual transcript replacement are external outcomes. Ariadne may maintain the candidate as upstream evolves without representing upstream adoption as completed.

## Workflow diagnostics and timeline

Add a synchronized cross-thread timeline and evidence-backed diagnostics for unjoined delegation, interruption, failed tools, approval waits, unintegrated child results, compactions, and unusual resource consumption. Diagnostics must describe recorded behavior rather than infer agent intent, include positive and false-positive synthetic tests, and open their supporting nodes.

The current browser uses index-backed adjacency, cached current-level rows and inspector content, viewport-only rendering, and locator-based descent and return. Future performance work should begin with representative measurement and target unresolved gaps such as chunk-bounded JSONL reading, cooperative rich-replay cancellation, and content-level merge conflict detection.

## Observability extensions

Propose recorder, persisted-schema, core-runtime, or app-server additions only for evidence gaps demonstrated by prior browser work. Keep sensitive capture explicit and opt-in, and justify every field through a failing synthetic observability scenario with versioning, bounds, and privacy behavior.

## Release evidence

Before describing the browser as cross-platform, obtain successful Linux, macOS, and Windows runs of the focused trace workflow for the exact release-candidate commit using platform-neutral synthetic fixtures.
The workflow configuration and release-candidate instructions define the evidence route but are not themselves evidence that any platform passed.
