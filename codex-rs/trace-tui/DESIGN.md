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

The picker owns precomputed normalized labels, cached query matches, selection, and query state over catalog summaries. The browser navigation controller owns a current container and locator-based return stack; sibling detail and search controllers own their caches, generations, visibility, and lazily installed payload content.

Every browser instance receives a unique epoch. Background session and payload results carry their root-session identity, while search, detail, and payload work is admitted through latest-request state containing the browser epoch, request generation, and complete view key. Results replace state only when all three remain current, including when the same session is reloaded into a replacement browser. Cancellation, invalidation, or failure returns to the previous stable screen and exposes an error without installing a partial session.

## Event loop

The terminal loop polls completed jobs, draws current stable state, reads an input event, and converts that event into an `AppAction`. Discovery, selected-root loading, trace-index construction, initial browser-state construction, and payload reads run in spawned jobs.

No operation proportional to file size or total trace size belongs directly in the event loop. A future long-running model operation must expose progress and cancellation rather than extending a synchronous key handler.

The current rich reducer is synchronous inside its background task. Aborting the task does not guarantee cooperative interruption inside replay; this limitation belongs to the source layer and must remain visible in loading behavior.

## Screens and layout

The root picker shows bounded summary metadata and source badges without loading a full trace. Matching indexes are rebuilt only when a query changes, and each frame formats only the visible match window.

The browser always renders one full-width surface: either the current container's direct children or full-screen detail for one record.

Listing rows contain an elastic name, caller-selected aligned metadata values without labels, optional headers, and an opportunistic bounded preview. Lower-priority columns and previews disappear as width contracts; navigation and record availability do not change with width.

One data-driven column plan owns metadata headers, widths, omission priorities, value extraction, and row assembly. Overview labels, metadata, previews, picker rows, search snippets, and identifiers use a terminal-safe single-line policy; intentional line breaks are preserved only by the separate multiline policy used for detail and message content. All list controllers share one bounded-selection primitive rather than duplicating index arithmetic.

Every row passes through the injected `TraceVisualRenderer`. The parent Codex TUI adapter reuses its terminal palette, message backgrounds, Markdown renderer, and syntax highlighter, while the standalone fallback remains deterministic and plain. Role and type remain visible in text so color is never the sole distinction.

The later removal of the overview border, shared horizontal scrolling, progressively appended bounded content, and footer mode placement are specified in [the open borderless overview plan](../../.ariadne/plans/open/2026-07-29-borderless-horizontal-trace-overview.md). They are not part of the current rendering contract until that plan's interaction and configuration decisions are resolved.

## Navigation

Navigation is locator-based so filtering and sorting do not invalidate identity. Enter moves to a child container or opens leaf detail; `i` opens any record; back closes detail or restores the exact parent selection and viewport. There is no fold, expansion, pane, or disclosure state.

Visible search runs over the enabled semantic classes, while all-record search may temporarily reveal one hidden result without mutating the filter. Submitted searches run in generation-tagged background jobs over shared immutable trace/index snapshots; an edited, cancelled, or superseded query cannot install a stale result. Moving between attributed hits selects their locators and exposes the matching field and snippet. Raw payload search is not part of the current state model.

## Rendering and performance contract

Rendering must scale with terminal area and visible detail, not with total trace size. Lists render a viewport window. Wrapped detail should be cached by locator, width, display mode, and payload state. Repeated labels and summaries should be prepared when state changes rather than serialized during every frame.

The browser builds one `TraceIndex` with its selected session and retains only the current container's filtered child positions. Ordinary cursor movement and redraw therefore touch a terminal-sized viewport rather than flattening or scanning the complete trace.

The detail cache is keyed by locator, width, content mode, and payload generation. Semantic extraction and host rendering run in generation-tagged background jobs over shared immutable trace/index snapshots, so resizing or changing modes cannot install stale work. Semantic detail is capped at 64 KiB and discloses truncation; loaded raw payloads remain bounded by the source reader. A deterministic 100,000-node profile measures current-level construction, return, and warm navigation without expansion-specific state.

Do not solve a rendering freeze by dropping provenance, truncating without disclosure, eagerly loading raw payloads, or hiding malformed nodes.

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

The ignored `profile_hundred_thousand_node_navigation` test deterministically builds a limit-sized ordinary trace, records index/browser construction, first large-level entry, 1,000 warm navigation samples, and repeated level return/entry samples, and enforces a 50 ms p95 for navigation and entry. End-to-end tests operate without network or authentication and compare every input byte before and after browsing.
