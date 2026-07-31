# Decision: Materialized Trace Presentation Index

Date: 2026-07-31

Status: accepted

## Context

Ariadne currently normalizes persisted ordinary rollouts and rich rollout-trace bundles into a provenance-aware `SessionTrace`. `TraceIndex` is a structural snapshot over those canonical nodes and owns locator, root, and parent-to-child lookup.

The Codex TUI applies a different transformation for display. Live event handlers correlate starts, outputs, and ends into mutable `HistoryCell`s; exploration commands may share one cell; persisted app-server `ThreadItem`s are reduced into transcript cells; and some control-plane facts are omitted or summarized. This behavior is grounded in `tui/src/history_cell/`, `tui/src/exec_cell/model.rs`, `tui/src/chatwidget/command_lifecycle.rs`, `tui/src/chatwidget/tool_lifecycle.rs`, `tui/src/multi_agents.rs`, and `tui/src/thread_transcript.rs`.

Those presentation rules are useful, but neither `HistoryCell` layout nor timestamp proximity is a suitable trace topology. Ariadne needs a reversible presentation projection that supports grouped conversation, expanded event, structural, thread, group-member, and structured-value views without modifying canonical evidence or rebuilding groups during rendering.

## Decision

Build one bounded, renderer-neutral `PresentationIndex` for each immutable loaded `SessionTrace` snapshot. The presentation index is a derived view over canonical trace positions and durable correlation facts. It is not persisted, does not mutate `SessionTrace`, and is rebuilt whenever the canonical snapshot changes.

For live integration, apply typed live presentation facts incrementally to the same deterministic presentation reducer. Once a live session is complete and its persisted facts are available, batch and incremental reduction must converge to deeply equal presentation indexes except for facts explicitly classified as transient and unavailable from persistence.

## Ownership

| Concept | Owner | Contract |
| --- | --- | --- |
| Persisted ordinary and rich records | `codex-rollout` and `codex-rollout-trace` | Authoritative source formats and source-local identities |
| Canonical normalized evidence | `SessionTrace` in `codex-trace` | Retained observations, provenance, evidence grade, structural parent, semantic detail, and raw-payload handles |
| Structural lookup | `TraceIndex` in `codex-trace` | Locator lookup, roots, containment children, and immutable node positions |
| Typed order and correlation facts | A focused `codex-trace` snapshot companion | Source-order domains, thread ownership, turn ownership, durable correlations, and explicit missing or conflicting facts |
| Presentation reduction | `PresentationIndex` and policy modules in `codex-trace` | Group identity, membership, order bands, summaries, completeness, default visibility, and derived diagnostics |
| Browser state and rendering | `codex-trace-tui` | Lens selection, reversible navigation, filtering, search, viewport state, content interpretation, styling, and terminal layout |
| Parent integration | A narrow parent-TUI adapter | Current-thread selection, typed live facts, lifecycle integration, Codex visual renderer, and reversible launch/return |

No renderer, parent `HistoryCell`, or browser controller may infer presentation groups independently.

## Canonical Evidence And Presentation Projection

A canonical trace node is one retained normalized observation with a stable source-namespaced locator. Canonical does not mean exact: the node retains its `EvidenceGrade`, and repeated or conflicting observations remain distinct nodes.

`TraceIndex` describes containment and lookup only. It does not own event chronology, causal inference, grouping, summaries, visibility, or UI order.

A presentable event is a canonical node admitted to the grouped or expanded presentation views by a typed disposition. The initial dispositions are:

- `primary`: the node belongs to exactly one primary presentation group;
- `reference_only`: the node is reachable from one or more groups but is not moved out of its canonical structural location;
- `structural_only`: the node remains available in the structural lens but does not become a conversation event.

Sessions, threads, turns, and inference containers default to `structural_only`. Raw payload handles default to `reference_only`. Messages, reasoning summaries, plans, tool and agent lifecycle facts, compactions, diagnostics, and unknown event-like records default to `primary`.

Every primary event has exactly one primary group. Secondary references may point from inference, compaction, terminal, interaction, diagnostic, or raw-evidence views to the same canonical node without changing primary ownership.

Presentation grouping never rewrites model-visible history, persisted rollout JSONL, rich bundles, app-server threads, canonical locators, structural parents, or evidence grades.

