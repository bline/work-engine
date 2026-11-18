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

## Open

- The Responses (`openai-codex`) relay tools digest is unobserved (OpenAI
  exhausted until 2026-11-21); relay requests on that wire are refused.
- No live run. L, S, T, Q (lifecycle-service runner, allow-list, projection,
  composed qualification) follow on this branch.
