# Credible Historical Trace Browser

Updated: 2026-07-31

Status: closed; superseded by the quality-remediation and phased-presentation plans

## Objective

Turn the existing offline `codex trace` prototype into a responsive, tested, documented historical trace browser that is credible as a public Codex extension and as feasibility evidence for an OpenAI Researcher Access application.

An operator must be able to browse ordinary and delegated synthetic traces containing up to 100,000 nodes without the interface freezing on routine navigation or eagerly formatting complete large payloads.

## Context

The fork already adds trace discovery, normalization, delegation reconstruction, bundle support, a read-only Ratatui browser, and CLI integration. The current hot paths repeatedly scan all nodes to resolve roots and children, rebuild visible rows, pretty-print complete structured details, and wrap complete payloads during rendering. These costs explain the observed pauses on large sessions and files.

The architecture and public-data boundary are defined by `.ariadne/DESIGN.md`, `.ariadne/decisions/2026-07-25-fork-first-historical-trace-browser.md`, `codex-rs/trace/DESIGN.md`, and `codex-rs/trace-tui/DESIGN.md`. Preserve the historical, offline, read-only scope and the `codex trace` interface.

This plan is linked to `/home/avrana/workspace/get-bb/bugger/plans/open/2026-07-28-openai-research-program-applications.md`. This repository owns the public prototype and its evidence; Bugger owns application copy and claim review.

## Scope

Owned implementation paths:

- `codex-rs/trace/`;
- `codex-rs/trace-tui/`;
- the existing `codex trace` CLI integration where required;
- focused Ariadne documentation, synthetic demonstration material, and CI under `.ariadne/`, `docs/`, and `.github/workflows/`.

Preserve existing public `SessionTrace` fields and method signatures where practical. Prefer additive model-owned indexes and browser caches over schema changes.

Explicitly deferred:

- live `/trace` inspection inside an active conversation;
- recording or app-server changes;
- a generalized timeline UI;
- speculative diagnostics or policy judgments;
- a new trace schema or demo-only browser mode;
- real, redacted, or private session fixtures;
- external release creation, pushes, repository settings, or default-branch changes.

## Movements

### Movement 1: Establish Performance Evidence

- Add deterministic synthetic builders for 10,000-node and 100,000-node traces.
- Measure initial model/index construction, first render preparation, warm navigation, expansion, search, and large-detail opening on a recorded reference host.
- Preserve benchmark commands and results without committing huge generated traces.

### Movement 2: Index The Model

- Add a model-owned trace index for locator lookup, root order, and parent-to-child adjacency.
- Build it once per loaded session and use it throughout the browser.
- Retain stable event ordering and delegation semantics.
- Add structural tests that make accidental return to whole-model child scans observable.

### Movement 3: Bound Rendering Work

- Cache flattened visible rows and rebuild them only when expansion or model state changes.
- Render only the active viewport for tree, child, search, and diagnostics lists.
- Cache inspector summaries by selected locator, width, mode, and payload state.
- Bound structured-detail formatting and disclose truncation.
- Index payload line boundaries and wrap only the requested visible window rather than the complete file on every frame.
- Move any remaining expensive, interruptible operation off the frame path or make it explicitly cancellable.

### Movement 4: Compatibility And Debugging

Cover ordinary rollouts, rich indexed traces, merged parent-child bundles, nested delegation, compaction, failed and interrupted tasks, missing child traces, malformed records, Unicode, terminal control characters, and path edge cases. Assertions should verify semantic structure and metadata, with snapshots used only for stable presentation surfaces.

### Movement 5: Public Prototype

- Add a deterministic synthetic demo generator and launcher that exercise the normal `codex trace --bundle` path.
- Document side-by-side installation as `codex-ariadne` so it does not compete with a global upstream `codex`.
- Add a clear fork banner and architecture/limitations index on the Ariadne branch while keeping upstream-owned files minimally changed.
- Add focused Linux, macOS, and Windows CI for trace crates and CLI integration.
- Prepare release-candidate notes and reproducible build instructions without creating a tag or external release.

## Acceptance Evidence

- A deterministic 100,000-node synthetic trace is navigable without unbounded per-frame scans or complete-payload wrapping.
- On the recorded reference host, warm navigation and expansion actions have p95 latency below 50 ms.
- Opening a large detail or payload does not serialize and wrap the complete content on every frame; tests or instrumentation demonstrate cache/window behavior.
- Locator, root, and children lookups use the built trace index.
- The compatibility matrix passes for ordinary, rich, merged, nested, compacted, failed, malformed, Unicode/control-character, and path-edge fixtures.
- Existing trace, trace-TUI, CLI, and rollout-trace test suites remain green.
- Focused Linux, macOS, and Windows CI configuration is present and validates the supported commands.
- Public documentation explains scope, trust boundaries, installation beside upstream Codex, demo use, performance limits, and known omissions.
- Only synthetic trace fixtures and aggregate benchmark results are committed.
- Release-candidate materials exist, but no agent has pushed, tagged, published, or changed repository settings.

