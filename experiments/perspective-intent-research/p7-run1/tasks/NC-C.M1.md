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
# Disposable job runner cleanup requirement

## Terminal cleanup

After a job is cancelled, an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent from the container engine that launched it. If the container is still present, or its state cannot be established, the job is not reported as cleanly cancelled. The requirement does not say how the container is removed or by whom.
=== END DOC-1 ===

=== DERIVED DOCUMENT DOC-2 ===
# Cleanup implementation disposition

## Implementation choices (the runner team's, revisable)

These are our own choices for meeting the cleanup requirement. They are not new requirements, and we may change them if a better route appears.

- We will create each job's container with the container engine's automatic-removal option, so the engine removes the container after the job's process exits.
- On cancel, the runner issues the stop request to the job's process.
- We will keep the independent post-exit absence check exactly as the requirement states.

## Acceptance

Unchanged from the cleanup requirement: a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine.
=== END DOC-2 ===
