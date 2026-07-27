# Ariadne fork and historical trace browser

Updated: 2026-07-25

Status: closed; milestones 1–4 completed

## Objective

Create a public Codex fork named Ariadne whose local, read-only `codex trace` interface lets an operator select a root session, follow nested agent relationships, navigate turns and events, search the selected tree, and inspect semantic or raw records while distinguishing exact evidence from reconstructed, conflicting, or unavailable data.

The first accepted release is complete when ordinary Codex rollouts and opt-in `codex-rollout-trace` bundles can be browsed through a historical tree, inspector, and search TUI without authentication, model calls, network access, or mutation of trace storage.

## Context

Codex already persisted ordinary session JSONL and contained a richer opt-in `codex-rollout-trace` writer and reducer. Its main TUI could resume transcripts and switch among loaded agents, but it did not provide an investigation-oriented historical tree, source-capability labeling, or raw-record provenance view.

This made delegation failures difficult to diagnose because an operator could not reliably separate parent instructions, later task changes, child behavior, interruption, and missing observability. Ordinary history could be lossy or contain encrypted collaboration content, while rich bundles could preserve exact inference and runtime boundaries.

The accepted architectural decision was to implement the browser inside a Codex fork, use dedicated model and Ratatui crates, ship `codex trace` before a live `/trace` overlay, reuse existing ordinary and rich persisted formats, and keep capture changes conditional on a demonstrated observability gap. See the [fork-first decision](../../decisions/2026-07-25-fork-first-historical-trace-browser.md).

## Scope

The completed scope included:

- fork identity, upstream synchronization guidance, public-data policy, architecture, local instructions, project history, and an upstream issue draft;
- reducer hardening and synthetic Multi-agent V2 observability checks;
- bounded ordinary-session and rich-bundle discovery;
- ordinary, rich, and merged provenance-aware projection;
- stable locators, graph diagnostics, capabilities, evidence grades, attributed semantic search, and contained lazy payload reads;
- `codex trace`, direct session selection, explicit bundle and trace-root selection, a searchable picker, adaptive browser, inspector, and deterministic snapshots;
- offline and byte-for-byte read-only validation using synthetic data.

The V1 scope excluded synchronized timelines, workflow diagnostics, live following, `/trace`, export, annotations, remote traces, trace mutation, archive or delete operations, and a user-facing capture toggle.

## Constraints

- Preserve the Apache-2.0 license and applicable notices.
- Keep browsing local, read-only, offline, and independent of Codex authentication and model configuration.
- Do not imply exact generation context or decrypted inter-agent instructions when an ordinary source does not contain them.
- Keep rich capture explicitly opt-in through `CODEX_ROLLOUT_TRACE_ROOT`.
- Reuse `codex-rollout` and `codex-rollout-trace`; introduce no third persisted schema.
- Preserve source contradictions and label missing evidence explicitly.
- Commit only deterministic synthetic fixtures, never real or redacted traces.
- Keep Ariadne changes localized and compatible with eventual upstream extraction.

## Completed movements

### 1. Fork and documentation bootstrap

Ariadne established an upstream-mirror `main`, rebased `ariadne` integration branch, public fork identity, architecture, privacy boundary, synthetic-fixture policy, working instructions, project history, and gated upstream issue draft.

### 2. Rich trace hardening

The rollout-trace reducer was fixed to truncate non-ASCII JSON summaries at a valid UTF-8 boundary. Rich discovery gained manifest-only metadata. Deterministic synthetic bundles covered inference, tools, compaction, terminals, code cells, and nested delegation.

Synthetic tracing demonstrated that exact spawn and follow-up text is retained in opt-in raw tool or runtime payloads even when child-side content is opaque, so no new recorder event was required.

### 3. Catalog and trace model

`codex-trace` implemented bounded root discovery and direct bundle selection, projected ordinary and rich sources into a common source-namespaced graph, merged matching roots additively, exposed capabilities and evidence grades, preserved broken and cyclic parent links as diagnostics, and provided attributed semantic search and contained lazy payload access.

Discovery, rich replay, ordinary record retention, projected nodes, search fields, search hits, and payload display gained explicit limits. Reaching a limit leaves a visible diagnostic instead of silently implying completeness.

### 4. Historical TUI

`codex trace`, direct session selection, `--bundle`, `--trace-root`, and `CODEX_ROLLOUT_TRACE_ROOT` were wired to an offline read-only Ratatui application. The TUI implemented a searchable root picker, adaptive tree, children and inspector layouts, breadcrumbs, provenance and capability labels, selected-tree search, diagnostics, and collapsed raw payloads.

Search runs on submission and reports the matching field, byte range, evidence grade, provenance, and a centered snippet. Malformed rich event spines retain usable nodes but downgrade unsupported semantics rather than claiming exactness.

## Acceptance evidence

- `just test -p codex-rollout-trace`: 70 passed.
- `just test -p codex-trace`: 11 passed.
- `just test -p codex-trace-tui`: 6 passed with nine deterministic snapshots covering narrow, medium, and wide layouts.
- `just test -p codex-cli`: 318 passed, including trace-subcommand parsing and conflict tests.
- Package-scoped `just fix` passed for the reducer, trace model, trace TUI, and CLI; the final targeted Clippy check was clean.
- Locked Cargo metadata and package-scoped Rust formatting checks passed.
- Bazel 9.0.0 lock regeneration produced no changes, and the new trace and TUI target query resolved.
- The optional cold-cache Bazel test was stopped after analysis because it expanded into a 13,737-action V8 and libc++ toolchain build; no test action had run.
- The repository-wide formatting wrapper could not run because `dotslash` was absent from the validation host. Rust formatting and both new BUILD files were checked with their underlying formatters.
- End-to-end TUI tests compared every synthetic ordinary rollout, rich manifest, event spine, and payload byte before and after discovery, loading, rendering, navigation, search, and lazy raw-payload inspection.

## Closure

Implemented: fork/bootstrap documentation; hardened rich reduction and manifest discovery; ordinary, rich, and merged provenance-aware catalog; bounded selected-session materialization; stable locators; resilient diagnostics; contained lazy payload reads; attributed semantic search; adaptive `codex trace` picker, browser, and inspector; CLI integration; deterministic narrow, medium, and wide snapshots; and byte-for-byte read-only synthetic tests.

Rejected: no new recorder event was added because opt-in raw runtime payloads already retained exact spawn and follow-up arguments.

Known limitations: ordinary rollouts do not provide exact per-generation context and may contain encrypted collaboration data; opaque child ciphertext cannot always be correlated to plaintext follow-up text; rich replay is not cooperatively cancellable after synchronous reduction begins; one complete JSONL line may be allocated before its size is rejected; picker metadata is manifest-limited; content-level conflict detection is incomplete; search excludes unopened raw payloads; and causal edges are inspectable nodes rather than direct jump links.

Validation scope: the recorded integration evidence is Linux-only. macOS and Windows build evidence remains a release prerequisite.

Residual and successor work moved to [`.ariadne/ROADMAP.md`](../../ROADMAP.md). That roadmap is non-executable; each successor requires its own accepted open plan.
