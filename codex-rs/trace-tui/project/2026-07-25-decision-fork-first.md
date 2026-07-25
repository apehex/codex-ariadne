# Decision: fork-first historical trace browser

Date: 2026-07-25

Status: accepted

## Context

Codex already contains rollout readers, a rich rollout-trace recorder and
reducer, app-server protocol types, state indexing, and a Ratatui TUI. An
external app-server client would minimize coupling, but it would not expose all
raw context and would provide less leverage for studying or improving runtime
observability.

The intended contribution is both a useful debugger and an informed proposal
for first-class Codex observability.

## Decision

Develop Ariadne as a Codex fork, using dedicated trace model and TUI crates.
Ship a historical `codex trace` browser first. Add `/trace` after the model and
privacy behavior are proven.

Keep upstream synchronization cheap by preserving an upstream-only `main`,
rebasing an `ariadne` integration branch, and staging work in small milestone
branches.

## Consequences

The project can reuse and improve internal trace facilities and can add missing
capture at the correct runtime boundary. In exchange, it must continuously
manage upstream API churn and keep fork-wide edits small.

The viewer must not treat privileged access to source code as permission to
expand data collection. Rich capture remains explicit, local, and opt-in.

Upstream work begins with analysis and an issue. A pull request is prepared only
after an invitation under the upstream contribution policy.
