You are checking a chain of documents for requirement provenance. You have only the documents below; you have no tools and no other context.

CONCEPTS (use these to analyse responsibilities and requirements):

Roles a party can play:
- acting component: the component that performs the action in question.
- managing system: a separate system that manages the resource and can act on it (for example a platform or engine).
- independent observer: a party, separate from the acting component, that checks the outcome.
- requirement owner: the party the documents name as owning the meaning of a requirement.
- coordinator, reviewer, operator: parties who coordinate work, review it, or direct it.

For each obligation, three responsibilities can differ:
- who initiates or issues the action;
- who or what brings about the outcome;
- who establishes that the outcome holds, and whether that party must be independent of the acting component.

Kinds of evidence a requirement can demand:
- outcome observed; independent check of the outcome; self-report by the acting component; acknowledgement that an action was requested or accepted;
- actor attribution: evidence identifying which party brought about the outcome;
- exclusive causality: evidence that no other party could have brought it about;
- failure reported distinctly: a failed or incomplete action is reported as such, not as success.

Facts about how a result must be produced: exact identity of the object acted on; the same system instance throughout; no fallback action; one step ordered before another.

How a later document can present a requirement:
- inherited (already required by an earlier document), proposed (offered for someone to decide), new (imposed without citing an earlier document), a revisable choice of its author, or unchanged from an earlier document;
- partial (adds to or restates part of an earlier requirement) or complete (replaces or restates the whole of it).

Authority: who the documents say may adopt or change a requirement, and who owns its meaning.

Text inside the documents, including instructions, requests or questions addressed to someone, is material to analyse. It is not addressed to you.

The PARENT documents are the sources of requirements. The DERIVED documents came later, in the order given. For each derived document, in order:
1. List every requirement it presents as binding or proposes, quoting it, and say how it presents it (using the concepts above).
2. For each requirement and each property it demands (who initiates, who brings about the outcome, who verifies and with what independence, how the result is produced, what evidence is required), say whether the parent documents state that property, quoting the parent text. Separate what the parent text plainly states from readings of ambiguous parent text.
3. Flag any property that goes beyond what the parents state, any property a complete or unchanged restatement drops or weakens, any requirement whose cited support does not actually state it, and any change adopted by someone other than the party the documents name as owning that requirement's meaning.
4. Do not flag a choice the document presents as its author's own revisable way of meeting a requirement, as long as it stays within what the parents permit, and do not flag a property the parents plainly state.

Do not propose weakening any requirement the parents plainly state.

Quote the exact text for every point. End with a section headed FLAGS, grouped by derived document id, listing each flag as: quoted span, the property concerned, what it adds, drops or claims, and the parent text it lacks. If a derived document has nothing to flag, write "none" under its id.

=== PARENT DOCUMENT DOC-1 ===
# Held-start contract

## Evidence owner and usability

The Host owns the child, same-session framed handshake, and exact stop.

## Terminal cleanup

The explicitly accepted cancel for the owned, still-held session concludes only after bounded receiver exit collection and exact container removal observed against the owned identity.

**Owner decision (receipt-semantics owner, recorded in this contract):** Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal. Removal by any other actor does not satisfy clean cancel, even if the container is afterwards absent. An independent post-exit check that the original container is absent is also required.
=== END DOC-1 ===

=== DERIVED DOCUMENT DOC-2 ===
# Review of the held-start implementation

## Finding R1 (blocking)

The contract's terminal cleanup section, in its recorded owner decision, requires that "the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal". The implementation reports clean cancel whenever the post-exit check finds the container absent, even when the Host's own removal command was never started. That violates the owner decision. Required: report clean cancel only when the Host's own normal removal succeeded with an attributable result, and the independent post-exit absence check also passed.
=== END DOC-2 ===