## Closure

Close the plan after recording the implementation diff, exact validation commands, benchmark host and results, compatibility outcomes, known residual gaps, and release-candidate state. If the performance or compatibility gate is not met by 2026-08-24, record the blocker so the linked Researcher Access application can defer from the September to the December review rather than overstate readiness.

## Implementation progress

- Added an additive `TraceIndex` API in `codex-rs/trace` with constant-time locator and node lookup, root and child traversal, compact node-position accessors, explicit rebuild semantics after structural mutation, and focused model tests.
- Refactored `codex-rs/trace-tui` to construct the parsed model, index, and browser state in a background worker; cache compact ordered roots, children, visible rows, selected children, inspector content, and search state; splice only affected subtrees on expansion and collapse; and render only the visible viewport.
- Bounded structured inspector preparation to 64 KiB with an explicit truncation notice and changed raw payload inspection to wrap only the requested viewport window. Terminal control sanitization remains in the display path.
- Added deterministic 100,000-node performance coverage. On Linux 7.1.4-arch1-1 x86_64 with a four-core AMD EPYC 9354P and the repository Rust 1.95.0 toolchain, the debug-profile run measured 390.572 ms background construction, 35.845 ms first expansion, 0.001 ms navigation p95 across 1,000 movements, 39.170 ms expansion p95 across 20 expansions, and 0.494 ms collapse p95 across 20 collapses.
- Added a deterministic synthetic parent/child-agent demo generator and verifier under `.ariadne/demo/`, release-candidate notes and a compatibility matrix under `.ariadne/releases/`, updated fork and module documentation, and added `.github/workflows/ariadne-trace.yml` for Linux, macOS, and Windows validation.
- Preserved the existing `codex trace` command shape and the public trace-model fields and traversal methods. Existing terminal snapshots remain unchanged.

## Validation record

- `just fix -p codex-trace`
- `just fix -p codex-trace-tui`
- `just fmt`
- `cargo test -p codex-trace -p codex-trace-tui`: 14 trace tests passed; 6 TUI tests passed; the deterministic performance profile remains ignored for ordinary test runs and passed when invoked explicitly.
- `cargo test -p codex-rollout-trace`: 70 tests passed.
- `cargo test --locked -p codex-cli --bin codex`: 230 tests passed.
- The synthetic demo generated and verified a two-thread, three-turn trace with one tool call, a `spawn_agent` edge, and an `agent_result` edge. Its deterministic fingerprint is `b84b9337e8e708abaecc2878d3ce6640bbcec3ae4a07fb4f11ad804ef8253a37`.
- The demo was replayed through the production reducer and opened in the real terminal UI.

## Current state and residual gates

The local engineering acceptance criteria are satisfied. The plan remains open because the candidate is not yet committed or pushed, the configured Linux/macOS/Windows workflow has not run on the candidate commit, and no public tag or release artifact exists. Repository About metadata and default-branch decisions also remain research-operator actions. No push, tag, release, GitHub setting change, or external publication was performed.

## Successor UI plan

The [closed single-panel plan](2026-07-28-single-panel-trace-browser.md) supersedes this plan's three-pane, flattened-tree, fold/expand, and expansion/collapse performance requirements. The implementation receipt above remains historical evidence; correctness, privacy, offline/read-only, compatibility, CI, and public-prototype gates remain active.

## Supersession Record

This plan closed on 2026-07-30 as a historical implementation receipt rather than as a claim that every original publication outcome occurred.

The [open quality-remediation plan](../open/2026-07-28-trace-browser-quality-remediation.md) owns the remaining correctness, resource-bound, lifecycle, documentation, compatibility, test-depth, CI, and public-prototype gates for the current baseline.

The phased presentation program beginning with the [closed architecture contract](2026-07-30-trace-presentation-architecture-contract.md) owns the future order, correlation, grouping, lens, surface, transcript-parity, and parent-integration work. Its [parent-compatible candidate phase](../open/2026-07-30-parent-compatible-transcript-candidate.md) treats local compatibility evidence as its closure condition; upstream adoption remains an external outcome.

The implementation and validation evidence above remains valid historical evidence where it describes the code at that time. Obsolete pane, fold, expansion, and publication requirements are not carried forward.

No push, tag, release, repository-setting change, or external publication is claimed by this closure.
