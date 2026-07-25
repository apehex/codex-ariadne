# Ariadne trace browser

Ariadne turns Codex's local execution records into a navigable tree. It is
designed for operators who need to understand not only what an agent answered,
but how work moved between turns, tools, model requests, and delegated agents.

The first interface is:

```bash
codex trace
codex trace <session-id>
codex trace --bundle <trace-bundle>
codex trace --trace-root <directory>
```

`codex trace` opens a historical root-session picker. Supplying a session or
bundle opens it directly. A later milestone adds `/trace` to the live Codex TUI
using the same source and navigation model.

## Status

The first historical browser is implemented on the `ariadne` branch: local
ordinary sessions and opt-in rich bundles can be selected, navigated, searched,
and inspected through `codex trace`. See
[`project/2026-07-25-v1-receipt.md`](project/2026-07-25-v1-receipt.md) for
validation and known limitations, and
[`project/2026-07-25-plan.md`](project/2026-07-25-plan.md) for successor
milestones.

## Mental model

The browser treats an agent workflow like a filesystem:

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

Entering a directory-like node reveals its children. Opening an item-like node
shows semantic metadata and, where available, the exact raw records that support
it. Breadcrumbs and parent/child jumps preserve orientation across delegated
work.

The initial browser provides:

- a searchable root-session picker;
- nested parent/child agent navigation;
- turns, inference calls, messages, tools, compactions, and diagnostics;
- semantic and raw inspectors with source-capability labels;
- search across the selected root and all descendants;
- lazy loading suitable for large histories.

The synchronized cross-thread timeline, live following, export, annotations,
and `/trace` integration are later milestones.

## Trace sources

### Ordinary rollouts

Normal Codex JSONL sessions under `~/.codex/sessions` contain the durable
transcript and lifecycle records available to every user. They support useful
historical browsing, but may not contain an exact context snapshot for every
model generation. Multi-agent messages may also be encrypted.

### Rich rollout-trace bundles

Setting `CODEX_ROLLOUT_TRACE_ROOT` opts a run into the existing
`codex-rollout-trace` recorder. Its bundle contains ordered raw events, payload
references, and a reducer that derives model-visible and runtime objects. This
is the preferred source for exact inference boundaries and causal edges.

Rich traces are local diagnostic artifacts, not telemetry. They can include
prompts, responses, commands, tool inputs and outputs, terminal output, and
paths. Treat the entire bundle as sensitive.

### Merged view

When an ordinary rollout and rich bundle identify the same root, Ariadne shows
a merged tree. It preserves both sources and reports their capabilities:

- `exact`: directly supported by a raw captured payload;
- `semantic`: represented by an ordinary or reduced semantic record;
- `reconstructed`: derived deterministically from surrounding records;
- `unavailable`: expected evidence was not recorded or cannot be decrypted;
- `conflicting`: sources disagree and both observations remain inspectable.

## Privacy and safety

Browsing is local, offline, and read-only. It makes no model calls and does not
require Codex authentication. Ariadne must not change a rollout, bundle,
payload, state database, archive marker, or timestamp.

Raw payloads start collapsed for readability. Opening them is an ordinary local
inspection action, not a network disclosure. Payload paths are contained to
their bundle and content is rendered as inert, control-character-safe text.

Only synthetic trace fixtures belong in this repository. See the public-data
policy in [`FORK.md`](../../FORK.md).

## Navigation

The browser follows Codex and Vim conventions:

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

The wide layout uses hierarchy, entries, and inspector panes. Medium and narrow
terminals collapse to two and one pane without changing the navigation model.

## Contributing

Start with [`DESIGN.md`](DESIGN.md) and the local [`AGENTS.md`](AGENTS.md).
The fork strategy and upstream route are documented in [`FORK.md`](../../FORK.md).

OpenAI Codex currently accepts outside pull requests only by invitation. The
project therefore keeps a reviewable issue draft and clean upstream-candidate
route rather than assuming that fork changes will be accepted.
