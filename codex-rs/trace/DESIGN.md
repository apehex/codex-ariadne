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

The private `facts` tree owns this normalization boundary. Focused modules define source order and ownership, activity policy facts, object correlations, aggregate node facts, and immutable indexed queries. Its facade explicitly re-exports the stable crate API. Deterministic index ordering shares the order module's same-domain comparison, while causal `TraceNodeFacts::source_cmp` continues to reject incompatible clocks and ordinary positions from different threads.

## Projection and reconciliation

Ordinary projection uses `codex-rollout` and protocol records. It provides durable transcript and lifecycle semantics but does not claim exact generation context or decrypted collaboration content when those values were not persisted.

Rich projection uses the `codex-rollout-trace` manifest, ordered event spine, reducer, runtime objects, interaction edges, and payload references. Reducer output remains governed by that crate; `codex-trace` adapts it rather than copying the rich schema.

Rich node construction is routed through one projector and declarative node specifications. The projector owns presentation derivation and graph admission; the private `rich` adapter tree owns fact projection, shared fact constructors, terminal facts, and parent topology as separate responsibilities. The matching `ordinary` adapter tree separates JSONL reading, topology validation, and fact projection. Neither adapter implements a forced common projector abstraction; their shared boundary is the normalized node and typed-fact vocabulary.

Catalog discovery likewise accumulates ordinary and rich observations through one deterministic accumulator before producing public summaries. Bounded filesystem traversal is isolated from observation reconciliation and selected-session loading.

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

The accepted [presentation-index contract](../../.ariadne/decisions/2026-07-31-trace-presentation-index-contract.md) defines `PresentationIndex` as a second immutable snapshot over one loaded `SessionTrace`. It does not extend `TraceIndex` or change canonical structural parents.

The implemented static index owns primary group membership, bounded secondary references, partial-order bands, typed aggregate metadata, completeness, persisted origin, visibility defaults, and bounded derived diagnostics. It retains canonical node positions and locators rather than cloning complete detail values or raw payloads. The browser constructs one snapshot beside `TraceIndex` while installing a selected session; mutation of nodes, locators, or typed facts requires an explicit rebuild.

Presentation grouping policy version 1 derives a small typed activity at each ordinary or rich source adapter. It classifies terminal command/write/poll, patch, MCP, web, image, code-cell, agent, interaction, and compaction observations without reopening arbitrary node detail JSON. The shared Codex shell parser marks a command exploration-eligible only when its non-empty parsed actions consist entirely of reads, file listings, or searches.

Lifecycle membership uses durable model-call, runtime-tool, terminal-operation, code-cell, interaction, or compaction identities. It never uses timestamps, labels, previews, or nearest-neighbor matching. Reused ordinary call IDs receive source-order occurrence numbers; ambiguous and cross-thread identities remain visible through singleton degradation and bounded diagnostics. Tool sessions and raw payloads are reference-only because several lifecycles may refer to them.

| Family | Identity and primary membership | Children or references | Failure behavior |
| --- | --- | --- | --- |
| Direct terminal, patch, MCP, web, image, or dynamic tool | Thread plus runtime tool ID, or model-call occurrence when no runtime ID exists | Terminal sessions and raw artifacts remain references | Orphan or ambiguous evidence remains partial, indeterminate, or singleton |
| Code cell | Thread plus code-cell ID; source and output items join the cell | Runtime tools requested by the cell remain independent child groups | Missing cells leave nested tools top-level with a diagnostic |
| Agent lifecycle | Thread plus agent tool ID; interaction edges join through tool endpoints, otherwise use edge identity | Child threads and carried items remain navigable references | Missing or cross-scope endpoints are retained without proximity inference |
| Compaction | Thread plus compaction ID; checkpoint, marker, and requests join | Input and replacement history remain in their original groups and become references | Missing referenced history marks the group partial |
| Exploration batch | Thread, policy version, and first eligible child group | Consecutive model-requested read/list/search tool groups are children | Single children remain top-level; any non-eligible group anchor flushes the batch |
| Unsupported event | Canonical singleton locator | Existing typed references remain available | Always remains visible rather than being dropped |

