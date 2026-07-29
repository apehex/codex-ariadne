# Trace browser quality ledger

Updated: 2026-07-29

Status: implementation evidence; cross-platform workflow results pending

## Resource contracts

| Resource | Default bound | Accounting and exhaustion |
| --- | ---: | --- |
| Matching rollout or manifest files | 100,000 per configured root | Discovery stops before retaining another match and records a diagnostic. |
| Inspected filesystem entries | max of 1,024 or 16 times the file bound | Discovery stops before visiting another entry and records a diagnostic. |
| Retained nodes | 100,000 per selected root | The graph builder checks before mutation; session, source, and diagnostic nodes share the cap, and the first rejection records one diagnostic. |
| Ordinary JSONL record | 1 MiB | The reader retains at most the limit while discarding the rest of the line; a later record remains readable. |
| Rich event records | 100,000 events, 1 MiB per line | Replay counts non-empty attempted events, retains at most the line bound, skips invalid UTF-8 or oversized lines with diagnostics, and continues. |
| Rich semantic JSON payload | 1 MiB per referenced payload | Selected-bundle replay reads at most limit plus one byte and rejects an oversized payload. This read is eager semantic replay, not lazy display access. |
| Raw display payload | 1 MiB by default | A lazy handle reads at most limit plus one byte, reports truncation, and sanitizes terminal controls. |
| Search results | 1,000 hits | Stable node order is truncated after the exact hit count. |
| Search fields | 4,096 string fields per node | JSON traversal stops before collecting another field. |
| Search field text | 16,384 Unicode scalar values | Truncation occurs at a character boundary before matching. |
| Structured detail document | 64 KiB | Background rendering exposes a truncation notice. |
| Picker and record frames | Terminal-visible window | Rendering formats only the current visible slice plus fixed diagnostic/header rows. |

## Behavior and ownership coverage

| Production owner | Covered contract | Evidence |
| --- | --- | --- |
| `rollout-trace/reducer/bundle_path.rs` | Canonical relative regular-file containment for event logs and semantic payloads | `inspection_tests::replay_rejects_manifest_event_log_escape`, `semantic_payload_tests::rejects_oversized_and_escaping_payloads`, and Unix symlink tests |
| `rollout-trace/reducer/inspection.rs` public metadata and replay types/functions | Manifest-selected event log, bounded records, invalid-record recovery, event limit, diagnostics | `inspection_tests.rs` custom-path, malformed, invalid UTF-8, oversized-followed-by-valid, and event-limit tests |
| `rollout-trace/reducer/semantic_payload.rs` | Zero, exact, plus-one, invalid UTF-8, absolute, lexical, and symlink behavior | `semantic_payload_tests.rs` |
| `trace/catalog.rs` repository builders, discovery, summaries, selection, and diagnostics | Ordinary, rich, merged, lazy discovery, malformed input, roots, cycles, conflicts, and selected-session bounds | `trace_tests.rs` |
| `trace/graph.rs` admission types and methods | Zero, exact, plus-one, missing parent, duplicate preservation, conflicting evidence, and repeated-parent remapping | `graph_tests.rs` |
| `trace/ordinary.rs` bounded reader and projection | Exact and oversized-followed-by-valid line behavior plus ordinary integration trees | `ordinary_tests.rs` and `trace_tests.rs` |
| `trace/rich.rs` rich projection helpers | Parent validation, semantic evidence downgrade, raw payload handles, and merged integration | `rich_tests.rs` and rich cases in `trace_tests.rs`; broader semantic-family depth remains pending |
| `trace/presentation.rs` presentation constructor, classifiers, decoders, sanitizers, and bounded documents | Role/channel/class selection, interpreted newlines, terminal controls, and UTF-8 bounds | `presentation_tests.rs` |
| `trace/index.rs` index type and lookup/traversal methods | Stable ordering, locator lookup, roots, children, compact positions, and rebuild behavior | `index_tests.rs` |
| `trace/search.rs` search functions and match type | Attribution, hit/field/character exact limits, UTF-8, JSON pointers, controls, ASCII case behavior, and empty queries | `search_tests.rs` |
| `trace/payload.rs` limit, result, reader, and path sanitizer | Exact whole-object result, containment, regular-file rejection, controls, and display cap | payload case in `trace_tests.rs` |
| `trace/model.rs` public data types and trivial accessors | Deep equality throughout the owning module tests; constructors and read-only accessors are intentionally exempt from isolated tests when the integration tests exercise them | all `trace/src/*_tests.rs` |
| `trace-tui/picker.rs` picker state and query methods | Cached match replacement and restoration | `picker_tests.rs`; visible-window snapshots cover rendering |
| `trace-tui/app.rs` screens, actions, catalog/session installation, cancellation, and worker-result gates | preferred and exact source selection, error fallback, stale session identity, and payload identity through interaction tests | `render_tests.rs`; explicit worker-panic and pseudo-terminal coverage remain pending |
| `trace-tui/input.rs` key dispatch | Single-depth entry/return, paging, search, filters, help, content modes, and payload request | state transitions and snapshots in `render_tests.rs` |
| `trace-tui/browser.rs` navigation state | selection and viewport restoration, indexed windows, cycles, filtering, and 100,000-node profile | `render_tests.rs` |
| `trace-tui/browser/detail.rs` detail and payload controller | stale mode/width key rejection, lazy read, truncation, failure sanitization, and cross-session payload rejection | `render_tests.rs` |
| `trace-tui/browser/search.rs` search/filter controller | query generations, stale rejection, visible/all scope, and hidden-hit reveal | `render_tests.rs` |
| `trace-tui/jobs.rs` background detail and search jobs | semantic modes, bounded detail, generation result installation, and search attribution | controller tests and snapshots in `render_tests.rs` |
| `trace-tui/render.rs` record table and shared formatting helpers | narrow, medium, wide, header, column, preview, control, and viewport behavior | `render_tests.rs` and reviewed snapshots |
| `trace-tui/render/picker.rs` windowed picker rows | root and filtered picker snapshots; structural visible-window assertion remains pending | `render_tests.rs` |
| `trace-tui/render/overlay.rs` detail, search, filter, and help surfaces | corresponding deterministic snapshots | `render_tests.rs` |
| `trace-tui/view.rs` options and visual-renderer contract | option-controlled columns, headers, previews, and injected renderer behavior | `render_tests.rs` and parent TUI renderer tests |
| `.ariadne/demo/demo.py` generator functions and `BundleBuilder` | deterministic generation, verification, and authoritative replay in focused CI | `.github/workflows/ariadne-trace.yml` |

