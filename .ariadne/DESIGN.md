# Ariadne architecture

## Purpose

Ariadne explains a Codex execution from persisted local evidence. An operator should be able to select a historical root session, follow delegation in both directions, inspect semantic and raw events, search the selected agent tree, and understand whether each displayed fact is exact, semantic, reconstructed, conflicting, or unavailable.

The viewer is a local debugger, not a transcript editor, rollout repair utility, retention system, remote observability service, or new trace recorder.

## Product boundary

Browsing is offline and read-only. It makes no model calls, initializes no authentication or telemetry, requires no app-server connection, and does not edit or intentionally touch the metadata of trace inputs.

Ordinary rollouts remain the broadly available source. Opt-in rollout-trace bundles provide richer model and runtime boundaries and can contain substantially more sensitive material. The viewer consumes both sources but never enables recording.

## Layering

```text
ordinary rollouts ── codex-rollout ───────────┐
                                               ├─ codex-trace ─ SessionTrace ─┬─ TraceIndex ─────────┐
rich bundles ─────── codex-rollout-trace ─────┘                              └─ PresentationIndex ──┼─ codex-trace-tui ─ codex trace
future live facts ───────────────────────────────────────────────────────────────────────▲         │
contained lazy payload reads ─────────────────────────────────────────────────────────────────────┘
```

[`codex-rs/trace/`](../codex-rs/trace/) owns discovery, source projection, reconciliation, provenance, capabilities, stable locators, diagnostics, bounds, semantic search, safe raw-payload access, and the future renderer-neutral order, correlation, and presentation snapshots.

[`codex-rs/trace-tui/`](../codex-rs/trace-tui/) owns Ratatui application state, background jobs, navigation, layout, and rendering. It consumes the trace model rather than reconstructing source semantics.

The CLI owns argument parsing and launch. A future live `/trace` view reuses both crates and feeds typed live presentation facts into the same reduction instead of creating a second inspection model.

## Distribution and public demonstration

Source builds produce the upstream-compatible `codex` executable.
Ariadne's release instructions copy that artifact under the side-by-side name `codex-ariadne`; fork identity is a distribution boundary and does not require a second Cargo binary target or a change to upstream's internal help spelling.

The public demonstration is a deterministic generator for an existing rollout-trace bundle plus a launcher for `codex trace --bundle`.
It is not a fixture captured from a real session, a new persisted format, a demo-only reducer path, or a live tracing mode.
The authoritative Rust reducer remains the compatibility oracle for generated bundle content.

## Sources and evidence

Ordinary session JSONL supports transcript and lifecycle navigation but is not assumed to contain an exact request context for every generation. Collaboration content may be encrypted or otherwise unavailable.

Rollout-trace bundles provide an ordered event spine, referenced payloads, reduced model-visible conversation, runtime objects, and interaction edges. `codex-rollout-trace` remains the authoritative rich schema and reducer; Ariadne does not introduce a third persisted format.

A merged view is additive. Source-local identities and records remain inspectable, stable recorded identity is required for deduplication, exact raw evidence may enrich display details without erasing its semantic counterpart, and contradictions become diagnostics rather than silent precedence choices.

Every inspectable value has provenance and one of these evidence grades:

- `exact`: directly supported by a captured raw payload;
- `semantic`: represented by a persisted or reduced semantic record;
- `reconstructed`: derived deterministically from surrounding records;
- `unavailable`: expected evidence was not recorded or cannot be read;
- `conflicting`: source observations disagree and remain inspectable.

## Navigable model

The catalog lists root sessions using bounded metadata without reducing every rich bundle. Selecting a root loads its descendant graph and summary indexes.

The normalized tree contains sessions, agent threads, turns, inference calls, conversation items, agent communication, tool and terminal activity, compactions, interaction edges, raw payload references, and diagnostics.

