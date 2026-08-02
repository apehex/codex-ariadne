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

`SessionTrace::fact_index()` constructs a `TraceFactIndex` snapshot over retained typed source facts. It exposes per-domain thread order, source ownership, and exact correlation lookup without parsing `TraceNode::detail`. Ordinary ordinals are comparable only inside their owning thread, rich event sequences share the rich source domain, and wall-clock times remain display facts. Rebuild the snapshot after structurally mutating the public node vector.

Each `TraceNode` carries a bounded `TraceRecordPresentation` for listing, styling, and visibility filters. `TraceNode::content_document()` extracts bounded semantic Markdown, text, JSON, or code without following a raw-payload reference; exact payload access remains explicitly lazy.

The crate explicitly re-exports its supported model types from `src/model.rs`, structural and fact indexes from `src/index.rs` and `src/fact_index.rs`, typed facts from `src/facts.rs`, and the contained payload reader from `src/payload.rs`.

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
| `src/catalog.rs` | Repository configuration, bounded discovery, selection, summaries, and source reconciliation |
| `src/graph.rs` | Bounded node admission, duplicate preservation, parent remapping, and conflict diagnostics |
| `src/ordinary.rs` | Bounded plain or compressed ordinary-record reading and projection |
| `src/ordinary_facts.rs` | Typed ordinary ordinal, ownership, and durable protocol-identity extraction |
| `src/rich.rs` | Rich bundle reduction and normalized rich-node projection |
| `src/rich_facts.rs` | Typed rich sequence, producer, runtime, compaction, and interaction extraction |
| `src/rich_terminal_facts.rs` | Terminal ownership and model-observation correlation extraction |
| `src/model.rs` | Public catalog, node, locator, capability, evidence, limit, diagnostic, and search-result types |
| `src/presentation.rs` | Record classification and bounded semantic content documents |
| `src/index.rs` | Snapshot index for locator lookup, root and child traversal, and compact node positions |
| `src/facts.rs` | Renderer-neutral source-order, ownership, availability, identity, and relation vocabulary |
| `src/fact_index.rs` | Immutable typed-fact lookup and compatible-domain thread ordering |
| `src/presentation_*.rs` | Renderer-neutral policy facts, lifecycle ownership, hierarchy reduction, summaries, validation, and immutable presentation queries |
| `src/search.rs` | Bounded attributed semantic search |
| `src/payload.rs` | Canonically contained, size-limited, terminal-safe payload reads |
| `src/*_tests.rs` | Sibling unit and integration evidence for each owning module |

## Development route

Read the local [`AGENTS.md`](AGENTS.md), the cross-crate [Ariadne design](../../.ariadne/DESIGN.md), and this module's [`DESIGN.md`](DESIGN.md) before changing source semantics or public interfaces.

Run the package-scoped tests and lints required by the repository root instructions. New trace cases must use deterministic synthetic data and must remain executable without authentication or network access.
