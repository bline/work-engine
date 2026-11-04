# Private Pi SDK observation receiver candidate

Status: source and fake-adapter qualification candidate only. Root and S10 must
review these exact bytes before an immutable disposable real-probe grant. No
actual Pi, Docker, provider, deployment or live subscription was run here.

## Owner and supported profile

The existing `work-engine.pi-docker-custody.v1` Job and four-control CLI profile
retain their command and receipt behavior when `receiver` is absent. The new
explicit nested profile is `work-engine.pi-observation-sdk.v1`. It uses the same
Rust Host, journal, effect-intent reservations, process ownership, daemon
reconciliation, artifact snapshots and receipt ACK. It adds no inbox, task,
lifecycle or budget owner. There are no Cargo/npm dependency changes.

Supported source target: already provisioned Pi 1.5.0, its entire unchanged
runtime inventory, and the existing exact image
`node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da`.
No package/image downloads occur. `receiver/pi-observation.mjs` imports only the
fixed `/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js` SDK plus
Node builtins. Admission hashes its exact embedded bytes; arbitrary shims fail.
The installed SDK source and package manifest are pinned in custody inputs.

Source-owner exposure change: this profile replaces the CLI command with
`node /receiver/receiver.mjs` and adds exactly one read-only bind of that pinned
shim file. No ingress directory, host root, Docker socket, credentials or mutable
runtime/workspace is mounted. Ingress is the existing private bounded attach
stdin, held open at readiness. Network-none, UID1000, read-only root/workspace/
runtime, no capabilities, no-new-privileges, CPU/memory/PID limits, tmpfs and
private writable exports retain the accepted host's constraints. Realization
inspection checks the new command and exactly four mounts; other mounts fail.
The shim disables all discovery, MCP, tools, automatic compaction/retry and cache
warming. Model catalog refresh/network is disabled with a fresh tmpfs auth path.
It exposes no prompt, steer, follow_up, tool or general RPC command. Its agent
stream function also throws if any code attempts provider execution.

## Exact request shape and native readiness barrier

Existing Job adds only the optional strict `receiver` object:

```json
{
  "schema": "work-engine.pi-observation-sdk.v1",
  "shim": {"path": "/absolute/pinned/pi-observation.mjs", "sha256": "64hex"},
  "event": {"path": "/absolute/authorized-event.json", "sha256": "64hex"},
  "attempt_number": 1,
  "recipient_without_session": {
    "recipient": "target", "principal": "trusted-principal", "job": "job-id",
    "lane": "lane-id", "workspace": "/bound/workspace", "generation": 1,
    "address": "role:target", "fixture_mode": "observe",
    "expires_at": 4102444800, "active": true
  }
}
```

Unknown profile/release fields fail. The Event artifact is the existing delivery
Event DTO; its original raw_payload and digest remain unchanged. Recipient
constraints are the existing Binding except `session`, which cannot be guessed
before runtime creation. `fixture_only:true` remains on the existing envelope;
it is not probe authorization. This profile accepts only one Event, first attempt and
recipient generation. Job deadline is at most30seconds and idle_seconds is0.

SDK initialization creates a fresh native session, disables controls, subscribes
to actual native events, and emits a ready frame from native session state.
Rust requires idle, noncompacting, no pending messages, disabled compaction and
retry, and a nonempty actual native session. It durably binds that session and
process index in the existing journal, then publishes `receiver-ready.json` as
a read-only projection with profile, input_digest, container_id, native_session_id
and deadline. It releases no event before this barrier.

The existing delivery/service owner must then bind its recipient to that actual
native session and prepare the exact v1 envelope/attempt. It atomically publishes
`STATE/receiver-release.json`, with strict shape:

```json
{
  "schema": "work-engine.pi-observation-sdk.v1",
  "input_digest": "actual-job-input-digest",
  "container_id": "actual-inspected-container-id",
  "native_session_id": "actual-ready-session-id",
  "envelope": {"path": "/absolute/exact-envelope.json", "sha256": "64hex"}
}
```

