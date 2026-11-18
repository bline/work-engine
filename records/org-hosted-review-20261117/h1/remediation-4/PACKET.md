# H1 remediation 4 packet (fresh builder): tests, mutants and record for the claude-sdk retarget

Role: bounded builder (worker), fresh context. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children.

Objective: the binding objective is `/home/bline/cbm-work/org-hosted-review-20261117/h1/remediation-2/PACKET.md` (items 1–6, operator decision D). Remediation 3 (`h1/remediation-3/{PACKET.md,RESULT.md}`) got the crate compiling and wrote the Messages delivery path in `src/check.rs`, then was stopped by the step budget. Your job is only what remains; do not rework code that is correct.

Entry state: worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117/rust`, crate `crates/work-engine-review-hosted` (uncommitted on `700f45e3`). Root check at entry: `cargo test -p work-engine-review-hosted --offline` = 44 passed, 1 failed: `tests/subject.rs` `job_inputs_binary_refuses_and_writes_nothing` — the binary now requires `--provider` and the test does not pass it, so it is refused before the asserted `scope.digest` refusal. Fix the test (pass `--provider openai-codex`), keep its assertion.

Do, in this order, and keep `h1/remediation-4/RESULT.md` current after each step:
1. Fix that test; full crate tests green.
2. Read `src/check.rs` Messages path once against the packet's item 3 and the source at `/home/bline/code/work-engine` @ `5af34cbf` (`rust/crates/work-engine-manual-host/src/pi_coding_hosted.rs` and `records/dev-vertical-h2-route-20261117/README.md` for the retained Messages body: task first text block of `messages[0]` + Host suffix, Claude Code `system` insertions, tool names `mcp__pi__snapshot_list`, `mcp__pi__snapshot_read`, `DeferredToolPlaceholder`). Fix only real mismatches; cite lines in RESULT.md.
3. Messages-path tests in `tests/check.rs`: one positive synthetic Messages body; refusals: wrong model, extra tool, missing tool, task not the first text block, extra user text, body digest mismatch; and thinking-only final answer if the checker sees answers. Plus the r2 derivation test if missing (diff vs frozen = `execution_confinement` removed from both conditions, new run id, propositions) and the claude-sdk-vs-r2 no-`KnownIncompatibility` test if missing.
4. Three mutants, each killed by a named test, with exact restore verified by `sha256sum`: disable the model check, the tool-set check, the first-text-block check. Record command, failing test and restore hash under `h1/remediation-4/mutants/`.
5. CLAIMS.md item 5 (requested provider/model per profile; serving route and billing guard Host-reported, not claimed; confinement unestablished).
6. Final: `sha256sum` of every crate file + `rust/Cargo.toml` + `rust/Cargo.lock` (cwd `rust/`) into `h1/remediation-4/final-hashes.txt`; test output into `h1/remediation-4/test-output.txt`; append "Remediation 2–4" to `h1/MATRIX.md`.

Tools: one simple Bash command at a time (no `&&`, `;`, pipes or `cd`; compound commands are refused in this permission mode). Use `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline`. Redirect output to files with the Write tool, not shell redirection, if needed. No commit/stage/branch/mutating git; never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, `~/.pi`, credentials, `~/.codex`, other `~/.claude` sessions; no network, Docker or live providers.

Budget: target $2.0, **hard cap $2.5**. If you near the cap, stop after updating RESULT.md truthfully. Final message `handoff_ready` + path.
