# S1 recheck 2 (read-only, Fable, focused)

Role: read-only reviewer, fresh context. From `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Write only `/home/bline/cbm-work/org-hardening-owners-20261117/s1/recheck-2/RECHECK-2.md`.

Scope: ONLY finding R-1 of `/home/bline/cbm-work/org-hardening-owners-20261117/s1/recheck/RECHECK.md` (read its R-1 section) and its repair in `/home/bline/cbm-work/org-hardening-owners-20261117/s1/remediation-2/RESULT.md`. The rest of S1 was already closed by that recheck; do not re-review it.

Subject: worktree `/home/bline/cbm-work/wt-org-hardening-20261117`; changed files in remediation 2 only: `rust/crates/work-engine-review-preparation/tests/fixture_consumer.rs`, `tests/common/mod.rs`, `Cargo.toml` (diff them against their entry blobs `85e50a3b`, `85fe6c64`, `57f01833` with `git -C <wt> diff <blob> <file>` or `git show <blob>`). Custody: `/home/bline/cbm-work/org-hardening-owners-20261117/s1/remediation-2/runs/gate-2` (passed 12/12), `mutants/runs-2` (S1-P8 killed at fixture_consumer.rs:192, case "second obligation's vantage"). Root verified current blobs equal gate-2 hashes.

Check: (1) the new test really obtains an S-A admission for the second obligation's vantage (not a refusal path) and the refusal is the consumer's `GateProofMismatch{vantage}`; (2) the control case is meaningful; (3) test-support changes do not weaken other tests (`Ask` default, `bound_vantages`); (4) wording corrections truthful. Verdict `accept_private_readiness` | `changes_required`, ≤ 40 lines.

Constraints: read-only; no builds, no mutating git, no network. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions. Budget: target $1.5, cap $2.5.
