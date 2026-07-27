# Trace TUI instructions

Load [`../../.ariadne/AGENTS.md`](../../.ariadne/AGENTS.md) before substantial work in this module. These instructions add only `codex-trace-tui` constraints.

## Ownership

This crate owns terminal application state, job dispatch, navigation, adaptive layout, and rendering for the historical browser. Source discovery, evidence semantics, graph projection, reconciliation, search attribution, and payload containment belong in `codex-trace`.

## Responsiveness

- Never perform source discovery, rich replay, payload I/O, or whole-trace serialization in the terminal event loop.
- Keep background work cancellable where its source operation permits it, and retain the previous stable view on failure or cancellation.
- Render only visible rows and visible detail. Cache or precompute wrapping and formatted detail rather than rebuilding complete content every frame.
- Do not scan every trace node during ordinary cursor movement, pane changes, or redraws; request model-owned adjacency or indexes when needed.
- Preserve explicit bounds and show progress or limit diagnostics for work that grows with trace size.

## Presentation

- Keep raw payloads collapsed until requested and render them as inert terminal-safe text.
- Preserve provenance, evidence grades, conflicts, and unavailable capabilities in the inspector.
- Keep navigation usable without color and across narrow, medium, and wide terminals.
- Do not infer intent or hide malformed and missing evidence for a cleaner presentation.

## Change discipline

Update [`DESIGN.md`](DESIGN.md) when screen state, event-loop behavior, navigation, layout, caching, or rendering contracts change.

User-visible changes require deterministic Ratatui snapshot coverage. Interaction changes require state-transition tests. Performance fixes require a representative synthetic trace and an assertion or benchmark for the repeated cost being removed. End-to-end tests must run offline and verify that every synthetic input remains byte-identical.
