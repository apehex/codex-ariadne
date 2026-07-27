# Agent workflow doctrine

Substantial work uses two logical responsibilities even when one agent performs both.

The controller owns:

- the desired observable state and acceptance evidence;
- constraints, scope, sequencing, and plan authority;
- selection of the next gap-directed movement;
- task contracts and any delegation decisions;
- inspection, reconciliation, and final integration of results.

The executor owns:

- the bounded objective it accepted;
- the paths and operations placed in scope;
- implementation and proportionate validation;
- prompt reporting of discovered constraints or ownership conflicts;
- a result that separates changes, validation, limitations, and residual gaps.

A substantial cycle should be expressible as:

```text
objective → observed state → selected gap → bounded contract → result → integration receipt → reassessment
```

The representation may remain implicit for a trivial local edit. Materialize it in an open plan when work spans sessions or milestones, requires consequential judgment, or must be handed between agents.

The controller does not accept a result merely because execution completed. It inspects the diff and evidence, reconciles overlap and contradictions, runs integration-level validation where needed, and records what is actually complete.
