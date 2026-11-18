# H1 remediation 3 packet (fresh builder): finish the claude-sdk retarget

Role: bounded builder (worker), fresh context. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Do not load the orchestration skill beyond this packet.

## Why you exist

A previous builder (remediation 2) worked the retarget packet `/home/bline/cbm-work/org-hosted-review-20261117/h1/remediation-2/PACKET.md` and was stopped at its cost cap **mid-work**. Its full objective (items 1–6, evidence, outputs) is still the objective; read that packet first — it is binding, including operator decision D. Your job is to finish it from the current partial state, not to restart.

Entry state: worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117` (uncommitted on `700f45e3`), crate `rust/crates/work-engine-review-hosted`. Files must equal `/home/bline/cbm-work/org-hosted-review-20261117/h1/remediation-3/entry-hashes.txt` (`sha256sum -c` with cwd `rust/`). Remediation-1 baseline hashes (before remediation 2 touched anything): `h1/remediation-1/runs/main-1/hashes.stdout`; `diff` against them shows what remediation 2 changed.

What remediation 2 already did (verify, don't redo): `src/revision.rs`, `src/task.rs`, `src/inputs.rs` edited; `fixtures/hosted-route-qualification-r2/{requirements.json,DERIVATION.md}` and `templates/hosted-review-task-claude-sdk-v1.txt` written; derivation script `h1/remediation-2/make_r2.py`.

What is known missing (root check, 2026-11-18):
- The crate does **not compile**: `inputs` gained a 7th parameter; callers in `src/bin/review-job-inputs*` and tests `revision`, `subject`, `check` still pass 6 (E0061).
- `src/check.rs` still only has the Responses delivery path; the Messages path (packet item 3) is not written. `tests/check.rs` and `tests/revision.rs` are unchanged.
- No r2 derivation test, no Messages-path tests, no mutants, no custody runs, no RESULT.md, no MATRIX entry. CLAIMS.md (item 5) unverified.
Re-check items 1, 2, 4 against the packet; finish 3, 5, 6.

## Tools and limits

Allowed Bash runs **one simple command at a time** — no `&&`, `;`, pipes, `cd`-chains or subshells (compound commands are refused in this permission mode; that is what blocked the previous builder). Use `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline`, `sha256sum -c <file>` from the right cwd via `python3 -I` helpers if needed. Custody: `python3 -I /home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run --plan <plan.json> --out <dir>` (see `h1/remediation-1/plans/` for the plan shape used before; extend hashes to the new files).

Same prohibitions as the remediation-2 packet: no commit/stage/branch/mutating git; never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, `~/.pi`, credentials, `~/.codex`, other `~/.claude` sessions; no network, Docker or live providers.

## Outputs

Evidence into `h1/remediation-3/runs/` and `h1/remediation-3/mutants/` (exact restore, hashes). `h1/remediation-3/RESULT.md` (what changed per packet item, test count, mutants killed, source citations at `5af34cbf` for the Messages body shape and tool names, open items) and append "Remediation 2+3" to `h1/MATRIX.md`.

Budget: target $3.0, **hard cap $4.0**. Write RESULT.md early and update it as you go, so a cap stop still leaves an accurate record. If the crate compiles and all tests pass but mutants are unfinished, record that honestly rather than overrun. Final message `handoff_ready` + path.
