# H1 remediation 4: result (COMPLETE: handoff_ready)

Builder: remediation 4 (fresh context). Binding objective: `h1/remediation-2/PACKET.md` (items 1-6, operator decision D 2026-11-18), finishing what `h1/remediation-3` left.
Worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117` (uncommitted on `700f45e3`; no commit, stage, branch or mutating git was run). Crate `rust/crates/work-engine-review-hosted`.

Final state: `cargo test -p work-engine-review-hosted --offline` = **58 passed, 0 failed** (check 27, revision 10, subject 17, task 4), no compiler warnings. `final-hashes.txt`: 24 lines (22 crate files, `rust/Cargo.toml`, `rust/Cargo.lock`; expected 24), `sha256sum -c` from the worktree root: 24 OK.

## Step 1: failing test fixed - DONE

- `tests/subject.rs` `job_inputs_binary_refuses_and_writes_nothing`: added `--provider openai-codex` to the binary's args; its `scope.digest` assertion and `!out.exists()` are unchanged.
- Entry state was 44 passed, 1 failed (as the packet said); after the fix, 49 passed, 0 failed.

## Step 2: Messages path vs source @ `5af34cbf` - DONE, no code change

Main checkout HEAD = `5af34cbf7c177d216c5b19dc6ca8c96e258052f8` (checked with `git rev-parse HEAD`). Read once; each `src/check.rs` line against its source line:

| `check.rs` | Source at `5af34cbf` | Result |
|---|---|---|
| `messages_body` `parsed["model"] != model` (:404) | `pi_coding_hosted.rs:732` (`parsed["model"] != plan.model`) | match |
| tool names sorted == `profile.request_tools()` (:407-418) | `pi_coding_analysis.rs:118-143` (`mcp__pi__` + `profile_tools`, plus `DeferredToolPlaceholder`; `TOOLS` = `["snapshot_list","snapshot_read"]`, `:9`) | names match (`CLAUDE_TOOLS`, `revision.rs:48-52`) |
| tools must be an array (:407-410) | `pi_coding_analysis.rs:115-117` refuses absent tools | match |
| role-system message tolerated at `messages[1]` only; hashed whole, string or blocks (:449-456) | Host fixtures `pi_coding_hosted.rs:2919-2926` (`{"role":"system","content":[...]}` rewritten to a string) | match to the fixture |
| `messages[0]` content is a string or one text block (:427-446) | `pi_coding_hosted.rs:2923-2925` (Claude Code's rewrites: string content, moved `cache_control`) | match |
| task + Host suffix, 202 bytes (`task_with_suffix`, :270-283) | `pi-pressure-context-host/src/analysis.rs:42-47` (`"{task}\n\nBounded read-only analysis job. ..."`, no provider branch); `"\n\n"` + sentence measured with python = 202 = `HOST_SUFFIX_BYTES` | match |
| assistant turns not read; `thinking` never answer (:457-458) | README `records/dev-vertical-h2-route-20261117/README.md:376-379` (non-semantic; answer = final text blocks) | match |

Not duplicated by the checker: the Host also pins the canonical `tools` array digest (`pi_coding_analysis.rs:144-149`), `tool_choice`, top-level keys, the retry header and the three leading system blocks (README `:345-365`). Those are Host-enforced; the checker records `system[]` digests and does not interpret them.

Limit: the retained `coding-hosted-<n>.body.json` of the dev-vertical runs is not in the repository (`find` under `records/dev-vertical-h2-route-20261117`, depth 4: no match). The body shape is therefore checked against the Host's unit-test fixtures and the README, not an observed body. Whether a real Claude Code body has extra content blocks in `messages[0]` or text in later user turns is `not established`; the checker refuses both (per the packet's "extra user text" refusal), so a real body with either would be refused, not silently accepted.

## Step 3: Messages-path tests - DONE (`tests/check.rs`, 9 added)

Helpers: `claude_world` (claude-sdk record on r2, via `prepare_world_for` / `render_for` / `frozen_world_for`), `messages_body_with` (user turn + environment message; request 2+ adds an assistant turn with a `thinking` block and a `tool_use`, then a user `tool_result`), `assert_claude_delivery_refused` (each case must end `checks_failed`, fail only `delivery`, and carry the expected reason substring, so a case cannot pass for the wrong reason).

- Positive: `claude_sdk_messages_body_passes_and_records_insertions_by_digest`; also `responses_profile_records_no_request_insertions`.
- Refusals: `claude_sdk_refuses_a_request_for_another_model` (wrong model, no model); `claude_sdk_refuses_a_wrong_tool_set` (extra, missing, missing placeholder, unprefixed name, duplicate, no tools array); `claude_sdk_refuses_a_task_that_is_not_the_first_text_block` (another text block first, non-text block first, non-text block carrying the task bytes, empty and null content, altered task, text after suffix, missing suffix; string content accepted); `claude_sdk_refuses_extra_user_text_and_stray_roles` (extra text block in `messages[0]`, extra user message, text beside a tool result, system message after `messages[1]`, unknown role, `messages[0]` not user, no messages); `claude_sdk_refuses_digest_mismatch_and_missing_body` (extra attempt digest, `None` digest, missing body, no requests, body edited after its digest).
- Thinking: `claude_sdk_thinking_is_neither_answer_nor_evidence`. The checker receives the answer only as `analysis-result.json`'s `answer` string and never reads assistant turns, so this pins non-reading: an assistant `thinking` block holding a valid review answer changes neither the envelope answer nor the recorded insertions; an empty answer (what a thinking-only final message leaves) fails `strict_json`; a thinking block offered as the answer fails `result_contract`. It does not test the Host's analysis grammar.
- Not claiming: `claude_sdk_envelope_never_claims_confinement_or_route` pins the `not_claimed` wording and that `hosted_attempts` is carried as reported.
- Already present, not re-added (cited): `tests/revision.rs` `revision_r2_is_the_frozen_requirements_without_only_execution_confinement` (diff vs frozen = runId + both conditions' realizationConstraints and proposition; kept constraint `provider_family` `["anthropic"]`; DERIVATION reproduces r2) and `revision_r2_compiles_and_the_claude_sdk_claim_has_no_known_incompatibility`.
- `cargo fmt -p work-engine-review-hosted` was run; it also reflowed one line in `src/check.rs` (`prepare_world_for` call), no logic change.

## Step 4: three mutants - DONE, 3/3 killed (n = 1 mutant per property)

Records: `mutants/M1-model-check.md`, `M2-tool-set-check.md`, `M3-first-text-block-check.md`. Method: one `Edit` on `src/check.rs`, run `cargo test ... --test check`, inverse `Edit`, `sha256sum`. Baseline `src/check.rs` sha256 `caf32cf8b73c98576aa6fc5ac3b0232dcb26787fa44991e36f8b3e721a917a15`; restored hash equal to baseline after each mutant (and again after the earlier combined-test runs).

| Mutant | Edit | Killed by (only failure) | Restore hash |
|---|---|---|---|
| M1 | `if parsed["model"] != model` → `if false && …` (Messages path) | `claude_sdk_refuses_a_request_for_another_model`, case `wrong model` | `caf32cf8…` |
| M2 | `if names != tools` → `if false && …` | `claude_sdk_refuses_a_wrong_tool_set`, case `extra tool` | `caf32cf8…` |
| M3 | `.filter(\|b\| b["type"] == "text")` → `.filter(\|_\| true)` | `claude_sdk_refuses_a_task_that_is_not_the_first_text_block`, case `non-text block carrying the task bytes` | `caf32cf8…` |

Process notes: (a) M1 and M2 were first run against one combined test (`claude_sdk_refuses_wrong_model_and_a_wrong_tool_set`), both killed; I then split it so each mutant has its own named killer and re-ran both. The mutated-file hash for M1's first run was computed but not copied into the record, so the record states that. (b) A `cp` backup of the original file was refused by the permission mode and was not taken; restore relied on the inverse `Edit` plus the hash check. (c) For M3 only one case distinguishes the type filter (a non-text block that carries the task text); the other cases are still refused under M3 by the block-count and `text`-string checks, as `M3-*.md` says.

## Step 5: CLAIMS.md item 5 - DONE

`CLAIMS.md` rewritten: per-profile table (requested provider id, requested model, wire, revision), authority including decision D for r2, item 1 per wire, item 5 "requested provider and model per profile, nothing about the route" (serving route, payer and the `claude-sdk` subscription billing guard are Host-reported and carried verbatim, not checked; insertions recorded by digest only), item 7 (frozen world unmet on both profiles), and May-not-claim lines for confinement (unestablished on both profiles; r2 removes the constraint, it does not establish it), route, insertions, and the real-body limit from step 2.

## Step 6: final evidence and MATRIX - DONE

- `final-hashes.txt` (24 lines; paths relative to the worktree root, so `sha256sum -c` is run from the worktree root rather than `rust/`, because `cd` is not allowed in this permission mode; the packet's `rust/`-relative form differs only by the `rust/` prefix). Verified: 24 OK.
- `test-output.txt`: final test run, transcribed from the tool output (no shell redirection available).
- `h1/MATRIX.md`: "Remediation 2–4" appended.

## Not done / open

- No custody `runs/` directory or `mutate.py` run for remediation 2-4 (the packet's step list for this remediation did not include it); the evidence is the files above.
- `h1/remediation-2/RESULT.md` and `remediation-2/mutants/` were not produced by remediation 2 or 3 as far as this builder saw (not checked beyond the listing the packet gave).
- Real retained Claude Code Messages body: not available, shape not established beyond the Host fixtures.
- Nothing committed; the worktree remains uncommitted for the owner/root.

Spend: not measured by this builder (no usage readout available inside the session); tool calls not counted (unverified); no subagents, no live provider calls, no network.
