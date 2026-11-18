# Pi Host realization H1-H5: qualification record

Standing: dev-vertical qualification record for H1-H5 of the
[Pi Host realization plan](../../planning/dev-vertical-pi-host-realization-20261117.md)
(P0 placement A, [sizing](../../planning/research-lifecycle-h2-pi-realization-sizing-20261117.md)
section 3). Branch `pi-host-realization-20261118`. Not owner acceptance; no
lifecycle-service, store or task-host code yet (L, S, T, Q follow). Nothing
pushed. Offline only: no live inference was run for this record.

## What was built

- `native-realization` (new leaf crate): the Pi Host lifecycle protocol
  ([PI_HOST_PROTOCOL.md](../../rust/crates/native-realization/PI_HOST_PROTOCOL.md),
  digest pinned as `pi_host::PROTOCOL_DIGEST`), realization identity, bounds
  and the pinned lifecycle tools (`snapshot_read`; `question_open`,
  `result_publish`) in receiver and `dynamicTools` form.
- `work-engine-manual-host`:
  - lifecycle relay profile `work-engine.pi-lifecycle-hosted.v1` (analysis
    family: Host-held access and dispatch, no workspace effect); thread
    deadline up to 3 600 s (H4); request tools pinned per wire (the Messages
    digest `7e97ed24…` was taken from the first run's refusal and equals the
    digest of the array rebuilt from `pi_host::tools`; the Responses digest
    is unobserved and its zero value refuses every request);
  - H2 relay: Pi's tool frame goes to the gate's new `relay` method; intent
    persisted first, the controller's answer retained create-only, Pi's
    native completion must carry exactly that text;
  - H1/H3/H4 controller mode (`pi_host_serve.rs`, binary
    `work-engine-pi-host serve --config`): handshake, one container per
    thread with one Pi prompt per turn in the same session, `item/tool/call`
    with the Host-added namespace, Host-ledger items and `thread/read`, the
    per-turn capture (payload per physical send, response streams, calls
    joined to exactly one model tool call in a completed attempt's retained
    response with its response id, results by digest, Pi's history by
    digest, eligibility reasons), per-turn bounds (64 sends, 8 MiB bodies,
    64 KiB answer, 1 000 000-byte history frame), provider attempts fenced to
    the active turn;
  - H5 `pi-host/observe` and the capture's observation: a fresh `docker
    inspect` checked by the Host's launch containment verifier, running, no
    `/auth` mount.
- receiver: the relay profile registers the pinned tools from the job file
  (`Type.Unsafe` of the pinned JSON schema) and returns each answer
  unchanged.
- `pi_hosted_messages_observer` scenario `relay`; harness [serve.py](serve.py).

## Runs

Run root `~/.local/state/work-engine/dev-vertical-pi-host-realization-20261118/offline`,
provider `claude-sdk` (Claude Code in the container, Messages), synthetic
loopback routes and synthetic Host-held access.

| Run | Result |
| --- | --- |
| `r4` (`relay`) | Exit 0, receipt `component_passed`, no failures, no containers left. Observation verified before turn 1 (network none, read-only root, user 1000:1000, no `/auth`). Turn 1: request 1 refused on the primary (429) and completed on the secondary with the same body; relayed `snapshot_read` and `result_publish` (namespaces `workspace`, `task`); final answer. Turn 2: one request whose body extends turn 1's messages (same Pi session). Both turns `completed` with `error: null`; both captures eligible, observation verified, calls joined (`request-1` attempt 2 `msg_1`; `request-2` attempt 1 `msg_2`). |
| `n1-controller-refuses` (`relay`, `PI_HOST_CONTROLLER_REFUSE=1`) | The controller answers the relayed call with an error: the Host job fails closed ("controller refused tool call toolu_1_0"), the turn is reported `failed` with that reason, the capture is ineligible, `thread/read` shows the failed turn with no items, no containers left. |
| `r1`-`r3` | Development runs: r1 found the Messages tools digest (refused before retention, as designed); r2 passed before H5; r3 found the observation's oversize-artifact read of the Docker client (fixed: file digest). |

Unit tests: `native-realization` (protocol digest, tool pins),
`pi_host_serve` (handshake refusals, a full turn through the gate with
capture and `thread/read`, unrelayed and missing calls, Responses call
join).

## Review remediation and L, S, T, Q (second commit)

Fable review of `047dba58`:
[REVIEW.md](../../reviews/pi-host-h1-h5-fable-20261118/REVIEW.md) (accept
with changes). Dispositions:

| Finding | Change |
| --- | --- |
| P1 relay thread killed at 300 s | The inference process wall bound is the relay thread bound for relay jobs (`process_wall_seconds`, unit test); run `r6-pause330` idles 330 s between turns and both turns complete (375.8 s). |
| P2 no `userMessage` | Protocol: turns are domain work; the ledger carries no `userMessage`. |
| P2 capture does not bind the input | Capture `turn.prompt_sha256`; the lifecycle-service runner requires it to equal the prompt it sent. |
| P2 missing response stream silently omitted | Eligibility reason for any attempt relayed to Pi without its retained stream. |
| P2 `MAX_TURNS` undocumented | Protocol: at most 16 turns per thread. |
| P3 `success: false`, duplicate final text | `success: false` makes the capture ineligible (the store settles only successful calls); protocol states `completed` is the Host's verdict. |
| P3 request handling during a turn | Protocol states it. |
| P3 notify order, inspect buffering, over-built parts | No change (accepted as is by the review). |

Also: the capture names the job's nonsecret route table; the Messages tools
pin and the canonical continuity rule moved to `native-realization`
(`pi_host::tools::MESSAGES_TOOLS_SHA256`, `pi_host::canonical`), shared by the
Host and the task host.

S (ported from `claude-realization-20261117` `cff1cb5c`/`704bd087`, Claude
Host replaced by Pi Host): Codex digests moved to `native-realization::codex`
and re-exported unchanged; `Realization {Codex, PiHost}`;
`settle_native_turn` positive allow-list (every Pi grammar digest still
refused); causal scope by protocol digest with the Codex bytes pinned.

L: `lifecycle-service` `operations/pi_host.rs`: route `pi-host-v1`,
`<root>/pi-host.json` (Host binary, serve configuration and job by digest,
state directory inside the native root), loopback routes only under
`controlled-proof`, session binary identity from Host and job, protocol
digest `pi_host::PROTOCOL_DIGEST`, no successor; runner: handshake,
`thread/start`, `pi-host/observe` check, turns over the eight actor
messages, capture checked (identities, prompt digest, payloads re-read inside
the state directory) into `CaptureHostInput`; an orderly Host close (end of
input, then wait) before any kill, because a killed Host leaves its
container (found by Q run `q2`).

T: `work-engine-task-host` `verify_delivery` dispatches on the session's
realization; Pi Host on the Messages wire uses the `mcp__pi__snapshot_read`
projection (no text fallback), request capability checks and the shared
continuity rule; a Pi Host input on another wire is refused
(`pi-host-wire-unqualified`).

Q: `lifecycle-service` `tests/pi_host_join.rs` (ignored; needs Docker, the
Host binary, the `claude-sdk` runtime and the observer): task owner,
actor, Pi Host runner, the Host and Pi in the container, observer
scenario `steps` (`result_publish`, then the answer). Run `q3`: result
published through the actor, causal record with the Pi Host scope, input
eligible, `CompletedSettled`, causal read returns the operation, no Host
failures, no container left.

## Second review remediation (third commit)

Fable review of `fab290f4`:
[REVIEW.md](../../reviews/pi-host-lst-q-fable-20261118/REVIEW.md) (accept
with changes).

| Finding | Change |
| --- | --- |
| P1 T continuity refuses every Host resend (one payload per physical send) | Sends of one inference call must be byte-identical; the canonical extension rule applies only across calls; unit test with a failover resend, a differing resend and two calls with one body. |
| P2 L close bound shorter than a turn | Mid-turn the orderly-close wait covers the turn bound; the Host now notices end of controller input during a turn at the next provider attempt or Pi frame and ends the job (protocol text; unit test). |
| P2 L request tools only Host-asserted | The runner digests the `tools` of every retained Messages body against the shared pin; any other wire refuses. |
| P3 L capture bindings | `request-<n>` must name `coding-hosted-<n>.body.json`; `(call, send)` unique; every call names a retained send and a verified observation is required for eligibility; unit tests. |
| P3 L vs Codex runner | `thread.ephemeral == false`, `thread/read` turn `completed`, a stray response recorded before the conflict, a failed reap reported `cleanup_unobserved`. |
| P3 S unreachable protocol comparisons | Kept as the explicit fence with a comment (the profile comparisons are not unreachable). |
| P3 T redundant capability call | Removed (the selected request is one of the payloads). |
| P3 Q record | Command below; `summary.json` `binary` is the session binary identity (Host and job), not the Host file digest. |

Q command (offline, Docker required):

```
cd rust && WE_PI_HOST_OUT=<fresh dir> WE_PI_HOST_BIN=target/debug/work-engine-pi-host \
  WE_PI_HOST_OBSERVER=target/debug/examples/pi_hosted_messages_observer \
  WE_PI_HOST_RUNTIME_DIR=~/.local/state/work-engine/dev-vertical-h2-route-20261117/runtime-claude-sdk \
  WE_PI_HOST_RECEIVER_DIR=crates/work-engine-manual-host/receiver \
  cargo test -p lifecycle-service --features controlled-proof --lib pi_host_causal_join -- --ignored
