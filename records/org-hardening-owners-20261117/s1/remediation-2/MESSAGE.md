# S1 remediation 2 (same builder context, resumed)

Operator authorized option (a) for the Fable recheck finding R-1: `/home/bline/cbm-work/org-hardening-owners-20261117/s1/recheck/RECHECK.md` (read it all).

Do:
1. Add one negative control: a two-obligation candidate (derive it the way `work-engine-review-preparation/tests/preparation.rs:551-575` `with_second_obligation` does, from the frozen requirements), a world in which the second obligation's vantage is bound and accepted, an S-A admission at a Consumption boundary for that second vantage via `Ask.vantage`, and a declaration for the first obligation. Expect `ConsumeRefusal::GateProofMismatch { field: "vantage" }`. Assert first that S-A actually admitted the second vantage (so the refusal is the consumer's, not S-A's).
2. Re-run mutant `S1-P8-vantage-unchecked` against the new test under custody; it must be killed for that reason (panic in the new case).
3. Correct the comment at `tests/fixture_consumer.rs` ~776-782, the MATRIX row and remediation RESULT wording (no longer "defense in depth"). Fix the P3 nit in `work-engine-review-preparation/Cargo.toml:17-19` (add the caller-assertion qualifier for freshness).
4. If S-A refuses the second vantage for a reason you cannot satisfy without changing S-A, stop and report the exact refusal (do not change S-A).

Evidence: full gate (`s1/remediation-1/gate-plan-2.json`, extend the hashes step to every changed file) via custody into `s1/remediation-2/runs/gate-1/`; mutants into `s1/remediation-2/mutants/` (copy `remediation-1/mutants/mutate.py`; re-run S1-P1..P5 only if their anchors changed). Record entry blob hashes of every file you will touch (including `tests/common/mod.rs`) before editing.

Output `s1/remediation-2/RESULT.md` (entry hashes, changes, runs, mutant result, diff stat vs `0fe9dc74`) and append "Remediation 2" to `s1/MATRIX.md`. Final message `map_ready`/`handoff_ready` + path. Budget: target $1.8, cap $2.5; stop honestly near the cap. Same constraints as before (no commit/stage/branch; no protected paths; no network).