Thread nesting expresses ownership; causal edges express information flow. These relationships must not be collapsed into one another. Stable viewer locators combine source identity with source-local object identity and remain stable across filtering and sorting within one source version.

## Presentation projection

The accepted [presentation-index contract](decisions/2026-07-31-trace-presentation-index-contract.md) adds a derived snapshot beside the structural index. `SessionTrace` remains the canonical normalized evidence, `TraceIndex` remains the containment and lookup index, and `PresentationIndex` owns renderer-neutral grouping, order bands, summaries, completeness, default visibility, and reversible links back to canonical locators.

Every event admitted to grouped or expanded views has exactly one primary group. Secondary references may connect compactions, inference requests, terminal sessions, interactions, diagnostics, and raw artifacts without moving or duplicating their canonical evidence. Sessions, threads, turns, and inference containers remain available through structural navigation even when conversation views omit them.

Order is source-typed and thread-local unless rich global sequence evidence establishes a cross-thread order. Ordinary ordinals, rich event sequences, structural positions, wall-clock timestamps, and stable tie-breakers remain distinct domains. Wall-clock time is display metadata, and deterministic linearization of incomparable source bands is labeled non-causal.

Groups use durable correlation identities. Missing identifiers produce singleton or partial groups, never timestamp- or adjacency-based guesses. Higher-order groups such as exploration batches contain child group IDs rather than copying their nodes. Group summaries are bounded derived facts and never replace the evidence that supports them.

Canonical evidence grade, group completeness, and live origin remain orthogonal. Batch and incremental construction must converge for completed facts; live-only state is labeled and may not be reconstructed from a persisted trace that lacks it.

The presentation snapshot is bounded linearly by retained canonical nodes and admitted references, is built outside rendering, and contains no Ratatui, terminal-width, `HistoryCell`, app-server, authentication, or model-context types.

## Loading and responsiveness

Discovery, reduction, retained nodes, search fields, search hits, and payload display all require explicit bounds. Reaching a bound produces a visible diagnostic rather than an implication of completeness.

The picker reads metadata only. Root loading and payload reads run outside the terminal event loop and are cancellable where their upstream operations permit it. Raw payload bytes are read only when selected.

The target design does not format, wrap, clone, or scan an entire selected trace on every frame. Render paths consume precomputed summaries, visible windows, and cached detail representations. Work that grows with total trace size belongs in background loading or explicit search, with progress and cancellation.

The current reducer can still perform synchronous rich replay once started. Ordinary and rich inspection readers enforce record bounds while consuming each line, so the TUI may rely on bounded retained record memory but not cooperative cancellation within a reducer call.

## Privacy and containment

Trace content is untrusted terminal input. Prompts, responses, commands, tool arguments, outputs, paths, and payloads begin collapsed where appropriate and are rendered as inert text with control sequences sanitized.

Payload references are resolved relative to a canonical bundle root. Candidates that escape through path components or symlinks, are not regular files, or exceed configured bounds are rejected as navigable diagnostics without hiding unaffected data.

Only deterministic synthetic fixtures belong in the repository. Synthetic means generated independently for the scenario, not a redacted or transformed real trace.

## Failure behavior

Unknown versions remain inspectable where safe and produce compatibility diagnostics. Malformed records are isolated at the smallest practical unit. Orphans remain reachable, cycles are cut at the repeated locator, missing payloads do not hide their parent nodes, and a failed background load leaves the previous stable view intact.

Unavailable and malformed evidence must be represented honestly. The viewer does not invent intent, causal links, plaintext, or source agreement to fill an observability gap.

## Upstreamability

Ariadne isolates fork-owned behavior in dedicated crates and depends on the smallest practical upstream surfaces. A recorder, persisted-schema, core-runtime, or app-server change requires a failing synthetic observability scenario and a separately reviewable decision.

An upstream candidate contains generalized model or UI changes only. Fork governance, branding, branch machinery, and project history remain in Ariadne.
