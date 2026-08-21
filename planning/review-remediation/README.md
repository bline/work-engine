# Verified Review Remediation Campaign

## Status

Ready to use as a `$slice-supervisor` work source. This document preserves the
durable consequences of the 2026-08-21 review; it is not an implementation
transcript and does not make the provisional slice order invariant.

## Objective

Restore truth, user authority, lifecycle coherence, and model decision authority
across the uncommitted completion-commit and planning changes while preserving
the useful checkpoint, recovery, and durable-state capabilities already present.

`DESIGN.md` is the binding doctrine. `PHILOSOPHY.md` explains its rationale but
does not create additional product requirements.

## Preserved review evidence

The review used Codebase Memory project `home-bline-code-work-engine`, generation
`2026-08-21T07:42:47Z`, with no recorded coverage gap for the cited paths. Exact
source was read for every material finding. That coverage signal is best-effort,
not proof of completeness.

Observed deterministic baseline:

- 66 slice-supervisor Python tests passed;
- 2 slice-checkpoint Python tests passed;
- 5 slice-completion-commit Python tests passed;
- 3 campaign-preflight Node tests passed; and
- `git diff --check` passed.

Those checks do not cover the reproduced semantic failures below.

### Reproduction A: proposal scope differs from published commit scope

Given:

1. slice 1 is accepted into a private checkpoint and its user-visible commit is
   declined;
2. slice 2 uses the accepted private checkpoint as its baseline;
3. the user's branch still points to the pre-slice-1 commit; and
4. the slice-2 proposal authorizes only `slice2.txt`;

the completion adapter returned `created`, but the published commit contained
both `slice1.txt` and `slice2.txt`.

```text
proposal paths:      [slice2.txt]
actual commit paths: [slice1.txt, slice2.txt]
```

### Reproduction B: fabricated created receipt passes validation

`validate_completion_commit_projection` accepted a schema-valid `created`
receipt naming `/definitely/not/a/repository` and a fabricated commit OID. The
finalizer revalidates checkpoint receipts against Git but directly copies the
completion-commit receipt before shape validation.

### Reproduction C: pending cannot reach a later terminal decision

After a `pending` completion state was embedded in the unique terminal slice
receipt, a later `created`/`declined` state for the same run and slice could not
be persisted because the append owner correctly rejects duplicate terminal
identities. When the metrics destination is inside the repository, persisting
pending can also change the worktree after checkpoint acceptance and make later
tree-exact commit preflight refuse.

## Campaign-wide invariants

- A user-visible Git mutation contains exactly the content the user authorized.
- Durable receipts never claim a commit or lifecycle transition that the owning
  adapter did not authoritatively establish.
- One terminal audit receipt remains immutable and unique per run and slice.
- Pending human interaction is not disguised as terminal history and does not
  silently block checkpoint-based continuation.
- User-authored and pre-existing workspace state remains attributed and
  preserved.
- Runtime commands protect product structure, not a preferred provider, model
  identity, evidence sequence, storage strategy, or coordination ritual.
- Model context is inherently unsafe as the sole owner of operational state in
  this environment because compaction can lose, distort, or resurrect facts.
  Context continuity may improve judgment, but every consequence required for
  correct recovery must have a durable owner outside that context.
- Roadmap current-evidence claims remain faithful to executable behavior.
- Every durable artifact has one discoverable owner; duplicate convenience
  copies do not become competing sources of truth.

For every slice whose acceptance includes a semantic or doctrine-alignment
claim, the fresh independent reviewer receives and reads `DESIGN.md` directly
as binding doctrine and `PHILOSOPHY.md` directly as non-normative rationale.
Projected invariants alone are insufficient evidence for that review claim.

## Slice candidates

Each linked document defines a bounded outcome and acceptance evidence. The
builder may combine, reorder, or revise candidates when repository evidence
shows a more coherent route, provided every campaign invariant and acceptance
consequence remains covered and route revisions are recorded.

1. [Authorize the exact published Git delta](slice-01-authorized-git-delta.md)
2. [Make completion-commit receipts authoritative](slice-02-authoritative-completion-receipt.md)
3. [Give pending completion interaction a coherent lifecycle](slice-03-pending-completion-lifecycle.md)
4. [Remove completing-builder identity from product validity](slice-04-proposal-origin-openness.md)
5. [Reframe review coordination around evidence validity](slice-05-review-coordination-consequences.md)
6. [Keep persistent-state architecture route-open](slice-06-persistent-state-route-openness.md)
7. [Restore roadmap truth and artifact ownership](slice-07-roadmap-and-artifact-truth.md)

## Completion condition

The campaign is complete when every verified defect has either been repaired
with observed downstream proof or truthfully superseded by a better design that
preserves the same invariant, and when the roadmap no longer promotes the
reviewed implementation choices as binding product structure.

The campaign is not complete merely because existing tests pass or the reviewed
text is reworded. The two Git/receipt reproductions and the pending-lifecycle
case require vertical semantic proofs.