## Evidence, Completeness, And Live Origin

Canonical evidence grade, group completeness, and observation origin are orthogonal:

- evidence grade remains `exact`, `semantic`, `reconstructed`, `unavailable`, or `conflicting`;
- completeness is `complete`, `running`, `partial`, or `indeterminate`;
- origin is `persisted`, `live`, or `mixed`.

A group retains the set or bounded counts of member evidence grades instead of collapsing them into one supposedly strongest grade. Exact evidence may enrich a summary but never deletes its semantic counterpart. Contradictory member facts make the relevant aggregate field conflicting and retain links to every observation.

`partial` means an expected lifecycle member, referenced object, or source interval is missing or truncated. `running` means a live start has no terminal observation yet. `indeterminate` means the source cannot establish whether the lifecycle was complete. None of these states invents an endpoint.

Live-only facts are labeled by origin and availability; they are not assigned a stronger evidence grade merely because they were observed in memory. When a live-only fact disappears after reload because it was never persisted, parity documentation records the gap.

## Order Domains

Order is a typed partial order, not a timestamp sort.

- Ordinary rollout records use their thread-local numeric ordinal or retained line order.
- Rich events use the raw event spine sequence, which may establish order across threads when the source records one global sequence.
- Structural containers use their containment position and recorded creation metadata only for structural navigation.
- Wall-clock timestamps are display metadata and never reorder causal events.
- Stable locators provide deterministic tie-breaking only; they do not create a causal claim.

Each thread has canonical order bands. Members within one source domain retain source order. Stable cross-source identities align equivalent observations and form merge anchors. Unmatched observations between the same anchors remain in an explicitly unordered cross-source band.

The presentation index may linearize one unordered band deterministically for display by source kind and stable locator, but the band remains marked unordered so the UI does not describe that linearization as causality.

If merged sources have no stable alignment anchors, their event streams remain separate source segments rather than being interleaved by timestamps.

Ordinary evidence does not create a total order across concurrent threads. A synchronized cross-thread event lens is available only when rich global sequence evidence supports it; otherwise navigation follows thread containment and explicit interaction edges.

## Group Identity

`GroupId` is a typed, snapshot-stable identity rather than a display index or formatted string.

- A singleton group is identified by its canonical node locator.
- A correlated lifecycle group is identified by thread, group kind, and a durable source correlation such as a model-visible call ID, runtime tool-call ID, terminal operation ID, compaction ID, or interaction ID.
- A higher-order batch is identified by thread, policy kind and version, and the stable ID of its first child group.
- A partial lifecycle with no durable correlation remains a singleton or a source-scoped occurrence group; it is never joined by timestamp or adjacency alone.

When one source reuses the same correlation for independent lifecycles, the source-domain occurrence ordinal disambiguates them. Repeated observations of the same lifecycle remain members of the same group and preserve their separate canonical locators.

Group IDs are stable only for the same immutable source version or for an incremental live group whose anchor has already been admitted. They are not persisted compatibility identifiers across upstream schema migrations, changed trace limits, or changed policy versions.

## Group Model

The initial group families are:

- user message;
- assistant message, including commentary and final answer;
- reasoning or reasoning summary;
- proposed plan and plan update;
- direct tool lifecycle for exec, terminal interaction, MCP, dynamic tools, web, image, patch, and code-cell work;
- exploration batch containing correlated read, list, and search tool groups;
- agent spawn, assignment, send, resume, wait, result, and close lifecycle;
- compaction lifecycle;
- system or developer context;
- diagnostic or failure;
- unknown-event singleton.

Every group records a kind, stable ID, owning thread, anchor order band, direct members, optional child groups, typed aggregate metadata, completeness, origin, default visibility, and bounded derived diagnostics.

Direct member roles include invocation, model-visible input, runtime start, runtime output, runtime end, model-visible result, message content, reasoning content, summary, auxiliary evidence, conflict evidence, and reference.

Higher-order groups contain child group IDs rather than copying their canonical node members. A group has at most one containing group, nesting is acyclic, and entering a child group remains possible.

Summaries are typed presentation facts such as actor, operation, target, status, duration, counts, and bounded content preview. They are not rendered Ratatui lines and are never the only route to supporting evidence.

