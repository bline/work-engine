You are checking a chain of documents for requirement provenance. You have only the documents below; you have no tools and no other context.

The PARENT documents are the sources of requirements. The DERIVED documents came later, in the order given. For each derived document, in order:
1. List every requirement it presents as binding, quoting it.
2. For each, say whether the parent documents state that requirement, quoting the parent text that does. Separate what the parent text plainly states from readings of ambiguous parent text.
3. Flag any requirement that goes beyond what the parents state, any requirement supported only by a general citation rather than specific parent text, and any requirement presented as unchanged that the parents do not actually contain.
4. Do not flag a choice the document presents as its author's own revisable way of meeting a requirement, and do not flag a requirement the parents plainly state.

Do not propose weakening any requirement the parents plainly state. If a requirement seems to need an owner's decision, say whose, using only what the documents say about ownership.

Quote the exact text for every point. End with a section headed FLAGS, grouped by derived document id, listing each flag as: quoted span, what it adds or claims, and the parent text it lacks. If a derived document has nothing to flag, write "none" under its id.

=== PARENT DOCUMENT NC-B-CONTRACT ===
# Held-start contract with explicit removal attribution (synthetic fixture NC-B)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract. Derived from the shape of a held-start contract, with one owner decision added.

## Evidence owner and usability

The Host owns the child, same-session framed handshake, and exact stop.

## Terminal cleanup

The explicitly accepted cancel for the owned, still-held session concludes only after bounded receiver exit collection and exact container removal observed against the owned identity.

**Owner decision (receipt-semantics owner, recorded in this contract):** Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal. Removal by any other actor does not satisfy clean cancel, even if the container is afterwards absent. An independent post-exit check that the original container is absent is also required.
=== END NC-B-CONTRACT ===

=== DERIVED DOCUMENT NC-B-REVIEW ===
# Review of the held-start implementation (synthetic fixture NC-B, derived document)

Status: constructed negative control for P7 run 1.

## Finding R1 (blocking)

The contract's terminal cleanup section, in its recorded owner decision, requires that "the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal". The implementation reports clean cancel whenever the post-exit check finds the container absent, even when the Host's own removal command was never started. That violates the owner decision. Required: report clean cancel only when the Host's own normal removal succeeded with an attributable result, and the independent post-exit absence check also passed.
=== END NC-B-REVIEW ===
