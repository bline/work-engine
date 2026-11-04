# Fixed MCP Pi observation adapter candidate

This is a private source candidate for one preauthorized provider-disabled SDK
observation. Offline tests use synthetic source transcripts, a fake Docker ELF
and a fake native session. They establish no live Dot, OAuth deployment, general
coding, provider inference, signed-edit or production qualification. Root owns
source acceptance, deployment and the actual pilot.

## Executable and owners

Build offline from the pinned private subject:

```
cargo build --offline --manifest-path rust/Cargo.toml -p session-delivery --example mcp_pi_observation
```

The callable ELF is `rust/target/debug/examples/mcp_pi_observation`. Copy it to a
private immutable location, chmod 0700, and pin its SHA-256. Its MCP interface is
`mcp_pi_observation --config-fd FD rpc`: the trusted Python transport passes a
verified config descriptor and one bounded JSON request on stdin. It accepts no
shell, command, payload, target session or receipt imports. The example directly
reuses `pressure_release.rs`; `VerifiedRun` remains privately constructed by the
actual Host run. Host, journal, reservation, custody and native receipt ACK stay
with `work-engine-manual-host`. Event, delivery, attempt, uncertainty and
`target_observed` stay with the existing Store. There is no new queue, daemon,
lifecycle, grant-issuing API or budget authority.

## Trusted config (Root builds and admits these bytes)

The strict top-level config is:

```json
{
  "schema": "work-engine.mcp-pi-observation.v1",
  "principal": "the-existing-server-startup-principal",
  "subject": "the-exact-configured-OAuth-subject",
  "target": "the-one-recipient",
  "handle": "opaque-one-shot-handle",
  "revision": "exact-admitted-revision",
  "idempotency_key": "fixed-one-shot-key",
  "expires_at": 0,
  "revoked_path": "/private/pilot/native-state/cancel.json",
  "operation_dir": "/private/pilot/operation",
  "job": {},
  "startup": {}
}
```

`job` is the existing strict `work-engine.pi-docker-custody.v1` Job, with the
accepted `work-engine.pi-observation-sdk.v1` receiver profile. Use the accepted
full runtime/workspace manifests, authority/packet pins, exact image, pinned
Docker executable and exact shim. Configure a new private empty native state
directory, `idle_seconds:0`, command timeout at most3 seconds, and an original
absolute Job deadline no more than30 seconds away at native admission. Do not
reuse an expired Job. The grant expiry covers the Job deadline.

`startup` is the existing strict pressure-release Startup DTO: `config`,
`event_path`, `ready_path`, `envelope_path`, `release_path`, `input_digest`,
`container_id`, `ready_sha256`, `job`, `deadline_unix_seconds`, `recipient`.
Its `config` is the existing source-only delivery Config (`targets:[]`, configured
pressure source/reader and recipient allowlist). Preserve `fixture_only:true`
and the actual source provenance. `recipient` is the exact existing Binding
without `session`; its principal/recipient must equal top-level principal/target,
and its job/deadline must match Job. Its fixture mode is `observe`.

Both Job.receiver.event.path and Startup.event_path name the same new absolute
file. The event is collected once from the trusted configured source after the
operation reservation, before Host admission. Set the template event SHA to64
zeroes; the adapter replaces it with the actual source-owned Event hash in the
final immutable `operation/job.json`. Initial input_digest/container_id/
ready_sha256 are empty; only actual Host readiness fills them. Ready/release
paths must be `native-state/receiver-ready.json` and
`native-state/receiver-release.json`. Envelope and event paths are fixed trusted
new files. The final Job and Startup are durable and hashed in result evidence.
Root should use distinct private operation/native directories and Store DB, all
with preserved filesystem ownership; do not point them at historical evidence.

## Explicit HTTP admission

Keep the live authenticated-read-only service unchanged until Root has reconciled
the exact candidate delta against its preserved deployed baseline. The private
service already differs from the live service at its delivery-helper seam;
copying an entire old live file over newer source is not a deployment procedure.
No Stytch configuration, JWT decoding or auth provider is added by this adapter.

For an explicitly admitted pilot, configure all five startup values:

```
PI_OBSERVATION_ENABLED=1
PI_OBSERVATION_BINARY=/private/pilot/mcp_pi_observation
PI_OBSERVATION_BINARY_SHA256=<exact-ELF-sha256>
PI_OBSERVATION_CONFIG=/private/pilot/observation.json
PI_OBSERVATION_CONFIG_SHA256=<exact-config-sha256>
```

