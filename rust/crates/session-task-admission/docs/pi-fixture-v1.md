# Signed Pi fixture contract v1

This contract is fixture-only. It uses synthetic identity, a fixture signing key
and a deterministic fake provider. MCP HTTP, the signed-answer verifier,
registry/admission/outbox, actual pinned Pi SDK, Docker, workspace tool, read-only
validator and exports are exercised. It grants no real owner identity, live
provider credential/network access, deployment, or owner acceptance.

The task intent has schema `dot.task-intent/pi-coding-v1`, provider `pi`,
operation `start`, execution profile `disposable-pi-coding-fixture-v1`, and model
`fake-pi-model` / effort `off` / tier `fixture`. Effects permit one concurrent
task, zero children, network false, and exactly `src/lib.rs` as the persistent
write path. Scratch is ephemeral. Existing exact artifact/ref, mode, prerequisite,
clock and budget fields and `work-engine.owner-answer/v1` signing bytes apply.
Codex intent/profile validation remains a distinct branch.

`pi_pilot::Profile` uses `dot.pi-coding-fixture-profile/v1`. Its exact JSON bytes
are pinned by a signed `dot.scoped-control/v1` activation under the existing
current-registry owner. It binds task/proposal/mode/configuration/parent refs,
actor, objective, exact serialized Host job digest, executing native helper
binary digest, transport digest and expiry. The profile classification is
`fixture-signing-and-identity; fake-provider; network-none; actual-pi-sdk-and-docker`.
The future native UUID is deliberately absent from signing inputs.

`pi_pilot::Descriptor` binds protected admission configuration, exact profile,
admission request, and a transport tuple: helper SHA-256, principal, verified HTTP
subject, handle, revision and idempotency key. `Transport::sha256` hashes its
Serde JSON serialization in declared field order. This is a named local contract,
not a new canonical codec for existing owner answers. The HTTP verifier validates
the token, including exact audience and subject. The fixed endpoint maps that
verified subject to the configured principal. MCP cannot select a profile, path,
executable, job, shell command, actor or arbitrary principal.

The domain exposes `PiOwner::open`, `admit`, `enter`, `seal`, `check_native`,
`record_result`, `read`, `check_transport` and explicit `ingest_control`.
`PiOwner` and `EffectLease` cannot be deserialized or publicly constructed.
The executing binary must match the signed profile. Ordinary
`ProtectedRecollector` remains unqualified. The separate Codex `ScopedOwner`
contract is not a Pi qualification capability.

Admission consumes proof and capacity through the existing admission/outbox
transaction. Registry schema v3 adds only a Pi job/native binding and per-effect
uncertainty records, not another monetary or lifecycle ledger. Host provider
reservations and tool journals remain authoritative for their facts. Control,
task and proposal updates order against effect entry on the same SQLite write
lock. External board/key/source selection remains trusted and must be protected
by the enclosing deployment; same-UID files are not host-wide isolation.

Before allocation, an unknown intent and outbox fence are durable. The secret-free
receiver calls the actual `SessionManager.create`, reports its native UUID and
whether access exists, then waits. The owner rechecks signed current state and
seals that UUID once for the exact job. A registry lease covers fake access staging
and initialization IPC. Only then does the receiver read fake access and construct
`ModelRuntime` / `createAgentSession` with the same manager. Ready must equal the
sealed UUID. Unknown allocation or a replacement UUID never authorizes another
allocation. Provider grants hold registry ordering through IPC dispatch; tool
leases hold it through actual tool execution. Expiry and signed current authority
are checked immediately before dispatch. Missing ACK leaves a unique effect
entry, preventing replay.

`pi_coding_submit`, `pi_coding_status`, and `pi_coding_result` accept only
`handle`, `revision`, `idempotency_key`. They are disabled unless the operator
configures all `PI_CODING_*` pins and `PI_CODING_FIXTURE_ENABLED=1`, and selects
`--coding-only` authenticated HTTP. Status/result are immutable reads: no source
collection, ACK, reconciliation, retry, launch or implicit cleanup. Repeated exact
submit returns existing evidence, including unknowns. Changed bindings conflict.
Native custody cleanup is separate from transport receipt ACK and owner acceptance.

Result `dot.pi-coding-result/v1` retains admission/operation/profile/descriptor/job
IDs, native UUID, outbox state, effect-entry evidence and the actual Host receipt.
Fake provider/model/raw terminal/status/token usage remain labeled fake. Zero
synthetic cost does not assert real billing or quota usage. The Host receipt binds
raw frames/history, process results, tested snapshot, acceptance, exports and
container/access removal. A recorded runtime result is not owner acceptance.

The opt-in `test_pi_coding.py` exports `ui-golden-vectors.json` under the owned
fixture evidence root. It contains exact raw task/control and owner-answer JSON,
authenticator/client/signature bytes and their pinned profile/descriptor bindings.
UI consumers must display Pi and fixture/fake classification, persistent path,
ephemeral scratch, network-none, first-native-before-initialization rule, exact
request/profile/job and budget/deadline pins before presenting any signature.
These vectors do not make the separately owned auth/UI accept Pi; its guard must
remain explicit. A generic/Codex label, unknown fields, widened network/write
scope, changed bytes/challenge/key or stale current revision must fail closed.
