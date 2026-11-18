# H1 remediation 5: tolerate Claude Code's observed `<total_tokens>` system messages

Status: handoff_ready. Offline only; no network, Docker, live provider, commit or git mutation.

## Entry check

`sha256sum -c h1/remediation-4/final-hashes.txt` from the worktree root: 24/24 OK before any edit.

## Change (`src/check.rs`, `messages_body`)

A role-`system` message in `messages[]` is tolerated only when
(a) it is `messages[1]`, or (b) the preceding message is a `user` message whose content is a
non-empty list of only `tool_result` blocks (new helper `is_tool_results_only_user`, also used for
the existing "later user messages hold only tool results" rule, behaviour unchanged).
Its content must be a string or a list whose blocks are all `type: "text"`.
Every tolerated message is recorded by digest and never interpreted:
- `messages[1]`: `environment_message_sha256` (unchanged field).
- later ones: new `inserted_system_messages: Vec<{message_index, sha256}>` in
  `host_reported.request_insertions[]` (`#[serde(default)]`, so older envelopes still deserialize).
Any other position or content refuses with the delivery failure
`role system message at messages[i]; tolerated only at messages[1] or directly after a user message holding only tool results`
or `... holds content other than a string or text blocks`.

Two consequences to read as stated, not as relaxations:
- The content rule now also applies to `messages[1]` (previously unchecked). This is a tightening;
  the observed `messages[1]` is a text-block list or a string, so it still passes.
- "System message right after a user text message" is not separately reachable at index >= 2: such a
  user message is itself refused earlier in the same loop ("other than tool results"). At index 1 a
  system message after the task user message is the tolerated case (a). The test for this case
  therefore asserts the user-message refusal text.

Nothing else was relaxed: model, tools, task prefix + suffix, user-content, thinking and
`system[]` handling are unchanged.

## Tests (`tests/check.rs`)

- `claude_sdk_tolerates_total_tokens_system_messages_and_records_digests`: exact observed message
  structure for requests 1-3 (request 2: 5 messages with a string message at [1] and a text-block
  `<total_tokens>` message at [4]; request 3: 8 messages, string at [1] and [4], text-block at [7]);
  synthetic text. Asserts `checks_passed`, per-request `(index, sha256)` records, the
  `messages[1]` digest, and that no inserted text appears in `host_reported`.
- `claude_sdk_refuses_system_messages_outside_the_observed_positions`: system after an assistant
  message; two system messages in a row; system with a non-text block; `messages[1]` system with a
  non-text block; system with no string/list content; system after a user text message. Each case
  must fail only `delivery` and with the expected reason text.
- Existing `claude_sdk_refuses_extra_user_text_and_stray_roles` case "system message after
  messages[1]": same intent (refused); expected substring updated to the new refusal text.

Result: 60 passed, 0 failed (58 before; +2 new) - `test-output.txt`.

## Mutant R5-M1 (accept a system message at any position)

Edit: `if i != 1 && !is_tool_results_only_user(&messages[i - 1])` -> `if false && i != 1 && ...`.
With the mutant, `cargo test --test check`: 27 passed, 2 failed:
`claude_sdk_refuses_system_messages_outside_the_observed_positions` (named killer) and
`claude_sdk_refuses_extra_user_text_and_stray_roles`. Mutated-file hash not recorded.
Restore: edit reversed; `check.rs` sha256 `af1e66ed...fea6c` after restore equals the hash taken
before the mutation (exact). n = 1 mutant for this property. Not mutated: the content rule and the
digest recording (the recording is asserted by the positive test, the content rule by the
non-text-block and no-content cases; no mutant was run for either).

## CLAIMS.md

One paragraph added under claim 5 stating the tolerated positions, content, digest recording and
that nothing is interpreted.

## Limits

- The observed shape is taken from the root's description of three retained bodies; I did not open
  `e2e/org-step3-offline-1/state/*.body.json`, so the packet's structure is relied on, not
  re-verified by me. The Host source `pi_coding_hosted.rs@5af34cbf` was not read; the crate does not
  depend on a claim about it beyond the existing comments.
- The end-to-end run that failed was not re-run (no Docker/live); only the synthetic structure is
  tested.
- Other `<total_tokens>`-style insertions in shapes not listed (for example after an assistant
  message, or with `tool_result` plus text in one user message) remain refused by design.

## Files

`final-hashes.txt` (24 files, same form as remediation 4; run `sha256sum -c` from the worktree
root), `test-output.txt`, one line appended to `h1/MATRIX.md`. Changed vs remediation 4:
`CLAIMS.md`, `src/check.rs`, `tests/check.rs`.
