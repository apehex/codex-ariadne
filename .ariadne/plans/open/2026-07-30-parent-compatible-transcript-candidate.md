# Parent-Compatible Transcript Candidate

Updated: 2026-07-31

Status: open

## Objective

Prepare and maintain a minimal parent-compatible candidate showing that Ariadne can serve as the Codex transcript surface without requiring upstream-owned trace topology, rendering, or grouping implementations to be duplicated.

## Context

This is Phase 8 and depends on the working opt-in integration from [Phase 7](2026-07-30-live-parent-trace-integration.md). Candidate extraction must preserve the ownership and upstream-separation rules of the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

Ariadne can complete the engineering candidate locally and update it as upstream evolves. Actual replacement of the upstream transcript depends on upstream product interest, review, and acceptance; those outcomes are outside this plan's authority and closure criteria.

## Scope

Owned work includes candidate extraction, compatibility shims, feature boundaries, migration notes, parity and performance evidence, fallback strategy, upstream-drift tracking, and draft discussion material under `.ariadne/upstream/`.

No issue, discussion, pull request, comment, push, tag, release, default change, or removal of the upstream fallback is authorized without explicit operator approval.

## Movements

### Movement 1: Separate The Candidate

- Identify the smallest generalized commits required for the presentation model, browser surface, visual adapter, command launch, and live bridge.
- Exclude Ariadne branding, governance, private evidence, release machinery, and unrelated fork history.
- Keep most implementation in dedicated crates and minimize edits to high-touch upstream TUI modules.

### Movement 2: Demonstrate Replacement Behavior Locally

- Add a local candidate mode in which the parent transcript shortcut uses Ariadne by default while the legacy transcript remains an explicit fallback.
- Demonstrate grouped, expanded, structural, thread, group-member, and structured-value views for completed and live sessions.
- Preserve raw/copy-friendly transcript behavior, configurable key bindings, accessibility, performance, and terminal restoration.

### Movement 3: Track Upstream Compatibility

- Rebase or adapt the candidate against selected upstream baselines and record conflicts, changed transcript families, and required shim updates.
- Keep a versioned parity matrix and synthetic compatibility suite rather than relying on manual visual inspection.
- Distinguish local candidate completeness from upstream adoption status in every report.

### Movement 4: Prepare Optional Upstream Material

- Draft a concise architecture description, behavioral demonstration, change-size breakdown, compatibility argument, and staged landing proposal.
- Record alternative adoption scopes such as shared presentation indexing, optional `/trace`, transcript overlay replacement, or standalone crate reuse.
- Leave all external communication and submission as operator-controlled follow-up.

## Acceptance Evidence

- A clean candidate can be constructed from an upstream baseline with a documented, bounded diff.
- Local tests demonstrate that Ariadne can replace the transcript surface while retaining an explicit legacy fallback.
- Synthetic parity, live-update, cross-thread, JSON navigation, accessibility, performance, offline, and terminal-restoration evidence is reproducible.
- Upstream drift is represented by versioned compatibility results and explicit failures.
- Documentation never describes upstream adoption as completed unless an upstream artifact proves it.

## Closure

Close this plan when the local replacement candidate, extraction recipe, compatibility suite, fallback, drift record, and optional upstream material are complete.

Record upstream acceptance, rejection, inactivity, or changed direction as an external outcome in the roadmap or a separately authorized follow-up; none is required to close the local engineering phase.
