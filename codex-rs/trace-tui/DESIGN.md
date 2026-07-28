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

The picker owns selection and query state over catalog summaries. The browser owns a current locator, breadcrumb path, active pane, inspector state, search query and matches, viewport positions, and lazily installed payload content.

Background results are tagged by their requested operation before they replace state. Cancellation or failure returns to the previous stable screen and exposes an error without installing a partial session.

## Event loop

The terminal loop polls completed jobs, draws current stable state, reads an input event, and converts that event into an `AppAction`. Discovery, selected-root loading, trace-index construction, initial browser-state construction, and payload reads run in spawned jobs.

No operation proportional to file size or total trace size belongs directly in the event loop. A future long-running model operation must expose progress and cancellation rather than extending a synchronous key handler.

The current rich reducer is synchronous inside its background task. Aborting the task does not guarantee cooperative interruption inside replay; this limitation belongs to the source layer and must remain visible in loading behavior.

## Screens and layout

The root picker shows bounded summary metadata and source badges without loading a full trace.

The browser has three conceptual surfaces:

1. breadcrumb and thread hierarchy;
2. ordered children of the selected node;
3. semantic or raw inspector.

Wide terminals show all three. Medium terminals combine hierarchy with the active content surface. Narrow terminals show one surface at a time. The same enter, back, pane, and search actions apply at every width, and no information is available only through color.

## Navigation

Navigation is locator-based so filtering and sorting do not invalidate identity. Enter moves to a directory-like child or opens item detail; back returns through the breadcrumb path; parent and child thread movement uses model relationships rather than parsing display text.

Search runs on explicit submission and returns model-attributed hits. Moving between hits selects their locators and exposes the matching field and snippet. Raw payload search is not part of the current state model.

## Rendering and performance contract

Rendering must scale with terminal area and visible detail, not with total trace size. Lists render a viewport window. Wrapped detail should be cached by locator, width, display mode, and payload state. Repeated labels and summaries should be prepared when state changes rather than serialized during every frame.

The browser builds one `TraceIndex` with its selected session, caches the flattened visible tree until expansion state changes, caches the ordered children of the selected node, and keeps the selected visible-row position. Ordinary cursor movement and redraw therefore do not scan the complete trace. Tree, child, search, and diagnostic renderers materialize only a terminal-sized viewport.

The inspector cache is keyed by locator, width, and payload generation. Structured JSON display is capped at 64 KiB and discloses truncation. Loaded raw payloads remain bounded by the source reader, and the renderer wraps only the requested scroll window; it does not construct a complete wrapped payload on open or on every frame.

Normal expansion splices the selected node's materialized descendants immediately after its visible row, and normal collapse drains the contiguous descendant range below that row. Their cost is proportional to the affected subtree rather than the complete visible tree; a full rebuild remains only as defensive recovery when cached selection state is inconsistent. The deterministic 100,000-node profile records expansion and collapse separately from warm navigation so a pathological single-parent fan-out remains visible rather than being averaged away.

Do not solve a rendering freeze by dropping provenance, truncating without disclosure, eagerly loading raw payloads, or hiding malformed nodes.

## Inspector

The semantic inspector shows kind, source, evidence grade, timestamps or sequence, capabilities, source-local identity, relationships, diagnostics, and bounded structured fields.

Raw payload references begin collapsed. Opening one dispatches a background read through the model's contained reader and installs sanitized bounded text when complete. Rendering never interprets ANSI, hyperlink, shell, or terminal control content. The inspector presents only its active window, while the underlying bounded payload remains available for scrolling.

Conflicts preserve both source observations. Unavailable data is labeled unavailable, not represented by an empty string or a plausible reconstruction.

## Failure behavior

A catalog failure leaves an actionable error screen. A selected-root failure returns to the picker or prior stable browser. A payload failure remains local to its inspector. Unsupported terminal size degrades layout rather than panicking.

Errors should identify the failed operation without echoing excessive sensitive payload content. Diagnostics from the source model remain navigable.

## Tests

State-transition tests cover key actions, loading, cancellation, errors, search, and payload installation. Ratatui snapshots cover narrow, medium, and wide layouts using deterministic synthetic data.

The ignored `profile_hundred_thousand_node_navigation` test deterministically builds a limit-sized ordinary trace, records index/browser construction, first large-subtree expansion, 1,000 warm navigation samples, and 20 warm expand/collapse samples, and enforces a 50 ms p95 for navigation and expansion. Ordinary tests assert that inspector rendering retains at most a viewport of a large payload and that visible-row state is cached. End-to-end tests operate without network or authentication and compare every input byte before and after browsing.