Nested code-cell tools are excluded from exploration batching so one group never has two containing parents. Higher-order batches have no copied canonical members: expanded events continue to name their lifecycle group, while collapsed scope queries omit child groups and return the container. Batch summaries use typed activity and counts without adding free-form text to the global summary budget.

Canonical order is a partial order. Ordinary ordinals and rich raw-event sequences retain their source meaning; stable correlations align equivalent observations; incomparable cross-source intervals remain explicitly unordered; and wall-clock timestamps never determine event position. Session-wide synchronization is available only when rich global sequence evidence supports it.

Every primary presentable event belongs to exactly one primary group. Sessions, threads, turns, and inference containers are structural-only; raw artifacts are reference-only; every retained node stays reachable through `TraceIndex`. Missing correlation yields a singleton or partial group rather than a proximity inference.

The hard structural limits are derived from the retained node count `N`: at most `2N` groups and `3N` direct membership entries. `PresentationLimits` exposes caller-selected bounds for secondary references, group and snapshot summary text, previews, diagnostics, diagnostic messages, and group depth. Its defaults retain the prior `4N` reference budget with at most 4,096 per group, depth 4, 4 KiB per group summary, 16 MiB total summary text, 256 preview characters, 1,024 diagnostics, and 4 KiB diagnostic messages. Values above those safety maxima clamp to the defaults; zero disables the corresponding retained data except that one diagnostic slot remains available to explain degradation. Existing constructors delegate exactly to the defaults.

Presentation implementation is private behind the stable crate-root exports. One build context owns effective limits, unavailable-fact fallback, thread scopes, and source-identity positions. Classification, interpreted content, bounded text, ordering, lifecycle correlation, owner indexes, hierarchy attachment, exploration batching, references, summaries, validation, and queries live in focused modules. This keeps renderer and source adapters from reconstructing presentation facts or bounds independently.

Static construction belongs in selected-session loading and is bounded by retained nodes, memberships, references, summaries, and diagnostics. Indexed lookup by group, canonical node, scope, or band does not scan the complete trace, parse arbitrary node detail, or open raw payloads. The exploration reducer accepts ordered facts in arbitrary chunks and produces the same completed containers; origin-aware live canonical admission and public incremental snapshots remain Phase 7 work.

The presentation types contain no Ratatui lines, styles, widths, wrapping, key bindings, overlays, parent `HistoryCell`s, or app-server clients. Parent `ExecCell` behavior is a parity oracle only: its all-read/list/search rule and incompatible-cell flush boundary are reproduced through shared typed parsing rather than importing TUI state.

## Search

Semantic search runs only when explicitly requested. It examines bounded semantic string fields and returns attributed hits containing the locator, field, provenance, evidence grade, match range, and centered snippet.

Search does not eagerly stringify every raw JSON value and does not currently inspect unopened raw payload bytes. Raw search, if added, must be separately bounded, cancellable, and explicit because it changes both cost and sensitivity.

## Payload access

Semantic JSON payloads needed by rich reduction are read eagerly only for the selected bundle and have a separate replay byte limit. `RawPayloadHandle` defers display bytes until the consumer requests them. Both paths canonicalize the bundle root and candidate and reject absolute paths, lexical or symlink escape, and non-regular files; the display reader also decodes lossily and sanitizes terminal control content.

Missing, rejected, unreadable, or oversized payloads are local failures. Their parent nodes and the rest of the trace remain navigable.

## Failure and compatibility

Malformed ordinary or rich records are isolated where the source reader permits it. Unknown versions retain safely readable information and emit diagnostics. A reducer failure prevents speculative downstream semantic replay but does not erase intact raw evidence.

New persisted or protocol fields are not owned here. A missing observation must first be represented as an unavailable capability and demonstrated through a synthetic test before a separate recorder or protocol proposal is considered.
