# Ariadne roadmap

Status: non-executable successor directions

The first historical `codex trace` browser and its ordinary, rich, and merged trace model are implemented. This roadmap preserves later directions without granting authority to modify the codebase. Each direction requires a draft plan, current-state inspection, explicit acceptance evidence, and promotion to `plans/open/`.

## Workflow diagnostics and timeline

Add a synchronized cross-thread timeline and evidence-backed diagnostics for unjoined delegation, interruption, failed tools, approval waits, unintegrated child results, compactions, and unusual resource consumption. Diagnostics must describe recorded behavior rather than infer agent intent, include positive and false-positive synthetic tests, and open their supporting nodes.

The current browser uses index-backed adjacency, cached visible rows and inspector content, viewport-only rendering, and subtree-local expansion and collapse. Future performance work should begin with representative measurement and target unresolved gaps such as chunk-bounded JSONL reading, cooperative rich-replay cancellation, and content-level merge conflict detection.

## Live `/trace`

Embed the stable browser components in the primary Codex TUI and add incremental app-server-backed updates without weakening the offline historical path. Historical and live views should render a completed synthetic trace equivalently except for explicitly transient state.

## Observability extensions

Propose recorder, persisted-schema, core-runtime, or app-server additions only for evidence gaps demonstrated by prior browser work. Keep sensitive capture explicit and opt-in, and justify every field through a failing synthetic observability scenario with versioning, bounds, and privacy behavior.

## Release evidence

Before describing the browser as cross-platform, obtain successful Linux, macOS, and Windows runs of the focused trace workflow for the exact release-candidate commit using platform-neutral synthetic fixtures.
The workflow configuration and release-candidate instructions define the evidence route but are not themselves evidence that any platform passed.
