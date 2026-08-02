# Trace Grouping Policies

Updated: 2026-08-01

Status: closed

Closed: 2026-08-01

## Objective

Complete deterministic, versioned presentation-group policies for the Codex record families needed by the grouped conversation and expanded trace views.

## Context

This is Phase 3. The minimal index from [closed Phase 2](../closed/2026-07-30-static-trace-presentation-index.md) proves the partition and query model. This phase adds the group families and degradation rules defined by the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md) without adding terminal rendering or browser state.

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

## Implementation Record

Phase 3 completed on 2026-08-01 with presentation policy version 1. `TraceNodeFacts` now retains typed tool requester, terminal command/write/poll, exploration eligibility, code-cell, agent, interaction, and compaction activity directly from ordinary and rich source objects. Grouping never parses serialized node detail, reads raw payloads, or imports parent TUI state.

Durable model-call occurrences, runtime tool IDs, terminal-operation ownership, code-cell IDs, interaction endpoints, and compaction IDs form lifecycle identities. Terminal sessions and raw artifacts remain references. Code-mode tools remain independent child groups of their requesting code cell. Agent groups reference child threads rather than embedding their contents. Compaction groups own checkpoints, markers, and requests while input and replacement history keep their existing primary groups.

Versioned exploration batches reproduce the inspected parent `ExecCell` rule through the shared `codex-shell-command` parser: a model-requested exec group is eligible only when its non-empty parsed actions are all read, list, or search operations. A non-eligible top-level group anchor, scope boundary, or end of input flushes the run. One-child runs remain ordinary tool groups. Containers retain child IDs without copying canonical members and derive their stable ID and anchor from the first child.

The rejected heuristics are wall-clock proximity, display-label matching, preview parsing, arbitrary detail-JSON inspection, cross-thread adjacency, and attaching an orphan endpoint to the currently open group. Missing, reused, conflicting, cross-scope, or unsupported facts remain partial, indeterminate, diagnostic, or visible singleton evidence.

Focused synthetic tests cover typed tool classification, command/write/poll refinement, agent actions including resume, lifecycle interleaving, nested tools, child-thread references, compaction references, exploration flushes, chunk-independent batching, stable first-child identity, whole-index convergence after incremental admission, unsupported fallback, ambiguity, truncation, conflicts, resource bounds, and deterministic rebuilds. The separately authorized structure-derived conversation fixture was explicitly deferred; Phase 3 uses a hand-authored topology fixture and does not inspect any real or redacted trace.

## Compatibility And Deferred Evidence

The implementation changes only Ariadne-owned `codex-trace` modules plus its Cargo/Bazel dependency locks. The parent Codex TUI, persisted rollout formats, recorder, protocol types, and trace-TUI rendering are unchanged. Existing trace-TUI snapshots remain valid.

Public origin-aware live fact admission, transient `running` completeness, in-place parent integration, and final live-versus-persisted parity remain Phase 7 work. Phase 6 owns broader transcript parity. The open structure-derived fixture plan remains optional future shared evidence rather than a Phase 3 closure dependency.

## Module And Validation Record

The final production presentation-module sizes are `presentation_policy.rs` 126 lines, `presentation_index.rs` 153, `presentation_facts.rs` 180, `presentation_owners.rs` 197, `presentation_build.rs` 327, `presentation_order.rs` 342, `presentation_hierarchy.rs` 368, `presentation.rs` 378, `presentation_validate.rs` 381, `presentation_model.rs` 397, `presentation_summaries.rs` 402, and `presentation_tools.rs` 457. Every production presentation module remains below the 500-line doctrine target, and the new policy, hierarchy, and integration tests live in separate sibling test modules.

Validation passed with `just bazel-lock-update`, `just test -p codex-trace` (70 passed), `just test -p codex-trace-tui` (24 passed and one existing skipped test), `cargo clippy -p codex-trace --tests -- -D warnings`, the scoped all-target argument-comment lint for `codex-trace`, `just fix -p codex-trace`, `just fmt`, and `git diff --check`. The argument-comment fixer also cleaned the older positional-literal call sites in the Ariadne-owned trace crate. The repository-wide Bazel argument-comment lint now passes `codex-trace` and stops on 68 pre-existing findings in the unchanged `trace-tui` crate; those UI-only cleanups are outside this grouping phase and do not affect the scoped pass.
