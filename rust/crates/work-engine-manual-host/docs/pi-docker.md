# Provider-disabled Pi/Docker component

`work-engine-manual-host pi-docker` is a bounded realization/custody adapter.
The existing Codex commands keep their schema and behavior. The Pi journal is
separate container/process evidence, with no task, lifecycle, inbox or review
semantics. Component receipts reserve acceptance for the named owning workflow.

This version supports Pi **1.5.0**, the exact `pi_docker::IMAGE` Node image,
Docker's local default Unix socket, Linux process groups, and a caller-owned
disposable read-only workspace. It sends exactly four commands: `get_state`,
disable automatic compaction, disable automatic retry, and `get_state` again.
No prompt, provider request, shell tool, MCP tool or session-change command is
exposed. The container has no network, credentials, Docker socket or host-root
mount. This profile does not admit credential-bearing inference or engineering
effects. It does not implement canonical Rust context replacement or external
event delivery.

## Input and commands

The JSON `Job` schema uses `work-engine.pi-docker-custody.v1` and rejects unknown
fields. It binds a job ID, fresh canonical `state_dir`, SHA-256 references to
the authority and packet, the Docker executable, the exact image digest, and
runtime/workspace trees. The caller owns the authority decision; an artifact
reference records that decision's bytes and creates no authority itself.

Each tree has `root` and a SHA-256 `manifest` reference. Obtain the complete
inventory with `pi-docker inventory --root ABSOLUTE_DIRECTORY`, save its JSON,
and hash the saved bytes. The inventory includes every regular file and symlink;
symlink targets must resolve within that tree. Special files, changed files,
extra files, noncanonical roots, overlapping roots and Docker mount syntax in
paths refuse admission. Runtime package metadata must name the supported Pi
version. Tree identities are checked before launch and after collection.
The pinned runtime must be provisioned before admission; this host never
downloads packages or images.

`deadline_unix_seconds` must initially fall within the next 300 seconds.
`command_timeout_seconds` bounds each Docker control command to 1–30 seconds.
The optional `idle_seconds` (0–30, default 0) retains the idle Pi target after
the replies for bounded cancellation/recovery qualification. Cleanup has its own
bounded control commands after the job deadline. A hard time guarantee against
an unavailable daemon or uninterruptible OS operation is not established.

```
work-engine-manual-host pi-docker run --request /private/job.json
work-engine-manual-host pi-docker inspect --state /private/custody
work-engine-manual-host pi-docker cancel --state /private/custody
work-engine-manual-host pi-docker reconcile --state /private/custody
work-engine-manual-host pi-docker ack --state /private/custody --receipt SHA256
```

The state directory must exist and be empty. It and both tree roots must be
disjoint. Only its `exports/` directory is writable inside the container.
The realization uses UID/GID 1000, a read-only root, no capabilities,
`no-new-privileges`, 64 PIDs, 512 MiB memory, one CPU, and a bounded `/tmp` tmpfs.
The Docker CLI receives an empty environment with only a fixed PATH and
nonexistent HOME, so ambient Docker contexts and provider credentials are not
inherited. The adapter verifies the realized image, labels, identity, command,
entrypoint, mounts and containment before custody effects.

`cancel` durably requests cancellation without acquiring the active owner's
lock. An active host observes it without a model ACK. If that host has exited,
invoke `reconcile` to act on the request. SIGINT/SIGTERM also request collection.
A killed host does not itself provide an autonomous daemon-side deadline;
restart reconciliation remains required.

## Recovery and evidence

Creation and start intent are persisted before effects. Reusing `run` refuses;
`reconcile` never creates, starts or resends RPC. It verifies the exact container
binding, collects any identifiable prior CLI group, stops a running orphan,
and obtains daemon inspect/wait facts before removal and a successful daemon
absence query. A created but unstarted container can be removed without claiming
an exit. Missing or failed daemon evidence remains unknown; it cannot justify
launch replay. An uncertain create with no observed container may still commit
later, so retain the journal and reconcile again under its owning process policy.

Raw stdout/stderr and process identities remain in the state directory. Inputs,
exports and raw observations are also copied to content-addressed snapshots.
Receipts bind the job, container/session IDs, actual export names and hashes,
process records, failures, daemon exit and removal evidence. Exact-byte receipt
ACK verifies its snapshots and only acknowledges delivery. It neither accepts
the component nor settles domain effects. `component_passed` requires the full
correlated fixture response set, a stable session, idle provider-disabled state,
daemon exit zero, removal and no recorded failures.

Process records are durably reserved before log creation. Recovery retains and
bypasses unpublished log reservations without truncation. Prefix RPC readiness
does not establish completed-stream validity: success also requires a durable
completion proof bound to the exact completed stdout digest and process index.
Journals without that proof retain `not_established` after cleanup, including
when a valid prefix precedes a partial trailing record.

A `sessionFile` path in RPC is prospective. A durable session-file claim requires
an actual corresponding export entry; this zero-message fixture normally writes
no session JSONL. Provider/window/usage data remains unavailable; model defaults
or displayed zero costs are not billing or subscription evidence. `agent_settled`
is not exercised by this fixture and no effect-settlement claim follows.

## Validation

Ordinary package tests preserve the existing native route and check strict
RPC framing/identity, inventory escape and admission failures. The explicitly
fake Docker example tests command failures, uncertain create, restart without
replay, malformed/truncated replies, deadline/cancellation, removal uncertainty,
receipt ACK and artifact tampering:

```
cargo build -p work-engine-manual-host --bins --examples --locked --offline
WE_PI_DOCKER_FIXTURE=/absolute/target/debug/examples/pi_docker_fixture \
  cargo test -p work-engine-manual-host fake_daemon_failure_restart_cancel_and_integrity -- --ignored
```

`real_daemon_cancel_and_restart_without_launch_replay` is separately ignored
because it creates actual disposable containers. It requires `WE_PI_LIVE_JOB`
pointing to an admitted private template whose `state_dir` is an existing empty
parent for two fresh test directories. Run it only within an authorized Docker
fixture assignment. Its cancellation and restarted-owner paths collect actual
daemon evidence; fake success is never substituted for that evidence.

The optional, explicitly separate SDK observation receiver candidate is described
in [pi-observation.md](pi-observation.md). Its source/fake qualification does not
extend the accepted four-control profile or authorize a real receiver probe.
