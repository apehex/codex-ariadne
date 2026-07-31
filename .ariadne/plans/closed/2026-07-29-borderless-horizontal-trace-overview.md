# Borderless Horizontal Trace Overview

Updated: 2026-07-30

Status: closed; superseded by the phased borderless-surface plan

## Objective

Evolve the trace browser overview into a borderless, full-width, single-line record surface that behaves like a compact `less -S` timeline: metadata, progressively available content, and navigation state remain aligned on one horizontal canvas, while the selected record still opens as a full-screen detail view.

This is a subsequent UI/UX phase. The internal factorization completed on 2026-07-29 provides terminal-safe single-line text, data-driven columns, reusable selection, and request-lifecycle primitives, but it does not authorize or implement the visual redesign described here.

## Context

The [closed single-panel plan](2026-07-28-single-panel-trace-browser.md) replaced folding and split panes with one-level-at-a-time navigation. The [quality remediation plan](../open/2026-07-28-trace-browser-quality-remediation.md) remains the correctness, bounds, lifecycle, documentation, and compatibility gate. This plan owns only the next overview presentation and navigation changes.

The current browser encloses the record list in a bordered block, fits each row to the terminal width, and progressively drops configured metadata columns and preview content as space contracts. That behavior is safe and responsive, but it prevents the overview from using the complete terminal width and makes a wide record impossible to inspect without entering detail.

The intended design preserves one logical line per overview record. Newlines, tabs, carriage returns, ANSI escapes, and other terminal controls must remain neutralized in the overview; interpreted line breaks belong only to full-screen detail. Horizontal movement changes the visible slice of the overview canvas and never changes the logical row structure.

## Scope

Owned paths are `codex-rs/trace-tui/`, focused trace-TUI snapshots and tests, and trace-browser design and release evidence under `.ariadne/`.

The following are explicit exclusions:

- live `/trace` integration in an active parent Codex TUI conversation;
- changes to persisted rollout, rich bundle, protocol, app-server, configuration, or model-visible context formats;
- raw payload search, mutation, execution, or unbounded eager payload reads;
- changes to upstream-owned Codex TUI modules beyond the existing narrow renderer adapter;
- implementation before the unresolved interaction and configuration choices below are discussed and recorded.

## Movements

### Movement 0: Resolve Interaction And Configuration Choices

Record decisions for:

- the horizontal step used by Left/Right and `h`/`l`, including whether modified keys move by a half-page or full page;
- whether the horizontal offset is shared across all rows and headers, and when it resets during descent, return, filter changes, resize, or session replacement;
- the default and configurable byte, character, or display-width limit for progressively appended record content;
- whether content expansion is automatic by available width, explicitly configurable, or both;
- how footer mode information and key hints degrade or elide when the terminal is too narrow;
- whether adaptive metadata-column omission remains enabled before horizontal scrolling, becomes a configurable policy, or is disabled in favor of a stable wide schema.

Exit evidence is a short decision table added to this plan before implementation begins.

### Movement 1: Model One Horizontal Overview Canvas

Extend the data-driven column plan so headers and record rows expose a common logical display width and can render a display-width-safe slice at one shared horizontal offset.

Keep metadata values label-free in rows and keep optional headers aligned to the same slice. Preserve configurable column visibility, header visibility, preview policy, and semantic row styling.

Append bounded record content progressively after configured metadata instead of wrapping it. Content preparation must reuse bounded semantic presentation and must not open raw payload handles or scan complete trace subtrees during rendering.

Define an explicit horizontal viewport state with clamped movement, resize behavior, and locator-based restoration where the Movement 0 decision requires it. Keep this state orthogonal to vertical selection, current-level navigation, filtering, search, and detail scrolling.

Required tests cover zero-width and narrow canvases, wide Unicode, combining and double-width characters, controls and ANSI sequences, empty metadata, omitted columns, content caps, offset clamping, and exact header/row alignment.

### Movement 2: Remove The Overview Border And Rebalance The Footer

Render the current-level overview directly into the full body rectangle without a surrounding border or title. Preserve a readable header/body boundary through spacing or restrained styling rather than another enclosing panel.

Place current mode information at the bottom right on the same terminal line as the left-aligned key hints. Define deterministic collision behavior for narrow terminals and keep role or record type visible without relying on color alone.

Retain the full-screen detail, search, filter, help, picker, loading, and error surfaces unless a separately reviewed change demonstrates that the borderless language should apply to them too.

Required snapshots cover narrow, medium, wide, zero-record, long-content, Unicode, control-character, horizontally offset, filtered, searched, and parent-renderer color states.

### Movement 3: Add Conventional Horizontal Navigation

Bind Left/Right and `h`/`l` to the chosen horizontal movement while the overview has focus. Preserve existing meanings in search editors, overlays, and detail views.

Expose the horizontal position and remaining overflow unobtrusively when content exists outside the visible slice. Avoid a persistent scrollbar unless the Movement 0 discussion finds it materially clearer than a compact footer indicator.

Document the shortcuts in help and snapshot the key-hint and mode-information layouts. Keep page up/down, richer search, marks, and type visibility controls as separate later movements unless their implementation is already independently planned.

## Acceptance Evidence

- Every overview record and optional header is one terminal-safe logical line before viewport slicing.
- Header, metadata, preview, and appended content remain aligned at every horizontal offset.
- No overview render path opens raw artifacts, performs unbounded semantic work, or visits records outside the visible vertical window.
- Horizontal navigation is independent of vertical selection, filtering, search hits, level descent, and detail scrolling.
- The borderless overview uses the complete body width and the footer presents left key hints with right mode information under a documented narrow-width policy.
- Reviewed snapshots demonstrate the default position, non-zero offsets, Unicode display width, controls, parent-renderer colors, and narrow-terminal degradation with no pending `.snap.new` files.
- Focused `codex-trace-tui` tests, the 100,000-node profile, scoped lint fixes, and formatting pass in the repository-prescribed order.

## Closure

Close this plan only after recording the Movement 0 decisions, the final shortcut and configuration surface, structural performance evidence, reviewed snapshot paths, exact local validation commands and results, and any residual navigation or accessibility work.

Closing this plan does not close the quality remediation plan and does not authorize a push, tag, release, live `/trace` integration, or modification of upstream-owned TUI modules.

## Supersession Record

This plan closed on 2026-07-30 without claiming implementation under its original ungrouped-row model.

Its useful decisions and acceptance requirements for the shared horizontal canvas, footer placement, Unicode display width, bounded content, navigation shortcuts, snapshots, and structural performance were transferred to the [Borderless Horizontal Trace Surface](../open/2026-07-30-borderless-horizontal-trace-surface.md).

The replacement is Phase 5 of the presentation program and depends on the earlier order, presentation-index, grouping-policy, and browser-lens phases. That dependency changes the unit rendered by the surface from an assumed record row to a lens-provided group, event, trace node, or structured value.

No implementation evidence, validation result, push, tag, release, or upstream integration is claimed by this closure.
