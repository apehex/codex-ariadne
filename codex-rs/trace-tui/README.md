# Ariadne trace browser

`codex-trace-tui` is the Ratatui interface behind `codex trace`. It lets an operator select a historical root session, follow nested agent threads, inspect semantic and raw trace nodes, and search the selected tree.

```bash
codex trace
codex trace <session-id>
codex trace --bundle <trace-bundle>
codex trace --trace-root <directory>
```

With no target, `codex trace` opens a searchable root-session picker. Supplying a session or bundle opens it directly. The browser uses the source and evidence model provided by [`codex-trace`](../trace/).

## Status

The first historical browser is implemented on the `ariadne` branch. It supports ordinary sessions, opt-in rich bundles, and merged roots with a picker, adaptive tree browser, inspector, selected-root semantic search, diagnostics, and lazy raw payloads.

The implementation receipt and validation are preserved in the [closed V1 plan](../../.ariadne/plans/closed/2026-07-25-ariadne-codex-trace-browser.md). Later timeline, diagnostic, live, and observability directions are non-executable items in the [roadmap](../../.ariadne/ROADMAP.md).

## Mental model

```text
root session
└── root thread
    ├── turn
    │   ├── inference
    │   ├── message
    │   ├── tool call
    │   └── compaction
    ├── delegated child thread
    │   └── ...
    └── diagnostic
```

Entering a directory-like node reveals its children. Opening an item-like node shows semantic metadata and, when available, the raw records supporting it. Breadcrumbs and parent navigation preserve orientation across delegated work.

The synchronized cross-thread timeline, live following, export, annotations, and `/trace` integration are not part of the current browser.

## Privacy and source behavior

Browsing is local, offline, and read-only. The application does not initialize authentication, make model calls, require the network, or mutate rollout and bundle inputs.

Raw prompts, responses, commands, tool arguments, outputs, terminal content, and paths may be sensitive. Payloads start collapsed, referenced paths remain contained to their bundle, and displayed content is sanitized as inert terminal text.

The cross-crate [Ariadne design](../../.ariadne/DESIGN.md) defines source reconciliation and evidence grades. The [`codex-trace` design](../trace/DESIGN.md) defines projection, bounds, search, and payload behavior.

## Navigation

| Key | Action |
| --- | --- |
| `j` / `k`, arrows | Move |
| `l`, Enter | Enter or open |
| `h`, Backspace | Parent |
| `g` / `G` | First / last |
| `/` | Search |
| `n` / `N` | Next / previous match |
| Tab | Change pane |
| Esc | Close inspector or search |
| `q` | Quit |

Wide terminals show hierarchy, entries, and inspector panes. Medium and narrow terminals collapse to two and one pane without changing the enter/back navigation model.

## Development route

Read the local [`AGENTS.md`](AGENTS.md) and [`DESIGN.md`](DESIGN.md) before changing application state or rendering. Project identity, plans, decisions, privacy policy, and upstream material are indexed by [`.ariadne/README.md`](../../.ariadne/README.md).
