# Trace model design

## Responsibility

`codex-trace` converts persisted ordinary rollouts and rich rollout-trace bundles into one read-only inspection model. It owns source mechanics and semantic normalization; consumers own presentation.

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

Containment and causality are distinct. Parent-child edges answer where an item belongs; interaction edges answer how information moved. Broken parent links remain observable, orphans remain reachable, and cycles are cut with a diagnostic.

Children use recorded sequence when available and stable timestamp-and-identity ordering otherwise. Locators survive filtering and sorting within a source version but are not a persisted compatibility promise across upstream schema migrations.

## Projection and reconciliation

Ordinary projection uses `codex-rollout` and protocol records. It provides durable transcript and lifecycle semantics but does not claim exact generation context or decrypted collaboration content when those values were not persisted.

Rich projection uses the `codex-rollout-trace` manifest, ordered event spine, reducer, runtime objects, interaction edges, and payload references. Reducer output remains governed by that crate; `codex-trace` adapts it rather than copying the rich schema.

Reconciliation is additive:

- source-local IDs remain present;
- stable recorded identity is required for deduplication;
- exact values may enrich semantic presentation without deleting the semantic record;
- disagreements retain both observations and add a conflict diagnostic;
- absent capabilities remain unavailable rather than guessed.

## Bounds and performance

`TraceLimits` bounds discovery, ordinary records, rich events and sizes, projected nodes, search fields, and search hits. A reached limit produces a diagnostic that explains the incomplete view.

The selected trace is retained as a public node vector. `TraceIndex` provides a separately owned snapshot over that vector for locator lookup, roots, and parent-to-child adjacency. It builds in linear time, preserves node-vector order, provides expected constant-time selection before result iteration, and avoids duplicating graph semantics in consumers.

The index is deliberately not embedded in `SessionTrace`: its public nodes and their structural fields remain mutable for compatibility, so a hidden cache could silently become stale. Consumers build one index per loaded session and must rebuild it after inserting, removing, or reordering nodes or changing a locator or parent. Existing `SessionTrace` lookup methods retain their signatures and scan behavior for compatibility; routine browser navigation should use `TraceIndex`.

Rich replay is selected-root lazy but the upstream reducer is synchronous once invoked. An individual JSONL line can be allocated by the buffered reader before its configured size is rejected. Cooperative replay cancellation and chunk-bounded line reading are known hardening gaps.

## Search

Semantic search runs only when explicitly requested. It examines bounded semantic string fields and returns attributed hits containing the locator, field, provenance, evidence grade, match range, and centered snippet.

Search does not eagerly stringify every raw JSON value and does not currently inspect unopened raw payload bytes. Raw search, if added, must be separately bounded, cancellable, and explicit because it changes both cost and sensitivity.

## Payload access

`RawPayloadHandle` defers bytes until the consumer requests them. `SafePayloadReader` canonicalizes the bundle root and candidate, rejects escape and non-regular files, enforces a display-size limit, decodes lossily where necessary, and sanitizes terminal control content.

Missing, rejected, unreadable, or oversized payloads are local failures. Their parent nodes and the rest of the trace remain navigable.

## Failure and compatibility

Malformed ordinary or rich records are isolated where the source reader permits it. Unknown versions retain safely readable information and emit diagnostics. A reducer failure prevents speculative downstream semantic replay but does not erase intact raw evidence.

New persisted or protocol fields are not owned here. A missing observation must first be represented as an unavailable capability and demonstrated through a synthetic test before a separate recorder or protocol proposal is considered.