```

Runs on the remediated code (2026-11-18, offline): `q4` from the worktree
that became `3f8007d3` (composed test with the command above: passed;
eligible, `CompletedSettled`, 3 wire payloads including the failover
resend, protocol `604f1a10…`; `summary.json` sha256 `02e38c9f…`) and `r7`
(`serve.py` `relay`: both turns completed and eligible, no failures, no
container left).

## Recheck (fourth commit)

Fable recheck of `3f8007d3`:
[REVIEW.md](../../reviews/pi-host-lst-q-recheck-fable-20261118/REVIEW.md)
(accept with changes; P3 only). Changes: the q4 and r7 runs recorded above
(P3-1); the runner's close wait ends at the active turn's own bound (set
before `turn/start`, so a turn whose binding failed is covered) or 120 s,
whichever is later (P3-2, P3-3); the task host refuses a call number that
goes backwards (`messages-request-call-order`, unit test; P3-5). P3-4
(identical bodies across distinct calls are refused) is kept fail-closed and
noted. Run `q5` from the worktree that became the fourth commit: passed
(eligible, `CompletedSettled`; `summary.json` sha256 `81287463…`).

## Owner ruling 5900 (fifth commit)

Research-lifecycle owner ruling (board 5900; review
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/015-pi-host-review/REVIEW.md`):
accept with required changes.

