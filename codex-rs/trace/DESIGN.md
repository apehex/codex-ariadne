# Trace model design

## Responsibility

`codex-trace` converts persisted ordinary rollouts and rich rollout-trace bundles into one read-only inspection model. It owns source mechanics, semantic normalization, and renderer-neutral presentation reduction; consumers own terminal presentation and interaction.

The fork-wide evidence grades, product boundary, privacy rules, and crate layering are defined in [`.ariadne/DESIGN.md`](../../.ariadne/DESIGN.md).

## Discovery and selection

`TraceRepository` receives a Codex home, optional rich roots or exact bundle, and explicit `TraceLimits`. Discovery reads bounded ordinary-session metadata and rich manifests to produce root summaries without replaying every selected history.

An exact rich bundle can be opened independently. When sources share recorded root identity, the catalog presents a merged root and retains both source capabilities.

Filesystem discovery is the default. The model does not open a writable Codex state database, repair indexes, or update archive state.

## Normalized graph

`SessionTrace` stores source-namespaced `TraceNodeLocator` keys, normalized `TraceNode` values, root locators, capabilities, diagnostics, search bounds, and raw payload handles.

Node kinds cover sessions, agent threads, turns, inference calls, messages, agent communication, tools, code and terminal activity, compactions, interaction edges, raw payloads, and diagnostics.

Each node also retains a bounded presentation descriptor derived during projection: a primary record class, optional role/channel/status, and one-line preview. Consumers use this typed boundary for styling and filtering rather than rescanning arbitrary JSON during every frame.

`TraceNode::content_document` extracts bounded interpreted Markdown, text, JSON, or code while preserving decoded newlines. It never follows raw payload references; exact artifact access remains an explicit lazy operation through `RawPayloadHandle`.

Containment, lineage, and causality are distinct. For ordinary rollouts, upstream `SessionMeta.session_id` defines the catalog session, `SessionMeta.id` identifies one thread, and `parent_thread_id` is the only ordinary containment edge. `forked_from_id` and `history_base` remain inspectable lineage metadata and do not merge otherwise independent sessions. Rich containment follows `AgentOrigin`. Interaction edges answer how information moved. Broken or cross-session parents remain observable, affected threads stay reachable from their recorded session, and cycles are cut with a diagnostic.

Ordinary parent observations are normalized into one session-scoped topology before any thread is projected. Parent lookup, cross-session detection, cycle checks, and diagnostics therefore share one interpretation instead of repeatedly rescanning discovered files. Conflicting parent observations for the same thread are treated as ambiguous evidence and fall back to the recorded session rather than choosing whichever file was visited last.

Siblings use typed source positions: structural thread containers use recorded start time, ordinary records use numeric rollout ordinals, and rich semantic nodes use their first raw event sequence. Timestamped containers precede the causal event stream when both share a parent, rather than comparing incompatible timestamp and sequence units. Unpositioned diagnostics and raw artifacts follow positioned siblings in stable admission order. Display timestamps are never used to reorder records within a thread, so delayed or regressing wall-clock observations remain in causal order. Locators survive filtering and sorting within a source version but are not a persisted compatibility promise across upstream schema migrations.

Graph finalization retains those positions as `TraceNodeFacts` instead of discarding them after sibling sorting. The fact vocabulary keeps ordinary, rich, structural, and unspecified domains distinct; ordinary positions compare only for the same recorded thread, while rich positions use the bundle-wide raw-event sequence. Execution facts retain optional end sequence and start/end wall-clock values, plus a deterministic admission tie-break that does not manufacture causal order.

`TraceNodeFacts` also retains source ownership, an explicit complete/partial/unavailable/conflicting state, and typed correlations. Correlations cover source identities, parent and spawn edges, turn inputs, conversation producers, model-visible call IDs, runtime tool/MCP/code-mode IDs, inference inputs and outputs, code-cell membership, compaction membership, terminal ownership and model observations, interaction endpoints and carried items, and raw payload references. Ordinary sources expose only durable fields present in rollout records and remain partial rather than guessing rich runtime relationships.

## Projection and reconciliation

Ordinary projection uses `codex-rollout` and protocol records. It provides durable transcript and lifecycle semantics but does not claim exact generation context or decrypted collaboration content when those values were not persisted.

Rich projection uses the `codex-rollout-trace` manifest, ordered event spine, reducer, runtime objects, interaction edges, and payload references. Reducer output remains governed by that crate; `codex-trace` adapts it rather than copying the rich schema.

Rich node construction is routed through one projector and declarative node specifications. The projector owns parent validation, typed positions, presentation derivation, and graph admission; semantic branches own only extraction of their source values. Catalog discovery likewise accumulates ordinary and rich observations through one deterministic accumulator before producing public summaries.

Reconciliation is additive:

- source-local IDs remain present;
- stable recorded identity is required for deduplication;
- exact values may enrich semantic presentation without deleting the semantic record;
- disagreements retain both observations and add a conflict diagnostic;
- absent capabilities remain unavailable rather than guessed.