## Explicit residual evidence gaps

- The pseudo-terminal offline/read-only CLI test is not yet implemented.
- Rich projection still needs deep whole-graph expectations for every semantic family.
- Picker rendering is windowed by construction and snapshots, but a visit-counter structural test is still needed.
- Worker panic and shutdown-during-work need deterministic lifecycle coverage.
- Cross-platform workflow results must be attached to the candidate commit before the quality plan can close.

## Local validation

- `just test -p codex-rollout-trace`: 74 passed.
- `just test -p codex-trace`: 30 passed.
- `just test -p codex-trace-tui`: 12 passed and one profile test skipped by default.
- `just test -p codex-trace-tui profile_hundred_thousand_node_navigation -- --ignored --nocapture`: passed with 100,000 nodes, 323.096 ms index construction, 2.578 ms first entry, 0.000 ms warm navigation p95, 4.921 ms warm level-entry p95, and 0.096 ms warm return p95.
- The focused `codex-cli` parser tests passed, including the compatibility assertion that `codex -- trace` remains the literal interactive prompt while `codex trace` selects the browser command.
- `cargo insta pending-snapshots --manifest-path trace-tui/Cargo.toml` reported no pending snapshots.
- `PYTHONPYCACHEPREFIX=/tmp/ariadne-quality-pycache python -m py_compile .ariadne/demo/demo.py` passed.
- `just fix -p codex-rollout-trace` and `just fix -p codex-trace -p codex-trace-tui -p codex-cli` passed, followed by `just fmt`.
- The repository-wide argument-comment Bazel target enumeration is currently blocked by the pre-existing unsupported `binary_test_target_compatible_with` attribute in `codex-rs/windows-sandbox-rs/BUILD.bazel`. The packaged scoped fallback is also blocked because its pinned Rust 1.92 nightly cannot compile the workspace's `sqlx` 0.9, which requires Rust 1.94. This is a validation-infrastructure gap rather than a trace-browser lint finding.
