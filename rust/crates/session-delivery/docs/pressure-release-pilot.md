# Private pressure-to-receiver pilot

This bounded example is an operator tool, not an MCP endpoint or dispatcher.
Its inputs, directories and readiness digest must be supplied by trusted startup.
Never use an MCP caller to select them. The source phase uses disposable synthetic
telemetry and a pinned reader; it creates one Event artifact before receiver startup.
An empty target list is valid only for a valid configured pressure source with a
fixed nonempty recipient whitelist. It grants no subscription or delivery.

Create an absolute private `startup.json` with this shape, replacing every value
with trusted, reviewed pilot inputs. `config` is the existing fixture Config with
`targets: []` and its complete unchanged pressure config including the pinned
reader, synthetic rollout and whitelisted recipient. The recipient object is the
receiver profile's existing `recipient_without_session`, with no session field.
The Event, envelope and ready paths are absolute; release is exactly
`STATE/receiver-release.json`, next to `STATE/receiver-ready.json`. All output
paths must initially be absent. `input_digest`, `container_id`, deadline and ready
SHA-256 must be populated from the exact trusted job and actual readiness before
the release phase, never guessed. `recipient.job` equals `job` and recipient
expiry covers the deadline. The source phase does not consume those readiness
values, so trusted startup may fill them after readiness, retaining all other
inputs and source configuration unchanged.

```json
{"config":{"schema":"session-delivery-fixture-config-v1","fixture_only":true,"database":"/private/store.sqlite","repository":"/private/workspace","repository_identity":"synthetic-pilot","source_id":"unused-board","board_reader":"/absolute/board-reader","source_adapter":"/absolute/adapter","targets":[],"pressure":{"reader":"/absolute/pinned-reader","reader_sha256":"PIN","incarnation":"pilot","session":"SOURCE-SESSION","host":"codex","agent":null,"rollout":"/absolute/synthetic.jsonl","levels":{"window_percent":{"trigger":58.0,"band_floor":66.0}},"step_percent":5.0,"lead_calls":3,"recipients":["target"]}},"event_path":"/private/event.json","ready_path":"/private/state/receiver-ready.json","envelope_path":"/private/envelope.json","release_path":"/private/state/receiver-release.json","input_digest":"TRUSTED-JOB-DIGEST","container_id":"TRUSTED-CONTAINER-ID","ready_sha256":"SHA256-OF-EXACT-READY-BYTES","job":"job-id","deadline_unix_seconds":4102444800,"recipient":{"recipient":"target","principal":"principal","job":"job-id","lane":"lane","workspace":"/private/workspace","generation":1,"address":"role:target","fixture_mode":"observe","expires_at":4102444800,"active":true}}
```

From the Rust workspace, using the same private startup file and database:

```sh
cargo run -p session-delivery --example pressure_release -- source /absolute/startup.json
# Give the exact event.json and its SHA-256 to the reviewed receiver job. After
# the host writes readiness, trusted startup pins its actual bytes and job values.
cargo run -p session-delivery --example pressure_release -- release /absolute/startup.json
```

The helper selects the single retained Event, uses the actual ready session in a
new fully validated Binding, subscribes and calls Store.prepare once. It publishes
immutable files with exclusive hard links after syncing their contents. A crash
or failed publication after prepare leaves the attempt prepared/unknown and held;
rerunning cannot prepare or send again. Root must reconcile that attempt. The source/release phases never ACK. Publication and send acceptance are not target proof.
Use the official owning Host receipt verification and evidence boundary described
in `work-engine-manual-host/docs/pi-observation.md`: exact job, session, attempt,
payload, native message_end, SDK messages and native history snapshot, plus daemon
closure. Until those have been verified, leave Store.ack held. An absent native
JSONL is not evidence of failure or a fabricated JSONL; the SDK history export is
separate. This candidate's tests are fake and invoke neither Pi nor Docker.

After a separately authorized actual job, the component owner can collect and
verify custody using the reviewed binary and its private state directory:

```sh
work-engine-manual-host pi-docker inspect --state /absolute/STATE
work-engine-manual-host pi-docker reconcile --state /absolute/STATE
work-engine-manual-host pi-docker ack --state /absolute/STATE --receipt RECEIPT_SHA256
```

Reconcile may perform daemon cleanup and requires the separate real-probe grant;
these commands were not run here. Receipt ACK verifies the receipt and artifact
hashes and acknowledges receipt delivery only. The component owner must match
its verified receipt's job, input digest, container and native session, receiver
observation, release-intent envelope and exact Event/attempt/payload against the
prepared Store record and review the native proof and daemon closure. The `observe-ack` composition below joins that result without treating receipt-delivery
ACK as service observation. The separate Host receipt ACK remains an operator operation.

## Callable native proof to Store ACK composition

Build the example once and pin that executable in the immutable pilot admission.
Only after Root admits the exact job, launch this **instead of** launching the
standalone manual-host executable:

```sh
pressure_release observe-ack /absolute/pinned-job.json /absolute/trusted-startup.json
```

This phase performs runtime effects through the existing Pi Host. Source/fake
qualification does not authorize running it. The example calls `Host::admit` and
`Host::run` itself, preserving Host's current-executable custody binding and
using its native stdout/events/messages/history verifier in the same process.
It accepts no asserted receipt file, success flag, or hand-authored ACK. The
standalone host's state cannot be imported as proof into this executable.
An already admitted job refuses another launch. Failure/crash/unknown does not
retry or resend, including when native proof exists but the Store ACK did not
commit. Root retains reconciliation of such unknowns; this helper supplies no
post-crash ACK recovery claim.

While that process waits at native readiness, the trusted operator finalizes
startup with exact job input digest, actual container, ready digest and deadline,
then invokes the existing `release` phase in another process using that same
startup and Store. Startup, job and all output paths must remain private and
operator-owned; filesystem write access is not human identity or action authority.
Do not rewrite startup after release. The composition reads it after Host run
returns, checks its identity against the admitted job, ready projection, recipient
constraints, immutable event, release intent and exact prepared envelope, and
revalidates pinned inputs, complete runtime/workspace inventories, final stdout,
Host receipt artifacts and successful cleanup before calling `Store::ack`.
Store validates attempt, principal, binding generation, payload and expiry and
owns the durable `target_observed` transition. Exact repeats within the retained
composition are idempotent; no send API is called by the ACK path.

This remains one fixture-only observation. It asserts neither cognition nor
application, native JSONL persistence, task completion, effect settlement, human
identity, or action authority. The SDK history snapshot retains its existing
meaning. Missing, altered, incomplete or unknown proof leaves Store Unknown.
The `native_ack_fixture` test executable is explicitly fake and operator-owned;
it emits protocol fixtures and composes the existing fake daemon without importing
Pi, contacting a provider, or executing Docker. It is not a production driver.

The successful same-process join also acknowledges delivery of the exact native Host receipt, separately from Store target observation. If the owned process fails, `pressure_release custody-reconcile /absolute/STATE` uses the same executable pin to finish existing daemon custody only. It never creates, starts, sends, imports proof or advances a Store ACK. An unknown result remains held.
