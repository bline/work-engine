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

## Open

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
- No live run.
