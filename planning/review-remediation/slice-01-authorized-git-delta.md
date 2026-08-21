# Slice 1 — Authorize the Exact Published Git Delta

## Protected consequence

A completion commit changes the user's branch only when its complete delta from
the approved parent is exactly the content presented for approval. Prior private
checkpoints, declined slices, user-owned baseline changes, overlaps, generated
dependencies, and validation dependencies never enter public history merely
because they are present in the accepted tree.

## Observed defect

The completion adapter validates the current slice's proposal path set, compares
the worktree with the full accepted checkpoint tree, and creates that tree with
the current branch `HEAD` as parent. When the checkpoint baseline contains an
earlier privately accepted slice that was not committed to the branch, the Git
delta is larger than the proposal.

The review reproduced a proposal containing only `slice2.txt` that published a
commit containing both `slice1.txt` and `slice2.txt`.

## Likely ownership boundary

The real-branch mutation adapter owns the final authorization-to-Git equivalence
check. The checkpoint adapter owns immutable accepted content, not authority to
publish every difference between that content and an arbitrary branch parent.
The supervisor owns the approval interaction and must present the complete
authorized consequence.

This placement is provisional. Planning must confirm the producer, authority
owner, consumer, and commit-publication lifecycle before implementation.

## Acceptance consequences

- The adapter derives or verifies the complete changed-path/content boundary
  between the approved parent and proposed commit tree.
- That boundary is exactly represented by the approval artifact, including
  additions, modifications, deletions, and relevant attribution.
- A mismatch refuses without moving the branch or real index.
- The reproduced declined-slice scenario cannot publish slice 1 through a
  slice-2-only proposal.
- A deliberately authorized aggregate commit remains possible when its full
  accumulated delta is presented and approved.
- Existing protections for branch attachment, expected parent, staged user
  state, hooks/signing, atomic ref update, and clean postconditions remain true.

## Required vertical proof

Construct two accepted private checkpoints over one unchanged user branch,
decline the first public commit, then attempt the second public commit with a
proposal naming only slice 2. Observe either a safe refusal or an approval
artifact whose scope truthfully includes both deltas. No branch mutation may
occur under a slice-2-only authorization.

## Insufficient substitutes

- Comparing only the worktree and checkpoint trees.
- Checking only the current slice manifest.
- Relying on the user to infer hidden accumulated changes from a tree OID.
- Documenting that private baselines may diverge from branch `HEAD` without
  enforcing authorization equivalence.

## Relationship to later slices

This repair establishes what a completion receipt may truthfully claim, but it
does not by itself make that receipt authoritative at terminal finalization.
