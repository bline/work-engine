# S1 recheck packet (read-only, Fable)

Role: read-only reviewer (fresh context). From `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Write only `/home/bline/cbm-work/org-hardening-owners-20261117/s1/recheck/RECHECK.md`.

Subject: worktree `/home/bline/cbm-work/wt-org-hardening-20261117`, uncommitted diff vs `0fe9dc74` (`git -C <wt> diff 0fe9dc74`). Contract `s1/PACKET.md`; prior review `s1/review/REVIEW.md` (Opus xhigh, `changes_required`: P2-1, P2-2, P3-1..P3-4); builder reports `s1/RESULT.md`, `s1/MATRIX.md`, `s1/remediation-1/RESULT.md`. Custody: `s1/runs/gate-1`, `s1/mutants/runs-1`, `runs-2`, `s1/remediation-1/runs/gate-3` (passed 12/12; `gate-2` failed in an abandoned attempt), `s1/remediation-1/mutants/runs` (S1-P8 survives). All paths under `/home/bline/cbm-work/org-hardening-owners-20261117/`.

Check, in order:
1. P2-1 and P2-2 closed truthfully (docs now true in every feature combination; freshness label reaches the success output and is asserted).
2. P3-1 and P3-3 as stated; P3-2 disposition: is the surviving S1-P8 mutant acceptable as defense in depth given the builder's evidence that S-A R2 admits only a vantage named by the candidate's requirements (verify in `work-engine-organization-authority/src/gate.rs`), or is a wrong-vantage admission reachable (e.g. a candidate with two obligations/vantages) such that the check is load-bearing and must be tested?
3. The remediation changed nothing else (gate.rs, binding.rs, Cargo files unchanged vs gate-1; `tests/common/mod.rs` reverse-edited — confirm it contains no leftover of the abandoned vantage world).
4. Anything in the whole S1 diff the prior review missed that blocks private readiness (brief; prior review covers invariants 1–4).

Output `RECHECK.md` (≤ 60 lines): verdict `accept_private_readiness` | `changes_required`, findings with file:line, carried items. Final message: verdict + path.

Constraints: read-only; no cargo builds, no mutating git, no network. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions. Targeted reads. Budget: target $2, cap $3.
