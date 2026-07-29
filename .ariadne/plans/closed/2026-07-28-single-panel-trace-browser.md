# Single-Panel Trace Browser And Semantic Record Viewer

Updated: 2026-07-29

Status: closed; single-panel implementation completed and residual quality gates transferred

## Objective

Replace the expandable multi-pane trace browser with a single-panel, single-depth interface that behaves like a focused file browser and provides full-screen semantic inspection of every record.

At each level, display only the current node's direct children. Enter descends into containers or opens leaf detail; `i` opens detail for any record; Escape closes detail or returns one level while restoring selection and viewport.

Listing rows show an elastic name followed by aligned metadata values without per-row labels. Optional headers identify columns, remaining width hosts a bounded one-line preview, and every record passes through one styling interface so user, LLM, and tool records can receive distinct theme-aware horizontal backgrounds.

Full-screen detail supports Rendered, Text, and Raw modes. Semantic decoding restores real line breaks and structured content while terminal controls remain inert. Raw mode displays an exact bounded artifact when directly available and otherwise explicitly labels normalized JSON as semantic rather than exact evidence.

## Context

Before this plan, the browser maintained expansion state, a flattened tree, separate child selection, three panes, inspector scrolling, and width-dependent pane switching. The implemented model replaces these with a locator-based stack of single-level list states.

Rich reduction already preserves roles, channels, content parts, code languages, and raw-payload references; ordinary rollouts retain typed response items. Classification and content extraction belong in `codex-trace`, not in per-frame JSON parsing.

The parent `codex-tui` owns mature Markdown, syntax, wrapping, ANSI, terminal-palette, and message-style modules, but most are private. Reuse them through a renderer contract owned by `codex-trace-tui` and implemented by `codex-tui`; do not make `codex-trace-tui` depend on `codex-tui`, because the future `/trace` embedding needs the opposite dependency direction.

This plan supersedes only the multi-pane, fold/expand, and expansion-specific UI requirements in the credible-browser and quality-remediation plans. Their correctness, evidence, source-bound, payload-containment, lifecycle, privacy, compatibility, CI, and documentation requirements remain authoritative.

## Scope

Owned work includes normalized record presentation, trace-TUI state and rendering, a narrow parent-TUI renderer adapter, CLI construction of that adapter, focused tests and snapshots, design documentation, and reconciliation of the existing open plans.

Defer live `/trace`, persisted UI configuration, new CLI settings, `:` commands, marks, jump lists, relationship motions, comparisons, exports, annotations, custom highlight rules, and live follow.

Preserve the offline, read-only boundary. Do not modify source traces, initialize authentication, make model calls, or load raw payloads merely because a row becomes visible.

Keep complex implementation changes below 500 changed lines and non-mechanical changes below 800 lines. Write Markdown as one physical line per logical paragraph, list item, table row, or code line.

## Movements

### Movement 1: Typed Record Presentation

- Give every node a bounded presentation descriptor containing a primary record class, optional role/channel/status, and a one-line preview.
- Classify structure, system, developer, user, assistant, commentary, final answer, reasoning, tool input/output, code, delegation, compaction, diagnostics, raw artifacts, and unknown records deterministically for ordinary and rich sources.
- Keep visibility derived from view filters rather than storing mutable evidence state.
- Add bounded semantic content extraction for Markdown, text, JSON, and code without duplicating unbounded bodies.

### Movement 2: Shared Visual Rendering

- Define an object-safe `TraceVisualRenderer` in `codex-trace-tui`.
- Implement `CodexTraceVisualRenderer` in a focused parent-TUI module using the existing Markdown, highlighting, wrapping, ANSI, palette, and styling helpers.
- Pass the renderer and `TraceViewOptions::default()` from the CLI.
- Run expensive width-dependent content rendering in generation-tagged background work and keep deterministic fake rendering available to trace-TUI tests.

### Movement 3: Single-Level Navigation And Listing

- Replace panes, expansion sets, flattened rows, and disclosure controls with a navigation stack that stores container, selection, and viewport.
- Enter descends or opens leaf detail, `i` opens any detail, and Escape closes overlays/detail or restores the parent list.
- Render a single full-width list with a required name, configurable aligned metadata values, optional headers, and an opportunistic preview.
- Use explicit `TraceViewOptions`, `TraceColumn`, `HeaderMode`, and `PreviewMode` types rather than positional flags or global settings.
- Apply one row-style resolver to all records; use subtle terminal-aware backgrounds for user, LLM, and tool families with redundant glyph/text distinctions.

