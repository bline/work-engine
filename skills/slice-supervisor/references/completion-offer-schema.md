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
