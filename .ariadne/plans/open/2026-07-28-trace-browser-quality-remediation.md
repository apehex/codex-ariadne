# Trace Browser Quality Remediation

Updated: 2026-07-28

Status: open

## Objective

Bring the historical trace browser to a quality-reviewable state in which:

- every source, model, replay, search, rendering, and payload operation has an explicit and tested resource bound;
- conflicting or repeated trace observations remain inspectable instead of being silently overwritten;
- routine TUI input never performs work proportional to the complete catalog or trace;
- the supported CLI surface and its compatibility cost are an explicit product decision;
- every fork-owned production function and type in scope has at least a one-line documentation comment or docstring, and every non-trivial behavior is represented in a test coverage ledger;
- focused continuous integration exercises all crates and profiles required by the public trace-browser claims.

Completion means another operator can reproduce the evidence below without using private rollouts, changing Codex recording formats, or trusting a timing claim in place of a structural bound.

## Context

The quality review was performed against `.ariadne/doctrine/quality.md`, the root `AGENTS.md`, and the local design contracts in `codex-rs/trace/DESIGN.md` and `codex-rs/trace-tui/DESIGN.md`. It found no model-visible context mutation, app-server API change, configuration compatibility change, or rollout-resume regression. Those surfaces must remain outside this remediation unless a separate accepted plan expands the scope.

The existing `.ariadne/plans/open/2026-07-28-credible-historical-trace-browser.md` records useful implementation and performance evidence, but it remains open and does not waive the findings below. This plan is a successor quality gate for that candidate. Do not close either plan or repeat its public-readiness claims until this plan's correctness, responsiveness, compatibility, and CI evidence is recorded.

The review's local baseline was:

- `just test -p codex-trace -p codex-trace-tui`: 20 tests passed and one performance profile was ignored;
- `just test -p codex-rollout-trace`: 70 tests passed;
- the focused CLI trace tests passed;
- no pending trace-TUI snapshots were present;
- the fork differed from its upstream base by more than 8,000 lines, so remediation must be split into independently reviewable changes.

Passing this baseline does not cover the missing invariants. In particular:

1. Ordinary catalog entries can be inserted before the configured node cap is checked.
2. Rich sessions can be admitted before their parent operation, leaving an orphaned terminal node when the cap is reached.
3. Repeated ordinary and rich observations sharing an identity can overwrite one another instead of preserving both pieces of evidence.
4. Rich replay can eagerly read referenced JSON payloads without the display payload byte bound.
5. Bundle manifests describe a configurable raw event log, while replay still assumes `trace.jsonl`.
6. Search runs synchronously from the TUI input loop, and the initial picker can scan and format the full catalog on each frame.
7. Adding a top-level `trace` subcommand consumes an input that previously started an interactive prompt; unlike other protected command words, this compatibility change has no explicit disposition.
8. Rich projection, lifecycle races, payload containment, search bounds, browser interactions, presentation states, and real offline/read-only CLI behavior lack sufficient automated coverage.
9. The deterministic 100,000-node profile is ignored in routine tests, and the focused workflow does not run `codex-rollout-trace`.
10. The TUI browser module exceeds the repository's large-module threshold, public exports and docs are broader or thinner than necessary, a new Rust test module is inline, opaque positional test arguments are undocumented, and the Python demo uses a disallowed `__future__` import and lacks docstrings.

Treat tests as behavioral evidence, not as a requirement to create one shallow test per helper. The implementation must maintain a coverage ledger mapping each fork-owned production function and type to a unit, integration, snapshot, or structural test, or to a written exception for a trivial accessor or statically defined value. Repository guidance against tests of static values still applies.

## Scope

Owned paths:

- `codex-rs/trace/`;
- `codex-rs/trace-tui/`;
- trace-related code in `codex-rs/rollout-trace/`;
- the existing trace command integration in `codex-rs/cli/`;
- trace demo code under `.ariadne/demo/`;
- focused trace documentation, release evidence, and `.github/workflows/ariadne-trace.yml`.

The following are explicit exclusions:

- live `/trace` support in an active TUI conversation;
- changes to recording, rollout, protocol, configuration, model-visible context, `codex-core`, or app-server formats;
- ingestion of real, redacted, or private user traces into fixtures or logs;
- generalized timeline, debugging, or policy-analysis features;
- external pushes, tags, releases, repository settings, or publication;
- broad cleanup of upstream Codex modules unrelated to the trace browser.

If a required fix crosses one of these boundaries, stop that movement and open a separate draft plan rather than expanding this authority implicitly.

## Movements

Each movement is a separate review unit. Complex logic changes should remain below 500 changed lines and every non-mechanical change below 800 changed lines. Split a movement further before implementation when its actual diff would exceed those limits. Preserve unrelated worktree changes.

