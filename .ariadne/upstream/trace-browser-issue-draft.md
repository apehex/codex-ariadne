# Upstream issue draft: local agent trace browser

> **Submission gate:** Draft only. Do not submit, comment, cross-post, or open a pull request without explicit operator approval. Re-check for duplicate issues and current contribution policy immediately before any approved submission.

## Proposed title

Proposal: add a local TUI for inspecting historical multi-agent traces

## Draft body

### Problem

Codex persists enough local execution data to resume sessions and diagnose many individual events, but it has no integrated historical interface for following an execution from a parent thread through delegated children and correlating turns, model generations, tools, compactions, and inter-agent communication.

The existing chat transcript and agent picker answer "what is this thread doing now?" They do not provide a filesystem-like view of a completed agent tree or a forensic inspector that distinguishes semantic projections from exact raw evidence.

This makes several failure classes difficult to understand:

- a child stopped or failed and its causal inputs are unclear;
- a parent delegated work but did not integrate the returned result;
- compaction changed the effective model-visible history;
- a tool lifecycle spans several low-level records;
- ordinary and diagnostic traces contain different levels of detail;
- evidence is encrypted, absent, malformed, or too large for eager loading.

### Proposed interface

Add `codex trace [SESSION_ID]`, a local, read-only Ratatui browser. With no ID it opens a searchable root-session picker. The selected session is presented as a nested tree of threads, turns, inference calls, messages, tools, compactions, interaction edges, diagnostics, and raw payloads.

The browser would:

- use normal rollouts for broadly available historical navigation;
- use the existing opt-in `codex-rollout-trace` bundle for exact inference and runtime evidence;
- merge matching sources without hiding contradictions;
- label evidence as exact, semantic, reconstructed, unavailable, or conflicting;
- load large raw payloads only when opened;
- search the selected root and descendants;
- later provide the same model through `/trace` for live sessions.

### Privacy and safety boundary

This is an offline local debugger, not telemetry:

- no model call, authentication, network connection, or upload;
- no mutation, repair, archive, delete, or retention operation;
- rich capture remains explicitly enabled through `CODEX_ROLLOUT_TRACE_ROOT`;
- raw content starts collapsed, is path-contained to its bundle, and is rendered as inert terminal-safe text;
- tests and examples use deterministic synthetic traces only.

Normal rollouts would remain unchanged. If a required causal input is missing from both normal and rich traces, any new recorder event should be proposed separately, justified by a failing synthetic observability test, and active only for explicit rich tracing.

### Why this fits the existing code

Codex already has the major primitives:

- `codex-rollout` for persisted session discovery and replay;
- `codex-rollout-trace` for raw event capture and deterministic reduction;
- protocol and state types for thread relationships;
- Ratatui history renderers and agent navigation.

The proposed work is primarily a provenance-aware inspection model and historical navigation surface. It can live in dedicated crates and avoid growing `codex-core`.

### Related issues

- [#32753: Multi-agent V2 regression: subagent instructions are no longer observable](https://github.com/openai/codex/issues/32753) describes the missing operator-visible causal input for encrypted delegation.
- [#31198: subagent session logs grow to 145GiB from repeated compacted replacement_history](https://github.com/openai/codex/issues/31198) shows why a browser must stream, bound, and lazily load historical records.
- [#34282: rollout trace reducer panics when truncating non-ASCII JSON summaries](https://github.com/openai/codex/issues/34282) is a concrete reducer-hardening dependency.
- [`codex-rs/rollout-trace`](https://github.com/openai/codex/tree/main/codex-rs/rollout-trace) provides the existing rich diagnostic foundation.

### Proposed staging

1. Harden the rich reducer and verify delegation observability with synthetic fixtures.
2. Add the read-only ordinary, rich, and merged trace model.
3. Add `codex trace` with picker, tree, inspector, and search.
4. Add timeline and evidence-backed workflow diagnostics.
5. Reuse the stable model for live `/trace`.

### Acceptance criteria

- Root-child-grandchild sessions are navigable and searchable end to end.
- Exact rich context is inspectable and ordinary missing evidence is explicit.
- Malformed records or payloads do not hide unaffected trace content.
- Input files remain byte-identical after browsing.
- The browser works without authentication or network access.
- UI and graph behavior have deterministic synthetic and snapshot coverage.

### Feedback requested

- Does a dedicated `codex trace` surface fit the intended CLI and TUI architecture?
- Should the trace model become a reusable internal crate for future IDE and app-server inspection surfaces?
- Are there privacy or compatibility constraints beyond the explicit opt-in rich-capture boundary that the design should incorporate?

## Pre-submission checklist

- Obtain explicit operator approval.
- Search current issues and discussions for duplicates.
- Re-verify all linked issue states and titles.
- Re-read `docs/contributing.md`.
- Replace speculative wording with evidence from implemented synthetic tests.
- Remove fork-specific terminology and ensure no private paths or traces appear.
- Attach text screenshots or deterministic fixtures only if requested.
