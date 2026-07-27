# Delegation doctrine

Delegate when context isolation, independent perspective, specialization, or useful concurrency is worth the startup and integration cost. Do not delegate merely to imitate a hierarchy.

Every delegated task is an expiring contract containing:

- objective and acceptance evidence;
- owned paths or conceptual boundary;
- authoritative context and assumptions;
- forbidden or out-of-scope actions;
- required validation;
- expected return shape;
- stop, escalation, and expiration conditions.

Executors must not silently expand their contract, transfer approval authority, or overwrite overlapping work. When new evidence changes the task materially, return the evidence and proposed movement to the controller.

The controller owns fan-out and fan-in. Before parallel work, assign non-overlapping ownership or an explicit reconciliation strategy. After results return, inspect them independently, resolve conflicts against current repository state, validate the integrated behavior, and record residual gaps.

Nested delegation preserves the same contract structure. Each controller remains responsible for integrating its direct children before returning a result upward.
