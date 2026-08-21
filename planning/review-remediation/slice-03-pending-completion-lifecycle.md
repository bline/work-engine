# Slice 3 — Give Pending Completion Interaction a Coherent Lifecycle

## Protected consequence

A delayed user decision about an optional branch commit remains visible and
recoverable without rewriting the immutable terminal audit receipt, fabricating
a terminal decision, blocking accepted-checkpoint continuation, or making later
safe commit creation impossible.

## Observed defect

`pending`, `declined`, `created`, and `refused` are represented as values of one
completion receipt embedded in the unique terminal slice receipt. Once pending
is appended, the terminal identity cannot be appended again with the eventual
decision. Persisting an in-repository terminal receipt may also change the
worktree after the accepted checkpoint, causing later exact-tree preflight to
refuse.

## Ownership question to resolve

Pending interaction is live mutable state; the terminal audit receipt is
immutable historical evidence. Planning must identify the owner and lifetime of
the optional completion offer without forcing unfinished state into the
terminal receipt. The owner may be existing or new, but duplicate canonical
copies are not acceptable.

## Acceptance consequences

- Pending, create, decline, expiration/unavailability, and safe refusal have a
  coherent transition model with one authoritative owner.
- The immutable terminal receipt remains unique and truthful.
- A pending decision survives context replacement when that persistence is part
  of the selected product behavior.
- Checkpoint-based continuation does not depend on resolving the optional
  branch commit.
- A later create decision rechecks current branch/index/worktree authority
  rather than relying on stale acceptance-time facts.
- Metrics or state writes do not create an unaccounted tree delta that can be
  silently committed or that makes the lifecycle internally impossible.
- Campaign behavior when no user responds is explicit and does not manufacture
  success, decline, or authorization.

## Required vertical proof

Accept a slice, persist a pending interaction, reconstruct the owning runtime
state, then resolve it once as create or decline while preserving the original
terminal receipt bytes and checkpoint continuation identity. Exercise an
in-repository metrics/state destination.

## Insufficient substitutes

- Allowing duplicate terminal records.
- Mutating the prior terminal receipt in place.
- Keeping pending only in model context.
- Calling pending non-blocking while providing no durable path to its eventual
  resolution.

## Open product decision

Whether prompts are enabled by default for interactive, unattended, all, or no
campaigns remains a user/product interaction decision. Do not silently resolve
that question through a universal default while implementing storage mechanics.
