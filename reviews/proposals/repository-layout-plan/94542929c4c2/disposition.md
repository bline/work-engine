# Disposition of the second external review of the repository layout plan

- Subject: [`subject.md`](subject.md), revision 2 of
  `planning/repository-layout-plan.md`, SHA-256
  `94542929c4c2512378127a5616caff233d4f66e21d8d7dbde704a25ccd06a8f8`.
- Review: [`chatgpt-review.md`](chatgpt-review.md).
- Disposition by Claude session `4b0947de-6afa-41b4-aade-862183380127`,
  2026-10-27, checked against the local repository. The result is revision 3 of
  [the plan](../../../../planning/repository-layout-plan.md).
- **Approval scope, as the review states it:** commit the plan, the revision-1
  subject, the first review and its disposition after the corrections below.
  It authorizes no repository move and no cleanup phase. This package (the
  revision-2 subject, this review, this disposition) and the refresh brief that
  the plan links to are committed with it, so that the committed plan's links
  resolve.

## Findings

| Finding | Disposition | Local verification |
| --- | --- | --- |
| Revision-1 identity | **Confirmed** | The reviewer's digest of the uploaded original, `38ecc141…abb40`, matches `../38ecc141b304/subject.md`. |
| Historical resolution basis (P2) | **Accepted** | `7583e209` (2026-09-27) does not contain `docs/rust-generation-integration-plan.md`; the file first appears at `df9ebb6e` (2026-10-07). The manifest line naming it was written at `9b898790` (2026-10-09), where the path exists and its SHA-256 matches the cited `8efd7b27…`. |
| `git status --porcelain` wording | **Accepted** | Revision 2 required an empty status for the move set, which the phase's own staged moves would always violate. |

## Further correction found by applying the accepted rule

The plan's own named-exception justification failed the rule this review
introduced.

- *Wrong claim (revision 2, phase 4):* the three links that stay broken inside
  digest-locked records "resolve at their own commits".
- *Error class:* a resolution basis was asserted without being established. It
  is the same class as the review's finding, in the plan's own text.
- *Consequence:* the exceptions were justified by a basis that does not exist.
  All three links were already dangling at the commit that wrote them
  (`df9ebb6e`):
  - `docs/model-selection.md` first appears on the branch at `0b0c72d0`
    (2026-10-21);
  - `docs/rust-development.md` has never been on the branch. Its only commit is
    in the slice-checkpoint ref
    `refs/work-engine/checkpoints/s9-v28-translation-a-20261013/slice-9/candidate-2`.

  After the move they resolve nowhere in git.
- *Check that catches it:* for each historical reference, run `git blame` on
  the referencing line and confirm the target exists at that commit.

**Same error in the first disposition.** Its finding-3 row in
`../38ecc141b304/disposition.md` says `design-sources.json`'s paths "resolve at"
its declared baseline commit. That row is left as written, as a record; this
section is its correction.

**Pre-existing condition, not caused by the plan.** 22 committed files name
`docs/rust-development.md`, which is untracked on the branch, so their links to
it are broken on GitHub today. Committing it is its owner's decision.

## Changes in revision 3

1. The historical reference class resolves "at a source revision established
   for that reference, or unresolved". A new paragraph sets the default
   candidate (the commit that wrote the referencing line, confirmed by path and
   digest) and records unestablished references as unresolved.
2. Phase 4 treats `design-sources.json`'s entry as resolving at `9b898790`, not
   at its declared baseline.
3. Phase 4's named exceptions are justified by digest locking alone and recorded
   as unresolved, with their intended targets named in `records/README.md`.
4. The done-check requires the move set to hold no changes other than the
   phase's own: none unrelated, none owned by another session.
5. The `docs/rust-development.md` condition is recorded beside the paths that
   must not move.
