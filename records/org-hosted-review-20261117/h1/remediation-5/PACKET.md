# H1 remediation 5 packet (fresh builder): tolerate Claude Code's observed `<total_tokens>` system messages

Role: bounded builder (worker), fresh context. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children.

Entry: worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117/rust`, crate `crates/work-engine-review-hosted`, files equal `/home/bline/cbm-work/org-hosted-review-20261117/h1/remediation-4/final-hashes.txt` (`sha256sum -c` from the worktree root); 58 tests pass. Background: `h1/remediation-4/RESULT.md` (step 2 states the real body shape was not established).

Finding (root, offline end-to-end run `e2e/org-step3-offline-1`, real Claude Code in the container against the offline Messages observer, pinned Host `5af34cbf`): `review-result-check` fails `delivery` with `request 2: role system message at messages[4]; only messages[1] is tolerated`. The real retained bodies (`/home/bline/cbm-work/org-hosted-review-20261117/e2e/org-step3-offline-1/state/coding-hosted-{1,2,3}.body.json`; you may read their structure, do not copy their text into the crate) have this message sequence:

- request 1: `[0] user [text]`, `[1] system [text]` (environment)
- request 2: `[0] user [text]`, `[1] system "<string>"`, `[2] assistant [tool_use]`, `[3] user [tool_result]`, `[4] system [text "<total_tokens>N tokens left</total_tokens>"]`
- request 3: as 2, then `[4] system "<string>"`, `[5] assistant [tool_use]`, `[6] user [tool_result]`, `[7] system [text]`

(Claude Code rewrites earlier system messages from a block list to a string on later requests.) The Host does not interpret role-system messages in `messages[]` (it pins `system[]` blocks and the canonical continuity rule; cite `work-engine-manual-host/src/pi_coding_hosted.rs` at `/home/bline/code/work-engine` @ `5af34cbf` if you rely on it).

Do (closed observed shape, same rule style as the thinking blocks):
1. `src/check.rs` Messages path: a role-`system` message is tolerated only (a) at `messages[1]`, or (b) immediately after a `user` message whose content is only `tool_result` blocks. Its content must be a string or a list of `text` blocks only. Every tolerated message is recorded by digest as host-reported (as `messages[1]` is today) and never interpreted. Any other position or content still refuses. Do not relax anything else.
2. Tests in `tests/check.rs`: positive with the exact observed 8-message structure (synthetic text); refusals: system message after an assistant message, two system messages in a row, system message with a non-text block, system message right after a user text (non-tool_result) message. Update any existing test that asserted the old rule, keeping its intent.
3. One mutant (accept a system message at any position) killed by a named test; exact restore by hash.
4. CLAIMS.md: one line on the tolerated insertions (positions, recorded by digest, not interpreted).
5. `h1/remediation-5/RESULT.md`, `h1/remediation-5/final-hashes.txt` (same form as remediation 4), `h1/remediation-5/test-output.txt`, one line appended to `h1/MATRIX.md`.

Tools: one simple Bash command at a time (no `&&`, `;`, pipes, `cd`, redirection). `cargo test --manifest-path /home/bline/cbm-work/wt-org-hosted-review-20261117/rust/Cargo.toml -p work-engine-review-hosted --offline`. No commit/stage/branch/mutating git; never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, `~/.pi`, credentials, `~/.codex`, other `~/.claude` sessions; no network, Docker or live providers.

Budget: target $1.0, **hard cap $1.5**. Final message `handoff_ready` + path.
