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

## Open

- The Responses (`openai-codex`) relay tools digest is unobserved (OpenAI
  exhausted until 2026-11-21); relay requests on that wire are refused by
  the Host and by the task host.
- Not composed yet: the task host's `verify_delivery` over a real Pi Host
  payload through a controlled research delivery (unit tests only); the
  actor's `snapshot_read` in Q (the fixture has no material port).
- Startup hold (L4) for the Pi Host is refused, as for the Claude Host: it
  needs the protection owner's contract (R9′).
- No live run.
