# Trace Browser Lenses And Structured Navigation

Updated: 2026-08-02

Status: closed

## Objective

Make `codex-trace-tui` navigate the materialized presentation index, canonical trace topology, group members, threads, and structured JSON values through one reversible single-depth navigation model.

## Context

This is Phase 4 and depends on the stable group model from [closed Phase 3](../closed/2026-07-30-trace-grouping-policies.md) and the internal boundaries established by [closed Phase 3.5](../closed/2026-08-02-trace-module-factorization.md). It implements the lens, reversibility, visibility, and structured-value boundaries from the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md) while intentionally preserving the current visual chrome so semantic and interaction defects are not hidden inside simultaneous layout churn.

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

## Implementation Receipt

Phase 4 is implemented in `codex-trace-tui` without changing persisted formats or canonical topology. `BrowserState` owns one explicit `BrowserLocation`, a reversible stack of locator/group/path-based frames, and a cached current-level vector of typed `BrowserRow`s. The browser modules are partitioned into controller, location, navigation, projection, display, detail, search, row, and structured-value responsibilities; every production module remains below 500 lines.

The default is the collapsed lens on the first structurally rooted thread that owns presentable events. `Tab` and `Shift-Tab` cycle collapsed, expanded, and structural lenses. Enter descends into groups, structural containers, group members, resolved references, or JSON children; `i` opens semantic detail; `s` opens normalized JSON; Escape or Backspace restores the exact prior selection, viewport, and cached row level. A caller may instead select the session or a named thread and any initial lens through `TraceViewOptions`.

Collapsed rows come from `groups_in_scope`, expanded rows from `events_in_scope`, group rows from direct members, child groups, and retained references, and structural rows from `TraceIndex`. Group visibility and record-class visibility remain independent. The filter overlay can reveal presentation-hidden groups, and visible search excludes those groups outside the structural lens. All-record search retains its canonical structural fallback.

Structured locations retain a typed key/index path and render an RFC 6901 pointer only as a label. They do not create trace nodes or presentation groups. Navigation is capped at 64 levels and 4,096 direct children; previews are capped at 256 characters and scalar display at 64 KiB. Depth, child, and scalar exhaustion is disclosed. Raw payload loading remains lazy and separate from normalized JSON navigation.

The reviewed Phase 4 snapshots are:

- `codex_trace_tui__phase4_tests__phase4_collapsed_groups.snap`
- `codex_trace_tui__phase4_tests__phase4_entered_group.snap`
- `codex_trace_tui__phase4_tests__phase4_expanded_events.snap`
- `codex_trace_tui__phase4_tests__phase4_structured_scalar.snap`

Existing narrow, medium, wide, detail, raw-payload, search, filter, and help snapshots were updated for the collapsed default, lens breadcrumbs, typed rows, and new shortcuts. The explicit 100,000-node profile initially found a full-timeline rebuild on group return; moving the immutable current-level row cache into navigation frames eliminated that regression and restored the existing navigation threshold.

Phase 5 retains ownership of border removal, horizontal styled-content slicing and scrolling, wrapped/unwrapped layout options, footer collision policy, and the final semantic color treatment. Phase 6 retains parent transcript parity; Phase 7 retains live `/trace` and `Ctrl+T` integration.

Validation on 2026-08-02:

- `just test -p codex-trace`: 76 passed.
- `just test -p codex-trace-tui`: 31 passed; the manual profile remained skipped by default.
- `just test -p codex-trace-tui profile_hundred_thousand_node_navigation --run-ignored only`: passed after cached frame restoration was added.
- `bazel test //codex-rs/trace:trace-unit-tests`: passed.
- `bazel test //codex-rs/trace-tui:trace-tui-unit-tests`: passed under Bazel after normalized JSON object keys were made recursively deterministic across feature sets.
- `cargo clippy -p codex-trace-tui --tests -- -D warnings`: passed.
- `bazel build --config=argument-comment-lint //codex-rs/trace-tui:trace-tui-unit-tests`: passed.
- `cargo insta pending-snapshots --manifest-path trace-tui/Cargo.toml`: no pending snapshots after direct review.
