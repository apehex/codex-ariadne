# Ariadne project index

Ariadne is a public Codex fork for understanding agent workflows from persisted local traces. The first interface is `codex trace`, an offline, read-only historical browser for ordinary Codex rollouts and opt-in rollout-trace bundles.

This directory is the fork-owned project control plane. It keeps Ariadne policy and history separate from upstream-owned files so upstream synchronization and candidate extraction remain straightforward.

## Repository map

| Path | Responsibility |
| --- | --- |
| [`AGENTS.md`](AGENTS.md) | Fork-wide working instructions and authority routing |
| [`DESIGN.md`](DESIGN.md) | Cross-crate architecture, invariants, and evidence model |
| [`doctrine/`](doctrine/) | Compact quality, gradient, agent, and delegation methods |
| [`plans/`](plans/) | Draft, executable, and closed work plans |
| [`ROADMAP.md`](ROADMAP.md) | Non-executable successor directions |
| [`decisions/`](decisions/) | Durable architectural decisions |
| [`demo/`](demo/) | Deterministic synthetic bundle generator and normal-interface launcher |
| [`releases/`](releases/) | Release-candidate build, validation, and operator handoff material |
| [`upstream/`](upstream/) | Generalized upstream discussion material |
| [`../codex-rs/trace/`](../codex-rs/trace/) | Source discovery, normalization, provenance, bounds, search, and safe payload access |
| [`../codex-rs/trace-tui/`](../codex-rs/trace-tui/) | Historical Ratatui application, navigation, and rendering |

## Public prototype quick start

Build the locked workspace release binary, install a copy beside upstream Codex as `codex-ariadne`, and run the deterministic synthetic demonstration:

```console
cargo build --locked --manifest-path codex-rs/Cargo.toml --release -p codex-cli
python .ariadne/demo/demo.py run --binary ./codex-rs/target/release/codex
```

See the [release-candidate instructions](releases/2026-07-28-trace-browser-rc.md) for Linux, macOS, and Windows side-by-side installation and evidence capture.
The focused [Ariadne trace workflow](../.github/workflows/ariadne-trace.yml) validates the trace crates, CLI parsing, and generated bundle on all three operating systems.

## Instruction discovery

The repository-root `AGENTS.md` belongs to upstream and must remain byte-identical to the tracked upstream version. Because Codex does not organically discover instructions inside a hidden sibling directory, each Ariadne-owned module has a local `AGENTS.md` that explicitly routes work through [`.ariadne/AGENTS.md`](AGENTS.md).

For work spanning Ariadne and upstream-owned modules, begin from an Ariadne-owned module or explicitly load [`.ariadne/AGENTS.md`](AGENTS.md) before editing. A task launched solely inside an unrelated upstream module will otherwise receive upstream instructions only.

## Working route

Before substantial Ariadne work, read [`.ariadne/AGENTS.md`](AGENTS.md), the applicable open plan, the cross-crate design when boundaries are involved, and the local module instructions and documentation. Use the doctrine documents only for the operation they govern.

Draft proposals are discussion material. Only plans under `plans/open/` authorize project-plan execution. Completed or superseded plans move to `plans/closed/` without losing their history.

| Doctrine | Load when |
| --- | --- |
| [`doctrine/quality.md`](doctrine/quality.md) | Reviewing artifacts, selecting validation, or improving project structure |
| [`doctrine/gradient.md`](doctrine/gradient.md) | Defining desired state, selecting gaps, planning movements, or reassessing results |
| [`doctrine/agents.md`](doctrine/agents.md) | Running a substantial controller/executor cycle |
| [`doctrine/delegation.md`](doctrine/delegation.md) | Splitting responsibility across agents or reconciling returned work |

## Ownership and documentation

Fork-wide identity, policy, architecture, decisions, roadmap, and project history live under `.ariadne/`. A module `README.md` explains its interface and development route; a module `DESIGN.md` owns module-internal design; a module `AGENTS.md` adds only local implementation constraints.

Do not duplicate upstream documentation or broad Ariadne doctrine in module files. Link to the owning document and record only the local delta.

## Public-data boundary

Only deterministic synthetic trace fixtures may be committed. Fixtures must be generated for their test scenario and must not be derived by redacting a real trace.

Never commit real user or agent traces, credentials, personal information, private filesystem or host metadata, private project material, copied Codex state databases, or sensitive investigation evidence. Public bug reports and benchmarks must use synthetic reproductions or aggregate measurements.

Rich traces may contain prompts, responses, commands, tool inputs and outputs, terminal output, and paths. Treat an entire bundle as sensitive local data even when only one record is being inspected.

## Branch and upstream model

`main` tracks `openai/codex` without Ariadne-only commits. `ariadne` is the public integration branch rebased onto `main`. Short-lived `ariadne/<milestone>` branches contain reviewable increments, and release tags are cut from `ariadne`.

Do not merge `ariadne` into `main`. An upstream candidate starts from upstream `main` and contains only the minimal generalized commits required by the proposal; it excludes Ariadne branding, fork governance, and unrelated history.

The material under [`upstream/`](upstream/) is draft material. Do not submit an issue, discussion, comment, or pull request without explicit operator approval and a fresh check of upstream policy and duplicate work.
