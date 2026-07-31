# Codex Transcript Parity

Updated: 2026-07-30

Status: open

## Objective

Demonstrate that Ariadne's grouped conversation lens can represent the semantic content and grouping behavior of the current Codex transcript while preserving additional metadata, evidence links, thread navigation, expanded events, and deeper inspection.

## Context

This is Phase 6 and depends on the stable browser and visual surface from [Phase 5](2026-07-30-borderless-horizontal-trace-surface.md). It is a comparison and compatibility phase, not authorization to replace `Ctrl+T` or add `/trace`.

The current parent TUI builds committed and live `HistoryCell`s through type-specific state machines. Ariadne must compare semantic outcomes rather than rendered bytes alone and must document where historical evidence cannot reproduce a transient live cell.

## Scope

Owned work includes Ariadne grouping and renderer adapters, deterministic parity fixtures, focused parent-TUI tests and snapshots, compatibility documentation, and the smallest test-only observation hooks needed for comparison.

Broad refactoring of upstream history cells, live command dispatch, recorder changes, app-server changes, model-visible context changes, and external submission are excluded.

## Movements

### Movement 1: Build A Parity Matrix

- Enumerate user, assistant, reasoning summary, proposed plan, plan update, exec, exploration, patch, MCP, web, image, agent, compaction, diagnostic, and raw transcript families.
- Record current parent grouping, selected fields, truncation, live behavior, transcript behavior, and available persisted evidence for each family.
- Classify differences as required parity, intentional Ariadne enrichment, unavailable historical fact, or upstream-version dependency.

### Movement 2: Compare Semantic Groups

- Feed equivalent deterministic facts through current Codex transcript construction and Ariadne presentation reduction.
- Compare group boundaries, ordering anchors, status, selected summaries, member coverage, and visibility decisions.
- Keep visual snapshot comparison separate from semantic deep equality.

### Movement 3: Resolve Or Document Differences

- Adjust Ariadne policies where current Codex behavior is deterministic, useful, and evidence-backed.
- Preserve richer Ariadne detail rather than deleting evidence merely to match transcript compression.
- Represent live-only or unpersisted facts explicitly and define the live adapter input required by Phase 7.
- Record version-sensitive grouping rules so upstream updates do not silently change parity claims.

## Acceptance Evidence

- Every parity-matrix row has a deterministic fixture, an explicit exemption, or an unavailable-evidence explanation.
- Completed synthetic sessions produce equivalent user-facing group boundaries for all supported transcript families.
- Ariadne can descend from every matched group to its canonical members and raw or structured evidence.
- Parent-renderer and Ariadne snapshots demonstrate intentional differences at narrow and wide widths.
- No parent command, key binding, overlay lifecycle, model context, or persisted format changes in this phase.

## Closure

Close this plan after recording the parity matrix, exact semantic and visual tests, known upstream-version dependencies, unavailable live facts, and the minimal integration contract required by Phase 7.
