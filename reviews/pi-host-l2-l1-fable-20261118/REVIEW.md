# Review: Pi Host L2 + L1 (`git diff 5af34cbf 9d048765`)

Worktree `/home/bline/cbm-work/wt-pi-host-realization-20261118`, branch
`pi-host-realization-20261118`, HEAD `9d048765` (commits `40722e42` L2,
`9d048765` L1). Read-only review; no repository file was modified except this
file. Builds used `CARGO_TARGET_DIR=/home/bline/.cache/fable-l2l1-target`.

## Verdict: accept with changes

No P1. Both rulings are implemented as stated and the tests exercise the
production seams. One P2 (the "verified absence" check parses the inner
`metadata.user_id` JSON non-strictly, so a body whose bytes carry a populated
`account_uuid` can still be marked verified-absent) should be corrected before
L1 is relied on against anything other than the pinned Claude Code client;
the rest are P3 notes.

## Findings

### P2 — `messages_account_identifier_absent` is not a byte-level verification: duplicate keys and one-level nesting let an identifier through

Evidence (verified by running a probe against `serde_json =1.0.151`, the
workspace pin, outside the repo):

- `rust/crates/native-realization/src/lib.rs:62` parses the `user_id` string
  with `serde_json::from_str::<Value>(s)`. For
  `{"account_uuid":"5f0c1a2e-…0001","account_uuid":""}` the probe printed
  `parsed account_uuid = String("")` (last key wins), so `lib.rs:73` returns
  `true`: the Host's `messages_policy`
  (`work-engine-manual-host/src/pi_coding_hosted.rs:814`) dispatches the body,
  `build_capture` (`pi_host_serve.rs:912-918`) reports
  `contains_account_identifier: false`, and the runner's re-check
  (`lifecycle-service/src/operations/pi_host.rs:489-493`) retains it — while
  the retained bytes literally contain the populated UUID.