| Item | Change |
| --- | --- |
| R1 thinking rule at the Host-ledger seam | `ledger_items` accepts a `thinking` block only with exactly `{type, thinking, thinkingSignature}`, all strings; the unit test carries `thinkingSignature`; negatives for a missing signature, an extra (redacted) key, a non-string signature and `redacted_thinking`. The shared canonical rule compares `thinking` exactly and refuses any request containing `redacted_thinking` (in either request). |
| R2 causal fence fail-open | `retain_native_causal` matches Codex and Pi Host only; an unknown session protocol is `Rejected`; the test now expects the refusal; the Codex record-byte pin is unchanged. Three store test fixtures that retained causal records under the literal protocol `hash` now bind the Codex digest. |
| R3 C1(iii) negative | Test: an altered earlier `tool_result` content refuses. |
| F6 (record) | The Pi Host's tool pin is `dynamicTools` equality at `thread/start` plus the request tools digest pins (Host before dispatch, runner over retained bodies); it is not the Codex `ExpectedToolSet`. |
| L3 (record) | The startup hold stays refused until R9′. The Responses wire stays refused for the Pi Host: 5879(3) "both wire families" is met by Codex for Responses and the Pi Host for Messages. |

Pending before any live or delivery claim: L1 (account-identifier rule and
the runner's check; operator consent on email retention pending) and L2 (a Q
run relaying `snapshot_read` through the actor and `verify_delivery` over
the real Pi payload). F4 (per-request attempts in capture metadata) is
recommended and not yet done.

Runs from the worktree that became the fifth commit: `q6` (composed test:
passed, `summary.json` sha256 `66619b0a…`) and `r8` (`relay`: both turns
completed and eligible, no container left).

## L2: snapshot reads through the actor and `verify_delivery` over the Pi payload

Owner acceptance 5902 required L2 before any live H2. Composed offline:

- `lifecycle-service` `tests/pi_host_join.rs` `pi_host_research_delivery`
  (ignored; `WE_A4_FIXTURE` in addition to the Q variables above). The
  snapshot member is the derived A4 research material
  (`request\ncard\nreconstruction`, 112 497 bytes). The observer scenario
  `steps` makes Pi call `mcp__pi__snapshot_read` four times (32 KiB ranges),
  relayed by the Host to the actor, which executes the reads against the
  store and commits each read's effect to the task owner. Pi then publishes
  a recommendation (`{dependencies, text}`, citing the target fact) and
  answers. The test writes the actor's scoped causal read (paged as the task
  host's `acquire` pages it) and the owner selector map.
- `work-engine-task-host` `projection_tests::pi_host_q_delivery_verifies`
  (ignored, `controlled-proof`: card reconstruction is available only there).
  It runs `verify_delivery` over that retained delivery, using the owner's
  original evidence from the run's task store.

