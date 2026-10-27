# Completion-offer lifecycle artifact

Schema version 1 is the supervisor-owned live state for one optional completion
offer. It is distinct from the immutable terminal audit receipt and is stored by
the current implementation as a JSON blob named by
`refs/work-engine/completion-offers/<run>/slice-<number>`. The private ref is a
current storage affordance, not required product doctrine.

The artifact binds `offer_id` to the complete validated completion request.
`open` has no result, reason, or predecessor. It transitions exactly once to
`created`, `declined`, or `refused` with an adapter-authored result, or to
supervisor-owned `expired_unavailable` with a truthful reason. A terminal
artifact names the prior open blob OID, and ref compare-and-swap prevents two
terminal decisions from becoming authoritative.

The ref and blob do not touch the user's branch, real index, or ordinary
worktree. Resume may expose the artifact additively; it never replaces the
accepted checkpoint as continuation authority. If publication may have occurred
before the live artifact was finalized, recovery uses the completion adapter's
read-only Git reconciliation. It never retries publication.

## Exact preservation request/result binding

Offer schema2 can carry the explicitly approved preservation request defined by
the [completion adapter](../../slice-completion-commit/references/receipt-schema.md).
Offer identity continues to hash the complete request using sorted compact JSON,
ensure_ascii=False, UTF-8. For any receipt2, shared offer validation compares
canonical bytes and recomputed digests of the offer's approved request with
the receipt's complete bound request, and checks the expected digest against
offer_id. A valid receipt for request B is not authority to attach it to offer A,
even if both name the same proposal/checkpoint/commit. This check belongs in
shared validate because transition-only checks would leave persisted reads and
reconciliation exposed.

Transitions validate before offer CAS; read_oid/load validate before returning
state; active and terminal reconciliation traverse the same join. Refusing an
A/B mismatch does not undo or deny B's historical Git effect. New preservation
requests require receipt2 binding even for declined/refused results and cannot
use offer schema1. Actual historical strict receipt1/offer1 remains readable
without fabricated bindings or inferred human decisions.

Read-only recovery uses the adapter's immutable prepared attempt and exact
commit, including later descendants, never reruns create or repairs the index.
Return created-with-unestablished-postconditions truthfully for interruption;
a later clean worktree cannot recreate missing historical evidence. Direct
authorized adapter use is supported here; the legacy Node publication request
validator still rejects the additional preservation block.
