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

The historical browser on the `ariadne` branch supports ordinary sessions, opt-in rich bundles, and merged roots with a picker, grouped and expanded conversation lenses, canonical structural navigation, group-member and thread-reference descent, structured JSON navigation, semantic search, visibility filters, full-screen record detail, diagnostics, and lazy raw payloads.

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

Each screen shows one level. The default collapsed lens shows one row per top-level presentation group; the expanded lens shows canonical primary events with their owning group; the structural lens exposes canonical containment. `Tab` and `Shift-Tab` cycle them. Enter descends according to row type, `i` inspects canonical detail, `s` navigates normalized JSON, and Escape restores the prior selection and viewport.

List rows use an elastic name followed by label-free metadata values like a long directory listing. Wide terminals add optional headers and a bounded one-line content preview; narrower terminals drop lower-priority columns rather than introducing more panes.

Detail view cycles through Rendered, Text, and Raw modes with `v`. Rendered mode delegates Markdown, JSON, and code to the parent Codex visual language, Text exposes decoded semantic content with real newlines, and Raw loads an exact bounded artifact lazily when one exists or clearly labels normalized JSON when it does not.

The synchronized cross-thread timeline, live following, export, annotations, and `/trace` integration are not part of the current browser.

## Privacy and source behavior

Browsing is local, offline, and read-only. The application does not initialize authentication, make model calls, require the network, or mutate rollout and bundle inputs.

Raw prompts, responses, commands, tool arguments, outputs, terminal content, and paths may be sensitive. Payloads start collapsed, referenced paths remain contained to their bundle, and displayed content is sanitized as inert terminal text.

Rendering is viewport-bounded. Structured node detail is limited to 64 KiB, loaded raw payloads retain the source reader's bound, and interpreted detail is cached by record, width, mode, and payload generation. The complete retained trace is not rescanned during ordinary cursor movement or redraw.

The cross-crate [Ariadne design](../../.ariadne/DESIGN.md) defines source reconciliation and evidence grades. The [`codex-trace` design](../trace/DESIGN.md) defines projection, bounds, search, and payload behavior.

## Navigation

| Key | Action |
| --- | --- |
| `j` / `k`, arrows | Move |
| Enter / `i` | Descend or inspect |
| `s` | Navigate normalized JSON |
| Tab / Shift-Tab | Next / previous lens |
| Escape / Backspace | Detail or parent |
| `gg` / `G` | First / last |
| Page Up/Down, Ctrl-U/D | Page or half-page |
| `/` / `g/` | Search visible / all records |
| `n` / `N` | Next / previous match |
| `f` / `F` | Edit / reset visibility filters |
| `v` | Cycle Rendered / Text / Raw |
| `?` | Navigation help |
| `q` | Quit |

Every terminal width shows one list or one detail surface. Responsive headers, columns, and previews change density without changing navigation.

## Development route

Read the local [`AGENTS.md`](AGENTS.md) and [`DESIGN.md`](DESIGN.md) before changing application state or rendering. Project identity, plans, decisions, privacy policy, and upstream material are indexed by [`.ariadne/README.md`](../../.ariadne/README.md).
