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
rerunning cannot prepare or send again. Root must reconcile that attempt. No ACK
is written by this helper. Publication and send acceptance are not target proof.
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
prepared Store record and review the native proof and daemon closure. No command
in this pilot converts that result into a delivery ACK. Root must arrange an
owner-reviewed Store.ack operation after those checks; until then leave it held.
