# Structure-Derived Conversation Fixture

Updated: 2026-07-29

Status: open

## Objective

Create a realistic deterministic trace fixture derived from the topology of this Ariadne development conversation without committing its raw trace, visible transcript, free-form content, sanitization map, or intermediate redacted artifacts.

The committed fixture may preserve explicitly approved structural facts from this conversation: event kinds, ordering, parent-child relationships, thread and turn boundaries, delegation edges, message roles and channels, tool-call/result pairing, lifecycle states, and bounded payload-size classes.

Every free-form or machine-specific value must be generated anew from public deterministic templates through an allowlist-only transformation. Unknown source fields or event variants must fail closed rather than pass through.

## Authorization And Privacy Decision

The user explicitly considers disclosure of this conversation's topology acceptable because the conversation concerns a public fork of a public tool.

This approval covers topology only. It does not authorize publication of verbatim or lightly redacted message text, reasoning, tool arguments or results, commands, paths, environment details, identifiers, timestamps, logs, raw payloads, encrypted content, URLs, credentials, or source-derived hashes.

Do not inspect unrelated conversations or select a source automatically. Implementation must require an explicit operator-provided bundle or session locator and must confirm the selected root before transformation.

Only the final generated fixture, its public generator or transformation code, its manifest, and aggregate validation evidence may enter the repository. Raw inputs, temporary redactions, identifier maps, salts, substring indexes, and leakage-scan corpora remain local and uncommitted.

This plan introduces a narrow permitted category for the quality-remediation work: a structure-derived synthetic fixture is not treated as a raw or redacted trace when all non-allowlisted values are regenerated, source intermediates are excluded, and the acceptance checks below pass.

## Relationship To Existing Plans

This plan supports Movement 4 of `.ariadne/plans/open/2026-07-28-trace-browser-quality-remediation.md` by providing a complex public fixture for rich projection, navigation, lifecycle, presentation, search, and offline/read-only tests.

It does not replace the deterministic hand-authored demo, exact-boundary unit fixtures, malformed-input fixtures, or platform containment fixtures required by the quality-remediation plan.

It does not expand authority to implement live `/trace`, change recording or protocol formats, publish a release, push commits, or ingest other real conversations.

The existing quality-remediation plan must be amended during implementation to record this fixture category and its completed safety evidence before the fixture is used as closure evidence.

## Target Artifact

Produce one deterministic synthetic bundle compatible with the ordinary `codex trace --bundle` and authoritative reducer paths.

Store the fixture where both Cargo and Bazel tests can consume it without runtime dependence on the private source. If compile-time Rust file access is introduced, update the owning `BUILD.bazel` data declarations in the same movement.

Attach a public manifest describing the transformation version, retained structural categories, replacement-template version, event and payload counts, bounded size classes, and deterministic fixture fingerprint.

Do not claim that the fixture reproduces the conversation's semantics. Describe it as a synthetic semantic narrative populated onto an approved structural skeleton.

## Transformation Contract

### Retained Allowlist

- Event variant or normalized node kind.
- Stable relative order within the selected trace.
- Parent-child and cross-thread relationship types.
- Thread, turn, message, inference, tool, compaction, diagnostic, and terminal-state boundaries.
- Role and channel enums.
- Tool names only when they are public Codex tool identifiers; unknown or extension-specific names become deterministic generic tool names.
- Success, failure, cancellation, interruption, and completion enums.
- Relative timing buckets such as immediate, short, or long; never exact source timestamps or durations.
- Payload-size buckets such as empty, small, medium, or large; never exact source byte or token counts.
- Presence of Markdown, JSON, code, multiline text, Unicode, and inert terminal-control test cases.

### Regenerated Values

- Replace all identifiers with first-seen deterministic sequential identifiers scoped by kind.
- Replace paths with fixed synthetic fixture paths that contain no username, home directory, repository checkout, or original basename.
- Replace timestamps with a fixed public epoch plus deterministic synthetic increments.
- Replace user, assistant, system, developer, commentary, final-answer, and reasoning text with independently authored public templates selected by structural role and size bucket.
- Replace tool arguments and results with schema-aware public templates; shell commands, patches, file contents, environment values, URLs, and errors must never be transformed from their source text.
- Replace Markdown, JSON, and code bodies with independently authored public examples that exercise the same renderer category without retaining source tokens.
- Replace raw artifacts, encrypted blobs, base64 data, and opaque payloads with newly generated fixtures or explicit synthetic omission records.
- Replace unknown enum strings, extension fields, and diagnostic text with bounded generic values.