The config file must be owned by the service user, mode0600 or stricter, and the
ELF must not be group/world writable. The transport opens both with NOFOLLOW,
checks owner/type/size/hash and passes their pinned descriptors. Start the
existing service's `serve-http` entrypoint with its existing auth config and
startup principal plus `--observation-only`; this mode is incompatible with
`--authenticated-read-only`. Keep the old bridge and fixture delivery switches
disabled. Observation cannot be enabled through stdio or a generic HTTP mode.

HTTP startup requires exact equality between observation subject and existing
`HttpAuth.config.subject`, and between observation principal and Server.principal.
Every RPC requires existing OAuth verification, including listing and reads.
The existing verifier owns signature/expiry/issuer/resource/client/project and
single-subject validation. No caller principal is accepted. The only mutation
allowed in this mode is `pi_observation_observe`; task, provisioning, approvals,
caller-written delivery ACK and other writes remain403 even with a valid token.
The permitted old public reads are delivery_status/replay_events. Basic watches
and the old bridge retain their existing behavior in other modes.

## Exact tools and results

All three tools accept exactly this object; additional properties are denied:

```json
{"handle":"opaque-one-shot-handle","revision":"exact-admitted-revision","idempotency_key":"fixed-one-shot-key"}
```

* `pi_observation_observe` is a WRITE (readOnlyHint:false), one-shot/idempotent.
* `pi_observation_status` and `pi_observation_result` read existing evidence and
  never run Host, source collection, process cleanup, Store ACK or a replacement.

A new operation reserves `intent.json` exclusively and fsyncs its immutable
config/key binding before any Store/event/Job writes. Duplicates can only read;
changed handle/revision/key/principal or config conflicts. A crash during intent
publication may temporarily yield unavailable rather than accepted, but the
exclusive file still fences execution. Never delete it to retry.

The owned native thread runs the existing Host. The foreground owner waits for
actual readiness, verifies the final Job digest, durably finalizes Startup,
rechecks expiry/revocation, and publishes the existing exact envelope/release
once. Host independently records its release intent before stdin. The foreground
always joins its thread, including release failures and HTTP disconnects, then
runs the existing exact completed-native-proof/Store ACK join with the late
ACK clock. Native receipt delivery ACK remains separate from Store ACK.

Results use `{schema_version:1, observation:{...}}`. Observation distinguishes
accepted intent, transport acceptance, target observation and application/
completion. Successful observation is `state:target_observed`, with explicit
fixture/source limitations, exact native session/receipt id, Store ACK, SHA-256
references for event/envelope/Job/Startup/native history/native receipt, plus a
result reference and input binding. Application result and completion remain
null. Failures or lost terminal projection are `unknown`, never inferred success
or permission to resend. Missing observations use null rather than false.
Terminal results persist their original immutable input binding. Result reads
validate it against intent/config, require the complete seven-item successful
evidence set (including readiness), and cross-check Job, Startup, receipt digest,
native session, source Event/envelope and typed observation ACK plus Store/native
receipt delivery ACK. Unknown results cannot carry successful observation fields.
Old unbound result projections fail closed; reads never regenerate evidence,
execute, clean up, acknowledge, or import caller proof.

HTTP disconnect does not cancel the owned request thread or helper. The helper's
native work is bounded by the original Host deadline and3-second command limits;
the transport retains it for up to180 seconds and reaps it. Machine/helper loss
remains Unknown. Root must reconcile the exact known native state through the
same `mcp_pi_observation custody-reconcile /absolute/native-state` executable; it can
stop/collect/remove only that Host's known custody, never start/replace/resend or
advance Store ACK. Status/result reads do not hide such a cleanup effect.
The configured revoked_path must equal exactly Job.state_dir/cancel.json.
Creating that marker prevents future admission; the existing Host checks it after
expensive input validation and again after create/inspect immediately before
start. The adapter uses the same marker before startup and release. It cannot undo an already released event.
Grant expiry/revocation does not erase authorized evidence reads.

## Offline qualification

`skills/session-events/tests/test_pi_observation.py` drives MCP JSON-RPC through
the real pinned adapter ELF with fake Host/native evidence and a synthetic
pressure source. It checks exact positive attribution, eight concurrent calls,
conflicts, injection, expired/revoked grants, pin changes, native proof failure,
lost terminal projection and read/restart no-resend. `test_http_auth.py` adds an
observation-only boundary test with real locally signed synthetic OAuth tokens.
The worker sandbox denies loopback socket creation, so the socket-based auth
suite is unestablished here. The focused suite additionally drives the exact HTTP
handler in memory using the real verifier and locally signed synthetic tokens,
including wrong subject, expired token, insufficient scope and denied writes.
The preserved pressure-release Rust tests cover the original native join and
late expiry clock. Full legacy host suites have known timeout limits from prior
qualification; this change does not claim those broad suites pass.
