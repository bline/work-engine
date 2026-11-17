# S-B remediation 1: local design correction (host-constructed admission request)

Role: bounded builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children.

Subject: worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`; its working tree must equal S-B checkpoint `dd68df6c7a7f526d934bc16354fe632c985620ff` at entry (verify tracked diff `git -C <wt> diff dd68df6c` and untracked S-A blobs vs `git -C <wt> ls-tree -r dd68df6c`). Do not commit, stage, branch or run mutating git.

Authority: root design decision `/home/bline/cbm-work/org-r2-core-20261117/sb/ROOT-SB-DESIGN-DECISION.json`, adopting `/home/bline/cbm-work/org-r2-core-20261117/sb/design-review/REVIEW.md` (read all; §4 inventory is the acceptance frame, §6 the scope, §7 the tests/mutants, §8 what to carry). Also `sb/review/REVIEW-1.md` P3-5/P3-6.

Implement in `rust/crates/work-engine-review-delivery` only (no S-A, organization or preparation change):

- §6 items 1–8: `FixtureAuthority` without `request`/`operation` (source; organization_id labelled caller-asserted/unanchored; requirements as digest-bound preimage; declared actor_identity/installed; a time source); one private host constructor of `OperationRequest` from the binding (vantage from the preparation, subject, candidate digest from the binding, requirements preimage digest-checked → `AdmissionMismatch{field:"requirements"}` before any observation); delete `compare_with_binding`; host-derived boundary operation; regate the delivery admission immediately before `run_native` (Current only); time source sampled at each evaluation; `HeldAdmission` holds its source and time source so `regate` takes no differing argument; success labels carry the constructed request identity and the caller-asserted inputs (`caller_asserted_unanchored`); `test_support` behind a separate non-default feature not implied by `fixture`, or narrow the absence sentences (`native.rs` ~593, ~1463–1468) — prefer the feature split if small.
- Tests T1–T7 with one recorded mutant each (custody, `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `/home/bline/cbm-work/org-r2-core-20261117/sb/remediation-1/mutants/`, exact restore with sha256). Retire the other-subject/candidate test and M5 as replaced claims in `sb/MATRIX.md`. Restore the dropped S3 positive assertion (P3-6) and fix the P3-5 comment.
- Update `sb/RESULT.md` unclaimed list with §8 items.

Validation via custody into `sb/remediation-1/runs/` using the plan `/home/bline/cbm-work/org-r2-core-20261117/sb/runs/gate-plan-1.json` (or `sb/gate-plan-1.json` if that is where it is), cwd worktree `rust/` (env as in that plan): fmt, clippy (fixture + default; add the new feature if introduced), four crates + S-A with fixture features, default features, authority-store, native once.

Output `sb/remediation-1/RESULT.md`: entry check; changes with reasons mapped to §6 items; diff stat vs `dd68df6c`; runs; mutants; the §4 inventory as implemented (one row per gate input: host-derived / digest-bound / labelled assertion); not done. Final message `map_ready`/`handoff_ready` + path.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions, protected configuration. No network or Docker. Bash: simple commands. Budget: target $4.5, cap $6 — the objective has about $17 left; if you approach the cap, stop with an honest `handoff_ready` listing what remains.
