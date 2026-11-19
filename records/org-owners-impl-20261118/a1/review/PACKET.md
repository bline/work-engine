# A1 review packet: scope.digest is the exact-subject digest (read-only adversarial code review)

Role: fresh-entry adversarial code reviewer. Read the repository `AGENTS.md` and `docs/contracts/core.md`. Do not edit, stage, or commit anything. Do not build into the builder's target directory. Write exactly one file: `/home/bline/cbm-work/org-owners-impl-20261118/a1/review/REVIEW.md`. No chatboard posts and no subagents.

## Subject

- Worktree `/home/bline/cbm-work/wt-org-owners-a1-20261118` (uncommitted diff against base `f772612f`; `git -C <worktree> diff f772612f`). The same bytes are in `/home/bline/cbm-work/org-owners-impl-20261118/a1/a1-1.patch`.
- Builder report: `/home/bline/cbm-work/org-owners-impl-20261118/a1/RESULT.md` and `MATRIX.md`. Builder gate and mutant records are in `runs/gate-1` and `mutants/runs-1` (13/13 passed, 7/7 mutants killed). The root is running its own gate on a clean export. Treat the builder's claims as claims and check them.

## Authority

- Plan §0, §1 and §8: `/home/bline/cbm-work/org-owners-planning-20261118/PLAN.md` (not reviewed itself).
- Accepted design decision D1 = (A), `scope.digest` is the exact-subject digest: `records/org-hardening-owners-20261117/d1/PROPOSAL.md`.
- Operator decision O-1: owner definitions are recorded as crate doc comments plus a records entry (RESULT.md).
- Fixture standing only. The hosted crate stays `owner_pending`.

## What to judge

1. Correctness: S1 passes exactly when every obligation served by the requested vantage has `scope.digest` equal to the subject's exact-subject digest, and refuses otherwise. Check the digest's derivation (episode codec, `{commit, tree, patchIdentity}`), the canonical encoding and hex case, and the empty and unknown vantage cases.
2. The builder's unplanned owner choices, each for accept or change: fail-closed for an unknown or unserved vantage (`ScopeDigestUnserved`); hex-string comparison rather than `Digest`; the new public `SubjectRef::scope_digest()`; the new dependency from S-A to `review-episode-core` (check for cycles and layering).
3. Delivery `discharge(ScopeDigestToSubject) = Admitted(&[S1])`: is this sound, and are the other mapping kinds still assumed and labelled?
4. Fixtures: the regenerated assumption-world `scope.digest` (`18108e56…`) is the real exact-subject digest; the frozen files are byte-identical; frozen-world admissions are now named negatives.
5. Tests and mutants: do the 7 mutants and the negatives kill the claims they map to? Are there unkilled routes, such as S1 missing from `ALL`, a short-circuit, or a check that compares a value with itself?
6. The test-only edits to review-preparation (deviation 1): necessary and test-only?
7. The doc-comment owner definition: accurate and consistent with D1.

## Output

REVIEW.md: verdict (`accept`, `accept_with_changes`, or `reject`); findings with severity P1 (blocks), P2 (fix before merge), P3 (optional), each with path:line, evidence and a concrete fix; anything you could not verify. Write a REVIEW.md skeleton early and keep it current. You may run read-only commands and `cargo test` with `CARGO_TARGET_DIR=/home/bline/cbm-work/org-owners-impl-20261118/a1/target-review` if needed, but prefer reading.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, `~/.claude` sessions. No network, Docker, live provider, or mutating git. Budget target $3, cap $4 including the report. Final message: the verdict and the REVIEW.md path.
