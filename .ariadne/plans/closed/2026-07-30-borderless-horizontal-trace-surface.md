# Borderless Horizontal Trace Surface

Updated: 2026-08-02

Status: closed

## Objective

Apply a borderless, full-body, horizontally navigable visual language to every stable Ariadne browser lens while preserving configurable wrapping, semantic colors, breadcrumbs, footer state, and terminal-safe bounded rendering.

## Context

This is Phase 5 and supersedes the closed [Borderless Horizontal Trace Overview](2026-07-29-borderless-horizontal-trace-overview.md). The earlier plan targeted ungrouped record rows; this plan retains its useful canvas, footer, Unicode, bounds, and shortcut requirements while applying them to groups, events, trace nodes, and structured values.

This phase depends on [closed Phase 4](2026-07-30-trace-browser-lenses-and-structured-navigation.md) and preserves the renderer-neutral ownership, effective-visibility, and viewport bounds of the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

Phase 4 supplies `TraceLens`, caller-selected initial scope, explicit group/detail/structured locations, typed current-level rows, cached reversible frames, presentation-hidden filtering, and bounded typed JSON paths. Phase 5 extends those types rather than rebuilding groups or introducing a second navigation controller.

## Scope

Owned paths are `codex-rs/trace-tui/`, the existing narrow parent visual-renderer adapter, focused tests and snapshots, and TUI design documentation.

Live `/trace`, `Ctrl+T` replacement, parent transcript grouping, persisted configuration, raw payload search, marks, exports, annotations, recorder changes, and app-server changes are excluded.

## Accepted Interaction Decisions

| Concern | Decision |
| --- | --- |
| Default content layout | Unwrapped leaf content; listings always remain single-line canvases |
| Default columns | Stable configured columns; adaptive omission remains caller-selectable |
| Small movement | Four display cells through Left/Right or `h`/`l` |
| Large movement | Half the data viewport through `H`/`L`; edges through `0`/`$` |
| Logical line bound | 4,096 display cells, represented by a validated option |
| Offset lifetime | Separate listing and leaf offsets; reset on new descent, lens, search-driven location, and session; restore on back; preserve and clamp on resize |
| Footer collision | Remove verbose then compact left hints, abbreviate right state, and retain the right-aligned horizontal position last |

## Movements

### Movement 1: Freeze View Options

- Add explicit renderer-neutral enums for wrapped versus unwrapped content, stable versus adaptive columns, headers, preview policy, horizontal step, and bounded content width.
- Keep content interpretation mode independent from line-layout mode.
- Define separate overview and detail horizontal offsets and their reset, restore, resize, and clamp behavior.
- Store horizontal offsets in the existing reversible location frames and keep them independent from `TraceLens`, JSON path, selection, and vertical viewport state.

### Movement 2: Build Styled Horizontal Viewports

- Slice Ratatui lines and spans by terminal display width without discarding syntax or Markdown styles.
- Handle zero width, combining characters, double-width characters, controls, offsets inside spans, and right-edge partial glyphs deterministically.
- Build one shared logical canvas for headers and rows and avoid eager raw-payload access.

### Movement 3: Remove Main-Surface Frames

- Preserve breadcrumbs at the top and render the current list or detail directly in the full body.
- Keep search, filter, help, and other focus-capturing overlays visibly distinct.
- Place left-aligned key hints and right-aligned interpretation, layout, lens, and horizontal-position state in the footer.
- Elide left hints before essential right-side mode state when the terminal is narrow.

### Movement 4: Apply Group And Record Styling

- Preserve semantic backgrounds for user, assistant, reasoning, tool, system, and developer families.
- Add non-color textual or glyph distinctions for group kind, record type, status, evidence, and selection.
- Use a stable gutter or restrained separators for group boundaries so interleaved groups do not overload semantic background colors.
- Keep normal JSON lines highlighted when one oversized value exceeds the parent highlighter's safe line limit, without weakening upstream safety bounds.

### Movement 5: Add Horizontal Navigation

- Bind Left/Right and `h`/`l` to the configured small movement in horizontally navigable surfaces.
- Add documented half-page or edge motions selected by the Phase 0 decision.
- Preserve editor and overlay key meanings and keep horizontal movement independent of vertical selection and search.

## Acceptance Evidence

- Every listing row and optional header shares an aligned display-width-safe horizontal canvas.
- Wrapped and unwrapped modes are both selectable through `TraceViewOptions` before any CLI or configuration wiring exists.
- Breadcrumbs remain stable, the main content is borderless, and the footer collision policy is deterministic.
- Syntax, Markdown, class, status, evidence, group boundary, and selection distinctions remain visible on large and narrow content without relying only on color.
- Reviewed snapshots cover every lens at default and non-zero offsets, narrow and wide terminals, Unicode, controls, long JSON values, partial groups, and parent-renderer colors.
- Rendering remains viewport-bounded with no pending snapshots.

## Closure

Close this plan after recording the option defaults, shortcut table, footer degradation policy, snapshot paths, structural performance evidence, exact validation commands, and remaining parent-parity work transferred to Phase 6.

## Implementation Receipt

