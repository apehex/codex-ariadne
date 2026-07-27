# Ariadne plans

Plans provide durable authority and acceptance evidence for substantial project work. Their directory is their execution state.

## States

- `draft/`: proposed or exploratory work with no execution authority;
- `open/`: accepted work that agents may execute within the stated scope;
- `closed/`: completed, rejected, or superseded work preserved with its outcome.

Create a state directory only when it contains a tracked plan. Move the same file between states so its identity and history remain stable.

The roadmap is not a plan state and does not authorize execution. Promote a roadmap direction into a draft, resolve its decisions, and move it to `open/` before implementation.

## Required plan content

```markdown
# Title

Updated: YYYY-MM-DD

Status: draft | open | closed

## Objective

Describe the observable desired state.

## Context

Record current evidence, constraints, dependencies, and relevant decisions.

## Scope

State owned paths and explicit exclusions.

## Movements

List bounded implementation stages and their dependencies.

## Acceptance evidence

Define checks another operator can use to determine completion.

## Closure

When closing, record implemented results, validation, rejected or deferred work, residual gaps, and successor routes.
```

Plans should link to canonical architecture and doctrine rather than duplicate them. Keep decisions that must outlive one plan under `decisions/`, and keep non-executable successor directions in the roadmap.
