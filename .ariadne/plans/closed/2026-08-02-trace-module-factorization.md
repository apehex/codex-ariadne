# Trace Module Factorization

Updated: 2026-08-02

Status: closed

Closed: 2026-08-02

## Objective

Refactor the Ariadne-owned `codex-trace` crate into small, orthogonal modules and reusable typed building blocks while preserving its crate-root API, trace semantics, deterministic presentation index, resource bounds, and upstream compatibility.

## Context

This is Phase 3.5 between the completed grouping policies and Phase 4 browser work.

Phase 3 established a complete renderer-neutral presentation index, but several implementation files still combine classification, correlation, hierarchy, reference aggregation, summary construction, and validation responsibilities.

The ordinary and rich adapters also contain parallel topology and fact-support code without a clear internal package boundary.

The refactor must remain entirely within Ariadne-owned modules so that a future upstream merge has a narrow footprint.

## Scope

Owned implementation paths are `codex-rs/trace/`, its tests and design notes, this plan, and `.ariadne/ROADMAP.md`.

Parent Codex modules, trace-TUI behavior and snapshots, persisted rollout formats, protocols, dependencies, and presentation policy semantics are excluded.

## Movements

### Movement 1: Establish Private Module Trees

- Replace the flat presentation implementation with a private `presentation` module tree and retain explicit public re-exports at the crate root.
- Partition record classification, content projection, ordering, grouping, hierarchy, references, summaries, validation, and index queries by responsibility.
- Introduce private ordinary and rich adapter trees without creating a shared projector trait or modifying upstream-owned code.

### Movement 2: Introduce Shared Typed Build Services

- Add a `BuildContext` that owns trace access, missing-fact fallback, source scopes, and identity-position lookup for one presentation build.
- Add a reusable bounded diagnostic sink and bounded text helpers.
- Consolidate correlation indexes so grouping policies reuse one deterministic scan instead of rebuilding overlapping maps.

### Movement 3: Make Presentation Bounds Explicit

- Add a documented public `PresentationLimits` value with safe defaults and clamped effective bounds.
- Add limit-aware presentation-index constructors while preserving existing constructors as exact default delegates.
- Keep structural partition bounds hard and keep deterministic grouping policy outside configuration.

### Movement 4: Partition Repository And Model Responsibilities

- Split catalog discovery, accumulation, and loading behind the existing catalog facade.
- Split source, record, catalog, and session model families behind the existing model facade.
- Preserve public type names, serialization, equality, and crate-root import paths.

### Movement 5: Localize Tests And Documentation

- Partition oversized presentation and integration test files by behavior and keep shared fixtures in focused test support.
- Add deep-equality tests for default construction and boundary tests for zero, reduced, exact-default, and above-maximum presentation limits.
- Update module design documentation to describe ownership and dependency direction.

## Acceptance Evidence

- Existing crate-root imports compile without downstream changes and default presentation indexes remain deeply equal.
- Every production Rust module remains below the doctrine target where coherent, with no newly created mixed-responsibility module.
- Ordinary and rich adapters share only typed facts and focused fact-support helpers, not a forced common abstraction.
- Presentation builds scan trace facts and correlation identities through shared build services while retaining canonical ordering and grouping results.
- Zero and reduced limits degrade deterministically, maximum-or-larger values cannot exceed safety caps, and diagnostics always retain evidence that truncation occurred.
- Focused Cargo and Bazel tests, scoped Clippy, argument-comment lint, formatting, and diff checks pass.

## Closure

Close this plan after recording the final module map, stable public API, effective bounds, exact tests and commands, module sizes, and any intentionally deferred factorization.

## Implementation Record

Phase 3.5 completed on 2026-08-02 entirely within Ariadne-owned trace modules and planning documentation.

The flat presentation implementation is now a private `presentation` tree.

The facade retains the existing crate-root exports while focused modules own activity classification, record classification, interpreted content, bounded text, construction, context, diagnostics, index queries, limits, order, policy, references, summaries, validation, and the renderer-neutral model.

The private `grouping` subtree separates lifecycle correlation, reverse-owner indexing, code-cell hierarchy, and exploration batching.

