# Coding-job operating guide

This guide covers the `coding-job-offline-fixture-v1` capability of the local server and command. It is qualified only offline, with a provider-disabled Pi Host fixture. It does not qualify a live provider job, relabel `offline-synthetic-v1`, or add an issuer, login, credential-minting API or deployment permission. The session-message capability, its grants and the claims and query reads are unchanged. Wire shapes are in [coding-job-v1.json](../rust/crates/work-engine-server/schemas/coding-job-v1.json); the server entry is documented in the [server README](../rust/crates/work-engine-server/README.md).

## Ownership

| Concern | Owner |
| --- | --- |
| Coding admission, effects, custody, validation, receipts, cleanup, acknowledgement | Existing Pi Host (`work-engine-manual-host pi-docker run \| inspect \| reconcile \| cancel \| ack`), unchanged and pinned by SHA-256 |
| Session-message grants and admission | `session-bridge`, unchanged. A session grant is not a job credential. |
| Whether this caller may run this exact job | The operator-prepared job profile (below) |
| Binding of the public request to the Host job, one owned collector, projection of Host results | `work-engine-server` `job.rs` |

The adapter never builds a job, never accepts a path, executable, job JSON or hash as authority, and never holds provider credentials.

## Authority contract

The operator writes a private (`0600`, caller-owned) job profile, referenced by the server config as `coding_job_profile` together with `"coding_job_capability":"coding-job-offline-fixture-v1"` (both or neither; any other capability string, including live ones, is refused at startup). The profile pins:

- the Host binary path and SHA-256, re-hashed before every Host invocation;
- a private `operation_root` (`0700`) holding one directory per operation;
- per authorization: `authorization_id`, `caller`, `credential_sha256`, the immutable `job_file` and its `job_sha256`, `packet_sha256`, `job_id`, `state_dir` and `host_input_digest`.

`host_input_digest` is the Host's own identity for the exact job: SHA-256 of `serde_json::to_vec` of the Host's typed `Job` for schema `work-engine.pi-docker-custody.v1` (struct field order, defaults such as `idle_seconds:0` emitted, `provider`/`receiver` absent). It is not the job-file digest, so a pretty-printed or reordered job file has a different `job_sha256`. The adapter recomputes it from the job with a serialization-only mirror of that offline shape (a different Host job schema or coding profile version, a provider, a receiver or any unknown field has no digest and is refused). The mirror is checked against a real Host journal digest in its tests. The Host binary is pinned by hash, so its algorithm is fixed for a profile. Profile load rejects overlapping state ownership: duplicate or nested `state_dir`, a `state_dir` inside or containing `operation_root`, duplicate `job_id` or `host_input_digest`, and non-normal or non-canonical paths (`overlapping_state_ownership`, `invalid_job_profile`).

Authority is a separate job credential. The operator provisions a random secret for one caller and one job and pins only its SHA-256; no API mints it. The service bearer authenticates the transport only. A session-message grant, the bearer, a wrong or unknown `authorization_id`, and a short or malformed credential are all refused identically as `unauthorized`.

At submit the adapter reads the pinned job file itself and checks, before anything can be entered:

1. job bytes equal the pinned `job_sha256` and the request's `job_sha256`; otherwise `job_mismatch`;
2. the packet file bytes equal the pinned `packet_sha256` and the request's value; otherwise `job_mismatch`;
3. `job_id`, `state_dir` and packet digest in the job equal the profile; otherwise `target_mismatch`;
4. offline class: a `coding` profile with no `provider` and no `receiver`; otherwise `capability_mismatch`;
5. the job's `deadline_unix_seconds` is inside the Host's fresh-admission window (now+5 s to now+300 s); otherwise `job_deadline_window`;
6. the digest recomputed from the job equals the pinned `host_input_digest`; otherwise `host_identity_mismatch`;
7. `state_dir` is a private directory without `pi-state.json`; otherwise `state_not_fresh`.

Prepare the job file immediately before submission; the Host only admits a fresh deadline within 300 seconds. A refusal at steps 1 to 7 leaves nothing bound and the authorization usable.

## Request flow

Operations (HTTP envelope, MCP tools, `LocalClient::job`, and `work-engine-command PROFILE OPERATION --json|--human ARGS|-`): `submit_coding_job`, `read_coding_job_operation`, `read_coding_job_result`, `cancel_coding_job`. Arguments are strict; unknown fields are rejected.

