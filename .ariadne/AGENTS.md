# Ariadne agent instructions

These instructions govern Ariadne-specific work after they are loaded by an Ariadne-owned module or explicitly by the operator. They supplement the repository-root upstream `AGENTS.md`; they do not replace or relax it.

## Precedence

Apply system and user instructions first, then the repository-root and otherwise applicable upstream instructions, then this file, then the applicable Ariadne module `AGENTS.md`. An open Ariadne plan authorizes its bounded project work; supporting design and doctrine documents explain how to perform it.

If an Ariadne rule conflicts with an upstream invariant, preserve the upstream invariant and record the unresolved fork requirement in a draft plan.

## Required route

Before substantial work, read in order:

1. the repository-root `AGENTS.md`;
2. [`.ariadne/README.md`](README.md);
3. the relevant plan under `plans/open/`, if one exists, using [`plans/README.md`](plans/README.md) as the route;
4. [`.ariadne/DESIGN.md`](DESIGN.md) when the work crosses modules or changes an invariant;
5. the relevant module `AGENTS.md`, `README.md`, and `DESIGN.md`;
6. only the doctrine documents needed for the operation.

The local module file may narrow this route and add module-specific tests, but it should not restate the fork-wide doctrine.

## Operating method

For substantial work, define the desired observable state, inspect current evidence, identify the highest-value gap, select a bounded movement, implement it, validate it, and reassess. Keep this loop implicit for trivial edits and materialize it in an open plan when work spans milestones, has consequential choices, or needs durable handoff.

The controller owns objectives, constraints, sequencing, acceptance evidence, and integration. An executor owns only the bounded contract it accepts. Delegation is useful for context isolation, independent review, specialization, or real concurrency; it is not required when startup and integration cost exceed the benefit.

Every delegated contract must state the objective, owned paths, relevant context, constraints, required validation, return shape, and stop conditions. The controller must inspect and reconcile returned work before treating it as integrated.

## Plan authority

Use [`.ariadne/plans/README.md`](plans/README.md) for plan lifecycle and format. `plans/draft/` contains proposals with no execution authority. `plans/open/` contains accepted executable work. `plans/closed/` preserves results, validation, residual gaps, and history.

Record durable project friction or a substantial proposed change in a draft plan. Record a module-local design gap in the owning module design when no project decision is yet required.

## Upstream discipline

Keep fork changes localized and reviewable. Prefer dedicated Ariadne crates and the smallest existing upstream interfaces. Do not modify `codex-core`, persisted formats, recorder behavior, or app-server protocols merely for convenience; require a demonstrated observability gap and an explicit open plan.

Keep the repository-root `AGENTS.md` byte-identical to upstream. Avoid opportunistic cleanup of upstream files. An upstream candidate must be separable from fork identity and project machinery.

## Trace and data invariants

The historical browser is offline and read-only. It must not require authentication, a model client, app-server connectivity, or network access, and it must not mutate rollouts, bundles, payloads, state databases, archive markers, or timestamps intentionally.

Preserve provenance and uncertainty. Never present reconstructed data as exact, infer plaintext for unavailable encrypted content, or silently choose one side of contradictory sources.

Commit deterministic synthetic fixtures only. Never commit real traces or fixtures produced by redacting them. Treat raw trace content as sensitive, load it on demand, contain referenced paths to their bundle, and render content as inert terminal-safe text.

## Documentation and validation

Write Markdown as unfolded semantic lines: one paragraph, list item, table row, or code line per source line. Update the owning design document when an invariant, boundary, public interface, source-precedence rule, or node model changes.

Validate in proportion to risk. Documentation-only work requires link, reference, privacy-string, formatting, and diff checks. Code work requires the smallest relevant unit, integration, snapshot, offline, read-only, and cross-platform checks identified by the owning module.

Preserve unrelated working-tree changes. Do not commit, push, submit upstream material, or alter external state unless the operator explicitly requests it.
