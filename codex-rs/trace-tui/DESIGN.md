# Trace TUI design

## Responsibility

`codex-trace-tui` presents the normalized `codex-trace` catalog and selected-session model. It owns terminal state and interaction, not source interpretation.

The cross-crate product boundary and evidence model are defined in [`.ariadne/DESIGN.md`](../../.ariadne/DESIGN.md). Source projection and performance characteristics are defined in [`codex-trace/DESIGN.md`](../trace/DESIGN.md).

## Application states

The application moves through:

```text
catalog loading → root picker → selected-session loading → browser
                      ↑                                  ↓
                      └──────── error or back ───────────┘
```

The picker owns precomputed normalized labels, cached query matches, selection, and query state over catalog summaries. The browser controller owns one typed location and a reversible return stack; focused navigation, projection, display, structured-value, detail, and search modules own their orthogonal state and transformations.

Every browser instance receives a unique epoch. Background session and payload results carry their root-session identity, while search, detail, and payload work is admitted through latest-request state containing the browser epoch, request generation, and complete view key. Results replace state only when all three remain current, including when the same session is reloaded into a replacement browser. Cancellation, invalidation, or failure returns to the previous stable screen and exposes an error without installing a partial session.

The accepted architecture is defined by the [presentation-index contract](../../.ariadne/decisions/2026-07-31-trace-presentation-index-contract.md). Browser construction materializes one immutable `PresentationIndex` beside `TraceIndex`; collapsed and expanded lenses consume its precomputed groups and events instead of deriving grouping, order, completeness, or summaries from rendered rows.

## Event loop

The terminal loop polls completed jobs, draws current stable state, reads an input event, and converts that event into an `AppAction`. Discovery, selected-root loading, trace-index construction, initial browser-state construction, and payload reads run in spawned jobs.

No operation proportional to file size or total trace size belongs directly in the event loop. A future long-running model operation must expose progress and cancellation rather than extending a synchronous key handler.

The current rich reducer is synchronous inside its background task. Aborting the task does not guarantee cooperative interruption inside replay; this limitation belongs to the source layer and must remain visible in loading behavior.

## Screens and layout

The root picker shows bounded summary metadata and source badges without loading a full trace. Matching indexes are rebuilt only when a query changes, and each frame formats only the visible match window.

The browser always renders one full-width surface: either the current container's direct children or full-screen detail for one record.

Listing rows contain an elastic name, caller-selected aligned metadata values without labels, optional headers, and an opportunistic bounded preview. Stable columns are the default and remain available through horizontal scrolling; the caller may instead select adaptive priority-based omission. Navigation and record availability do not change with width.

One data-driven column plan owns metadata headers, widths, omission priorities, value extraction, and row assembly. Overview labels, metadata, previews, picker rows, search snippets, and identifiers use a terminal-safe single-line policy; intentional line breaks are preserved only by the separate multiline policy used for detail and message content. All list controllers share one bounded-selection primitive rather than duplicating index arithmetic.

Every row passes through the injected `TraceVisualRenderer`. The parent Codex TUI adapter reuses its terminal palette, message backgrounds, Markdown renderer, and syntax highlighter, while the standalone fallback remains deterministic and plain. Oversized JSON or code lines fall back individually without disabling syntax colors for surrounding safe runs. Role, type, selection, and group boundaries remain visible through text and pinned gutter glyphs so color is never the sole distinction.

Browser lists, details, and structured scalar leaves render directly in the body without a main-surface frame. Breadcrumbs remain above the content; focus-capturing search, filter, and help overlays remain framed. The footer preserves right-aligned lens, interpretation, wrapping, and horizontal-position state, dropping left hints before compacting that state.

## Navigation

Navigation uses canonical locators, group IDs, and typed JSON paths so filtering and projection changes do not make display indexes authoritative. Enter performs location-specific descent; `i` opens any canonical record; `s` opens normalized JSON; back restores the exact parent selection, viewport, and cached current-level rows. There is no fold, expansion, pane, or disclosure state.

Left/Right and `h`/`l` move by the caller-selected small display-cell step, `H`/`L` move by half the visible data width, and `0`/`$` reach horizontal edges. Listing and leaf offsets are independent and stored in reversible frames. New descent and lens locations start at zero; return restores them; resize preserves and clamps them. Wrapped leaf content has no horizontal movement, while listings remain single-line canvases in either content layout.

`TraceLens` selects collapsed group, expanded event, or canonical structural projection. `TraceViewOptions` also accepts the initial session, root-thread, or named-thread scope. `Tab` and `Shift-Tab` cycle lenses; entered groups expose direct members, child groups, and resolved or unresolved evidence references. Group identity and canonical locator remain distinct.

