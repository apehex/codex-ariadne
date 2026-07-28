# `codex-trace`

`codex-trace` is Ariadne's offline, read-only model for persisted Codex execution traces. It discovers ordinary rollout sessions and opt-in rollout-trace bundles, projects them into a provenance-aware navigable graph, and exposes bounded semantic search and lazy raw-payload access without depending on a terminal UI.

## Public entry point

`TraceRepository` configures sources and limits. `discover()` returns bounded root metadata as a `TraceCatalog`; loading a selected ID returns a `SessionTrace` containing normalized nodes, capabilities, diagnostics, search access, and raw payload handles.

```rust
let repository = TraceRepository::new(codex_home).with_rich_root(trace_root);
let catalog = repository.discover().await;
let session = catalog.load_session(session_id).await?;
```

`SessionTrace::index()` constructs a `TraceIndex` snapshot for routine locator, root, and child lookup. The index also exposes compact node positions for consumers that retain an immutable loaded trace. Rebuild it after inserting, removing, or reordering nodes or changing a node locator or parent.

The crate re-exports its public model from `src/model.rs`, trace index from `src/index.rs`, and contained payload reader from `src/payload.rs`.

## Source pipeline

```text
ordinary rollout metadata and JSONL ─┐
                                     ├─ catalog → selected-session projection → SessionTrace
rich bundle manifests and events ───┘                                      └─ lazy payload handles
```

Discovery is metadata-oriented and does not eagerly reduce every rich bundle. Projection occurs for the selected root, retains source provenance, merges only stable identities, and represents conflicts or missing evidence explicitly.

## Module map

| File | Responsibility |
| --- | --- |
| `src/catalog.rs` | Repository configuration, discovery, selection, ordinary projection, and source reconciliation |
| `src/rich.rs` | Rich bundle reduction and normalized rich-node projection |
| `src/model.rs` | Public catalog, node, locator, capability, evidence, limit, diagnostic, and search-result types |
| `src/index.rs` | Snapshot index for locator lookup, root and child traversal, and compact node positions |
| `src/search.rs` | Bounded attributed semantic search |
| `src/payload.rs` | Canonically contained, size-limited, terminal-safe payload reads |
| `src/index_tests.rs` | Index ordering, lookup, compact-position, and snapshot-invalidation tests |
| `src/trace_tests.rs` | Ordinary, merged, discovery, graph, search, limit, and preservation tests |
| `src/rich_tests.rs` | Rich projection, payload, malformed-input, and compatibility tests |

## Development route

Read the local [`AGENTS.md`](AGENTS.md), the cross-crate [Ariadne design](../../.ariadne/DESIGN.md), and this module's [`DESIGN.md`](DESIGN.md) before changing source semantics or public interfaces.

Run the package-scoped tests and lints required by the repository root instructions. New trace cases must use deterministic synthetic data and must remain executable without authentication or network access.
