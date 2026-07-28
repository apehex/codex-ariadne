# Historical trace compatibility matrix

Recorded: 2026-07-28

State: local candidate evidence; cross-platform workflow runs pending

## Matrix

| Shape | Evidence | Local result |
| --- | --- | --- |
| Ordinary rollout and three-level delegation | `codex_trace::tests::ordinary_threads_form_a_searchable_tree_and_preserve_diagnostics` | Pass |
| Orphan and cyclic parent metadata | `codex_trace::tests::orphan_and_cycle_threads_remain_reachable_from_the_session` | Pass |
| Rich bundle and runtime graph | `codex_trace::tests::exact_bundle_directory_discovers_rich_semantic_nodes` plus the synthetic demo reducer | Pass |
| Merged ordinary and rich sources | `codex_trace::tests::matching_ordinary_and_rich_sources_merge_without_node_loss` | Pass |
| Mismatched source identity | `codex_trace::tests::mismatched_rich_root_is_a_conflicting_diagnostic` | Pass |
| Malformed rich records and partial replay | `codex_trace::tests::malformed_rich_spines_downgrade_surviving_semantic_nodes` and rollout-trace resilient replay tests | Pass |
| Failed child result and interrupted lifecycle shapes | rollout-trace agent-result fallback and failed inference/compaction tests | Pass |
| Compaction and context compaction | rollout-trace conversation and thread compaction tests | Pass |
| Unicode and terminal control content | rollout-trace UTF-8 normalization tests and trace payload sanitization test | Pass |
| Payload path escape, symlink, and regular-file checks | trace payload containment and rollout-trace payload-path tests | Pass |
| Node and event limits | trace selected-session bound and rollout-trace replay-limit tests | Pass |
| Narrow, medium, and wide terminal layouts | deterministic `codex-trace-tui` snapshots | Pass |
| Large structured detail and raw payload | 64 KiB disclosed detail cap and viewport-only raw-payload test | Pass |
| 100,000-node navigation and expansion | ignored deterministic responsiveness profile | Pass locally |

## Local commands

```console
cargo test -p codex-trace -p codex-trace-tui
cargo test -p codex-rollout-trace
cargo test -p codex-trace-tui profile_hundred_thousand_node_navigation -- --ignored --nocapture
```

All 14 `codex-trace`, 6 ordinary `codex-trace-tui`, the ignored 100,000-node profile, all 70 `codex-rollout-trace`, and all 230 `codex-cli` binary unit tests passed locally.

## Residual evidence gap

The matrix maps deterministic tests to compatibility claims; it does not replace successful operating-system runs.
Linux, macOS, and Windows evidence remains pending until the focused workflow passes on the exact committed candidate.
