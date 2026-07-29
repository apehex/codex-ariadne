# Reserve `codex trace` for historical inspection

Updated: 2026-07-29

Status: accepted

## Decision

Ariadne accepts `codex trace` as a top-level historical, offline, read-only browser command and therefore accepts that an unescaped first prompt token named `trace` no longer starts the parent interactive TUI.

The literal prompt remains available through the standard option terminator as `codex -- trace`; parser coverage must preserve that escape hatch.

The future live `/trace` command should reuse the dedicated trace crates without changing this historical-command decision.

## Rationale

The top-level command is the existing public Ariadne interface and keeps all fork behavior behind one explicit token. Preserving it avoids a second temporary command name before `/trace` exists, while the standard `--` spelling retains access to the formerly ambiguous prompt.