### Fail-Closed Rules

- Reject any source event kind or field that is neither explicitly retained nor explicitly regenerated.
- Reject generated output containing an absolute source path, source identifier, URL, email address, credential pattern, unexpected high-entropy token, or a source-derived hash.
- Reject any non-empty free-form output field that was not written by a public replacement template.
- Reject broken references, missing parents, duplicate generated identifiers, non-deterministic ordering, or output outside declared resource bounds.
- Never fall back to copying a source value when a replacement rule is missing.

## Movements

Each movement is an independently reviewable change. Keep complex logic below 500 changed lines and every non-mechanical movement below 800 changed lines.

### Movement 0: Freeze The Fixture Specification

- Enumerate the source event and normalized node variants that may contribute topology.
- Define the retained-field allowlist, regenerated-field registry, fail-closed behavior, relative timing buckets, size buckets, and public template categories.
- Define the fixture location, Cargo/Bazel access pattern, manifest schema, deterministic fingerprint algorithm, and maximum event, node, payload, and output sizes.
- Record which facts about this conversation are approved topology and which remain prohibited content.
- Add a source-selection contract requiring an explicit operator-provided bundle or session locator and visible confirmation of its root identifier.

Exit evidence:

- A reviewed specification maps every supported source field to retain, regenerate, or reject.
- No implementation depends on heuristic redaction or an undefined fallback.

### Movement 1: Extract A Structural Skeleton

- Add a local transformation entry point that reads one explicitly selected trace through existing bounded, contained trace readers.
- Convert the selected trace into a minimal in-memory skeleton containing only allowlisted enums, relationship references, ordering, and buckets.
- Assign generated identifiers by stable first-seen order without hashes or persistent salts.
- Keep the skeleton bounded and terminal-safe, and do not serialize it as a repository artifact.
- Test unknown variants, unknown fields, limit exhaustion, missing parents, cycles, duplicate identities, and deterministic repeated extraction.

Exit evidence:

- Tests prove that the skeleton type cannot represent free-form source text, paths, timestamps, raw bytes, or opaque payload values.
- Instrumentation or structural assertions prove that source content does not reach the generation boundary.

### Movement 2: Populate A Public Synthetic Narrative

- Build a small independently authored template library for roles, channels, Markdown, JSON, code, tool calls and results, delegation, compaction, diagnostics, failures, interruption, and final answers.
- Populate templates according to structural kind and bounded size bucket without token reuse from the source.
- Emit a valid deterministic trace bundle with contained relative payload paths, fixed synthetic timestamps, sequential identifiers, and no external dependencies.
- Keep all output compatible with the authoritative reducer and `codex trace --bundle`.
- Test determinism, referential integrity, reducer compatibility, declared resource bounds, and platform-neutral paths.

Exit evidence:

- Two transformations of the same approved topology produce byte-identical public bundles.
- The generated semantic narrative is independently readable but makes no claim to preserve source wording or meaning.

### Movement 3: Add Leakage And Provenance Gates

- Scan generated files against a locally held source-derived substring set without storing or logging the substrings.
- Reject meaningful source matches above a conservative threshold while ignoring only explicitly documented unavoidable syntax tokens.
- Scan for absolute paths, usernames, hostnames, emails, URLs, credential formats, secret-key patterns, unexpected control characters, and high-entropy opaque values.
- Verify that every generated free-form field carries template provenance in an internal validation report.
- Review the retained topology manifest for facts beyond the explicit authorization.
- Ensure failure output reports only generated paths, event indices, rule identifiers, and bounded categories rather than source values.

Exit evidence:

- Leakage tests intentionally seed prohibited values into every source category and prove none reach output.
- False-positive exceptions are narrow, documented, and contain no source excerpt.
- A human review confirms that the final fixture contains only approved topology and public generated content.

### Movement 4: Integrate The Fixture With Trace Tests

- Add focused reducer and trace-model tests over the committed fixture's turns, messages, tools, delegation, compaction, lifecycle, diagnostics, and relationships.
- Add trace-TUI state tests and reviewed snapshots for single-level navigation, role stripes, search, hidden-result reveal, Rendered/Text/Raw modes, multiline restoration, Unicode, terminal-control neutralization, truncation, failure, interruption, and narrow and wide layouts.
- Add an automated pseudo-terminal test that opens the fixture through the supported CLI, navigates, exits, and verifies byte-for-byte fixture preservation with network and credential access unavailable.
- Keep exact-boundary and malformed-input tests separate rather than mutating the realistic fixture into an adversarial catch-all.
- Update Bazel test data declarations when the fixture is accessed as a source-tree resource.

