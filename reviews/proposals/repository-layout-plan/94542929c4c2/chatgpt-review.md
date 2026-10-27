# Second external review of the repository layout plan (ChatGPT)

- Subject: [`subject.md`](subject.md), revision 2 of
  `planning/repository-layout-plan.md`, SHA-256
  `94542929c4c2512378127a5616caff233d4f66e21d8d7dbde704a25ccd06a8f8`.
- Reviewer: ChatGPT, with GitHub access to the repository. It checked the
  revision-1 digest against the uploaded original and checked commit
  `7583e209` on GitHub.
- Pasted into Claude session `4b0947de-6afa-41b4-aade-862183380127` by the user
  on 2026-10-27. Reproduced verbatim below the rule.
- Disposition: [`disposition.md`](disposition.md).
- `subject.md` is byte-identical to the reviewed revision, so its relative
  links are written from `planning/` and resolve from there, not from this
  directory.

---

Revision 2 is substantially improved, and I support committing the review package. There's one factual correction I would make first.

I verified that the original uploaded plan has SHA-256:

`38ecc141b3041a73d4db744d3e464299f60a1e4226f9cd54fafd6a25ac6abb40`

That matches the revision-1 identity recorded in revision 2. The review's subject is therefore anchored to the correct original document.

The important changes also address my earlier findings: replacing the unsafe 53-file move, separating reference classes, preserving the dated observation note, keeping placement distinct from acceptance, and requiring a separate reviewed Phase 5 inventory.

## One correction before committing

P2 — Historical resolution basis

Revision 2 says that `experiments/context-lifecycle-rust/design-sources.json` resolves historical paths at its declared baseline commit, `7583e209`.

I checked that commit on GitHub. It does not contain `docs/rust-generation-integration-plan.md`, even though the manifest records that file's path and SHA-256. In fact, that commit predates the Rust documents in question.

The manifest can still be valid historical evidence. However, its `baseline.repository_head` cannot automatically serve as the resolution commit for every document it references.

I would change the plan's rule to require identifying the appropriate source revision for each historical path. Where that revision isn't established, record the reference as unresolved rather than claiming it was verified against the baseline. Apply that correction in the reference-class table and Phase 4's historical-reference treatment.

There's also a small wording fix: the `git status --porcelain` condition should mean no unrelated or foreign-owned changes in the move set. The phase's own changes will naturally appear before committing.

## Decision

Yes — commit the four review and planning files after those corrections.

That approval covers the revision-2 plan, the exact revision-1 subject, the review, and its disposition. It does not authorize any repository moves or individual cleanup phases.

I agree that Phase 1 is the next sensible step. Its `AGENTS.md` edit should remain blocked until the other session's changes are committed.

One thing I particularly like about this revision is that it preserves the errors and their consequences rather than simply replacing the earlier proposal. That gives future reviewers evidence of why the migration rules exist, not merely what they require.
