# Automatic continuation requirement — 2026-10-28

The user clarified the required outcome:

> ok, because we need automatic resumption. the same work continues from the previous session

For outer development sessions, automatic resumption is part of the intended continuation behavior. The existing objective and authorization carry through context replacement. Recovery and continuation must not depend on another user message or a new approval for already authorized work.

Recovery is part of automatic continuation. The agent recovers durable meaning, reconciles current human direction and any uncertain operation, restores usable collection and observation, and continues the existing assignment. A pause, cancellation or pending approval retains its actual meaning. Technical readiness checks do not create a separate human approval requirement or an unspecified controller permission gate.

This clarification supersedes the supervised-only recommendation as the final objective of the [failure-study assessment](../metrics/probes/outer-context-failure-20261028.md). That dated study remains unchanged: its evidence gaps and prototype failures identify implementation and qualification work. They do not remove the automatic-continuation requirement or establish that automation should be prohibited.

A successful implementation must demonstrate continuation to actual authorized work without human intervention across the context boundary, with durable obligations, settings and collection ownership preserved. Uncertain requests must be reconciled rather than blindly replayed, duplicate wakeups must not duplicate effects, and failed delivery or recovery must have an observable outcome and owner. Ordinary continuation needs no renewed human grant. Ownership transfer retains its separately applicable authority and identity-binding requirements.

Scope remains the outer development skills and their host/chatboard/wait integration. This decision does not change the separate Rust runtime workstream.
