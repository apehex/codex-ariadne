# Ariadne trace browser design

## Goals

Ariadne explains a Codex execution from persisted local evidence. An operator
must be able to:

1. select a historical root session;
2. follow delegation from parent to child and back;
3. inspect turns, model generations, messages, tools, compactions, and results;
4. correlate a semantic item with its raw lifecycle records;
5. know whether displayed evidence is exact, reconstructed, conflicting, or
   unavailable;
6. search the whole selected agent tree without loading every raw payload.

The viewer is not a transcript editor, rollout repair utility, retention tool,
or remote observability service.

## Decision: fork-first, dedicated historical TUI

The implementation lives in the Codex workspace instead of beginning as an
external app-server client. This provides direct access to rollout readers,
rollout-trace reduction, protocol types, and existing Ratatui conventions while
making missing runtime evidence observable and upstream-discussable.

The initial surface is a dedicated `codex trace` application rather than a
`/trace` overlay:

| Option | Historical fit 30% | Upstream reviewability 25% | Reuse 20% | Future live use 15% | Cost 10% | Weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `codex trace` first | 5 | 5 | 3 | 3 | 4 | 84% |
| `/trace` first | 3 | 2 | 5 | 5 | 2 | 67% |
| both in V1 | 5 | 2 | 4 | 5 | 1 | 73% |

A dedicated historical browser has a clean offline/read-only boundary and can
evolve without increasing the complexity of the primary chat loop. Once its
model is stable, `/trace` can embed it and add live app-server updates.

## Layering

```text
ordinary rollouts ── codex-rollout ───────────┐
                                               ├─ trace catalog/model ─ trace TUI
rich bundles ─────── codex-rollout-trace ─────┘
                             │
                             └─ lazy raw payload reader
```

The implementation should use two crates:

- `codex-trace`: source discovery, identifiers, graph projection, source merge,
  capabilities, diagnostics, stable locators, and search;
- `codex-trace-tui`: Ratatui application state, navigation, layout, and
  rendering.

The CLI owns only argument parsing and launch. The later `/trace` integration
reuses `codex-trace` and embeds `codex-trace-tui` components instead of
constructing another model.

## Sources and reconciliation

### Ordinary source

Use `codex-rollout` and protocol types to discover and read normal rollouts.
Build thread relationships from persisted session source metadata and
collaboration records. Filesystem discovery is the default; do not take a
writable SQLite connection. If profiling later demonstrates that the state
database is needed, expose a query API that opens it read-only.

Ordinary rollouts are sufficient for transcript navigation and many lifecycle
items. They are not presumed to contain an exact model request for every
generation. Encrypted content remains unavailable unless an existing authorized
runtime surface can reveal it.

### Rich source

Use `codex-rollout-trace` as the authoritative rich schema and reducer. Its
manifest, ordered event spine, payloads, reduced model-visible conversation,
runtime objects, and interaction edges are not duplicated in the viewer.

Match a rich bundle to an ordinary rollout using the manifest's `rollout_id`
and `root_thread_id`. A rich bundle may also be opened without a matching
ordinary rollout.

### Merge rules

Merging is additive:

- Preserve source-local IDs and attach stable viewer locators.
- Deduplicate only when stable recorded identity proves equivalence.
- Prefer exact raw evidence for display details, while retaining the semantic
  record that points to it.
- If sources disagree, create a diagnostic and expose both values.
- If a capability is absent, report it; do not synthesize plausible content.

Every inspectable value carries its provenance and evidence grade. The UI may
hide repeated labels visually, but the inspector must make them available.

## Navigable model

The catalog lists root sessions without eagerly reducing every session.
Selecting a root loads its descendant graph and summary indexes. The browser
exposes these node kinds:

- session and agent thread;
- turn and inference call;
- conversation item and agent communication;
- tool call, code cell, and terminal operation;
- compaction and interaction edge;
- raw payload and diagnostic.

Nodes use stable locators composed from source identity and source-local object
identity. Locators must survive filtering and sorting within one source version;
they are not promised to survive an upstream trace-schema migration.

