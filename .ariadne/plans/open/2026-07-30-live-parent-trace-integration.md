# Live Parent Trace Integration

Updated: 2026-07-31

Status: open

## Objective

Produce an opt-in parent-compatible integration in which `/trace` and the configured transcript shortcut open the same Ariadne surface on the current thread, including bounded live updates and reversible return to the Codex composer.

## Context

This is Phase 7 and depends on the demonstrated semantic parity from [Phase 6](2026-07-30-codex-transcript-parity.md). Its live adapter and parent boundary must conform to the accepted [presentation-index contract](../../decisions/2026-07-31-trace-presentation-index-contract.md).

The goal is a locally working and reviewable compatibility candidate. Upstream acceptance, default enablement, and product replacement are external outcomes and are not closure requirements.

## Scope

Owned work includes the smallest parent-TUI command, overlay, key-routing, current-thread selection, live-event adapter, feature-gating, and tests needed to launch the Ariadne browser.

Keep general grouping, navigation, and rendering logic in Ariadne-owned crates. Changes to `codex-core`, model-visible context, persisted formats, recorder behavior, app-server APIs, authentication, telemetry, or unrelated parent-TUI surfaces are excluded unless a separately accepted observability plan demonstrates necessity.

## Movements

### Movement 1: Add A Narrow Launch Contract

- Add `/trace` as a read-only TUI control action that does not enter conversation history or model context.
- Route the configurable transcript shortcut, initially `Ctrl+T`, to the same launch contract under an explicit feature or fork boundary.
- Open at the current thread, grouped conversation lens, and latest group while retaining access to the session and child threads.

### Movement 2: Bridge Current And Historical State

- Load persisted ordinary and available rich evidence outside the input loop.
- Feed bounded live presentation facts into the same incremental reducer used by offline grouping.
- Mark live-only items and preserve partial groups when persisted evidence is not yet available.
- Preserve stable selection by group or trace locator across updates and avoid forced bottom-follow when the user has moved away.

### Movement 3: Preserve Parent Terminal Semantics

- Open through the parent alternate-screen or overlay lifecycle without modifying terminal-owned scrollback.
- Restore composer text, focus, viewport, raw-mode state, and terminal state on exit, error, cancellation, resize, and panic-safe cleanup paths.
- Keep the legacy transcript available as an explicit fallback during evaluation.

### Movement 4: Demonstrate Compatibility

- Add snapshots and pseudo-terminal tests for completed and running turns, live tools, agent activity, resize, error, cancellation, and return.
- Record the exact upstream-owned diff and keep every integration point independently reviewable.
- Provide a local demonstration route without pushing, submitting, or claiming upstream adoption.

## Acceptance Evidence

- `/trace` and the transcript shortcut launch the same Ariadne browser on the current thread.
- Navigation actions never become model input or persisted conversation messages.
- Live and completed synthetic traces converge to the same presentation index except for explicitly labeled transient facts.
- Terminal scrollback remains unchanged behind the overlay and all exit paths restore the parent surface.
- The old transcript remains selectable as a fallback.
- Parent-TUI snapshots, focused tests, offline tests, and pseudo-terminal restoration tests pass on supported local platforms.

## Closure

Close this plan when the integration works locally, the parent compatibility boundary and exact diff are recorded, the fallback is intact, and residual upstream-version risks are transferred to Phase 8.

Do not keep this plan open waiting for upstream interest, review, merge, release, or default enablement.