### Movement 4: Full-Screen Detail Modes

- Show labeled metadata followed by full-width content.
- Cycle Rendered, Text, and Raw with `v`.
- Preserve semantic newlines, render Markdown/JSON/code with parent helpers, and treat terminal controls as inert.
- Load an exact raw artifact lazily when directly linked; otherwise show bounded normalized JSON labeled as non-exact.
- Keep detail rendering and wrapping bounded and cached by locator, width, mode, theme, and payload generation.

### Movement 5: Visibility, Search, And Motions

- Add an all-enabled visibility filter over primary record classes; `f` edits it and `F` clears it.
- `/` searches visible records and `g/` searches all records; a hidden all-record result receives a temporary locator-specific reveal without mutating the filter.
- Support arrows, `j`/`k`, Page Up/Down, Ctrl-U/D, `gg`, `G`, `n`/`N`, `?`, and `q`.
- Show path, visible/total/hidden counts, content mode, filter state, and responsive column omissions in the status line.

### Movement 6: Evidence And Validation

- Replace expansion-specific performance evidence with current-level construction, navigation, paging, level return, filtering, search reveal, and detail-open evidence on deterministic 100,000-node input.
- Add state-transition tests and narrow/medium/wide Ratatui snapshots for all user-visible behavior.
- Add focused adapter and trace coverage to Linux, macOS, and Windows CI.
- Update module design and usage documentation.

## Acceptance Evidence

- Every width presents one list or one full-screen detail surface; fold/expand state, persistent panes, and Tab navigation are absent.
- Returning from a child or detail restores the exact locator and viewport.
- Row metadata is aligned and label-free, headers and previews are responsive, and only viewport rows are prepared.
- Every record uses the common style resolver and remains distinguishable without color.
- Rendered mode covers Markdown, JSON, code, messages, reasoning, and tool output; Text mode preserves decoded line breaks; Raw mode distinguishes exact artifacts from normalized semantic JSON.
- Raw payloads remain lazy, contained, bounded, and terminal-safe.
- Filtering is derived and reversible; visible and all-record search scopes behave distinctly and stale jobs cannot replace newer state.
- The deterministic large-trace profile proves viewport-bounded repeated work and is invoked by configured CI.
- Existing correctness, evidence, containment, byte-preservation, CLI compatibility, and cross-platform gates remain governed by their successor plans.

## Implementation progress

- Added typed record classes, roles, channels, status, bounded previews, and terminal-safe semantic Markdown/text/JSON/code documents to `codex-trace`, with ordinary and rich projection populating the presentation descriptor once.
- Replaced the pane and fold/expand model with a locator-based current-level list and return stack that retains parent selection and viewport.
- Added elastic name plus configurable label-free metadata columns, responsive headers and previews, semantic-class filters, visible/all search scopes, temporary hidden-result reveal, full-screen detail, Rendered/Text/Raw modes, help, paging, and Vim-style motions.
- Added generation-tagged background jobs for whole-trace search and width-dependent semantic extraction/rendering; immutable traces and indexes are shared through `Arc`, and stale query, mode, width, or payload generations cannot install results.
- Added a host renderer contract and a focused `codex-tui` adapter that reuses parent Markdown, syntax highlighting, wrapping, terminal palette, and user-message background rules; the `codex trace` CLI injects that adapter while the trace-TUI crate retains a deterministic plain fallback.
- Updated focused Linux, macOS, and Windows trace CI to compile and test the parent renderer adapter and added a Linux 100,000-node single-level navigation profile invocation.
- Kept every production module below the repository's 800-line review threshold. `model.rs` remains above the 500-line target because `TraceNode`, its backward-compatible serialized presentation descriptor, and semantic document extraction must evolve as one public model boundary. `browser.rs` remains above the target because locator return, filter reveal, and stale-generation installation invariants share one state owner; background job preparation was extracted to `jobs.rs`. `render.rs` remains above the target because it contains the single coherent screen renderer plus exhaustive textual mappings that preserve non-color accessibility across every node/class/status/evidence variant.

## Recorded validation