Aggregate status is conservative. Contradictory terminal states are conflicting; failure or abort is not hidden by a successful observation; a live start without an end is running; missing expected evidence is partial or indeterminate; and completed is used only when retained evidence supports completion.

## View Order And Navigation

The collapsed conversation lens orders top-level groups by their anchor order band, then by group kind and `GroupId` only as deterministic non-causal tie-breakers. A nested child group appears through its containing group rather than as a duplicate top-level row.

The expanded event lens orders primary events by canonical thread event order and annotates every row with its primary group. Non-contiguous group members remain non-contiguous. Optional boundary markers may repeat around separated spans without materializing duplicate groups.

Entering a group lists its direct canonical members and child groups in canonical event order. Member role may label a row but never reorder it ahead of recorded chronology.

The structural lens continues to use `TraceIndex` containment order and exposes every retained canonical node, including `structural_only` and `reference_only` nodes.

Thread navigation follows structural parent and child relationships. Interaction edges are navigable references, not containment. Return restores the exact lens, locator or group ID, selection, and viewport.

Structured-value navigation derives object-key and array-index locations from one bounded selected semantic value. These locations are browser-local paths and never become canonical trace nodes or presentation groups.

## Default Visibility

Default visibility is typed policy data on groups and primary events; effective visibility belongs to the browser lens and user filters.

| Family | Collapsed conversation | Expanded event | Structural |
| --- | --- | --- | --- |
| User, assistant, commentary, final, and plan | shown | shown | shown |
| Reasoning summary | shown | shown | shown |
| Raw reasoning content | hidden unless enabled | hidden unless enabled | shown when retained |
| Tool lifecycle and exploration batch | shown as group | members shown | shown |
| Agent lifecycle and wait | shown as group | members shown | shown |
| System and developer context | hidden by default | available through context filter | shown |
| Compaction and routine control-plane records | hidden by default | available through control filter | shown |
| Raw payload artifacts | reference only | reference only | shown |
| Diagnostics, failures, conflicts, and unknown event kinds | shown | shown | shown |

Hidden means omitted from that lens by default, not discarded. Search across all evidence may reveal a hidden item temporarily without mutating the configured filter.

## Resource Bounds

Let `N` be the number of retained canonical nodes after `TraceLimits.max_nodes_per_session` is applied, `P` the number of primary presentable events, and `G` the number of materialized groups.

The presentation snapshot obeys these exact structural bounds:

- `P <= N`;
- primary node memberships equal `P`;
- `G <= 2N`, allowing at most one primary group and one higher-order container per canonical node;
- total node and child-group membership entries are at most `3N`;
- group nesting depth is at most 4;
- each group has at most `N` direct primary members;
- secondary references are capped at 4,096 per group and `4N` across the snapshot;
- derived presentation diagnostics are capped at `min(N + 1, 1,024)`;
- each free-form group summary is capped at 4 KiB of valid UTF-8;
- the combined free-form summary budget is `min(4 KiB × G, 16 MiB)`;
- listing previews are capped at 256 Unicode scalar values;
- structured-value navigation is capped at 64 levels, 4,096 direct children per value, and 64 KiB of displayed scalar content.

Arithmetic uses checked or saturating operations before allocation. The index stores canonical positions and interned typed correlations instead of cloning complete node details or raw payloads.

Construction time and structural memory are `O(N + R)`, where `R` is the admitted membership and reference count. Construction occurs once during selected-session loading or an explicit live update, outside the terminal frame and input paths.

When an optional higher-order group would exceed a bound, its already admitted child groups remain visible and the container is omitted with a bounded diagnostic. When a required primary membership cannot be admitted, construction returns an explicit incomplete index rather than silently dropping the event. Secondary-reference, summary, structured-child, and diagnostic exhaustion each produce one bounded saturation notice.

Raw payload bytes remain lazy and are excluded from presentation-index construction.

## Renderer-Neutral Parent Boundary

The reusable parent boundary is an immutable presentation snapshot plus typed queries for group IDs, canonical locators, order bands, summaries, completeness, visibility, and semantic content references.

The boundary contains no Ratatui `Line`, color, terminal width, wrapping, key binding, overlay, `HistoryCell`, app-server client, authentication, or model-context type.

