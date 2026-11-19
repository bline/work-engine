# A2 remediation-1 packet: rebase onto merged A1, review P2-1 and P3-1

Role: one coherent remediator (worker). Read the worktree `AGENTS.md` and `docs/contracts/core.md`. Parent and sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. No listeners, children, or chatboard posts.

## State at entry

- Worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118`, branch `org-owners-a2-20261118`, HEAD `3c840c36` (the integration branch with slice A1 merged: S-A `CheckId::S1`, scope.digest exact-subject digest; see `records/org-owners-impl-20261118/a1/RESULT.md`).
- The root re-applied A2's uncommitted diff (`/home/bline/cbm-work/org-owners-impl-20261118/a2/a2-1.patch`, reviewed against base `f772612f`) on top of `3c840c36`. Six files carry conflict markers (10 hunks): S-A `Cargo.toml`, `src/gate.rs`, `src/lib.rs`, `tests/admission.rs`; delivery `src/gate.rs`, `tests/gate.rs`. Nothing is staged. Do not run mutating git (no add, commit, stash, checkout, reset); edit files only.
- A2 builder report: `/home/bline/cbm-work/org-owners-impl-20261118/a2/RESULT.md`, `MATRIX.md`. Review: `/home/bline/cbm-work/org-owners-impl-20261118/a2/review/REVIEW.md` (Opus xhigh, accept_with_changes).

## Outcomes

1. Resolve every conflict so both slices' semantics hold: S1 and OA both in `CheckId::ALL` and evaluated; delivery discharge keeps `ScopeDigestToSubject = Admitted([S1])` and A2's changes; test worlds use A1's exact-subject scope digest and A2's admission terms. No conflict markers remain.
2. Review P2-1: amend the `OrganizationAdmission` owner-definition doc comment (`records.rs`) so it does not read as gate-enforced: state that at fixture standing the gate does not check the predecessor chain (revision +1, same organization, predecessor superseded) or per-organization uniqueness, and that an issuer outside fixture standing requires that check first. Add a section "Known gap: admission chain" to `/home/bline/cbm-work/org-owners-impl-20261118/a2/RESULT.md` saying the same, and the review's owner question (admissions are keyed by problem scope including the slice number, so one-head-per-organization would block concurrent slices; settle before enforcing).
3. Review P3-1: add a test where a second matching admission is committed after admission; the regate must refuse (`OrganizationHeadsAmbiguous` or the precise outcome the code gives; assert it).
4. Regenerate the A2 mutants against the rebased text (`/home/bline/cbm-work/org-owners-impl-20261118/a2/mutants/`; exact-once replacement, pre/post hashes) into `/home/bline/cbm-work/org-owners-impl-20261118/a2/mutants/runs-2/` via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run`; all must be killed. Add one mutant for outcome 3.
5. Gate via custody into `/home/bline/cbm-work/org-owners-impl-20261118/a2/runs/gate-3/` using `/home/bline/cbm-work/org-owners-impl-20261118/a2/root-gate-plan.json` with cwd `/home/bline/cbm-work/wt-org-owners-a2-20261118/rust` and `CARGO_TARGET_DIR=/home/bline/cbm-work/org-owners-impl-20261118/a2/target`; update its `hashes` step to the current file list. All steps must pass.

## Output

`/home/bline/cbm-work/org-owners-impl-20261118/a2/remediation-1/RESULT.md`: each conflict and how you resolved it; P2-1 and P3-1 changes; mutant and gate results; diff stat vs `3c840c36`; deviations. Write a skeleton early. Final message `handoff_ready` and the path.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, `~/.claude` sessions. No network, Docker, live provider. Budget target $3, cap $4.5 including the report; stop with an honest `handoff_ready` near the cap or on a quota error.
