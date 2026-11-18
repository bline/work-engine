# S1 remediation 1 (bounded builder)

Role: bounded builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children.

Subject: worktree `/home/bline/cbm-work/wt-org-hardening-20261117` (uncommitted diff vs `0fe9dc74`). At entry its gated sources must equal `/home/bline/cbm-work/org-hardening-owners-20261117/s1/runs/gate-1/hashes.stdout` (rerun the `git hash-object` list from the last step of `s1/gate-plan-1.json`, cwd `rust/`). Do not commit, stage, branch or run mutating git.

Authority: review `/home/bline/cbm-work/org-hardening-owners-20261117/s1/review/REVIEW.md` (read all). Builder report `s1/RESULT.md`, `s1/MATRIX.md`; contract `s1/PACKET.md`.

Fix:
- **P2-1**: correct the false absence sentences (`review-delivery/src/native.rs` ~26-29, `lib.rs` ~25-27, and any matching sentence in `review-preparation/src/lib.rs`/`fixture.rs`) to the reviewer's true statement: without S-A `fixture` no episode can advance; with it any caller can mint a fixture consumption admission and consume outside review-delivery's gate; the consumer checks that S-A found *a* consumption admission current for its candidate, subject and vantage, not whose admission it is nor how fresh the observation is.
- **P2-2**: label observation freshness as a caller assertion on the success output (e.g. a constant `caller_asserted_regate_observation_freshness` exposed by `GateCheck`), assert it in the positive test, update the inventory row in MATRIX.
- **P3-1**: document in the module and `ConsumptionProof` docs that only the boundary kind is compared (operation and epoch are not; O10 class). No code change.
- **P3-2**: add the vantage negative case (the `Ask.vantage` hook) and one mutant (remove the vantage `return mismatch`), killed.
- **P3-3**: make `refused_with` name the loop case in its failure message.
- P3-4: leave as is; note it as carried.

Evidence: re-run the full gate plan `s1/gate-plan-1.json` via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `s1/remediation-1/runs/gate-2/` (update the hashes step if files change). Mutants: adapt `s1/mutants/mutate.py` (new id `S1-P8-vantage-unchecked`), run via custody into `s1/remediation-1/mutants/`, exact restore with hash steps. Re-run S1-P1..P3 only if their anchors changed.

Output: append a "Remediation 1" section to `s1/MATRIX.md` and write `s1/remediation-1/RESULT.md` (entry check, changes mapped to findings, diff stat vs `0fe9dc74`, runs, mutants, carried). Final message `map_ready`/`handoff_ready` + path.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions. No network, Docker, live providers. Bash: simple commands. Targeted reads. Budget: target $1.8, cap $2.5 — the objective has about $6 left; stop with an honest `handoff_ready` near the cap.
