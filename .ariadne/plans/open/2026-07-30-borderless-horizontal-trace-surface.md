# Borderless Horizontal Trace Surface

Updated: 2026-08-02

Status: open

## Objective

Apply a borderless, full-body, horizontally navigable visual language to every stable Ariadne browser lens while preserving configurable wrapping, semantic colors, breadcrumbs, footer state, and terminal-safe bounded rendering.

## Context

This is Phase 5 and supersedes the closed [Borderless Horizontal Trace Overview](../closed/2026-07-29-borderless-horizontal-trace-overview.md). The earlier plan targeted ungrouped record rows; this plan retains its useful canvas, footer, Unicode, bounds, and shortcut requirements while applying them to groups, events, trace nodes, and structured values.

This phase depends on [closed Phase 4](../closed/2026-07-30-trace-browser-lenses-and-structured-navigation.md) and must preserve the renderer-neutral ownership, effective-visibility, and viewport bounds of the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

Phase 4 now supplies `TraceLens`, caller-selected initial scope, explicit group/detail/structured locations, typed current-level rows, cached reversible frames, presentation-hidden filtering, and bounded typed JSON paths. Phase 5 must extend those types rather than rebuilding groups or introducing a second navigation controller.

## Scope

Owned paths are `codex-rs/trace-tui/`, the existing narrow parent visual-renderer adapter, focused tests and snapshots, and TUI design documentation.

Live `/trace`, `Ctrl+T` replacement, parent transcript grouping, persisted configuration, raw payload search, marks, exports, annotations, recorder changes, and app-server changes are excluded.

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
