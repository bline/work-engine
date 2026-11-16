# Phase 4 design: hosted provider transport for the Pi coding path

Prepared 2026-11-16 by the Claude takeover session. Standing: a proposed
design. It is not an implementation, acceptance, or change to any versioned
profile contract. It follows the
[completion plan](dev-vertical-completion-20261115.md), phase 4, and builds on
the offline results recorded in `records/dev-vertical-*-20261116/`.

## Current live path (source-read, not re-run)

The Pi coding Host (`pi_docker.rs` + `pi_coding*.rs`) runs Pi in RPC mode in an
inference container and runs model-selected tools in a separate network-none
tool container. For provider profiles:

- `coding_project_provider_access` reads the native auth source on the host
  and stages access, expiry and account in a private 0600 `access.json`, which
  is mounted into the inference container at `/auth`.
- The inference container uses `--network bridge` (`pi_coding.rs` `base`).
  The receiver (`receiver/pi-coding.mjs`) builds a read-only CredentialStore
  from `/auth/access.json`, and the SDK's own `fetch` calls the provider.
- Financial admission is already owned here. Each inference request goes
  `provider_request`/`coding_provider_request` → durable microUSD reservation
  → `provider_grant` → `provider_observation` (raw terminal) →
  `provider_complete`. Unknown entry keeps the full reservation and blocks any
  further grant.

This path is the "credential inside the inference container" exposure the plan
removes.

## Proposed target

A new versioned profile — working name `work-engine.pi-coding-hosted.v1` —
keeps every existing owner and changes only the transport:

1. **Inference container:** `--network none`, no `/auth` mount, and
   CredentialStore given a nonauthenticating marker, as in
   `pi-native-tools.mjs`. The receiver injects `fetch`. After the existing grant
   for request `id`, it sends a `provider_fetch` frame carrying the
   compressed/encoded request and consumes the response via
   `createResponseStream` (`pull`/`cancel`/`ack`).
2. **Host:** the access projection stays on the host as in-memory custody.
   `pi_coding::request`-style validation reconstructs the upstream request; the
   host dispatches it on the selected route and relays it through `pi_stream`,
   screening for the projected access token.
3. **Financial authority:** the existing provider reservation remains the
   **single** financial owner. The router does not keep a second ledger. Its
   allowance check is replaced by, or derived from, the grant that is already
   current for that request id (microUSD). `pi_route` keeps route selection,
   refusal-driven failover and provenance only. An authorized failover is a
   new attempt under the same grant; whether it needs a new reservation is the
   main owner decision below.
4. **Raw evidence:** the existing `provider_observation.rawTerminal`
   requirement is produced by the host from the relayed bytes
   (`pi_stream::usage` plus the raw terminal event), not reported by the
   container. This also addresses the review's "raw screened SSE" gap for this
   path.

## Required implementation work

- **`pi_stream` as a sans-I/O state machine.** The Host processes one frame at
  a time and returns replies, so the blocking `relay` loop cannot sit inside it.
  Split it into
  `StreamSession::{head(), on_frame(op) -> reply, poll_upstream()}`. Keep the
  current thread-backed reader so a `cancel` arriving while a chunk is pending
  is handled on the next Host loop tick.
- Host loop integration: route `provider_fetch`/`pull`/`cancel`/`ack` frames
  for the active request id. Refuse interleaving with tool or other frames
  while a stream is open, as `pi_native` does.
- Host dispatch: loopback synthetic route for offline qualification. Real
  HTTPS dispatch reuses `bounded-inference` (`public_https`), only in phase 6.
- Receiver changes are confined to the new profile, so existing fixture,
  signed and live profiles stay byte-identical.
- Qualification: the existing ignored actual Docker coding test with a
  synthetic hosted route, `S10_STRICT_SOURCE`, the validator and exports. Also
  the stream fault set (stall, truncate, mid-stream failure) and an inspection
  proving `NetworkMode=none` and no credential mount on the inference
  container.

Rough cost: a sizeable slice (Host loop integration, a new profile across
receiver/Rust/docs, and actual Docker qualification). It is not a small edit.

## Decisions needed before implementation

1. **Profile strategy.** Add a new versioned hosted profile (recommended,
   because existing contracts stay intact), or change the existing
   live/signed-subscription profiles in place. The signed subscription
   contract is documented by `session-task-admission`
   (`docs/pi-subscription-v1.md`); changing it in place needs that owner's
   acceptance.
2. **Reservation on failover.** Either an authorized route switch after a
   definitive refusal reuses the current grant (the refusal released nothing
   that had been charged), or it requires a fresh reservation and grant.
   Recommended: a fresh reservation per attempt. This keeps "one reservation
   per possible provider entry" and fits the existing unknown/no-replay rule.
3. **Retirement of the old path.** Once the hosted profile is qualified,
   either remove the in-container credential profiles or keep them labelled
   legacy. The pre-release policy permits removal; the operator decides.