Children are ordered by recorded sequence where available and timestamp plus
stable identity otherwise. Parent-child thread containment and causal
interaction edges are separate relationships: nesting answers ownership;
edges answer information flow.

## Loading and search

The picker reads bounded metadata only. Opening a root schedules discovery and
reduction in a background worker. Async discovery and payload reads can be
cancelled; the current synchronous rich reducer does not yet support
cooperative mid-replay cancellation. Discovery, event count and size, retained
nodes, and search results have explicit defaults and report limit exhaustion as
diagnostics. Raw payload contents are loaded only when selected.

Search operates on the selected root and descendants when the operator submits
the query. It walks bounded semantic string fields without formatting every
complete JSON value into a duplicate string. Results retain the node locator,
field, source, evidence, match range, and match-centered snippet. Explicit
raw-payload search is a successor feature; opening raw payloads remains lazy.

Large or repeatedly compacted rollouts are capped rather than materialized into
unbounded retained nodes or duplicate search strings. The ordinary reader and
rich reducer still read one complete JSONL line before enforcing the per-record
size limit. A chunk-bounded line reader and cooperative rich-replay
cancellation remain hardening work. Numeric latency budgets will be recorded
after representative synthetic benchmarks.

## UI structure

The root picker shows timestamp, working directory, model, status, archive
state, descendant count, and source badge (`ordinary`, `rich`, or `merged`).

The browser has three conceptual panes:

1. breadcrumb and thread hierarchy;
2. ordered children of the selected node;
3. semantic and raw inspector.

Wide terminals show all panes. Medium terminals show hierarchy plus the active
content pane. Narrow terminals show one pane and use the same enter/back model.
No information is available only through color.

Raw prompts, tool arguments, outputs, and terminal content begin collapsed.
The inspector sanitizes ANSI escapes and terminal control characters and never
executes link, escape, or shell content.

## Read-only and privacy boundary

All trace inputs are immutable. Tests must verify byte-for-byte identity before
and after an end-to-end browsing session. The viewer does not update access
times intentionally, but the read-only guarantee concerns content and Codex
state, not filesystem behavior imposed by the host.

Raw payload references are resolved relative to the bundle root. The reader
canonicalizes both root and candidate, rejects escape through `..` or symlinks,
and accepts only regular files. Missing or rejected payloads become diagnostics.

The viewer does not initialize Codex authentication, model clients, telemetry,
or app-server. Rich tracing remains opt-in through
`CODEX_ROLLOUT_TRACE_ROOT`; the viewer itself never enables recording.

## Missing evidence and recorder changes

The first implementation must run a synthetic Multi-agent V2 sentinel through
rich tracing and verify whether exact initial tasks and follow-up messages are
present at the authorized diagnostic boundary.

If plaintext is absent, the smallest permitted recorder extension is an
opt-in-only event after decryption and before dispatch, written through the
existing shared trace context. It must be inactive unless
`CODEX_ROLLOUT_TRACE_ROOT` is configured. Normal rollouts remain encrypted.

No recorder or app-server protocol extension should be introduced for
convenience. Each new event requires a failing synthetic observability test that
cannot be satisfied by the existing ordinary or rich sources.

## Failure behavior

- Unknown record versions remain raw and produce a compatibility diagnostic.
- Malformed records are isolated by record or payload when possible.
- Missing children become orphan nodes; cycles are cut at the repeated locator
  and reported.
- Missing, unreadable, or escaping payload references do not prevent browsing
  other nodes.
- Cancellation returns to the previous stable view without a partial model
  replacing it. A rich replay already executing synchronously may finish in the
  background before its aborted task observes cancellation.
- Schema versions newer than the reader display a warning and preserve all
  safely readable evidence.

## Compatibility and upstreamability

Workspace-private APIs may change with upstream, so changes should remain in
dedicated crates and depend on the smallest existing public surfaces. Avoid
fork-wide modifications and keep commits reviewable by milestone.

An upstream candidate should contain the generalized trace model and UI only.
A core recorder change is proposed separately, with its evidence gap and
privacy boundary documented. Ariadne-specific history, branding, and branch
machinery do not belong in an upstream candidate.