### Movement 0: Freeze Contracts And Decisions

Dependencies: none.

- Record the resource-limit contract for catalog records, projected nodes, event bytes, semantic payload bytes, display payload bytes, search results, indexed fields, and rendered windows. Define whether diagnostics count against each cap and ensure every admitted object has a deterministic place in the accounting.
- Add regression tests that fail on the current pre-check insertion, rich-session orphaning, duplicate overwrite, manifest-path mismatch, unbounded semantic payload read, synchronous search, and full-catalog picker preparation. Prefer structural counters or injected bounded readers over fragile wall-clock thresholds.
- Build a coverage ledger for production functions, methods, and types in the owned paths. Record the covering test and invariant, not merely a test filename. Mark trivial accessors and static definitions as intentionally exempt instead of adding boilerplate tests.
- Make an explicit product decision for the top-level `codex trace` token: either accept and document that it no longer begins an interactive prompt, or preserve prompt compatibility and reserve the feature for a non-conflicting surface such as the future `/trace` command. Add the corresponding CLI test. Do not infer the decision from the current parser shape.
- Correct public claims immediately if they currently imply that all payloads are lazy. Distinguish bounded semantic replay reads from on-demand raw display reads.

Exit evidence:

- a committed limit table and behavior-coverage ledger;
- focused failing tests or fixtures for every correctness defect;
- a durable CLI compatibility decision or a documented blocker;
- no implementation movement depends on an undefined limit or command shape.

### Movement 1: Make Admission And Evidence Preservation Correct

Dependencies: Movement 0 limit contract.

Land this as two changes if source admission and reconciliation together exceed the size budget.

- Apply the ordinary record cap before mutation. Prove that all record shapes, including terminal threads and diagnostics, keep total admitted nodes within the declared bound.
- Project rich parent operations and their terminal/session nodes atomically or in an order that cannot leave an orphan when the cap is exhausted.
- Replace identity-based last-write-wins behavior with an evidence-preserving representation. Repeated observations must remain navigable, retain source provenance and stable ordering, and emit an explicit conflict diagnostic when semantically incompatible representations share an identity.
- Keep parent links, roots, capabilities, source evidence, and conflict metadata deterministic across ordinary-only, rich-only, and merged inputs.
- Enumerate the crate's public exports instead of retaining a broad `pub use model::*`, unless a compatibility audit demonstrates that a specific wildcard export is necessary.

Required tests:

- exact-limit, limit-plus-one, zero-limit, and diagnostic-at-limit ordinary catalogs;
- rich limits at every boundary around parent operation, child operation, terminal session, and diagnostic insertion;
- duplicate identical, duplicate conflicting, ordinary-then-rich, and rich-then-ordinary observations;
- deep equality assertions over expected nodes, parents, roots, capabilities, evidence, and conflicts;
- missing-parent and cyclic input fixtures that remain bounded and navigable.

### Movement 2: Bound Replay And Honor Bundle Manifests

Dependencies: Movement 0 limits; coordinate model changes with Movement 1.

Keep replay/manifest resolution separate from containment/bounded-reader changes if the combined diff approaches 500 lines of logic.

- Resolve the raw event-log path from the manifest field instead of assuming `trace.jsonl`.
- Apply the same lexical and filesystem containment rules used by payload access: reject absolute paths, parent escapes, and symlink escapes while allowing valid manifest-relative paths on Linux, macOS, and Windows.
- Bound every event record before deserialization. An oversized or invalid UTF-8 event must yield a bounded diagnostic and must not prevent a following valid record from being projected.
- Introduce a distinct semantic-payload read limit for referenced JSON needed by rich projection. Do not reuse a UI viewport as a parser limit or claim that semantic reads are lazy.
- Preserve raw payload inspection as on-demand, read-only, and windowed. Truncation must state the applied byte or line bound without exposing the truncated contents in diagnostics.
- Ensure the reducer and trace browser use one manifest-path resolution contract rather than independently reconstructing bundle locations.

Required tests:

- default and non-default manifest raw event-log names;
- missing files, absolute paths, `..` escapes, symlink escapes, and platform separator edge cases;
- event records at the exact limit, one byte above it, invalid UTF-8, and an oversized record followed by a valid record;
- semantic payloads at zero, exact limit, one byte above it, invalid UTF-8, absolute paths, and symlink escapes;
- successful and truncated structured inspector details with exact visible windows and explicit notices;
- byte-for-byte fixture comparison before and after every offline browser run.

### Movement 3: Remove Complete-Catalog Work From TUI Input And Render Paths

Dependencies: Movement 0 structural performance tests and Movement 1 stable model identity.

Implement picker and search work as separate changes.

