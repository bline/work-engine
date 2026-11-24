You are checking a chain of documents for requirement provenance. You have only the documents below; you have no tools and no other context.

The PARENT documents are the sources of requirements. The DERIVED documents came later, in the order given. For each derived document, in order:
1. List every requirement it presents as binding, quoting it.
2. For each, say whether the parent documents state that requirement, quoting the parent text that does. Separate what the parent text plainly states from readings of ambiguous parent text.
3. Flag any requirement that goes beyond what the parents state, any requirement supported only by a general citation rather than specific parent text, and any requirement presented as unchanged that the parents do not actually contain.
4. Do not flag a choice the document presents as its author's own revisable way of meeting a requirement, and do not flag a requirement the parents plainly state.

Do not propose weakening any requirement the parents plainly state. If a requirement seems to need an owner's decision, say whose, using only what the documents say about ownership.

Quote the exact text for every point. End with a section headed FLAGS, grouped by derived document id, listing each flag as: quoted span, what it adds or claims, and the parent text it lacks. If a derived document has nothing to flag, write "none" under its id.

=== PARENT DOCUMENT NC-C-PARENT ===
# Disposable job runner cleanup requirement (synthetic fixture NC-C, parent)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract.

## Terminal cleanup

After a job is cancelled, an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent from the container engine that launched it. If the container is still present, or its state cannot be established, the job is not reported as cleanly cancelled. The requirement does not say how the container is removed or by whom.
=== END NC-C-PARENT ===

=== DERIVED DOCUMENT NC-C-DISPOSITION ===
# Cleanup implementation disposition (synthetic fixture NC-C, derived document)

Status: constructed negative control for P7 run 1.

## Implementation choices (the runner team's, revisable)

These are our own choices for meeting the cleanup requirement. They are not new requirements, and we may change them if a better route appears.

- We will create each job's container with the container engine's automatic-removal option, so the engine removes the container after the job's process exits.
- On cancel, the runner issues the stop request to the job's process.
- We will keep the independent post-exit absence check exactly as the requirement states.

## Acceptance

Unchanged from the cleanup requirement: a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine.
=== END NC-C-DISPOSITION ===