The earlier "no material port" gap was the Q fixture's task-owner adapter:
it refused `record_completed_effect`, so every actor `snapshot_read` was
refused (run `q7`: the Host ended the job, "controller ended during a
relayed call"). The adapter now forwards both effect methods to the
`TaskStore`, as `retained-task-workflow`'s port does. No production code
changed for L2.

The A4 fixture must be derived from the current source
(`WE_A4_FIXTURE_OUT=<dir> cargo test -p work-engine-task-host --features
controlled-proof --test research_fixture derive_full_a4_fixture -- --ignored`).
The 2026-11-13 copy under
`research-lifecycle-task-host-join-implementation-20261114/003-builder/a4`
has a stale `reconstruction.json` (96 429 bytes; current 96 387), which
`card_host::reconstruct` refuses.

Runs (2026-11-18, offline): `q8` used the stale fixture (Q passed, and the
delivery check refused at reconstruction). `q9` used the current fixture:

- Q passed: eligible, `CompletedSettled`, seven wire payloads (the
  failover resend of `request-1` and `request-2`..`request-6`), and five
  relayed calls (four reads and the publication). `summary.json` sha256
  `0065c0a3…`, `retained.json` `4fe771eb…`.
- `verify_delivery`: `complete`. The selected request is `request-5`
  (send 1, `msg_5`), the one that produced `result_publish`. The projection
  is the four `tool_result` reads joined to their `tool_use` ids, equal to
  the material (sha256 `519b57f3…`). Each of the request, the card and the
  reconstruction is uniquely visible.
- Refusals over the same retained delivery: ineligible input
  (`native-input-not-eligible-checked-completed`); unknown protocol digest
  (`unknown-realization-protocol`); metadata wire `responses`
  (`pi-host-wire-unqualified`); one changed byte of read content in every
  wire body, with digests recomputed
  (`snapshot-call-identity-content-or-range-mismatch`).
  `verify-delivery.json` sha256 `a8e07b4f…`.

`q10`: `pi_host_causal_join` (publication only) with the forwarding adapter
passed (`summary.json` sha256 `d6ebc5b9…`).

## L1: account identifier absence (owner ruling 5904)

The revised L1 replaces L1 in 5900. No email-retention consent is needed,
because the design keeps the identifier out:

- `native-realization` `pi_host::messages_account_identifier_absent` is a
  single check shared by the Host and the runner. Absence is verified when:
  `metadata` is absent, or `metadata` is an object whose `user_id` (if
  present) is a string holding a JSON object, its `account_uuid` is absent,
  `null` or empty, no key under `metadata` names an email, and no string
  under it contains `@`. Anything else counts as not verified.
- The Host's Messages request policy (`messages_policy`) refuses a body that
  fails the check, before dispatch. That covers every hosted Messages job,
  not only the Pi Host.
- The capture's `contains_account_identifier` is the result of that check
  over every request body of the turn. It is false only when each body was
  read and verified; a missing ledger record or an unread body makes it true.
  On the Responses wire it is true, because the check covers Messages only.
- The lifecycle-service runner refuses a Messages capture unless the flag is
  `false`, and re-runs the check on every retained body.
- Protocol: a new "Account identifier" section, and the eligibility and
  capture text changed, so `PROTOCOL_DIGEST` is now `8ec540f8…` (was
  `604f1a10…`). The Codex digests are unchanged.
- Tests: unit tests for the shared check (observed shape passes; a populated
  `account_uuid`, an email key, `@` in a value, non-JSON, a non-string
  `user_id` and a non-object `metadata` all refuse), Host policy cases (a
  populated `account_uuid` and an email field refuse with nothing opened),
  the capture flag (true for a populated `account_uuid` and for an unread
  body), and the runner (flag `true` or absent, and a retained body with a
  populated `account_uuid`, both refuse).

Present and retained: Claude Code's `metadata.user_id` in every observed
offline Messages body carries `device_id` (64 hex characters) and
`session_id` (a UUID) alongside the empty `account_uuid`
(`q9` `coding-hosted-1.body.json`). These stay in the retained bodies.

Runs on the L1 build (2026-11-18, offline):

- `q11` (`pi_host_research_delivery` plus `pi_host_q_delivery_verifies`):
  protocol `8ec540f8…`, eligible, `CompletedSettled`, seven payloads, flag
  `false`. `verify_delivery` is `complete` (`request-5`, four spans) with the
  same four refusals. sha256: `summary.json` `6aa9ba6a…`, `retained.json`
  `4022c8dc…`, `verify-delivery.json` `0900c933…`.
- `q12` (`pi_host_causal_join`): passed, flag `false`; `summary.json`
  `a537e00c…`.

Unit tests at this commit: `native-realization` 6/6, `work-engine-manual-host`
lib 103 passed (6 ignored), `lifecycle-service` lib with `controlled-proof`
97 passed (2 ignored), `lifecycle-store` lib 33 passed (2 ignored),
`work-engine-task-host` `research_delivery` with `controlled-proof` 6
passed (2 ignored).

## L2 and L1 review remediation

Fable review of `5af34cbf..9d048765`:
[REVIEW.md](../../reviews/pi-host-l2-l1-fable-20261118/REVIEW.md) (accept
with changes, no P1). An earlier attempt died when its build directory
filled `/tmp`; it produced no review.

| Finding | Change |
| --- | --- |
| P2: the inner `user_id` JSON was parsed non-strictly (a duplicate `account_uuid` read as the last value) and only one level deep (a doubly-encoded email passed) | The shared check parses `user_id` strictly, with duplicate keys refused (a copy of `lifecycle-wire`'s strict visitor; the crate is a leaf). It also decodes every string under `metadata` that is itself JSON, strictly and at most four levels deep, and applies the email, `account_uuid` and `@` rules at every level. Duplicate keys or deeper nesting count as not verified. Unit tests cover the duplicate `account_uuid`, an escaped email key, an escaped `@`, a nested `account_uuid`, a nested duplicate key and six levels of nesting. Because the Host, capture and runner share this function, all three tighten together. |
| P3: the protocol said "present and not empty" but the code treats `null` as absent | The protocol now says "anything other than `null` or an empty string", with a test for `null`. With the P2 text, `PROTOCOL_DIGEST` is now `5e0e4f3e…` (was `8ec540f8…`). |
| P3: `metadata` sub-keys are otherwise unconstrained (for example an `organization_uuid`) | Not changed: the ruling names `account_uuid` and email. An allow-list (`metadata` = `{user_id}`, `user_id` = `{device_id, account_uuid, session_id}`, the whole observed shape) is offered to the owner. |
| P3: the changed-read-content negative depends on the fixture | The test now asserts that the marker was found and a byte changed. |
| P3: the Q owner adapter has no causal-scope gate (pre-existing) | Not changed: the gate is a private `TaskStore` field that the fixture cannot forward; it is unit-tested in `port.rs`. Recorded under Open. |

Runs on the remediated build (2026-11-18, offline):

- `q13` (`pi_host_research_delivery` plus `pi_host_q_delivery_verifies`):
  protocol `5e0e4f3e…`, eligible, `CompletedSettled`, seven payloads, flag
  `false`. `verify_delivery` is `complete` (`request-5`, four spans) with the
  same four refusals. sha256: `summary.json` `db3ea741…`, `retained.json`
  `14a04fdc…`, `verify-delivery.json` `03837bf8…`.
- `q14` (`pi_host_causal_join`): passed, flag `false`; `summary.json`
  `06e68c6e…`.

Unit tests: `native-realization` 6/6, `work-engine-manual-host` lib 103
passed (6 ignored), `lifecycle-service` lib with `controlled-proof` 97 passed
(2 ignored), `lifecycle-store` lib 33 passed (2 ignored),
`work-engine-task-host` `research_delivery` with `controlled-proof` 6 passed
(2 ignored).

## Owner 5911: closed metadata shape

The owner accepted the allow-list offered for the open P3, using the same
closed-observed-shape rule as the thinking blocks (5886). The shared check
now also requires that `metadata`, when present:

- has exactly the key `user_id`;
- holds a strictly parsed object whose keys are among `device_id`,
  `account_uuid` and `session_id`;
- has `device_id` and `session_id` as strings without `@`;
- has `account_uuid` as `null` or `""`.

Anything else is not verified. The earlier checks remain on top: nested JSON
strings are decoded, and the email, `account_uuid` and `@` rules apply at
every level, so an encoded value inside an allowed key still refuses. Host
policy, capture flag and runner still use this one function. The protocol
section was rewritten, so `PROTOCOL_DIGEST` is now `9a33912a…` (was
`5e0e4f3e…`); Codex is unchanged.

New negatives: an extra `metadata` key; an extra `user_id` key
(`organization_uuid`), in the shared unit test and in the Host policy test
with nothing opened; `metadata: {}`; a non-string `device_id`; a `null`
`session_id`; and encoded values inside allowed keys (an escaped `@`, a
nested `account_uuid`, a nested duplicate key).

Runs on this build (2026-11-18, offline). Claude Code's observed body
`metadata` is exactly the allowed shape:

- `q15` (`pi_host_research_delivery` plus `pi_host_q_delivery_verifies`):
  protocol `9a33912a…`, eligible, `CompletedSettled`, seven payloads, flag
  `false`. `verify_delivery` is `complete` (`request-5`, four spans) with the
  same four refusals. sha256: `summary.json` `5493d96a…`, `retained.json`
  `e1e71d4c…`, `verify-delivery.json` `8e1285fc…`.
- `q16` (`pi_host_causal_join`): passed, flag `false`; `summary.json`
  `01eee491…`.

Unit tests: `native-realization` 6/6, `work-engine-manual-host` lib 103
passed (6 ignored), `lifecycle-service` lib with `controlled-proof` 97 passed
(2 ignored), `lifecycle-store` lib 33 passed (2 ignored),
`work-engine-task-host` `research_delivery` with `controlled-proof` 6 passed
(2 ignored). `cargo fmt --check` is clean for the two changed crates.

## H2 live preparation (owner 5923)

Branch `pi-host-h2-live-prep-20261118`, cut from the shared branch at
`bf64a1e3`. Scope is the owner's authorization 5923 of the sizing 5922/5924.
No live inference was used.

| Item | Change |
| --- | --- |
| M1 per-turn forwarder count | On a relay job with `claude-sdk`, the receiver adds `forwarderRefused` (its count of Claude Code requests the loopback forwarder answered locally) to every `coding_provider_request` and `coding_history` frame. The analysis profile's frames are unchanged. The Host's relay gate requires the count on `claude-sdk` frames and refuses a decrease, or the field on another provider. A count above zero is an eligibility reason in the turn's capture, so the turn fails before the controller settles it. The job-completion check is kept. |
| R1 serve record | `serve` retains its record (status, run receipt or error, attempt log) as `pi-host-serve-record.json` in the job's state directory, create-only, once that directory exists (admission can fail before it does). A refused finish is recorded as `refused` with its error. stderr still carries the record. |
| R2/F4 attempts | The capture lists every attempt of the turn's provider requests as `attempts` (`request`, `attempt`, `disposition`, `route`, `payer`), including one that was never dispatched. The runner copies `attempts` into the input metadata. |
| C1(vii) | On `claude-sdk`, admission refuses unless the runtime manifest has `node_modules/@anthropic-ai/claude-agent-sdk-linux-x64/claude` as a file with SHA-256 `0dc4f4a9…` (`CLAUDE_CODE_SHA256`). Admission already verifies the runtime tree against that manifest. |
| Comment | `ProviderPlan::provider`: `claude-sdk` is admitted on the analysis or the lifecycle relay profile. |
| Protocol (owner side) | Capture: the `attempts` list. Eligibility: on `claude-sdk`, the forwarder answered no request, by the count reported with each provider request and at the turn's end. `PROTOCOL_DIGEST` `56c00a88…`; Codex unchanged. |
| Live variants (owner side, test-only) | `lifecycle-service` `tests/pi_host_join.rs`: `prepare_host_job` and `config` take the job parameters (`JobParams`: routes, attempts, deadline, requests, budget, context bytes, tool calls, turn deadline), the snapshot and the grant expiry (the job deadline). One `drive` composes and runs a thread for the offline and the live tests. The grants are written as `grants.json`, and the task and subject go into `summary.json`. New `pi_host_live_arm` (ignored): an arm file `WE_PI_HOST_ARM` (`work-engine.pi-host-arm.v1`: name, snapshot id and source revision, members by path pinned by SHA-256, the task's kind, objective, question and facts, an optional dependency-to-fact `mapping`, and `JobParams`). It has no observer and refuses to start unless `WORK_ENGINE_HOSTED_CREDENTIALS` names a file and every route is `https:`. It writes the observed outcome before the closed-thread, eligibility and settlement assertions, records the relayed call count instead of asserting it, and writes `retained.json` and `mapping.json` for an arm with a mapping. `work-engine-task-host` `research_delivery.rs`: `pi_host_live_delivery_verifies` (ignored). Both delivery tests take the task and subject from the run's summary. The live one records whether the spans follow the 32 KiB plan (`spans_follow_plan`) instead of asserting it. The changed-byte control takes its 16-byte marker from the decoded projection: bytes JSON leaves unescaped, once in the projection and once in some wire body. |
| R3 (offline) | `serve.py` scenario `treatment` and observer `steps` entries with several tool calls in one response; see below. |

### R3: treatment-size relay check

`r10-treatment` ran with the H2 job parameters (`max_requests` 32, budget
42,240,032 µUSD, `max_context_bytes` 400,000, `max_tool_calls` 32, turn 900 s,
job 1,800 s). The snapshot was the treatment arm: the 13 members of the S2 r2
manifest at their pinned commits, with the ledger as its line range (5,753 B,
`e3b452a8…`), plus the fixture A4 material (112,497 B). That is 14 members and
238,593 bytes. Real Claude Code ran in the container against the offline
observer. Its first response asked for 12 `snapshot_read` calls, its second for
the other 5 (32 KiB pages, actor answer shape), then `result_publish`, then the
answer. Result:

- The turn completed and was eligible, with no reasons. 18 relayed calls.
  The capture has 5 payloads and 5 `attempts` (request 1 failed over from
  the 429 primary). No container was left and the serve record is `served`.
- `contextBytes` per request: 5,048, 226,634, 279,362, 280,483. The
  `max_context_bytes` margin at the last request is 119,517.
- `forwarderRefused` was 0 in all four request frames and the history frame.
- Container frames: `coding_provider_request` carries the context twice
  (`messages` and `agentMessages`). The largest was 561,468 bytes against the
  1 MiB frame bound, and the history frame 565,564. At `max_context_bytes`
  400,000 the request frame would be about 800 KB.
- Retained request bodies: 6,891, 226,714, 278,498, 279,142, total 791,245.
  The bound is 600,000 per body and 8 MiB per turn.
- `thread/read` reply: 272,539 bytes against the 1,000,000 bound.
- Container memory (cgroup v2, sampled every 0.25 s): `memory.peak`
  348,635,136 B (332 MiB) against `memory.max` 536,870,912. 39 samples, no
  `oom_kill`.
- Elapsed 68.9 s.

Not covered offline: thinking blocks and their signatures, which the live
bodies add (about 1.2 KB each, in both frame copies), and Claude Code's live
system-prompt size.

### Runs (offline)

State directory `~/.local/state/work-engine/dev-vertical-pi-host-h2-live-prep-20261118/`.

- `q17` (`pi_host_research_delivery` plus `pi_host_q_delivery_verifies`,
  before the test refactor): protocol `56c00a88…`, eligible,
  `CompletedSettled`, 7 payloads and 7 attempts, account flag `false`,
  serve record `served`; `verify_delivery` `complete`.
- `r9-relay` (`serve.py relay`): both turns completed and eligible;
  `forwarderRefused` 0 in every frame; no container left.
- `r10-treatment`: above.
- `q21` (after the test refactor: `pi_host_research_delivery`, then
  `pi_host_q_delivery_verifies` and `pi_host_live_delivery_verifies` over
  the same run): eligible, `CompletedSettled`, 7 payloads and 7 attempts,
  account flag `false`, serve record `served`, 4 grants retained.
  `verify_delivery` is `complete` (`request-5`, 4 spans, which follow the plan)
  with the same four refusals. The projection-picked marker is at offset
  58,444. Both delivery checks wrote the same `verify-delivery.json`
  (`93da5d4c…`). sha256: `summary.json` `f2250cf0…`, `retained.json`
  `3c49a8ea…`, `grants.json` `c7603120…`.
- `q22` (`pi_host_causal_join`): eligible, `CompletedSettled`, 3 payloads;
  `summary.json` `d9a85f80…`.
- `pi_host_live_arm` refusals, before any composition: no
  `WORK_ENGINE_HOSTED_CREDENTIALS` gives "refused: WORK_ENGINE_HOSTED_CREDENTIALS
  is not set"; an arm with a `loopback:` route gives "refused: a live arm
  routes only to https endpoints"; nothing was written.
- An earlier attempt at the refactored Q runs (`q19`, `q20`) failed with
  `upstream_head_eof`. The new `drive` dropped the observer's stdout reader,
  and the observer died on its next write. Fixed before `q21`/`q22`.

Unit tests on this build: `native-realization` 6/6; `work-engine-manual-host`
lib 108 passed (6 ignored); `lifecycle-service` lib with `controlled-proof` 97
passed; `lifecycle-store` all targets passed; `work-engine-task-host`
`research_delivery` with `controlled-proof` 6 passed. Among
`work-engine-manual-host`'s integration tests, `lifecycle.rs`
`mutation_during_snapshot_collection_is_a_conflict` failed twice under
parallel load on its 10 s wall-clock bound, and passed alone (9.14 s). It
exercises `collect`, which this unit does not change. The full
`work-engine-task-host` lib (`controlled-proof`) does not finish:
`selected_startup::tests::malformed_rehashed_installed_approval_refuses_before_every_startup_effect`
spins at full CPU. Run alone, it hits a 240 s timeout both on this branch and
at the base `bf64a1e3`, so it predates this unit. The other 17
`selected_startup` tests pass alone.

### Review (Fable) and remediation

Fable (`claude-sdk/claude-fable-5-1`), read-only, over `bf64a1e3..546a24a9`
(`reviews/pi-host-h2-live-prep-fable-20261118/REVIEW.md`): accept with
changes. It found nothing that makes the first live run unsafe.

| Finding | Disposition |
| --- | --- |
| P2-1: the delivery check's changed-byte marker could come from the prompt. A window that also occurs in the task text satisfies "once in some body" through request 1. The flip then changes only the prompt copy and the control fails for the wrong reason. | Fixed: the marker must not occur in the first body (the shared prefix, which holds no read result), must occur at most once in any body, and once in some body. |
| P2-2: the refactor dropped `capture_host` present / `capture_codex` absent. | Fixed: restored in `Run::assert_closed`, so offline and live both check it. |
| P3-1: a request the Host refuses before reserving any attempt (`not_dispatched`, for example not a Claude Code request on the subscription) has no `attempts` entry, and its reason said only "has no attempt". | Fixed: the reason names the `not_dispatched` cause; test added. `attempts` still lists only real attempts, and the protocol sentence (attempts, including a reserved one never dispatched) is unchanged. |
| P3-2: the forwarder count is job-wide and sampled at frame times. A refusal after a turn's history frame lands on the next provider request or the job completion, not on the turn already settled, whose answer preceded it. The job still cannot complete. | Recorded here; no change. |
| P3-3: a failed serve-record write replaced the run's error and the stderr record. | Fixed: the failure becomes `record_error` in the returned record; an existing record file is kept; test added. |
| P3-4: `cargo fmt --check -p work-engine-task-host` reports `observation.rs`. | Predates this unit, and this unit does not touch the file. |

Runs after the remediation (offline):

- `q23` (`pi_host_research_delivery`, then both delivery checks): eligible,
  `CompletedSettled`, 7 payloads and 7 attempts, `capture_host` asserted,
  account flag `false`, serve record `served`. `verify_delivery` is `complete`
  (`request-5`, 4 spans, which follow the plan) with the same four refusals;
  under the stricter rule the marker is still at offset 58,444. sha256:
  `summary.json` `98ad052f…`, `retained.json` `bb62e248…`,
  `verify-delivery.json` `7429c7cb…`, `grants.json` `ab150216…`.
- `q24` (`pi_host_causal_join`): eligible, `CompletedSettled`, 3 payloads;
  `summary.json` `e4ca0d5d…`.
- Unit tests: `native-realization` 6, `work-engine-manual-host` lib 108,
  `lifecycle-service` lib 97, `lifecycle-store` lib 33,
  `work-engine-task-host` `research_delivery` 6. Formatting is clean for the
  changed files.

Incident (validation only): the base comparison of the hanging task-host test
was built in this worktree's target directory. Cargo records workspace
source paths relative to the workspace root, and this branch's sources were
older than that build, so cargo kept base artifacts it should have rebuilt,
including `native-realization` with the old digest `9a33912a`. The first
remediation run failed as a result: the runner refused the Host's
`pi-host/initialize` digest, and the Host never started its thread. The
files that differ from base were touched and rebuilt. Every binary was
checked to carry `56c00a88…` and not `9a33912a…` before `q23`/`q24`. `q21`,
`q22`, `r9` and `r10` ran before the base build.

### R3 at the real-card composition (RL 5943)

Branch `pi-host-r3-real-card-20261118`, cut from `f772612f`. Offline, no
inference. The Host and observer binaries were built at `f772612f` in the
post-merge gate's own target directory (Host `0306b6f2…`, observer
`9c2a5d9e…`). `serve.py` gained `PI_HOST_S2_MANIFEST`, `PI_HOST_A4_MEMBER`
(the S2 r3 `a4_request_card_member`, checked against its pin), the
`advisory` scenario and `PI_HOST_OBSERVER`. The draft A4 bytes were read at
run time from the S2 r3 directory and are not in the repository.

The manifest was S2 r3 `snapshot-manifest.json` (`9fcaa325…`) and the A4
member `request-card.member` (`bdc8c2ef…`, 144,537 B, all ASCII). The H2 job
parameters were as in `r10`. Reads were 32 KiB pages, 12 calls in the first
response and the rest in the second.

| | `r11-treatment-real-card` | `r12-advisory-real-card` |
| --- | --- | --- |
| Members | 14, 270,633 B | 1 (`research/a4-request-card`), 144,537 B |
| Turn | completed, eligible; 19 calls (18 reads) | completed, eligible; 6 calls (5 reads) |
| `contextBytes` per request | 5,048; 275,918; 346,149; 347,270 | 5,048; 194,609; 195,730 |
| Margin to `max_context_bytes` 400,000 | 52,730 | 204,270 |
| `coding_provider_request` frame (largest; 1 MiB bound) | 695,042 | 391,962 |
| History frame | 699,242 | 394,678 |
| Request bodies (largest; total) | 345,812; 973,869 | 196,152; 398,551 |
| `thread/read` reply (1,000,000 bound) | 339,231 | 189,415 |
| `memory.peak` (512 MiB) | 344,199,168 (328 MiB), no `oom_kill` | 263,245,824 (251 MiB), no `oom_kill` |
| `forwarderRefused` | 0 in every frame | 0 in every frame |
| Containers left / serve record | none / `served` | none / `served` |
| Elapsed | 76.9 s | 55.9 s |
| `summary.json` sha256 | `6b1cbef7…` | `013ea538…` |

Context per member byte is about 1.27 for this composition and 1.32 for the
card alone. The fixture A4 in `r10` gave 1.15. The real card is dense JSON,
and its quotes are escaped twice, once in the actor's answer and once in the
context. The "about 1.07" ratio in 5929 was wrong. Live requests add thinking
blocks and signatures that the offline observer does not send, so the live
margin to 400,000 is below 52,730.

## Relay context bound 500,000 (RL 5947, operator decision)

Branch `pi-host-relay-context-500k-20261118`, from `24a77f7a` (the shared head
`f772612f` plus the R3 real-card records commit). No live inference.

- `work-engine-manual-host`: new `RELAY_MAX_CONTEXT_BYTES` = 500,000. On the
  lifecycle relay profile, `ProviderPlan::validate` admits `max_context_bytes` up
  to it (`context_bound`). The analysis profile keeps `MAX_CONTEXT_BYTES` =
  400,000. That bound exists because the analysis gate's
  `pi.provider_request` ingress, carrying the context twice, must fit the
  store's 1 MiB bound, and the relay gate stores no provider frame. The
  hosted coding profile keeps 32,768. Unchanged: `MAX_REQUEST_BODY` 600,000;
  `FRAME_BYTES` 1 MiB (the receiver refuses to emit, and the Host refuses to
  read, a larger frame); `HISTORY_FRAME_LIMIT` 1,000,000. The protocol states
  no context bound, so neither it nor `PROTOCOL_DIGEST` changes.
- Test `context_bound_is_per_profile`: 500,000 for relay, 400,000 for
  analysis, 32,768 otherwise. The relay bound is below the body bound, and
  twice it is below the frame bound.
- `serve.py`: `PI_HOST_MAX_CONTEXT` (the job's `max_context_bytes`) and
  `PI_HOST_PAD_BYTES` (an extra treatment member `zz/padding.md` of plain
  text, to reach a context near the bound).

Measurement (offline). The real-card treatment of `r11` plus padding, with
the H2 job parameters and `max_context_bytes` 500,000. Host built on this
branch:

| Run | Members | Outcome | `contextBytes` (last) | Request frame / history frame (1 MiB) | Largest body (600,000) | `thread/read` | `memory.peak` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `r13-context-pad` | 410,633 B (pad 140,000) | completed, eligible, 24 calls | 490,314 | 981,130 / 985,860 | 488,271 | 481,804 | 278 MiB |
| `r14-context-495k` | 415,219 B (pad 144,586) | completed, eligible, 24 calls | 494,902 | 990,306 / 995,036 | 492,859 | 486,392 | 273 MiB |
| `r15-cap-500001` | 270,633 B | `max_context_bytes` 500,001 refused at admission (`thread/start` error: provider lane/catalog/limits conflict) | n/a | n/a | n/a | n/a | n/a |
| `r16-over-cap` | 425,985 B (pad 155,352) | the third request (about 505,000 of context) was refused by the receiver's `contextBytes` bound before any frame or dispatch; the turn failed and was not eligible; no container was left | 275,918 | 552,351 / none | 275,998 | 496,523 | 272 MiB |

`summary.json` sha256: `r13` `676412a4…`, `r14` `876cfd0e…`, `r15`
`ec74e2cb…`, `r16` `32c86519…`. In every run `forwarderRefused` was 0 and no
container was left.

No other bound refuses a context below 500,000 first. The margin at the
full bound comes from the frames, which carry the context about twice: a
request frame at 500,000 is about 1,000,500 bytes (48 KB below 1 MiB). The
turn-end history frame also carries the final answer twice, so near the
bound a final answer above about 19 KB makes the receiver's history frame
exceed 1 MiB, and the turn fails closed. In `r16` the turn failure reads
"unknown coding frame": the receiver reports its refusal with a generic
`error` frame, which the Host does not name. That predates this change and
behaves the same at the 400,000 bound. Live thinking blocks count in
`contextBytes` like any other context.

## Open

- The Q fixture's owner adapter answers `original_result_operation` without
  the production controlled-research causal-scope gate.
- Identical request bodies across two distinct calls (an SDK-level retry
  after a completed stream) are refused by the task host; not observed.

- The Responses (`openai-codex`) relay tools digest is unobserved (OpenAI
  exhausted until 2026-11-21); relay requests on that wire are refused by
  the Host and by the task host.
- `verify_delivery` is composed with the actor's causal read directly, not
  over the service socket's `acquire` (same bytes and paging), and not through
  the controlled startup fixture (`checked_fixture_basis`) and basis mint.
- Startup hold (L4) for the Pi Host is refused, as for the Claude Host: it
  needs the protection owner's contract (R9′).
- The hosted analysis profile on `claude-sdk`
  (`pi-pressure-context-host` `analysis.rs`) settles its turn at the history
  frame, before the job-completion forwarder check, so it has the gap M1
  closes for relay turns. Its frames do not carry the per-turn count. This
  was outside 5923's scope.
- No live run.
