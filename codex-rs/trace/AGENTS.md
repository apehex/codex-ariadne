# Trace model instructions

Load [`../../.ariadne/AGENTS.md`](../../.ariadne/AGENTS.md) before substantial work in this module. These instructions add only `codex-trace` constraints.

## Ownership

This crate owns bounded discovery of ordinary rollouts and rich bundles, source projection, additive reconciliation, provenance and evidence grades, capabilities, stable locators, graph diagnostics, semantic search, and contained lazy payload reads.

Do not add Ratatui state or rendering here. Reuse `codex-rollout` and `codex-rollout-trace`; do not introduce a third persisted trace schema.

## Invariants

- Keep all source access offline and read-only.
- Preserve source-local identity and conflicting observations.
- Never infer unavailable plaintext, intent, or causal relationships.
- Resolve payload references against their canonical bundle root and reject escape or non-regular targets.
- Bound discovery, record sizes, retained nodes, search fields, search hits, and payload display; report exhaustion as a diagnostic.
- Keep raw payload loading lazy.
- Preserve usable unknown, malformed, orphaned, cyclic, and missing records as navigable diagnostics.
- Do not add runtime recording or upstream-core changes without a failing synthetic observability test and an open Ariadne plan.

## Change discipline

Keep work proportional to trace size out of frame rendering by exposing indexed, windowed, or cached model operations. Avoid public APIs that require consumers to clone, serialize, or scan the complete node set for routine navigation.

Update [`DESIGN.md`](DESIGN.md) when source projection, reconciliation, evidence, locators, graph relationships, bounds, payload containment, or public interfaces change.

Source-model changes require deterministic synthetic tests for ordinary, rich, and merged inputs as applicable. Cover malformed input, unavailable evidence, conflicts, bounds, containment, and byte-for-byte input preservation. Never use a real or redacted trace fixture.
