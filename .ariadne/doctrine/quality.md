# Quality doctrine

Quality is the degree to which work moves Ariadne toward an observable desired state without obscuring evidence, creating avoidable coupling, or spending more complexity than the result warrants.

Use these lenses:

- `scoped`: the change owns a clear boundary and does not absorb unrelated cleanup;
- `grounded`: claims and decisions point to repository state, tests, measurements, or explicit assumptions;
- `skeptical`: missing evidence, conflicting observations, and plausible failure modes remain visible;
- `observable`: acceptance can be checked by another operator without relying on the author's memory;
- `scaled`: process, abstraction, and validation cost match the risk and expected lifetime;
- `orthogonal`: identity, architecture, module internals, plans, and history each have one owner;
- `efficient`: the solution reduces repeated work and avoids eager or frame-dependent cost;
- `iterative`: each bounded movement can be reviewed, validated, and revised;
- `recursive`: friction discovered while working is routed to the artifact that can prevent recurrence.

Prefer the smallest coherent change that closes the selected gap. A smaller diff is not automatically better if it leaves duplicated authority, hidden performance cost, or unverifiable behavior.

For reviews, state the desired behavior, inspect the current artifact, name concrete gaps by consequence, distinguish required corrections from optional improvements, and validate the corrected result at its public boundary.
