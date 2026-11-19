# A2 builder packet: item 4, OrganizationAdmission record at fixture standing (S-A)

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read the worktree `AGENTS.md` and `docs/contracts/core.md`. Parent and sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your budget. No listeners, no children, no chatboard posts.

## Authority and inputs

- Operator 2026-11-18 approved the O7 owners implementation under `/home/bline/cbm-work/org-owners-impl-20261118/assignment.json`. Decisions: O-1 record owner definitions as crate doc comments plus a records entry (your RESULT.md section "Owner definitions" is that records entry); O-2 (slice B only) a new slice-campaign-owned crate; O-5 the hosted crate (`work-engine-review-hosted`) stays `owner_pending`, do not change it.
- The plan: `/home/bline/cbm-work/org-owners-planning-20261118/PLAN.md`. Read §0 (corrections) and the section for your item, plus §5 (shared files) and §8 (unverified items: verify any you rely on). The plan is a read-only planning pass, not reviewed; where it is wrong, follow the code and record the deviation.
- Accepted design D1: `/home/bline/code/work-engine/records/org-hardening-owners-20261117/d1/PROPOSAL.md` (decisions D1-D8 accepted as recommended).
- Comparable completed slice for shape: `/home/bline/code/work-engine/records/org-hardening-owners-20261117/s1/` (MATRIX.md, mutants/mutate.py, root-gate-plan.json).

Worktree: `/home/bline/cbm-work/wt-org-owners-a2-20261118`, branch `org-owners-a2-20261118`, clean at `f772612f`. Verify clean at entry. Do not commit, stage, branch or run mutating git. Another builder works concurrently on a sibling slice in its own worktree; do not touch other worktrees.

## Evidence

- `/home/bline/cbm-work/org-owners-impl-20261118/a2/MATRIX.md`: invariant → named tests → mutant → result, and negative controls.
- Mutants: copy and adapt `records/org-hardening-owners-20261117/s1/mutants/mutate.py` to `/home/bline/cbm-work/org-owners-impl-20261118/a2/mutants/` (exact-once replacement, original bytes and sha256 saved, exact restore, pre/post hash steps). Run through `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `/home/bline/cbm-work/org-owners-impl-20261118/a2/mutants/runs-N/`. One mutant per new claim at least.
- Gate via custody into `/home/bline/cbm-work/org-owners-impl-20261118/a2/runs/`: model the plan on `records/org-hardening-owners-20261117/s1/root-gate-plan.json` with cwd `/home/bline/cbm-work/wt-org-owners-a2-20261118/rust`, env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-owners-impl-20261118/a2/target` (your own; do not share), same `WE_PI_SDK_*`. Steps: toolchain, fmt, clippy (the feature sets S1 used, plus any you introduce), tests of every crate you change under default and fixture features, authority-store `--test store` if touched, the native suite once.

## Output

`/home/bline/cbm-work/org-owners-impl-20261118/a2/RESULT.md`: entry check; design note (route, rejected alternatives, any dependency edge or public API added); Owner definitions (the exact text you recorded in doc comments and where); files with reasons; diff stat vs `f772612f`; runs; matrix summary; deviations from the plan; unfinished; explicitly unclaimed (fixture standing only; production admission refuses; hosted crate stays owner_pending). Write a RESULT.md skeleton early and keep it current, so it holds your best answer if you are stopped. Final message `handoff_ready` and the path.

## Constraints

Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, `~/.claude` sessions or protected configuration. No network, Docker, live provider, mutating git. Targeted reads; do not read whole large files when a range suffices. Prefer the smallest change that meets the outcomes.

## Outcomes

Implement plan §4 (D4/D5): a new S-A record `OrganizationAdmission` (v1) at fixture standing that binds the admitted organization; the organization id used by the gate comes from that record (the envelope), and `organization_id` is removed from `OperationRequest`, updating the constructor and binding sites the plan lists. Record the OrganizationAdmission v1 owner definition as a doc comment on the record. Negative controls (about 10) and mutants as §4 lists. Excluded: items 1-3, hosted crate, production (L) admission, storage or issuance beyond fixture standing.

Slice A1 (scope.digest) runs concurrently and merges first; you will rebase onto it later. Keep edits to the shared files (§5: org-authority `gate.rs` CheckId/ALL/evaluate, delivery `gate.rs::discharge`, delivery `tests/common/mod.rs`) minimal and mechanical so the rebase stays small.

Budget: target $10, hard cap $13 including the report. Stop with an honest `handoff_ready` near the cap or on a quota error.