- Final API boundary: `codex-trace` owns `TraceRecordClass`, `TraceRecordRole`, `TraceRecordChannel`, `TraceContentFormat`, `TraceContentDocument`, and bounded semantic extraction; `codex-trace-tui` owns `TraceViewOptions`, `TraceColumn`, `HeaderMode`, `PreviewMode`, `ContentMode`, `TraceRenderRequest`, `TraceRowStyleRequest`, and the object-safe `TraceVisualRenderer`; `codex-tui` supplies `CodexTraceVisualRenderer`; the CLI injects that adapter through `run_with_renderer`.
- Module sizes after formatting are 761 lines for `trace/src/model.rs`, 579 for `trace-tui/src/app.rs`, 732 for `trace-tui/src/browser.rs`, 165 for `trace-tui/src/jobs.rs`, 739 for `trace-tui/src/render.rs`, 166 for `trace-tui/src/view.rs`, and 119 for `tui/src/trace_visual_renderer.rs`.
- `just test -p codex-trace` passed 19 tests.
- `just test -p codex-trace-tui` passed 11 tests with the manual profile skipped; `cargo insta pending-snapshots --manifest-path trace-tui/Cargo.toml` reported no pending snapshots after direct review of all changed and added snapshots.
- `just test -p codex-tui --lib trace_visual_renderer` passed three focused parent-renderer tests.
- `just test -p codex-cli --bin codex trace_command` passed two CLI parsing and conflict tests.
- `just test -p codex-trace-tui profile_hundred_thousand_node_navigation --run-ignored ignored-only --no-capture` passed with 100,000 nodes, 235.905 ms construction, 36.973 ms first entry, navigation p95 below timer resolution, 38.733 ms warm entry p95, and 0.168 ms return p95; all interactive latency gates remained below 50 ms.
- Interactive PTY validation used `RUST_LOG=trace just codex -c 'log_dir="/tmp/ariadne-trace-ui-logs.wbjjyu"' trace --bundle /tmp/ariadne-trace-ui-demo.nG04o9/bundle`; direct-child descent, full-screen detail, asynchronous preparation, Rendered/Text/Raw cycling, help, return, terminal restoration, and quit behaved as designed.
- SHA-256 manifests before and after the interactive run were identical, and the isolated trace log contained no panic or error.
- `just bazel-lock-update` completed successfully with Bazelisk and produced no additional `MODULE.bazel.lock` delta; the existing dependency-version and crate-annotation advisories remain outside this plan.
- Scoped `just fix` passes completed without warnings for `codex-trace`, `codex-trace-tui`, `codex-tui`, and `codex-cli`, followed by the required `just fmt`.
- The workspace-wide argument-comment lint cannot currently enumerate targets because `windows-sandbox-rs/BUILD.bazel` passes the unsupported `binary_test_target_compatible_with` keyword; the package-scoped packaged-linter fallback then exhausted disk while installing its pinned nightly toolchain and rolled back. Changed call sites were manually written with the repository's exact argument-comment convention, but an authoritative Dylint result remains an external closure item.
- Cross-platform results remain pending because no push or external CI run is authorized; the dedicated workflow now covers Linux, macOS, Windows, the parent adapter, CLI integration, deterministic demo replay, and the Linux-only 100,000-node profile.

## Closure

Implemented: typed record presentation; a single-panel, single-depth browser; locator-based descent and return; responsive metadata columns and previews; class-based visibility filters; visible and all-record search; generation-tagged background search and detail work; full-screen Rendered, Text, and Raw modes; a parent-TUI renderer adapter; reviewed snapshots; and an invoked deterministic 100,000-node profile.

Validated locally: the recorded focused tests, snapshot review, parent-renderer tests, CLI parser tests, large-trace profile, interactive PTY exercise, byte-for-byte fixture comparison, scoped fixes, formatting, and Bazel lock regeneration passed as described above.

Deferred: authoritative argument-comment linting remains blocked by the recorded Bazel target-enumeration failure; no external Linux, macOS, or Windows workflow has run on the candidate commit; and an automated offline/read-only PTY test is still required. Source admission, evidence preservation, bounded replay, picker scaling, lifecycle depth, coverage-ledger, documentation, compatibility, and focused-CI obligations are transferred to the open [trace-browser quality remediation plan](../open/2026-07-28-trace-browser-quality-remediation.md).

The future structure-derived conversation fixture remains governed by the open [fixture plan](../open/2026-07-29-structure-derived-conversation-fixture.md). Deferred Vim-like navigation, configuration, live `/trace`, and investigation features remain outside this completed plan.

No push, tag, release, repository-setting change, persisted configuration, external publication, or live `/trace` integration is authorized.
