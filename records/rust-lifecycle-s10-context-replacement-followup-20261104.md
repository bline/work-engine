# S10 context replacement compatibility follow-up — 2026-11-04

Status: open compatibility requirement; recommendation to defer implementation
until the lifecycle integration boundary. This note creates no execution
assignment, budget, selected replacement mechanism or release acceptance.

## Recommendation and revisit trigger

Continue integration of the accepted bounded Pi/Docker coding path. Its
qualification does not depend on automatic compaction or semantic continuation
after context replacement. A lifecycle redesign is premature without a failed
compatibility test identifying which contract cannot be preserved.

Revisit this requirement before admitting Pi jobs that depend on Rust-governed
context replacement and checkpoint rehydration, or claiming that part of the
working base, S10/M6 or release. Existing lifecycle requirements remain in force;
this is a dependency with an explicit consumption boundary, rather than a loose
future improvement. The current profile disables automatic compaction, and its
ordinary finite limits and collection remain the supported behavior.

## What is known

- The [real coding qualification](rust-lifecycle-s10-pi-docker-real-coding-result-20261104.md)
  passed with Pi's direct `openai-codex` provider. It did not run Codex CLI or
  exercise context replacement.
- The [existing Codex capability profile](../app-server/src/capabilities.mjs)
  binds `model_context_replacement` to target-model `new_context` with the
  `token_budget` feature. Its historical qualification is not proof that another
  harness or runtime version supplies equivalent behavior.
- Pi's pinned 1.5.0 SDK exposes `AgentSessionRuntime.newSession()`, which replaces
  the active session. Its documentation requires event subscriptions to be
  rebound. This supplies a candidate mechanism, not established compatibility
  with Work Engine's identity, authority or recovery contract.
- Pi's standard summary compaction retains conversational material. Enabling it
  alone does not establish the hard-replacement and durable-recovery behavior
  discussed here.

## Smallest useful future qualification

A bounded compatibility probe can use the recorded Codex harness behavior as a
reference and evaluate Pi's fresh-session operation against the same checkpoint
and recovery consequences. Provider-disabled checks can establish the local
session mechanics before minimal inference confirms the actual request path.

The decisive evidence covers exclusion of the old conversation from subsequent
model input, recovery from the accepted checkpoint, preserved authority and
unanswered human interaction, cancellation, queued inputs, uncertain effects,
session/context lineage, and restored observation and collection subscriptions.
A different physical session ID requires explicit compatibility with the
owning lifecycle contract, not an assumed identity substitution.

Passing evidence would support adapting the session replacement mechanism.
Failure would identify whether a Codex-harness lane or a specific contract change
is needed. Neither outcome justifies automatically restarting the broader S10
investigation or redesigning the whole lifecycle.

The [convergence amendment](../planning/s10-pi-docker-convergence-amendment-20261103.md#one-pressure-path-and-one-continuity-owner)
already reserves this qualification and rejects an independent Pi reset loop
that bypasses lifecycle ownership. The prior coding attempt remains closed.

Separate progress: board 4358 reports accepted real Dot → OAuth MCP → Docker/Pi
observation, superseding the earlier partial observation attempt for that exact
claim. It used a synthetic pressure fixture with providers disabled; it does not
close this context-replacement requirement or prove the combined coding workflow.
