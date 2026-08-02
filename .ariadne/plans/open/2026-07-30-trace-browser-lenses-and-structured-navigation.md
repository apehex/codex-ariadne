# Trace Browser Lenses And Structured Navigation

Updated: 2026-07-31

Status: open

## Objective

Make `codex-trace-tui` navigate the materialized presentation index, canonical trace topology, group members, threads, and structured JSON values through one reversible single-depth navigation model.

## Context

This is Phase 4 and depends on the stable group model from [closed Phase 3](../closed/2026-07-30-trace-grouping-policies.md). It implements the lens, reversibility, visibility, and structured-value boundaries from the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md) while intentionally preserving the current visual chrome so semantic and interaction defects are not hidden inside simultaneous layout churn.

## Scope

Owned implementation paths are `codex-rs/trace-tui/`, focused additions to `codex-rs/trace/` required for safe presentation queries, tests, snapshots, and module designs.

The borderless redesign, horizontal styled-content viewport, parent transcript parity, live `/trace`, persisted configuration, recorder changes, and parent-TUI orchestration are excluded.

## Movements

### Movement 1: Generalize Browser Locations

- Represent thread, group timeline, entered group, canonical container, trace node, structured value, and raw artifact locations explicitly.
- Retain locator-based selection and viewport frames for every reversible descent.
- Keep vertical selection, horizontal state placeholders, content mode, filtering, search, and payload state orthogonal.

### Movement 2: Add Presentation Lenses

- Add a collapsed conversation lens containing one row per group ordered by group anchor.
- Add an expanded event lens containing every event in canonical order with stable group annotations even when groups interleave.
- Enter a group to list only its direct members at one common detail level; enter a nested group or member to continue descent.
- Preserve the structural trace lens as the evidence-first topology view.

### Movement 3: Add Thread And Relationship Navigation

- Open on a selected or current thread while preserving access to the session root.
- Navigate parent and child threads and follow explicit interaction links without confusing containment with causality.
- Restore exact selection and viewport on return.

### Movement 4: Add Structured-Value Navigation

- Derive browser-local object-key and array-index rows from one selected bounded `serde_json::Value`.
- Identify structured locations by trace locator plus JSON Pointer or an equally stable typed path.
- Treat scalar values as leaves with interpreted line breaks and keep raw exact artifacts lazy.
- Bound direct children, depth, previews, and displayed scalar bytes.

## Acceptance Evidence

- Collapsed, expanded, structural, entered-group, thread, and JSON lenses are independently selectable and reversible.
- A non-contiguous group appears once in the collapsed lens, may appear in several annotated spans in the expanded lens, and shows all members when entered.
- JSON navigation never creates canonical `TraceNode`s or modifies upstream conversation topology.
- Routine movement and rendering inspect only current-level and viewport-sized state.
- State-transition tests and reviewed snapshots cover empty, narrow, interleaved, partial, nested-agent, compaction, Unicode, controls, and structured-value cases.
- Existing offline, read-only, payload-containment, stale-job, and 100,000-node guarantees remain intact.

## Closure

Close this plan after recording the final navigation-state model, lens defaults, shortcuts, bounds, snapshot paths, exact validation commands, and residual visual work transferred to Phase 5.
