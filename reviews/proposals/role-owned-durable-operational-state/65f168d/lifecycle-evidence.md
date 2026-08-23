# Lifecycle, Recovery, and Evidence Review

Role: read-only lifecycle, recovery, and evidence specialist. The initial pass
used a fresh role-scoped context; the `6460190` reassessment retained that
context and does not claim renewed independence.

Readiness consequence: `revise_before_review_closure`.

## Findings

1. **High — the history mechanism is made binding without supporting
   evidence.** Preserve reconstructability, idempotency, explanation, crash
   safety, and integrity as requirements; keep the exact history mechanism open
   until a recovery vertical discriminates among alternatives.

2. **High — a retained chain lacks an atomicity and reachability contract.** If
   a chain remains a candidate, specify either one compare-and-swap envelope or
   a prepare/commit protocol, immutable roots, crash recovery, reachability,
   retention, and tombstones. Current opaque durable state publishes one value
   under one ref and does not supply those semantics automatically.

3. **High — replacement correctness depends on deferred identity and
   visibility work.** An early vertical needs a generation/fence or proven
   quiescence rule, role read authorization, and integrity/freshness checks for
   referenced evidence.

4. **High — projections risk copying canonical facts.** Accepted boundaries,
   builder progress, findings, and dispositions have different owners. Require
   a field ownership/reference matrix, source revision, and fail-stale behavior.

5. **Medium — event identity and cleanup are incomplete.** Distinguish source
   event, delivery attempt, state transition, and protected consequence. Define
   causal identity, out-of-order and late delivery, uncertain completion,
   terminal suppression, retention, and cleanup.

6. **Medium — the proof can pass without resumed semantic work.** A recovery
   demonstration must perform the next role-owned action from durable state
   without a transcript, refresh referenced truth, enforce fencing, survive
   retries/crash injection, and produce a downstream artifact or consequence.

## Architecture correction consequence

The correction strengthens findings 1 and 3 by making future control-plane
structure explicitly exploratory. It narrows the architecture-evidence portion
of finding 4 but does not close the proposal's ownership defect. Other findings
are unaffected.
