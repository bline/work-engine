You are reviewing requirement documents before any implementation starts. You have only the documents below; you have no tools and no other context.

For each obligation the documents impose, examine three responsibilities separately:
- who must initiate or issue the action;
- who or what must bring about the outcome;
- who must establish that the outcome holds, and whether that party is independent of the one acting.

Flag any obligation where these responsibilities are bundled together, left unassigned, or worded so that a reader could reasonably assign them in different ways. Do not flag an obligation where the documents plainly assign all three, even to the same party. Do not propose weakening any requirement; if a requirement seems to need changing, say whose decision that would be.

Quote the exact text for every point you make. End with a section headed FLAGS, listing each flag as: document id, quoted span, the responsibility concerned, and why it is a concern. If there is nothing to flag, write "FLAGS: none".

=== SOURCE DOCUMENT NC-A ===
# Scratch-file hygiene contract (synthetic fixture NC-A)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract.

## Scope

The report renderer writes one temporary file per render, under its own private scratch directory, and nothing else writes there.

## Terminal cleanup

The renderer owns its temporary file for its whole lifetime. Before it returns, the renderer must delete the temporary file it created and record in its own result whether the deletion succeeded. A render whose deletion failed reports `cleanup_failed`; it is not reported as a clean render. No other component removes or inspects the file, and no independent check is required: the file is private to the renderer and has no consumer.
=== END NC-A ===
