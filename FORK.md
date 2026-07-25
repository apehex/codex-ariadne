# Ariadne

Ariadne is a public fork of [OpenAI Codex](https://github.com/openai/codex)
focused on local inspection of agent execution traces.

The project adds an operator-facing trace browser while preserving Codex's
normal runtime behavior. Its first interface is `codex trace`; a later
milestone will expose the same trace model through `/trace` in the interactive
TUI.

## Relationship to upstream

Ariadne is not an OpenAI product and is not endorsed by OpenAI. Codex is
licensed under Apache-2.0; this fork preserves the upstream license and notices.
Changes unique to Ariadne use the same repository license unless a file states
otherwise.

The branch model deliberately keeps upstream synchronization simple:

- `main` tracks `openai/codex` `main` without Ariadne-only commits.
- `ariadne` is the public integration branch, rebased onto `main`.
- Short-lived `ariadne/<milestone>` branches contain reviewable increments.
- Release tags are cut from `ariadne`.
- Any upstream candidate starts from upstream `main` and contains only the
  minimal generalized commits needed for that proposal.

Typical synchronization:

```bash
git fetch upstream
git switch main
git merge --ff-only upstream/main
git switch ariadne
git rebase main
```

Do not merge `ariadne` into `main`. Resolve rebase conflicts in the smallest
feature branch that owns the affected code, then record any architectural
consequence in `codex-rs/trace-tui/project/`.

## Product boundary

Ariadne's trace browser is a local debugger:

- It reads existing Codex rollouts and opt-in rollout-trace bundles.
- It does not make model calls, require authentication, or use the network.
- It does not edit, repair, archive, delete, or otherwise mutate trace inputs.
- It labels reconstructed, exact, contradictory, and unavailable evidence
  distinctly.
- It treats prompts, responses, tool inputs and outputs, terminal output, and
  paths as sensitive local data.

The normal `~/.codex/sessions` rollout remains the broadly available source.
The opt-in `CODEX_ROLLOUT_TRACE_ROOT` bundle is the richer source for exact
model and runtime boundaries. Enabling rich tracing must remain an explicit
choice because its payloads may contain substantially more sensitive content.

## Public-data policy

Only deterministic synthetic trace fixtures may be committed. Fixtures must not
be derived by redacting a real trace: structure, identifiers, timestamps,
paths, prompts, responses, commands, and outputs must all be generated for the
test scenario.

Never commit:

- user or agent traces from a real session;
- credentials, tokens, personal information, or project source;
- private filesystem paths or host metadata;
- vulnerability-research evidence;
- copied local Codex state databases.

Bug reports should use a minimal synthetic reproduction or aggregate
measurements. If a real trace is essential to diagnosis, keep it outside the
repository and agree on a private handling channel before sharing it.

## Development route

The trace work is documented under [`codex-rs/trace-tui/`](codex-rs/trace-tui/):

- `README.md` describes the operator interface and project status.
- `DESIGN.md` defines the architecture and evidence semantics.
- `AGENTS.md` defines local implementation constraints.
- `project/` preserves decisions, milestone receipts, and the upstream issue
  draft.

Follow the root `AGENTS.md` for Codex-wide build, test, and style rules. Trace
changes should remain in dedicated crates and avoid increasing `codex-core`
unless a demonstrated observability gap can only be recorded at a core runtime
boundary.

## Upstream participation

OpenAI Codex currently accepts external code contributions by invitation only.
Generalized findings and proposals should therefore begin as issues or
discussion in the upstream repository. The draft in
`codex-rs/trace-tui/project/upstream-issue-draft.md` must not be submitted
without explicit operator approval.

If upstream invites an implementation, prepare a clean branch from upstream
`main`. Do not include Ariadne branding, fork-maintenance records, or unrelated
history in that branch.