`BuildContext` supplies immutable trace access, effective limits, unavailable-fact fallback, thread scopes, and source-identity positions to the construction pipeline.

`CorrelationIndex` computes and owns the reverse maps reused while assigning lifecycle groups.

The ordinary adapter now separates bounded JSONL reading, topology validation, and typed fact projection.

The rich adapter separates the projector, topology and interaction ownership, semantic facts, terminal facts, and shared fact constructors without introducing a common ordinary/rich trait.

Catalog filesystem traversal and source reconciliation live in focused child modules.

Loaded session nodes, navigation, search hits, and lazy payload handles moved out of the general model facade.

All production Rust modules in `codex-trace` are below 500 lines after formatting.

The largest affected modules are rich fact projection at 450 lines, lifecycle correlation at 442, the general fact vocabulary at 444, the presentation model at 397, the model facade at 394, and the catalog facade at 388.

## Public API And Bounds

Existing public type names and crate-root import paths remain intact.

`PresentationIndex::new` and `SessionTrace::presentation_index` preserve their signatures and delegate to `PresentationLimits::default()`.

The additive APIs are `PresentationLimits`, `PresentationIndex::new_with_limits`, and `SessionTrace::presentation_index_with_limits`.

The defaults are 4,096 references per group, four references per retained node globally, 4 KiB summary text per group, 16 MiB summary text per snapshot, 256 preview characters, 1,024 diagnostics, 4 KiB per diagnostic message, and group depth four.

Requests above those values clamp to the safety defaults.

Zero disables the corresponding retained data, except that one diagnostic slot remains so truncation is observable.

Depth below two omits presentation containers while retaining their primary groups.

The hard `2N` group and `3N` membership invariants and versioned grouping policy are not configurable.

Default construction is covered by whole-index deep equality.

Additional tests cover above-maximum clamping, all-zero limits, exact UTF-8-safe reduced summary budgets, retained primary membership, and observable truncation.

## Validation Record

`just test -p codex-trace` passed 74 tests.

`just test -p codex-trace-tui` passed 24 runnable tests with one existing skipped test and no snapshot changes.

`bazel test //codex-rs/trace:trace-unit-tests` passed.

`cargo clippy -p codex-trace --tests -- -D warnings` passed.

The prebuilt scoped argument-comment runner completed with the repository's pinned-toolchain unknown-lint warnings, and `bazel build --config=argument-comment-lint //codex-rs/trace:trace-unit-tests` passed.

Final cleanup completed with `just fix -p codex-trace`, `just fmt`, `git diff --check`, a production-module size audit, and confirmation that no pending trace-TUI snapshots exist.

## Deferred Factorization

The catalog facade remains the owner of public repository configuration and selected-session loading because extracting that cohesive flow would add forwarding without shortening its dependency path; its discovery and accumulator mechanics are already separate.

The 825-line pre-existing presentation integration test module remains intact because splitting its shared synthetic construction vocabulary would add test-only public surface or duplicate a large fixture DSL.

It is excluded from the production-module size doctrine and should be partitioned only when Phase 4 introduces a natural lens-oriented test-support boundary.

## Facts Module Follow-Up

The Phase 3.5 factorization was extended on 2026-08-02 to make the typed-fact normalization boundary consistent with the presentation and source-adapter trees.

The private `facts` facade now owns focused activity, correlation, order, aggregate-node, and immutable-index modules.

`TraceFactIndex` moved beneath that facade, while every existing crate-root import remains unchanged.

Stable same-domain and tie-break comparison is owned by the order module; causal source comparison retains its stricter cross-domain and cross-thread rejection.

Existing fact and index tests moved beside their owners, with focused coverage added for structural, incompatible-domain, and unspecified stable ordering.

The facade is 25 lines; the production child modules range from 94 to 161 lines.

Validation passed with 76 `codex-trace` tests, 24 runnable `codex-trace-tui` tests with one existing skip, the Bazel trace unit target, strict scoped Clippy, and the Bazel argument-comment configuration.

No trace-TUI snapshots changed.
