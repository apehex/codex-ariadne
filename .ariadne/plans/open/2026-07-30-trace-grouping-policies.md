# Trace Grouping Policies

Updated: 2026-07-31

Status: open

## Objective

Complete deterministic, versioned presentation-group policies for the Codex record families needed by the grouped conversation and expanded trace views.

## Context

This is Phase 3. The minimal index from [Phase 2](2026-07-30-static-trace-presentation-index.md) proves the partition and query model. This phase adds the group families and degradation rules defined by the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md) without adding terminal rendering or browser state.

Some groups combine the lifecycle of one correlated object, while higher-order groups such as an exploration batch contain several child tool groups. Temporal adjacency may close or batch groups only where Codex presentation semantics explicitly require it; it must not replace durable correlation.

## Scope

Owned paths are focused grouping modules under `codex-rs/trace/`, presentation-index tests, design documentation, and deterministic synthetic fixtures.

Parent `HistoryCell` implementations may be inspected as behavioral evidence but are not modified in this phase. TUI rendering, JSON navigation, parent integration, persisted formats, and recorder changes are excluded.

## Movements

### Movement 1: Complete Lifecycle Groups

- Add terminal command, write/poll, MCP, patch, web, image, code-cell, nested-tool, and raw-result group policies.
- Assign typed invocation, model-visible input, runtime start/output/end, model-visible result, and auxiliary member roles.
- Define partial and conflicting behavior for orphan starts, orphan results, repeated outputs, and truncated evidence.

### Movement 2: Add Agent And Compaction Groups

- Group spawn, send, wait, result, resume, close, and child-thread links through stable tool, thread, and interaction identities.
- Represent child threads as navigable references rather than embedding or duplicating their contents.
- Group compaction marker, requests, input references, and replacement references without moving the referenced conversation items out of their primary groups.

### Movement 3: Add Higher-Order Presentation Batches

- Reproduce documented Codex exploration batching for consecutive read, list, and search tool groups.
- Define explicit flush boundaries and preserve the child tool groups for descent.
- Add fallback policies for new upstream record types and unsupported tool families.

### Movement 4: Prove Batch And Incremental Equivalence

- Express grouping as a deterministic reducer over ordered presentation facts.
- Feed the same completed facts through batch and incremental construction and compare the entire resulting index.
- Preserve running groups and update aggregate metadata in place without changing their stable group ID or anchor order.

## Acceptance Evidence

- A policy ledger maps every supported record family to identity, membership, summary, aggregate status, visibility, nesting, and failure behavior.
- Interleaved background work produces non-contiguous membership without reordering canonical events.
- Entered groups show every primary member exactly once in deterministic order.
- Unknown or newly added record kinds remain visible through singleton fallback groups.
- Batch and incremental construction produce deeply equal completed indexes.
- The structure-derived fixture exercises nested agents, exploration batches, compaction, failures, interruption, missing endpoints, and interleaving.

## Closure

Close this plan after recording the grouping-policy version, parity evidence against inspected Codex behavior, exact tests, rejected heuristics, and any live-only presentation facts deferred to Phase 6 or 7.
