# A2 focused review-2: rebase onto A1 and remediation-1 (read-only)

Role: fresh-entry adversarial code reviewer. Read the repository `AGENTS.md`. Do not edit, stage or commit. Write exactly one file: `/home/bline/cbm-work/org-owners-impl-20261118/a2/review-2/REVIEW.md`. No chatboard posts, no subagents.

Scope is narrow: judge only whether the rebase of A2 onto merged A1 and remediation-1 preserved both slices' semantics. Do not re-review A2's design (already reviewed: `/home/bline/cbm-work/org-owners-impl-20261118/a2/review/REVIEW.md`, accept_with_changes) or A1 (merged; `records/org-owners-impl-20261118/a1/`).

Inputs: worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118` (uncommitted diff vs `3c840c36`, same bytes as `/home/bline/cbm-work/org-owners-impl-20261118/a2/a2-2.patch`); the pre-rebase reviewed diff `/home/bline/cbm-work/org-owners-impl-20261118/a2/a2-1.patch` (vs `f772612f`); remediation report `/home/bline/cbm-work/org-owners-impl-20261118/a2/remediation-1/RESULT.md` (conflict table, semantic fixes, P2-1, P3-1, mutants runs-2, gate-3).

Judge:
1. Each conflict resolution and each "auto-merged but semantically broken" fix: both S1 and OA in `CheckId::ALL`, `evaluate`, the no-candidate list and regate; delivery `discharge` table correct for every mapping kind.
2. A1's S1 negatives still isolate S1 alone after their worlds were changed to admit their own request terms; A2's negatives still isolate OA/O8/R3. No test weakened to pass (removed assertions, widened expected sets without reason).
3. P2-1 doc comment and RESULT "Known gap" are accurate; P3-1 test asserts the exact outcome; A2-M11 kills a real claim.
4. Nothing outside the reviewed A2 scope changed (compare the two patches).

Output REVIEW.md: verdict (accept / accept_with_changes / reject), findings P1/P2/P3 with path:line, evidence and fix. Write a skeleton early. Prefer reading; no cargo builds needed (root gate runs separately). Constraints: no protected paths (/etc/work-engine, /var/lib/work-engine, ~/.local/state/work-engine, credentials, ~/.codex, ~/.claude sessions), no network or mutating git. Budget target $1.8, cap $2.5. Final message: verdict and path.
