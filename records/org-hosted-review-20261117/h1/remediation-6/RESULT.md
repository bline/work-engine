# H1 remediation 6: review-2 P3-1, P3-2, P3-4, P3-7

Status: handoff_ready. Offline only; no network, Docker, live provider, commit, stage or git mutation (`git diff --cached` empty).

Entry: `sha256sum -c h1/remediation-5/final-hashes.txt` from the worktree root: all OK before any edit; 60 tests passed.
Result: 64 tests pass (33 check + 10 revision + 17 subject + 4 task; +4 new), no warnings, `cargo fmt` applied. `test-output.txt`, `final-hashes.txt` (re-verified with `sha256sum -c`).

## P3-1 Host suffix content
- Change: `HOST_SUFFIX_SHA256` (`src/task.rs:42`, doc cites `pi-pressure-context-host/src/analysis.rs:42-47`; digest `7edf1b84…bbbd` of the 202 bytes after the format string's `{}`, computed from that source; the suffix text is not in the crate). `task_with_suffix` (`src/check.rs:~276-292`) now also requires `sha256(text[task.len()..]) == HOST_SUFFIX_SHA256` after the length check; used by both wires (Responses user text, Messages `messages[0]`).
- Tests (`tests/check.rs`): `host_suffix_constants_match_the_host_source` (test reads the Host source via `include_str!("../../pi-pressure-context-host/src/analysis.rs")`, so a changed Host suffix fails there; asserts length 202 and the digest), `delivery_refuses_a_same_length_different_suffix` (both profiles; fails only `delivery`, reason "not the Host suffix bytes"). `host_text` now builds the real suffix instead of the old stand-in padding.
- Mutant R6-M1: `if false && sha256_hex(...) != HOST_SUFFIX_SHA256` in `task_with_suffix`. `cargo test --manifest-path rust/Cargo.toml -p work-engine-review-hosted --offline --test check`: 32 pass, 1 fail: `delivery_refuses_a_same_length_different_suffix` (killer). Restore: `check.rs` sha256 `7d282178…d0049` = pre-mutant (exact).

## P3-2 `job_profile_bound`
- Change: `src/check.rs:~622` in `run_binding`: failing check requiring `job.json` `coding.provider.provider` and `.model` equal `profile.provider()` / `profile.model()`; detail prints both sides.
- Tests: `job_provider_and_model_must_be_the_profiles` (both profiles: positive `Passed`; wrong provider; wrong model, each failing only `job_profile_bound`). `run_for` fixtures now carry the profile's provider/model in `job.json`.
- Mutant R6-M2: condition replaced by `true || (...)`. Same command: 32 pass, 1 fail: `job_provider_and_model_must_be_the_profiles` (killer). Restore hash `7d282178…d0049` = pre-mutant (exact).

## P3-4 `CLAIMS.md`
- Change (`CLAIMS.md:94-101`): replaced the stale `not established` bullet with what two runs show (`e2e/org-step3-offline-1`, `e2e/org-step3-live-1`, 3 requests each; positions read from `state/coding-hosted-{1,2,3}.body.json` structure with `jq`, roles/content types only): `messages[0]` one user text block; role-`system` messages at `messages[1]` and after each tool-result-only user message, string content except the last message of each body (system, text-block list); assistant `tool_use` turns (up to 3 in one turn, live) answered by `tool_result` user messages. n = 2; other live shapes `not established`.
- No test (documentation).

## P3-7 detail text
- Change: `src/check.rs:268` `count_or_not_established`; `frozen_world_unmet` detail prints the number or `not established`.
- Test: `frozen_world_detail_prints_numbers_not_rust_options` (no `Some(`; exact "2 known incompatibilities, not established unestablished confinement" on openai-codex). Not mutated.

## Notes
- The new suffix test reads the Host source by a relative `include_str!` inside the same workspace (`crates/pi-pressure-context-host`, byte-identical to `/home/bline/code/work-engine/...` at entry); the test crate now depends on that file existing.
- Changed files: `src/task.rs`, `src/check.rs`, `tests/check.rs`, `CLAIMS.md`. All else unchanged (hashes in `final-hashes.txt`).
