# S-A remediation 1 (bounded)

Role: bounded builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children.

Subject: worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`; its working tree must equal private checkpoint `1b8bf3015a14d5465ba4d73beeb9d1db45b7fcde` at entry (check `git -C <wt> diff 1b8bf301` for tracked files and `git status --porcelain -uall`; untracked crate files must hash to the checkpoint's blobs, `git -C <wt> ls-tree -r 1b8bf301 rust/crates/work-engine-organization-authority`). Do not commit, stage, branch or run mutating git commands.

Read `/home/bline/cbm-work/org-r2-core-20261117/sa/review/REVIEW-1.md` (§2 F-1, F-2, F-4, F-5, F-7, F-11, F-12; §7) and the root rulings `/home/bline/cbm-work/org-r2-core-20261117/sa/ROOT-RULINGS-1.md`. Implement exactly the rulings that touch the crate `rust/crates/work-engine-organization-authority`: F-1 tests (+ ≥1 recorded mutant in the containment family), F-2 (`admit` takes the observation by value), F-4 (R4 profile equality + test), F-5 (O4 exactly-one principal + test), F-11 (compile-fail doctest), F-12 (exact reason codes in the ids-only test), choice-1 doc wording; update `/home/bline/cbm-work/org-r2-core-20261117/sa/RESULT.md` claims (F-1 correction; F-7/F-12 unclaimed additions) and `sa/MATRIX.md` rows.

Mutants: neutralize, run the mapped test via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `/home/bline/cbm-work/org-r2-core-20261117/sa/remediation-1/mutants/`, restore exactly (sha256 before/after).

Validation via custody.py into `sa/remediation-1/runs/` (env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`): `cargo fmt --check -p work-engine-organization-authority`; clippy `--offline --locked -p work-engine-organization-authority --all-targets --all-features -- -D warnings` and default features; `cargo test --offline --locked -p work-engine-organization-authority --all-features` and default.

Output `sa/remediation-1/RESULT.md`: entry check, changes with reasons, diff stat vs `1b8bf301`, runs, mutants, not done. Final message `map_ready`/`handoff_ready` + path.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions, protected configuration. No network or Docker. Bash allows simple commands (git, cargo, python3, rg, ls, head, cd); avoid long pipelines. Budget: target $3, cap $4.5.
