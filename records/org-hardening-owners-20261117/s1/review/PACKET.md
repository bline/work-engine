# S1 review packet (read-only code review, Opus xhigh)

Role: read-only reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` and load only `references/reviewer.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Do not edit the worktree; write only `/home/bline/cbm-work/org-hardening-owners-20261117/s1/review/REVIEW.md`.

## Subject

Worktree `/home/bline/cbm-work/wt-org-hardening-20261117` (uncommitted diff vs `0fe9dc74`; `git -C <wt> diff 0fe9dc74`, 12 files, ~730+/70-). The builder was stopped at its budget cap **after** implementation, gate and mutant runs but **before** writing RESULT.md/MATRIX.md, so there is no builder self-report or design note: review the code and the custody records directly. Root verified: current blob hashes of the gated sources equal `s1/runs/gate-1/hashes.stdout`; no test file changed after the gate.

Contract: `/home/bline/cbm-work/org-hardening-owners-20261117/s1/PACKET.md` (invariants 1–5 and the inventory acceptance frame). Background: `/home/bline/cbm-work/org-r2-core-20261117/sb/design-review/REVIEW.md` §4–§8 (P3-3), `sb/SB-ACCEPTANCE.json`, `sb/MATRIX.md`.

Evidence (custody, under `/home/bline/cbm-work/org-hardening-owners-20261117/s1/`): `gate-plan-1.json` + `runs/gate-1/` (12/12 passed: fmt, clippy fixture/default/test-support, preparation fixture lib alone, four crates + S-A fixture, preparation fixture tests, default features, authority-store, native once, hashes); `mutants/mutate.py`, `mutant-plan-1.json` (M1–M4, M6–M11, S1-P1..P7) + `runs-1/`, `mutant-plan-2.json` (M6 respecified) + `runs-2/`. Every mapped test exited 101 with a runtime failure (no compile errors; root grep); restores hash-verified. Note S1-P1..P5 all fail at `tests/fixture_consumer.rs:186` — check whether each mutant is killed by the assertion meant for its claim or by an earlier shared one.

## Questions

1. Invariant 1: is the consumption proof unforgeable outside S-A and checked against the consumer's own inputs before the transition, in every build/feature combination (default, preparation `fixture` alone, review-delivery `fixture`, `test-support`)? Any route that advances an episode without it? Is the stated standing truthful (regate time asserted; freshness and F-3 finality are host obligations; proof reusable — O10 class)? Inventory row for each proof input.
2. Invariant 2: is `native::test_support` absent under `fixture` alone by `cfg`, and are the absence sentences now true?
3. Invariant 3: delivery `regate_revision` is `Some` and asserted.
4. Invariant 4: are the re-established mutants faithful to their original claims, and are the new mutants killed for the right reason?
5. Regressions to S-B claims, overclaims in docs, scope creep.

## Output

`REVIEW.md`: verdict (`accept_private_readiness` | `changes_required` | `design_escalation`), findings P1–P3 with file:line and a concrete fix, an inventory table for the proof inputs, what is `not established`. Be concise (≤ 120 lines). Final message: verdict + path.

## Constraints

Read-only; no cargo builds (use the custody records), no mutating git, no network. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions. Targeted reads. Budget: target $2.5, cap $3.5 — the objective is nearly spent; prioritize invariant 1, then 4.