- Cache or incrementally index picker labels and match data. Rendering a picker frame may inspect only the visible window plus a fixed amount of state; it must not format every catalog item.
- Move search preparation and execution out of the input/event loop. Tag jobs with model and query generations, ignore stale results, and make replacement, cancellation, and shutdown explicit.
- Preserve the last stable view while work is pending. Surface bounded progress or failure state without blocking navigation.
- Keep the 1,000-hit result cap, 4,096 indexed-field cap, and 16,384-character searchable field cap explicit and testable. Define Unicode normalization and case-folding behavior without slicing invalid UTF-8 boundaries.
- Extract inspector state and behavior from `trace-tui/src/browser.rs` into a sibling module with a sibling test file. Extract picker or search-job ownership as well when needed to bring production modules toward the 500-line target and below the roughly 800-line hard-review threshold.
- Preserve stable selection and hidden-hit reveal behavior when expansion, collapse, model replacement, or asynchronous results change visible rows.

Required lifecycle tests:

- preferred-source and auto-open success, ambiguous selection, cancellation, session construction failure, stale result delivery, shutdown during work, and local payload failure;
- rapid query replacement, empty query, empty trace, worker panic/error, and results delivered after a new session is selected;
- exact 1,000-hit, 4,096-field, and 16,384-character boundaries; over-limit input; case behavior; pointers; control characters; and multibyte Unicode near every byte boundary;
- a structural assertion that warm navigation, picker rendering, and viewport rendering do not visit all catalog entries or nodes;
- deterministic 10,000-node and 100,000-node profiles, with timing retained as diagnostic evidence rather than the only correctness gate.

### Movement 4: Complete Behavioral And Presentation Coverage

Dependencies: Movements 1 through 3 APIs stable.

- Add rich-projection tests for turns, messages, inference items, tool calls and results, code execution, compactions, terminal states, diagnostics, and delegation edges.
- Test nested expansion and collapse, multiple roots, child-pane navigation, first/last movement, hidden-search-hit reveal, empty traces, cycles, malformed records, and diagnostics.
- Add trace-TUI snapshots for Unicode and terminal controls, successful, failed, and interrupted rich traces, compaction and terminal nodes, delegation edges, truncated structured details, payload notices, and narrow and wide terminals.
- Strengthen merged-input assertions to compare complete expected structures rather than selected fields.
- Add a real pseudo-terminal integration test that invokes the supported CLI command on deterministic synthetic fixtures, navigates the browser, exits, and proves the fixture bytes did not change. Run it with network access and credential access unavailable.
- Keep test functions in descriptive sibling `*_tests.rs` modules when adding new modules. Move the newly added inline search tests to that layout.
- Use `pretty_assertions::assert_eq` and whole-object equality where practical. Do not add tests whose only purpose is to restate a static constant.

Exit evidence:

- every row of the Movement 0 coverage ledger has behavioral coverage or a reviewed exemption;
- presentation changes have reviewed and accepted snapshots with no pending `.snap.new` files;
- the pseudo-terminal test demonstrates offline, read-only operation rather than only testing library components.

### Movement 5: Complete Documentation And Local Code Hygiene

Dependencies: stable behavior and module boundaries from Movements 1 through 4.

- Add at least one meaningful line of Rust documentation to every fork-owned production type, trait, function, and method in the owned paths, including private items. Trait implementation methods may inherit a documented trait contract when they add no new behavior. Test functions are represented by descriptive names and the coverage ledger rather than redundant doc comments.
- Enable a scoped `missing_docs` warning or denial for public trace APIs after the existing public surface is documented. Do not enable a workspace-wide lint or generate meaningless comments to silence it.
- Add one-line docstrings to every Python demo function and class, remove `from __future__ import annotations`, and keep the demo compatible with the nearest declared Python 3 minimum.
- Document bounds, conflict preservation, truncation, bundle path resolution, offline/read-only guarantees, supported command shape, and the distinction between semantic and display payload reads.
- Use explicit public crate exports and private modules by default.
- Correct opaque positional literals with exact `/*param_name*/` comments where the repository's argument-comment convention requires them.
- Do not create single-use helpers solely to reduce function length. Prefer cohesive modules that own their tests and invariants.

Exit evidence:

- the coverage ledger and documentation inventory contain no unexplained production callable or type;
- Rust public-doc linting, Python syntax checks, and `just argument-comment-lint` pass;
- module line counts and any justified exception are recorded;
- generated API documentation contains no accidental private paths or broad re-exports.

### Movement 6: Make Focused CI Match The Claims

Dependencies: Movements 1 through 5.