`codex-trace-tui` and a future Codex parent adapter consume the same snapshot. Each renderer chooses layout and visual style while preserving group identity, member coverage, status, evidence, and visibility semantics.

The live adapter emits bounded typed facts carrying current thread, durable correlation identity when available, order domain, lifecycle role, and origin. It does not mutate persisted traces or synthesize canonical raw evidence.

Parent `HistoryCell` behavior is a parity oracle and visual compatibility target, not the presentation-index API. Upstream-specific command dispatch, alternate-screen ownership, composer restoration, and fallback transcript behavior remain in the parent adapter.

## Worked Examples

### Interleaved Background Tool

Tool A starts, unrelated message B appears, and tool A ends later. The start and end retain their canonical event positions and share one tool group through call identity. The collapsed lens shows one group anchored at the start; the expanded lens shows A-start, B, and A-end in order with group annotations; entering A shows its members together in canonical order.

### Exploration Batch

Several correlated read, list, and search tool groups satisfy the versioned exploration policy. Each tool lifecycle remains its own child group. One higher-order exploration group owns the child-group references and summary. A later unrelated command flushes the batch. If the container cap is exhausted, the child groups remain visible independently.

### Agent Wait

A wait tool group references several child threads and their observed states. The child threads remain structurally owned by their recorded parent and are not embedded into the wait group. Entering the wait shows wait lifecycle members and navigable thread references. A missing wait result produces a running or partial group rather than an invented completion.

### Compaction

The compaction group owns its marker, request, and replacement lifecycle nodes. Input-history and replacement-history items remain in their original primary groups and are attached as bounded secondary references. The collapsed conversation hides routine compaction by default; expanded and structural lenses expose it.

### Missing Endpoint

A tool result without a retained invocation remains a visible partial singleton or source-scoped lifecycle group. A start without an end remains running for live origin and indeterminate or partial for persisted origin. Neither is attached to the nearest tool by time.

### Repeated And Conflicting Observations

Ordinary and rich sources report the same call identity, while one reports success and the other failure. Both canonical observations remain members of one correlated group. The status is conflicting, both evidence paths are navigable, and source-local order is preserved.

### Concurrent Child Threads

Two child threads run concurrently. Containment determines their parent, and interaction edges express messages or results. Ordinary timestamps do not create a session-wide order. If rich global sequence exists, an optional synchronized lens may order recorded cross-thread events; otherwise each thread retains its own order bands.

### Regressing Timestamp

A record written today carries a timestamp from ten days earlier. Its canonical source ordinal or rich sequence determines event position, while the timestamp is displayed unchanged as metadata. No presentation group moves backward because of the timestamp.

## Blocking Rules For Later Phases

The architecture has no unresolved choice that blocks Phase 1. Later phases must stop their affected movement rather than guess when any of these conditions occurs:

- Phase 1 cannot retain a typed source order or correlation without parsing arbitrary display JSON; the source adapter must be extended within plan scope or the fact must remain unavailable.
- Phase 2 cannot assign one primary group to a presentable event within the structural bounds; construction must return an incomplete index with evidence rather than omit the event.
- Phase 3 lacks a durable correlation for a proposed lifecycle group; the records remain singleton groups instead of being joined by adjacency or timestamp.
- Phase 3 cannot reproduce an upstream batching boundary from deterministic facts; the policy remains unsupported and child groups stay independent.
- Phase 4 would need canonical nodes for JSON keys or array indexes; it must keep them browser-local or stop.
- Phase 6 finds a parent transcript behavior that depends on unpersisted live state; it must classify the difference and define a Phase 7 live fact rather than reconstruct it.
- Phase 7 requires a persisted-format, recorder, `codex-core`, model-context, or app-server API change; integration stops pending a separate observability decision and accepted plan.

## Consequences

Ariadne gains a single presentation representation for historical, grouped, expanded, and future live views while retaining the evidence-first structural graph.

Non-contiguous groups and partial-order bands add model complexity, but they prevent timestamp sorting, renderer-time grouping, and accidental loss of interleaved work.

The fixed bounds can omit optional batching, references, or summary content on extreme traces. Such degradation remains explicit, while canonical structural evidence stays navigable.

Parent integration can remain a narrow adapter because presentation identity and reduction stay in Ariadne-owned modules. Actual upstream adoption remains an external product outcome rather than an architecture invariant.