This is a two-phase adapter API, not a pressure/public wire change. The receiver
does not construct or rewrite a delivery-owner envelope. If an enclosing service
cannot enroll and prepare after readiness, that integration remains blocked;
this prototype does not retarget a prewritten envelope or migrate a prior binding.
Root's real probe packet must name that service-side publication owner and its
exact artifacts. Readiness is available by reading the projection, without
opening the busy Host's exclusive lock or asking the recipient agent to wait.

Delivery and attempt IDs include the actual recipient binding. They cannot be
pinned as guessed constants before readiness. This profile pins first-attempt
ordinal1 and verifies the existing delivery-v1 tuple encodings after native
binding: delivery SHA256 over Serde JSON `[event_id,recipient,generation,binding_sha]`,
attempt SHA256 over `[delivery_id,1]`. This is a version-bound codec
check, not a new delivery identity owner or lifecycle order.

Rust verifies the release's exact job/container/native identity, immutable Event,
recipient/principal/job/lane/generation, attempt and original Binding-v1 digest.
It snapshots the exact raw envelope and durably writes release intent before
the single possible stdin write. The v1 Binding digest retains its named Serde
field ordering; no new global canonical codec is introduced. Raw UTF8 envelope
bytes are forwarded unchanged in one bounded JSON string; original source
payload bytes remain nested unchanged. A process/restart/error after intent is
unknown and never resends. Expiry/cancel prevents release. No release waits
beyond the original job deadline. A new ready/session or busy state refuses.

## Observation proof and exports

The shim admits only one `observe` command naming nativeSessionId,
envelopeSha256 and rawEnvelope. It checks idle and exact native identity again,
then inserts a fixed custom message with the exact raw envelope as text and in
details, `display:false` and `{triggerTurn:false}`. It never calls prompt. The
SDK API return is not an ACK.

Rust independently parses a completed stream containing, in order:

1. Native readiness for the contained session/process.
2. Exact admission digest (transport accepted only).
3. Actual subscribed native `message_end` with the exact custom message bytes.
4. `get_messages`: a fresh SDK read of native `session.messages`, the same
   owning property read by Pi's built-in RPC get_messages. This is explicitly
   an SDK observation frame, not a claim that a new built-in RPC exists.
5. A native history export with matching actual file digest.

Wrong sessions, raw contents, duplicate/error/unknown frames, incomplete tails,
missing native events/messages/history, or nonidle post-state cannot establish
observation. The history export is produced from native getHeader/getEntries
and messages and must actually exist, hash correctly, identify the same session,
and contain the same custom entry and message. The Host snapshots it and raw
stdout before receipt claims depend on them.

Pi's native SessionManager does not flush a new custom-only conversation to its
prospective JSONL file. The exported native-history snapshot is therefore
identified as an SDK snapshot, with nativeSessionFile:null when absent. No
native JSONL existence is inferred; no synthetic user/assistant message forces
persistence. Runtime model/window measurements remain unavailable. Neither
native insertion nor receipt asserts model cognition, application, completion,
effect settlement, compaction, acceptance or spending authority.

Transport accepted and target observed remain separate journal fields. A
completed native proof is bound to the exact process's final stdout digest and
actual history artifact. Existing daemon inspect/stop/wait/remove/absence
reconciliation remains mandatory for component success; CLI/process exit alone
cannot prove cleanup. Receipt ACK only acknowledges delivery of that receipt.

## Qualification and real-probe prerequisites

`pi_receiver_fixture` is explicitly fake and never imports Pi or invokes Docker.
It composes the accepted fake custody adapter and simulates one release producer
after reading the durable barrier. Tests exercise native proof success and
missing end/messages/history, wrong session/attempt, partial tails, busy/no
release and reopening without create/start/resend. Baseline real Docker tests
remain ignored. `node --check` parses the shim without executing its imports.
Static SDK/source alignment plus fake tests do not prove real SDK compatibility,
native insertion, live provider-disabled behavior or actual daemon cleanup.

Before real execution Root/S10 must bind: reviewed source/binary/shim hashes;
unchanged full runtime inventory and image; Docker executable; private fresh job,
state/workspace and deadline; exact authorized Event/delivery/attempt/recipient
constraints; the service-side after-readiness envelope publication owner;
provider-disabled containment and the single additional shim mount; raw process,
native-history and daemon exports; bounded reconciliation and receipt collection.
Only an exact subsequently reviewed immutable probe grant may admit those effects.