Phase 5 is implemented in `codex-trace-tui` without changing persisted trace formats, canonical topology, model-visible context, app-server APIs, or parent command orchestration. The only upstream-owned implementation change is the existing `CodexTraceVisualRenderer` adapter, where bounded safe-line runs allow ordinary JSON and code lines to retain parent syntax colors when one line exceeds the existing per-line highlighter limit. Aggregate and per-line safety limits remain authoritative.

`TraceViewOptions` now separates content interpretation, content layout, and column layout. Its defaults are unwrapped leaf content, stable configured columns, automatic headers and previews, a four-cell small horizontal step, and a validated 4,096-display-cell logical-line limit. Callers may select wrapped content or adaptive columns without CLI or configuration plumbing. Listings always use one aligned single-line logical canvas; headers and rows are sliced at the same terminal display offset.

Horizontal state is owned by `browser::horizontal` and keeps independent overview and leaf viewports. Left/Right and `h`/`l` move four configured display cells, `H`/`L` move half a viewport, and `0`/`$` move to the edges. New descent, lens changes, search-driven reveal, and session installation reset offsets; reversible frames restore prior offsets on return; render-time extent updates clamp them after resize. Wrapped leaves report `x 0/0` and reject horizontal movement.

The main browser body is borderless. Breadcrumbs retain the top line, while listing headers are underlined rather than framed. A pinned two-cell gutter identifies selection, groups, child groups, event/member boundaries, references, JSON values, structural nodes, and notices independently of color. The footer first removes verbose left hints, then compact hints, then abbreviates the right-side lens, interpretation, and layout state; the horizontal position remains the last retained state. Search, filter, help, loading, picker, and error overlays keep visible focus boundaries.

Styled slicing is isolated in `render::viewport`; gutters and footer composition are isolated in `render::gutter` and `render::footer`. Slicing operates on grapheme clusters and terminal display width, preserves span and line styles, neutralizes controls, and renders partial wide glyphs as styled blanks. Detail and structured-scalar caches retain bounded logical lines and maximum widths so frame rendering only slices the current viewport. Structured scalar display remains capped at 64 KiB, and listing previews remain capped at 256 characters.

The reviewed Phase 5 snapshots are:

- `codex_trace_tui__phase5_tests__phase5_collapsed_horizontal.snap`
- `codex_trace_tui__phase5_tests__phase5_expanded_horizontal.snap`
- `codex_trace_tui__phase5_tests__phase5_structural_horizontal.snap`
- `codex_trace_tui__phase5_tests__phase5_structured_horizontal.snap`
- `codex_trace_tui__phase5_tests__phase5_unwrapped_detail.snap`
- `codex_trace_tui__phase5_tests__phase5_wrapped_detail.snap`
- `codex_trace_tui__phase5_tests__phase5_narrow_footer.snap`

Existing Phase 4 and browser snapshots were reviewed and updated for the borderless body, shared stable canvas, pinned gutter, footer collision behavior, and expanded help. The parent adapter uses an inline style-presence snapshot for the safe/oversized/safe JSON sequence, keeping the narrow upstream-owned footprint to the adapter and its focused test.

The deterministic 100,000-node profile measured `build_ms=3857.858`, `expanded_ms=123.115`, `structural_ms=21.726`, `navigation_p95_ms=0.000`, `group_entry_p95_ms=31.619`, `return_p95_ms=0.007`, and `horizontal_render_p95_ms=6.438`. Navigation, entry, return, and horizontally offset render p95 values remain below their 50 ms gates.

Validation on 2026-08-02:

- `just test -p codex-trace-tui`: 40 passed and one manual profile remained ignored.
- `just test -p codex-trace-tui profile_hundred_thousand_node_navigation --run-ignored only --no-capture`: passed with the measurements recorded above.
- `just test -p codex-tui trace_visual_renderer`: four focused parent-renderer tests passed.
- `just test -p codex-tui`: 3,256 passed and four were skipped.
- `bazel test //codex-rs/trace-tui:trace-tui-unit-tests //codex-rs/tui:tui-unit-tests`: both targets passed; one unrelated safety-buffering assertion failed on its first parent shard attempt and passed on Bazel's retry, so Bazel reported that target as flaky but successful.
- `bazel build --config=argument-comment-lint //codex-rs/trace-tui:trace-tui-unit-tests //codex-rs/tui:tui-unit-tests`: passed.
- `cargo insta pending-snapshots --manifest-path trace-tui/Cargo.toml` and `cargo insta pending-snapshots --manifest-path tui/Cargo.toml`: no pending snapshots.
- An interactive `RUST_LOG=trace` run with an isolated `log_dir` opened a real local trace, rendered the borderless stable canvas, exercised small and half-page horizontal movement, edges, lens changes, and clean quit without errors.
- `just fix -p codex-trace-tui` and `just fix -p codex-tui`: passed; the trace-TUI pass applied one test-only simplification and the parent pass found no fix.
- `just fmt`: passed after all Rust changes.

Phase 6 retains semantic and visual comparison with parent `HistoryCell` behavior. Phase 7 retains live `/trace` and `Ctrl+T` integration, and Phase 8 retains the locally maintainable parent-compatible candidate. Persisted configuration, runtime layout toggles, search/filter extensions, marks, exports, annotations, and cross-platform release evidence remain outside this phase.
