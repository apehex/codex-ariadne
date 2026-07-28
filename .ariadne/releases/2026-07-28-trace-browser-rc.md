# Historical trace browser release candidate

Prepared: 2026-07-28

State: candidate documentation only; no tag or external release exists

## Candidate scope

This candidate adds Ariadne's historical `codex trace` browser to an otherwise recognizable Codex checkout.
It reads ordinary local rollouts and explicit rollout-trace bundles, projects them into a provenance-aware model, and presents the result in a read-only Ratatui interface.

The candidate does not add live `/trace`, enable trace recording, define a new persisted schema, authenticate, make model calls, modify source traces, or publish captured trace data.

## Build beside upstream Codex

Record the exact source commit before building:

```console
git rev-parse HEAD
rustc --version --verbose
cargo --version --verbose
cargo build --locked --manifest-path codex-rs/Cargo.toml --release -p codex-cli
```

On Linux or macOS, copy the resulting executable under a fork-specific name:

```console
mkdir -p ~/.local/bin
install -m 0755 codex-rs/target/release/codex ~/.local/bin/codex-ariadne
~/.local/bin/codex-ariadne trace --help
```

On Windows PowerShell:

```powershell
$AriadneBinDir = Join-Path $env:LOCALAPPDATA "Ariadne\bin"
New-Item -ItemType Directory -Force $AriadneBinDir | Out-Null
Copy-Item codex-rs\target\release\codex.exe (Join-Path $AriadneBinDir "codex-ariadne.exe")
& (Join-Path $AriadneBinDir "codex-ariadne.exe") trace --help
```

Add the selected directory to `PATH` if desired.
The executable keeps upstream's `codex` help spelling internally; the installed filename is deliberately `codex-ariadne` so a global upstream Codex installation is not replaced.

`--locked` fixes Cargo dependency resolution to the checked-in lockfile.
These instructions are source-reproducible but do not claim bit-for-bit identical artifacts across operating systems, toolchains, linkers, or signing environments.

## Synthetic smoke demonstration

With `codex-ariadne` available:

```console
python .ariadne/demo/demo.py run
```

The [demo documentation](../demo/README.md) describes its fixed synthetic scenario, expected interface, and privacy boundary.

## Validation gate

Before assigning a version or creating a tag, record successful results for:

```console
cargo test --locked --manifest-path codex-rs/Cargo.toml -p codex-trace -p codex-trace-tui
cargo test --locked --manifest-path codex-rs/Cargo.toml -p codex-cli --bin codex trace_command
```

Generate the public demo and replay it with the authoritative reducer:

```console
python .ariadne/demo/demo.py generate <new-empty-directory>
python .ariadne/demo/demo.py verify <same-directory>
cargo run --locked --manifest-path codex-rs/Cargo.toml -p codex-cli -- debug trace-reduce <same-directory> --output <temporary-state-file>
```

The focused [Ariadne trace workflow](../../.github/workflows/ariadne-trace.yml) runs these checks on Linux, macOS, and Windows.
Workflow configuration alone is not cross-platform evidence; record links to successful runs for the exact candidate commit.

Also record the reference host and performance results required by the active hardening plan before making responsiveness claims.

## Local performance evidence

The deterministic debug-build profile was run on 2026-07-28 from the uncommitted candidate worktree based on `2de9ee0940`:

```text
OS: Linux 7.1.4-arch1-1 x86_64
CPU allocation: 4 cores of AMD EPYC 9354P 32-Core Processor
Rust: rustc 1.95.0 (59807616e 2026-04-14)
Cargo: cargo 1.95.0 (f2d3ce0bd 2026-03-21)
Trace nodes: 100,000
Browser/index construction: 390.572 ms (background loading path)
First 100,000-child expansion: 35.845 ms
Warm navigation p95, 1,000 actions: 0.001 ms
Warm expansion p95, 20 actions: 39.170 ms
Warm collapse p95, 20 actions: 0.494 ms
```

Command:

```console
cargo test -p codex-trace-tui profile_hundred_thousand_node_navigation -- --ignored --nocapture
```

This is local feasibility evidence for one deterministic worst-case fan-out shape, not cross-platform evidence and not a universal latency guarantee.
The test regenerates the trace and enforces a 50 ms p95 gate for warm navigation and expansion.
Record a fresh result for the exact committed release candidate before publication.

## Trust and data boundary

- Treat every real bundle as sensitive local data.
- The browser is offline and read-only, but rendering a trace reveals its contents to the person at the terminal.
- Payload paths must remain contained under their selected bundle, and payload content is loaded only when requested.
- Evidence grades distinguish exact, semantic, reconstructed, unavailable, and conflicting observations.
- Only the deterministic synthetic demo belongs in a public release package.

## Known omissions

- no live view of an active conversation;
- no trace recording controls;
- no generalized timeline or policy diagnostics;
- no guarantee that unknown future persisted formats preserve every semantic field;
- no bit-for-bit cross-platform build claim;
- no signed or notarized artifacts prepared by this repository state.

## Operator-owned release actions

An operator must select the version and exact commit, review license and notice packaging, confirm all required CI and benchmark evidence, build platform artifacts in trusted environments, record SHA-256 checksums, perform signing or notarization if applicable, create the tag and release, and publish the final notes.
None of those actions is performed or authorized by this candidate document.