- The Host parses the *outer* body strictly for exactly this reason
  (`pi_coding_hosted.rs:719-726`, "a duplicate key could mean one thing here
  and another upstream"); the inner object is not held to that rule.
- Same mechanism one level deeper: the probe for
  `{"account_uuid":"","profile":"{\"email\":\"x\\u0040y\"}"}` printed
  `contains '@' = false`; the `email` walk (`lib.rs:44-52`) sees only the
  outer-decoded string, so an escaped `@` inside a doubly-encoded string
  passes. The ruling's wording ("false only when verified, not a heuristic")
  and the protocol text (`PI_HOST_PROTOCOL.md:128-135`) describe a
  verification; the implementation verifies one parse level.

This is reachable only from a client that deliberately crafts `metadata`;
Claude Code in the pinned container does not emit it, which is why it is P2
rather than P1.

Correction (smallest): in `messages_account_identifier_absent`, refuse the
`user_id` string unless it parses with duplicate-key rejection (a ~20-line
`StrictValue` visitor like `lifecycle-wire/src/json.rs:80-99`; `native-realization`
is a leaf crate, so copy rather than depend), and tighten the walk so that
any string under `metadata` or `user` that itself parses as a JSON object or
array is recursed into (or simply refused). Add the duplicate-key and
double-encoded cases to `account_identifier_absence_is_verified_not_assumed`
(`lib.rs:425`). Because the function is shared, the Host, the capture and
the runner tighten together; `PROTOCOL_DIGEST` need not change unless the
protocol text is amended.

### P3 — Protocol text and code differ on `account_uuid: null`

`PI_HOST_PROTOCOL.md:132-133` refuses when `account_uuid` is "present and
not empty"; `lib.rs:68-71` treats `null` as absent (`None | Some(Value::Null)
=> true`), and the record README says "absent, `null` or empty". A `null`
identifier is not an identifier, so the code is the sensible side; but the
digest-pinned text is the contract. Correction: either amend the protocol
sentence to "present and neither `null` nor empty" (re-pin
`PROTOCOL_DIGEST`, the `protocol_digest_is_the_spec_digest` test catches a
stale pin) or drop the `Value::Null` arm.

### P3 — The absence check keys on two names only; `metadata` sub-keys are otherwise unconstrained

`messages_policy` whitelists top-level keys (`pi_coding_hosted.rs:59`,
`:800`) but `metadata` may carry any sub-key; the check refuses only
`account_uuid` inside `user_id`, keys containing `email`, and strings with
`@` (`lib.rs:44-73`). `{"metadata":{"account_uuid":"5f0c…"}}` or
`user_id` = `{"organization_uuid":"…"}` pass. This meets the ruling as
worded (it names `account_uuid` and email), so report-only. Correction, if
the owner wants "no account identifier" rather than "no `account_uuid`":
restrict `metadata` to the single key `user_id` and `user_id` to the
observed key set `{device_id, account_uuid, session_id}` — the record already
states that is the whole observed shape.

### P3 — `pi_host_q_delivery_verifies` "changed-read-content" case is fixture-fragile

`research_delivery.rs:1678-1681` searches each payload for the raw 16 bytes
`material[40000..40016]` and `continue`s when absent. If a future A4
derivation places a JSON-escaped byte (`"`, `\`, newline) in that window the
marker matches no payload, no byte is flipped, `verify` succeeds and
`.err().unwrap()` at `:1689` panics. That is a loud failure, not a false
pass, so the negative case cannot pass for the wrong reason — but the test
will break for a reason unrelated to what it checks. Correction: assert
`flipped > 0` after the loop with a message, or pick the marker from the
decoded `tool_result` text of a span rather than from raw material.

### P3 — Fixture `Owner::original_result_operation` omits the production scope gate (pre-existing, now on the L2 path)

`lifecycle-service/tests/pi_host_join.rs:533-546` returns the result
operation after a subject check only, whereas the production port
(`retained-task-workflow/src/port.rs:15-33`) first refuses unless the store
carries the exact controlled research causal scope. L2's new causal-read loop
(`pi_host_join.rs:468-499` → `actor.rs:2763`, `:2780`) therefore runs
without that authorization in the loop. Not introduced by this diff and the
gate is unit-tested in `port.rs` (`causal_scope_tests`), so report-only; the
README's "Open" bullet already notes the read is not through the socket's
`acquire`. Correction if desired: wrap the fixture store with
`with_controlled_research_causal_scope("task:input","subject:input")` and
forward the gate as `port.rs` does.

## Verified correct

Verified by running (all with the main-disk target dir):

- `sha256sum rust/crates/native-realization/PI_HOST_PROTOCOL.md` →
  `8ec540f8a6f5da8df2f600528b232a04d57f39b09e105c4ccd8e1cee5dffb685`, equal
  to `pi_host::PROTOCOL_DIGEST` (`lib.rs:22-23`); the pin test
  `protocol_digest_is_the_spec_digest` passes.
- `cargo test -p native-realization`: 6 passed (incl.
  `account_identifier_absence_is_verified_not_assumed`).
- `cargo test -p work-engine-manual-host --lib`:
  `capture_account_flag_is_the_verified_absence_check` and
  `messages_body_policy_and_url_are_enforced` pass (2 passed, 107 filtered).
- `cargo test -p lifecycle-service --lib --features controlled-proof
  capture_must_state_the_launched_identities`: passes (includes the new
  flag-true/absent refusals and the populated-`account_uuid` retained-body
  refusal).
- `cargo test -p work-engine-task-host --features controlled-proof --lib
  projection_tests`: 6 passed, 1 ignored (`pi_host_q_delivery_verifies`
  needs a run directory; Docker-dependent runs are out of scope).
- serde_json duplicate-key / escaped-`@` probe (basis of the P2).

Verified by reading:

- Codex unchanged: the only `lib.rs` hunks are in `mod pi_host`
  (`git diff 5af34cbf 9d048765 -- rust/crates/native-realization/src/lib.rs`);
  `codex::NATIVE_BINARY_SHA256`, `SCHEMA_THREAD_START_SHA256`,
  `CAUSAL_SCOPE` (`lib.rs:9-15`) are untouched.
- Policy before dispatch: `messages_policy` is called at
  `pi_coding_hosted.rs:744` inside the `provider_fetch` validation that
  returns the `Fetch` the Host dispatches; the account check sits at `:814`
  ahead of the tools checks; the test asserts `w.opened` is empty for the
  two new `account-identifier` cases (`:2871-2879`, loop at `:2888-2896`),
  i.e. no upstream connection was opened. The failover resend re-sends the
  same validated body.
- No request number is skipped in the capture: `build_capture` iterates
  `a.first_request..=last_request` (`pi_host_serve.rs:899`), sets the flag
  true on a missing ledger record (`:902-903`) and on any unread/unparsable
  body (`:912-918`); `account` starts `true` for non-Messages wires
  (`:893`); the flag is emitted once (`:1009`) and no other code path sets
  it (`grep contains_account_identifier`).
- Runner: `check_capture` refuses a Messages capture whose flag is anything
  but `false` (`pi_host.rs:443-445`) and re-runs the shared check over every
  retained payload's bytes (`:489-493`); Responses payloads are refused
  outright (`:495-497`). The negative tests fail for the right reasons: the
  base fixture is `Ok` and eligible before mutation (`:971-974`), the flag
  mutations touch only the flag (`:1026-1030`), and the retained-body case
  changes only `metadata` with the digest recomputed (`:1031-1047`), leaving
  the pinned tools intact.
- Protocol text vs code: the "Account identifier" section
  (`PI_HOST_PROTOCOL.md:128-135`) matches `lib.rs:42-73` except for the
  `null` nuance in P3 above; the eligibility sentence (`:116-118`, `:124`)
  matches the Host and runner behaviour; "Device and session ids … remain"
  matches the record's "Present and retained" paragraph.
- L2 exercises production code: `pi_host_join.rs` is compiled into
  `actor.rs` (`#[path]` at `actor.rs:5177-5178`), drives
  `handle_native_message` for every `NativeMessage::Tool`, and the actor's
  snapshot path commits via `port.record_completed_effect`
  (`actor.rs:2184-2199`). `verify_delivery` is the private production
  function (`research_delivery.rs:599`) called directly by the test in the
  same module; the four refusal reasons asserted (`:1699-1707`) are
  production strings (`:659`, `:836/:906`, `:864`, `:923`).
- Owner adapter fidelity: `lookup_completed_effect`
  (`pi_host_join.rs:550-585`) and `record_completed_effect` (`:586-640`)
  mirror `port.rs:37-67` and `:88-120`/`:186-216` — same length and
  receipt-digest check before classification, same `MissingResult` /
  `NonUtf8` / `OverInlineLimit` (4 KiB) / `OverEscapedLimit` (8 KiB escaped)
  thresholds, same `completed-snapshot-read-attribution-v2` schema and
  `result_sha256 = receipt_sha256`, same membership check against
  `record.completed_effects`. The only divergence is the pre-existing error
  map (`:522-531`) folding SQLite-busy storage errors into `Uncertain`
  instead of `Busy`; irrelevant to the single-threaded fixture.
- Record/code consistency: "No production code changed for L2" — `git show
  40722e42 --stat` touches only README, `tests/pi_host_join.rs`, and the
  `projection_tests` module of `research_delivery.rs`. 112 497 bytes →
  `div_ceil(32768) = 4` reads, and the test asserts `reads == 4 + 1`
  (`pi_host_join.rs:497-498`) matching "five relayed calls". The README's
  descriptions of the flag semantics (Responses → true; missing record or
  unread body → true), the runner's refusals, the stale-fixture note, and
  the "Open" bullet (causal read direct, not via `acquire`) match the code.
  Run digests (`summary.json`, `retained.json`, `verify-delivery.json`)
  live outside the repo and were not re-derived.