## Bounds and performance

`TraceLimits` bounds discovery, ordinary record bytes, rich event count and bytes, semantic payload bytes, and projected nodes. Search independently caps hits, fields per node, and characters per field. A reached source or node limit produces a diagnostic that explains the incomplete view.

Ordinary plain and compressed records and rich event lines are retained only up to their byte limit while the rest of an oversized line is discarded. Oversized or invalid UTF-8 records do not suppress a later valid record.

Node admission is centralized in a graph builder. It applies the cap before mutation, preserves repeated identities with stable observation suffixes, remaps children to the latest retained parent observation, and reports incompatible observations without deleting either side.

The selected trace is retained as a public node vector. `TraceIndex` provides a separately owned snapshot over that vector for locator lookup, roots, and parent-to-child adjacency. `TraceFactIndex` is an additive companion for position-to-fact lookup, compatible-domain thread order, and exact reverse correlation lookup. Both build in linear time plus per-thread ordering, preserve the loaded node positions, and avoid arbitrary detail-JSON interpretation in consumers.

The indexes are deliberately not embedded in `SessionTrace`: its public nodes and their structural fields remain mutable for compatibility, so a hidden cache could silently become stale. Consumers build one snapshot of each required index per loaded session and must rebuild after inserting, removing, or reordering nodes or changing a locator or parent. Existing `SessionTrace` lookup methods retain their signatures and scan behavior for compatibility; routine browser navigation should use `TraceIndex`.

Each node retains at most 4,096 typed correlations. Graph admission additionally retains at most four correlations per configured node slot across one loaded session and emits one saturation diagnostic when that global budget truncates facts. Truncation marks affected complete facts partial; conflicting repeated observations mark both sides conflicting without deleting either observation.

Rich replay is selected-root lazy but the reducer is synchronous once invoked. Cooperative cancellation inside an already-running reducer remains a hardening gap.

## Presentation index boundary

The accepted [presentation-index contract](../../.ariadne/decisions/2026-07-31-trace-presentation-index-contract.md) defines a future renderer-neutral `PresentationIndex` as a second immutable snapshot over one loaded `SessionTrace`. It does not extend `TraceIndex` or change canonical structural parents.

The presentation layer in this crate will own typed source-order and correlation facts, primary group membership, secondary references, higher-order child groups, order bands, aggregate metadata, completeness, origin, visibility defaults, and bounded derived diagnostics. It will retain canonical node positions or locators rather than clone complete detail values or raw payloads.

Canonical order is a partial order. Ordinary ordinals and rich raw-event sequences retain their source meaning; stable correlations align equivalent observations; incomparable cross-source intervals remain explicitly unordered; and wall-clock timestamps never determine event position. Session-wide synchronization is available only when rich global sequence evidence supports it.

Every primary presentable event belongs to exactly one primary group. Structural containers and raw artifacts may remain structural-only or reference-only, and every retained node stays reachable through `TraceIndex`. Missing correlation yields a singleton or partial group rather than a proximity inference.

The default structural limits are derived from the retained node count `N`: at most `2N` groups, `3N` direct membership entries, `4N` secondary references with at most 4,096 per group, nesting depth 4, 4 KiB per free-form summary, and 16 MiB total summary text. Presentation diagnostics, previews, and structured-value navigation use the additional exact caps in the accepted decision.

Construction belongs in selected-session loading or explicit incremental live reduction and must remain `O(N + R)` in admitted nodes, memberships, and references. Rendering-facing queries must not scan the complete trace, parse arbitrary node detail, or open raw payloads.

The presentation types contain no Ratatui lines, styles, widths, wrapping, key bindings, overlays, parent `HistoryCell`s, or app-server clients. Batch and incremental reduction of the same completed typed facts must produce deeply equal snapshots except for explicitly transient live state.

## Search

Semantic search runs only when explicitly requested. It examines bounded semantic string fields and returns attributed hits containing the locator, field, provenance, evidence grade, match range, and centered snippet.

Search does not eagerly stringify every raw JSON value and does not currently inspect unopened raw payload bytes. Raw search, if added, must be separately bounded, cancellable, and explicit because it changes both cost and sensitivity.

## Payload access

Semantic JSON payloads needed by rich reduction are read eagerly only for the selected bundle and have a separate replay byte limit. `RawPayloadHandle` defers display bytes until the consumer requests them. Both paths canonicalize the bundle root and candidate and reject absolute paths, lexical or symlink escape, and non-regular files; the display reader also decodes lossily and sanitizes terminal control content.

Missing, rejected, unreadable, or oversized payloads are local failures. Their parent nodes and the rest of the trace remain navigable.

## Failure and compatibility

Malformed ordinary or rich records are isolated where the source reader permits it. Unknown versions retain safely readable information and emit diagnostics. A reducer failure prevents speculative downstream semantic replay but does not erase intact raw evidence.

New persisted or protocol fields are not owned here. A missing observation must first be represented as an unavailable capability and demonstrated through a synthetic test before a separate recorder or protocol proposal is considered.