- Add `codex-rollout-trace` to the focused Linux, macOS, and Windows workflow.
- Invoke the 100,000-node profile in a dedicated, non-routine job or add a deterministic structural large-trace test to routine CI and retain timing in a dedicated performance job. An ignored test that no configured job invokes is not acceptance evidence.
- Run the CLI compatibility tests and pseudo-terminal offline/read-only test on every supported platform where terminal facilities permit it; record an explicit platform-specific substitute when they do not.
- Verify bundle containment and path behavior on Linux, macOS, and Windows.
- Preserve exact commands, toolchain, host metadata, synthetic fixture fingerprints, and the candidate commit for all performance and release evidence.
- If a Rust dependency changes, run `just bazel-lock-update` from the repository root and include `MODULE.bazel.lock` in the same change.
- Update the existing compatibility matrix and release-candidate notes only after the configured workflow passes on the exact candidate commit.

Exit evidence:

- focused CI names every required crate and non-default profile explicitly;
- Linux, macOS, and Windows results are attached to the exact candidate commit;
- performance claims link to an invoked job and a reproducible synthetic input;
- no workflow, report, or fixture contains a home-directory path, credential, private rollout content, or unredacted terminal output.

## Acceptance Evidence

Another operator must be able to establish all of the following:

### Correctness And Bounds

- Ordinary, rich, merged, malformed, duplicate, and conflict fixtures never admit more records or nodes than the documented caps.
- Reaching a cap cannot create an orphaned terminal/session node or a parent link to a node that was not admitted.
- Repeated observations remain independently inspectable with provenance and a deterministic conflict diagnostic; no last-write-wins evidence loss remains.
- Custom manifest event-log paths work, while absolute and escaping paths are rejected consistently on every supported platform.
- Event, semantic-payload, display-payload, search, field, result, and viewport limits have exact-boundary and limit-plus-one tests.
- An oversized or malformed record does not suppress a following valid record.

### Responsiveness And Lifecycle

- Input handling, warm navigation, picker rendering, and viewport rendering have structural tests proving they do not scan or format the complete catalog or trace.
- Search and other interruptible expensive work run outside the frame/input path, use generation-tagged results, and are safely cancellable.
- Lifecycle tests cover success, ambiguity, replacement, cancellation, stale completion, worker failure, payload failure, and shutdown.
- The 100,000-node profile is invoked by configured CI, and its input, environment, measurements, and structural counters are recorded.

### Tests And Presentation

- The coverage ledger accounts for every fork-owned production function, method, and type in scope without manufacturing tests for static values.
- Rich projection, merged evidence, browser interactions, payload windows, and failure states use deep semantic assertions.
- User-visible states have reviewed snapshots at narrow and wide widths, with Unicode and terminal-control coverage and no pending snapshots.
- A pseudo-terminal test exercises the supported CLI surface offline and confirms fixture bytes are identical before and after browsing.

### Documentation, Compatibility, And Hygiene

- Every in-scope Rust production function and type has at least a one-line documentation comment or a recorded trait-contract inheritance, and every Python demo function and class has a docstring.
- Public Rust docs pass the scoped missing-docs gate; broad wildcard exports and the disallowed Python `__future__` import are absent.
- The CLI `trace` token has an explicit, tested compatibility disposition.
- Documentation accurately distinguishes semantic replay reads from lazy, windowed display reads and states every user-visible truncation limit.
- Production modules are below the repository's roughly 800-line review threshold, with a target below 500 lines, or carry a specific recorded justification.

### Validation Sequence

Run focused tests before final fix/format commands:

1. `just test -p codex-rollout-trace`
2. `just test -p codex-trace`
3. `just test -p codex-trace-tui`
4. `just test -p codex-cli --bin codex trace_command`
5. the explicit 100,000-node profile and pseudo-terminal offline/read-only test
6. `cargo insta pending-snapshots -p codex-trace-tui`, followed by direct review and intentional acceptance of any new snapshots
7. `just argument-comment-lint`
8. scoped `just fix -p ...` commands for every changed crate
9. `just fmt`

Do not run `cargo test` directly. Do not re-run tests after the final `fix` and `fmt` commands. If changes unexpectedly reach `common`, `core`, or `protocol`, stop and obtain approval before the complete workspace test suite, as required by the repository instructions.

## Closure

Close this plan only after recording:

- the finding-to-change mapping and final coverage ledger;
- the CLI compatibility decision;
- exact limit values and their boundary-test locations;
- the final module and diff sizes for each independently landed movement;
- exact local validation commands and results;
- reviewed snapshot paths;
- Linux, macOS, and Windows workflow links for the exact candidate commit;
- deterministic fixture fingerprints, performance job metadata, and results;
- any rejected approach, residual gap, or successor plan.

The closeout must also update the residual gates in `.ariadne/plans/open/2026-07-28-credible-historical-trace-browser.md`. Do not close this plan merely because implementation exists locally, a timing target passes once, or a release note asserts readiness. No push, tag, release, repository-setting change, or external publication is authorized by this plan. 