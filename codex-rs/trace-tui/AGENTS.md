# Trace TUI instructions

These instructions apply to `codex-rs/trace-tui/` and supplement the repository
root `AGENTS.md`.

## Invariants

- The viewer is read-only. Do not add write, repair, archive, delete, migration,
  or retention operations for source rollouts, trace bundles, or Codex state.
- The viewer is offline. Browsing must not require authentication, a configured
  model, app-server connectivity, or network access.
- Keep evidence provenance explicit. Never present reconstructed data as exact
  or silently select one side of contradictory sources.
- Raw payloads are sensitive. Load them only on demand, keep them local, and
  render them as inert text with terminal control sequences sanitized.
- Resolve bundle payload references against the bundle root and reject any
  path that escapes it after canonicalization.
- Preserve unknown, malformed, orphaned, cyclic, and missing records as
  navigable diagnostics when the rest of the session remains usable.
- Commit deterministic synthetic fixtures only. Never copy or redact a real
  trace into the repository.

## Architecture

- Keep source discovery, projection, merging, capability reporting, locators,
  and search independent of Ratatui rendering.
- Reuse `codex-rollout` for ordinary rollouts and `codex-rollout-trace` for rich
  bundle reduction. Do not introduce a third persisted trace schema.
- Prefer a dedicated crate over new code in `codex-core`. Add runtime recording
  only after a synthetic test demonstrates that the required evidence is absent
  from both existing sources.
- The first public surface is `codex trace`. `/trace` must reuse the same model
  after the historical browser stabilizes.
- Keep indexing, reduction, and search cancellable. Do not block the terminal
  event loop on large files.

## Change discipline

Update `DESIGN.md` when an invariant, source precedence rule, public interface,
or node model changes. Add a dated decision or milestone receipt under
`project/` for substantial changes.

User-visible changes require Ratatui snapshot coverage. Source-model changes
require synthetic tests for ordinary, rich, and merged inputs as applicable.
Every end-to-end viewer test must be able to run with network access disabled.