Structured JSON navigation retains typed object keys and array indexes and derives the displayed JSON Pointer from them. It is bounded to 64 levels, 4,096 direct children, 256 preview characters, and 64 KiB of scalar display. Structured locations are browser-local and never alter canonical topology or presentation membership.

Collapsed conversation order follows group anchors. Expanded event order follows canonical order bands and annotates non-contiguous group membership. Entering a group shows its direct members and child groups in canonical chronology. Structural navigation continues to expose every retained node, including records hidden by default from conversation views.

Visible search runs over the enabled semantic classes, while all-record search may temporarily reveal one hidden result without mutating the filter. Submitted searches run in generation-tagged background jobs over shared immutable trace/index snapshots; an edited, cancelled, or superseded query cannot install a stale result. Moving between attributed hits selects their locators and exposes the matching field and snippet. Raw payload search is not part of the current state model.

## Rendering and performance contract

Rendering must scale with terminal area and visible detail, not with total trace size. Lists render a viewport window against a schema-derived canvas width, so maximum horizontal offset never requires scanning the current level. Styled slicing operates on grapheme display widths, preserves Ratatui spans, and substitutes blank cells when a viewport clips a wide grapheme. Repeated labels and summaries should be prepared when state changes rather than serialized during every frame.

The browser builds one `TraceIndex` and one `PresentationIndex` with its selected session and retains only the current location's typed rows. Ordinary cursor movement and redraw therefore touch a terminal-sized viewport rather than flattening or scanning the complete trace. Reversible descent moves the immutable current-level row cache into its return frame; if visibility has not changed, returning restores it without rebuilding a large timeline.

The detail cache is keyed by locator, logical render width, content mode, and payload generation and retains its maximum line width. Semantic extraction and host rendering run in generation-tagged background jobs over shared immutable trace/index snapshots, so resizing or changing modes cannot install stale work. Structured scalar wrapping is cached by locator, typed-path display pointer, width, and layout. Semantic detail is capped at 64 KiB, logical lines at 4,096 display cells, and truncation remains disclosed; loaded raw payloads remain bounded by the source reader. A deterministic 100,000-node profile measures current-level construction, return, warm navigation, and horizontally offset viewport rendering.

Do not solve a rendering freeze by dropping provenance, truncating without disclosure, eagerly loading raw payloads, or hiding malformed nodes.

Presentation-index construction is selected-session or explicit live-update work, never frame work. The TUI consumes bounded typed summaries and canonical content references and applies effective visibility through lens and filter state. It may temporarily reveal hidden search hits without mutating presentation defaults.

The same renderer-neutral snapshot is suitable for the standalone browser and a future parent adapter. Parent palette, Markdown, syntax, terminal lifecycle, command dispatch, and composer restoration remain renderer or adapter responsibilities; parent `HistoryCell` types are a parity oracle, not a dependency of `codex-trace-tui`.

## Detail modes

Full-screen detail shows labeled kind, class, source, evidence grade, timestamp, status, and identity metadata above record content.

Rendered mode preserves semantic Markdown, JSON, and code formats for the host renderer. Text mode displays interpreted semantic content with decoded line breaks. Raw mode displays exact raw bytes only through a contained payload handle; otherwise it labels normalized JSON as semantic rather than exact evidence.

Raw payload references remain unloaded in listings. Opening one dispatches a background read through the model's contained reader and installs sanitized bounded text when complete. Rendering never executes or interprets source ANSI, hyperlink, shell, or terminal control content.

Conflicts preserve both source observations. Unavailable data is labeled unavailable, not represented by an empty string or a plausible reconstruction.

## Failure behavior

A catalog failure leaves an actionable error screen. A selected-root failure returns to the picker or prior stable browser. A payload failure remains local to its inspector. Unsupported terminal size degrades layout rather than panicking.

Errors should identify the failed operation without echoing excessive sensitive payload content. Diagnostics from the source model remain navigable.

## Tests

State-transition tests cover key actions, loading, cancellation, errors, level return, search, filters, detail modes, and payload installation. Ratatui snapshots cover narrow, medium, and wide layouts using deterministic synthetic data.

The ignored `profile_hundred_thousand_node_navigation` test deterministically builds a limit-sized ordinary trace, records index/browser construction, first large-level entry, 1,000 warm navigation samples, repeated level return/entry samples, and horizontally offset borderless renders, and enforces a 50 ms p95 for repeated operations. End-to-end tests operate without network or authentication and compare every input byte before and after browsing.