1. Submit takes a UUID `request_id`. The adapter creates `<operation_root>/<operation_id>/binding.json` (`create_new`, fsynced) binding `request_id`, `authorization_id`, `job_id`, `state_dir`, `job_sha256`, `packet_sha256`, the expected `host_input_digest` and Host binary digest. `operation_id` is derived from authorization, job and Host digests.
2. The adapter takes an exclusive `flock` on `collector.lock`, durably writes `entry-attempt.json`, then starts one owned collector: `pi-docker run --request <snapshot of the verified job inside the operation directory>` in its own process group, stdout and stderr pumped and bounded to 1 MiB each. Submit returns the durable operation identity promptly. The collector outlives the client connection.
3. An exact repeat of the same `request_id` observes the same operation. A different `request_id` for the same authorization returns `authorization_consumed`. Reads with another `request_id` return `identity_conflict`. The authorization is single-use, which is how a second Host admission is prevented.
4. Before any read, reconcile, cancel, result or repeat acts, the adapter reads the Host journal `pi-state.json` (no Host lock needed, so this also holds while the Host runs, owner and symlink checked) and requires its job identity, its `input_digest` and the digest recomputed from its own job to equal the digest bound before entry; otherwise `identity_conflict` and the Host is not contacted. The Host never supplies the identity it is later checked against. Reads then call `pi-docker inspect --state` (bounded, 20 s).
5. If nothing owns an entered operation and the Host journal shows custody incomplete, the next read or repeat starts a collector that runs `pi-docker reconcile --state` on the same state. The adapter never calls `run` again after `entry-attempt.json` exists.

States: `bound_not_entered`, `collecting`, `reconciling`, `collected` (Host phase `collected` and removed), `entry_uncertain` (entry possible, no Host state, so admission is unknown), and `host_state_unreadable` (state exists but cannot be read, typically a Host still holding its lock). Uncertain states are reported with limitations; they are never cleared by launching a replacement. Resolve them from the original operation id, or by operator inspection of the original state directory.

## Results, cancellation and process closure

`read_coding_job_result` returns the original Host receipt and separate projections: `terminal`, `validation`, `cleanup` (including the collector's own process disposition). Absent Host fields are `not_established`, never defaulted. `acceptance: owner_pending` and `domain_effect_settlement: not_claimed` are passed through; the adapter claims no settlement. The result's `receipt_id` is the Host wrapper's own `{receipt_id, receipt}` ID (outside the receipt body), required to be 64 hex characters, to equal the journal's recorded receipt when there is one, and to come with a receipt whose `job_id`, `job` and `input_digest` match the bound job (otherwise `identity_conflict`; a malformed wrapper is `custody_lost`). Receipt acknowledgement stays a Host action (`pi-docker ack`).

Operation, result and cancel outputs have strict published variants (`schemas/coding-job-v1.json`). `LocalClient::job` validates every `owner_result` against them, including schema version, nested identity and that the response describes the caller's own request, and returns `invalid_response` for anything else. The Host receipt payload stays opaque beyond those identity fields.

`cancel_coding_job` runs `pi-docker cancel --state`, which records a cancellation marker. The response always reports `cancellation_established:false`, `container_closure_established:false` and `settlement:not_claimed`. Closure is known only from the Host receipt and cleanup after later reads.

The collector deadline (`collector_deadline_seconds`, at most 900) and server shutdown both request Host cancellation, wait `cancel_grace_seconds`, then kill the owned process group, always collecting actual exit status. `collector-run.json` records exit code or signal, `killed_after_grace`, `status_collected`, truncation and whether the output readers joined. `process_cleanup` is `owned_process_group_collected` only when status was collected and readers joined; otherwise `not_established`. Containers belong to the Host: killing the Host process group does not establish container closure; read the Host receipt after `reconcile`.

On SIGTERM the server stops accepting work, closes the collector registry and cancels owned collectors, waiting up to 90 s for their status. Closing the registry and draining it are one atomic step: a request that outlives its transport timeout and reaches the collector start afterwards is refused with `shutting_down` before the exclusive lock and entry marker. After all request permits are released the registry is checked again before `drain complete` is reported. Unclosed collectors exit the server with code 2 and keep their operation records readable.

## Limits

- Offline fixture only. A live profile, provider lane or native auth lane needs a separate qualification and is refused by the capability check.
- A server crash (not SIGTERM) can leave a Host running in its process group with the adapter lock released. Reads then report `host_state_unreadable` while the Host holds its lock, or reconcile once it stops. Host cleanup is then the Host's reconcile path.
- The Host binary is hashed then executed by path; a swap between those steps is not prevented. The job file, by contrast, is snapshotted after verification and the Host runs the snapshot (re-hashed before spawn).
- The binding records the job-file digest before execution and the Host `input_digest` afterward, not the Host digest before execution.
- Receipt acknowledgement, workspace export acceptance and domain settlement are not exposed.
- Not exposed through the `work-engine-query` binary: every job read requires the job credential, so reads live on the command and server surfaces.