Exit evidence:

- The fixture provides deep behavioral assertions rather than existence-only tests.
- No pending `.snap.new` files remain after direct review.
- The pseudo-terminal test proves offline, read-only behavior over the committed fixture.

### Movement 5: Document And Run Focused CI

- Document the structure-derived synthetic methodology, topology authorization, non-semantic nature of the generated narrative, retained fields, discarded fields, limits, and known residual disclosure risk.
- Update the quality-remediation plan's coverage ledger, fixture inventory, and privacy boundary.
- Add fixture generation verification, leakage gates, reducer/model/TUI tests, and platform-appropriate pseudo-terminal coverage to the focused Linux, macOS, and Windows workflow.
- Record the exact candidate commit, fixture fingerprint, generator version, commands, toolchain, platform results, and any platform-specific substitutions.
- Keep source selection and transformation out of CI; CI operates only on the committed public synthetic bundle.

Exit evidence:

- Linux, macOS, and Windows results are attached to the exact candidate commit.
- Documentation never implies that the fixture is a sanitized transcript or an authentic semantic record.
- No workflow log contains raw input, local paths, leakage-scan substrings, or private transformation intermediates.

## Acceptance Evidence

- The committed fixture preserves only the explicitly approved topology categories and contains no source-derived free-form value.
- The transformation is allowlist-only, bounded, deterministic, fail-closed, and incapable of silently copying unknown values.
- Generated identifiers, paths, timestamps, text, commands, JSON, Markdown, code, errors, and raw artifacts are independent public values.
- Automated leakage tests cover every prohibited source category, and a human review approves the final fixture.
- The authoritative reducer and normal `codex trace --bundle` path accept the bundle.
- Deep model, TUI, snapshot, and pseudo-terminal tests use the fixture while exact-boundary and malformed-input coverage remains separate.
- Source fixture bytes remain unchanged during every offline browser run.
- CI transforms no private data and publishes no source-derived intermediate.
- The fixture fingerprint, transformation version, validation commands, and platform results are recorded for the exact candidate.

## Residual Risk

Even after content regeneration, topology can reveal the approximate number and order of turns, tool calls, delegations, failures, and compactions. This disclosure is explicitly accepted for this conversation but must not be generalized to another source.

Size and timing buckets can reveal coarse behavioral characteristics. Use broad declared buckets and omit a category when its disclosure adds no test value.

Leakage scanning reduces but cannot prove the absence of every semantic resemblance. The primary safety boundary is the skeleton type and allowlist-only generator, not pattern matching.

Template prose may accidentally resemble source prose by coincidence. Templates should be independently authored before inspecting source text where practical, and review should focus on distinctive multi-token overlap.

## Validation Sequence

Run focused tests before final fix and format:

1. Generator and fail-closed unit tests.
2. Leakage-seeding and determinism tests.
3. `just test -p codex-rollout-trace`.
4. `just test -p codex-trace`.
5. `just test -p codex-trace-tui`.
6. Focused CLI compatibility and automated pseudo-terminal tests.
7. Explicit 100,000-node profile to ensure the additional fixture does not regress structural performance.
8. `cargo insta pending-snapshots --manifest-path trace-tui/Cargo.toml`, followed by direct snapshot review and intentional acceptance.
9. `just argument-comment-lint`.
10. Scoped `just fix -p ...` for every changed Rust crate.
11. `just fmt`.

Do not run `cargo test` directly. Do not rerun tests after final `fix` and `fmt`. Ask before a complete workspace test suite if implementation reaches `common`, `core`, or `protocol`.

## Closure

Close only after recording the approved source selection, specification review, transformation diff, retained topology categories, regenerated-field inventory, limit values, leakage results, human fixture review, fixture fingerprint, exact tests and snapshots, pseudo-terminal byte-preservation result, module and diff sizes, candidate commit, and Linux/macOS/Windows workflow links.

Do not close merely because a generated bundle loads successfully or because a substring scanner reports no matches.

No source trace, visible transcript, mapping table, salt, temporary redaction, leakage corpus, push, tag, release, repository-setting change, or external publication is authorized by this plan.
