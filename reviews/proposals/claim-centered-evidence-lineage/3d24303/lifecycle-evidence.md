# Lifecycle and Evidence Review

Role: fresh read-only lifecycle and evidence specialist. Diagnostic and advisory
only.

Readiness consequence: `revise_before_review_closure`.

## Findings

1. **High — stable claim identity and revision topology are underdefined.** A
   claim needs a namespace, bounded subject/question continuity rule, explicit
   identity-fork conditions, collision-safe revision/event identities,
   relation-specific cardinality and acyclicity, and a non-self-referential
   container identity.

2. **High — impact, refresh, reliance, and obligation lifecycles lack terminal
   semantics and cross-owner publication safety.** Give nominations, refresh
   episodes, reliance records, applicability judgments, and obligations stable
   identities and explicit outcomes. Cover unchanged, changed, inapplicable,
   insufficient, contested, deferred, and superseded results. Version and retire
   reliance non-destructively. Across owners, use idempotent outbox/reconciliation
   keyed to the source event and distinguish semantic completion from delivery
   acknowledgement.

3. **Medium — projection rebuildability is not yet provable.** Define canonical
   edge/event inputs, schema versions, retention roots, retractions, deterministic
   ordering, duplicate/conflict behavior, source watermarks, migration, cycle and
   dangling-reference handling, and visibly stale or fail-closed reads.

4. **Medium — dogfood proofs cover only happy paths.** Add branching and
   concurrent revision, duplicate and out-of-order events, identity forks,
   unresolved refresh, candidate-warning retirement, empty rebuild, stale
   projection, dangling references, partial publication, delivery retries,
   authority loss, inactive recipients, and compaction around publication.

5. **Low — the prior defer decision does not approve the new semantics.** Keep
   explicit lineage: the old disposition remains in force, while this is a
   post-decision semantic amendment requiring its own revision-bound review or
   authority action before approval.

The compaction note truthfully exposes a gap but does not close it. Controlled
dogfood must prove recovery from durable owners and uncertainty when compaction
precedes semantic publication.
