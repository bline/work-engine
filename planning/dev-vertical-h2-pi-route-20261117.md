# Dev vertical: Pi-route capabilities for the research-lifecycle H2 run

2026-11-17, revision 1. Owner: the dev vertical (Pi session
`01a1e2a4-f353-743a-ba23-f42394958125`). Board claim `dev-vertical:pi-h2-route`.

## Authority and what this does not decide

Operator, 2026-11-17, after the dev-vertical closure report said the Pi route
was not ready to carry H2 and that the "Pi under the task host" route was the
operator's decision: "Once we finish review/remidiation we can build what H2
needs." The dev vertical closed at `4d5b0b34`. This plan reads that statement
as authority to build the Pi-route capabilities named in the
[route comparison](research-lifecycle-h2-route-comparison-20261117.md)
(research-lifecycle lane, uncommitted): the response-ceiling fix (open item
D4), a no-shell analysis profile with H2-sized bounds and request evidence,
and the connection to the research lifecycle's task host.

It does not answer the route comparison's decisions R3–R5 or R7, and it does not
accept anything on behalf of the research-lifecycle owners. Slices 1–3 change
only dev-vertical components (Pi Host and receiver, the Pi lifecycle gate, the
Pi grammar in `lifecycle-store`, the hosted server admission and job
preparation). Slice 4 touches `lifecycle-service` and the task host, so it
starts with a read-only sizing shared with that lane and needs their
architecture acceptance before code. If the operator meant less than this,
slices 1 and 2 remain useful on their own: D4 is required before any real
coding turn longer than the qualification task.

## Starting point

The hosted profile `work-engine.pi-coding-hosted.v1` runs one fixed coding
continuity job: 4 provider requests, one `workspace_exec` shell call, one
reset, model `gpt-6-luna`, context at most 24 000 bytes (Host plan at most
32 768), request body at most 45 000 bytes, receiver frames at most 64 KiB in
both directions (input 128 KiB), and an 8 MiB bound on the container's stdout
for the whole job. The Host re-reads the whole stdout file every 20 ms. Each
`message_update` the receiver forwards carries the accumulated message, so the
job's output grows quadratically with answer length (D4, about 1 500 output
tokens per response). The lifecycle gate records every forwarded native event
in the store, so the store grows the same way. The H2 snapshot is about
179 000 bytes, and the audit prohibits shell, edits and network.

## Slices

### 1. Response ceiling (D4)

The receiver stops forwarding `message_update` events in the hosted profile.
They are progress projections of the stream: the final message arrives in
`message_end`, and the exact provider bytes are already retained Host-side per
attempt (`stream.sse`, digest on the attempt record). Both Pi grammars in
`lifecycle-store` accept a turn without updates (the provider-disabled fixtures
never produce them), so neither grammar changes. Qualification: a new observer
scenario streams one long answer (far beyond the current ceiling) and must
settle Completed; the existing offline matrix must still pass.

### 2. Host output handling

Read the container's stdout incrementally (byte offset, complete lines only)
instead of re-reading and re-splitting the whole file on every tick, and make
the job-wide output bound and the per-frame bound profile parameters. The coding
profile keeps its current values.

### 3. Analysis profile

A new versioned profile, `work-engine.pi-analysis-hosted.v1`, on the same Host,
ledger, gate entry, transport and containment:

- **Tools.** No shell, no tool or validator container, no repository mount.
  The Host serves read-only tools over a Host-side snapshot whose members are
  pinned by path, size and SHA-256 in the job packet (`snapshot_list`,
  `snapshot_read` with offset and a bounded page). The answer is the final
  assistant text, bounded and retained. A tool list outside the profile is
  refused at `ready`, and the Host checks each request body's `tools` array
  against the profile's pinned tool-spec digest before any provider byte.
- **Request evidence.** Before dispatch, the Host retains each request body
  create-only and records its SHA-256 on the attempt record; failover attempts
  reference the same body.
- **Bounds.** Context, request body, request count, frame and job output
  bounds sized for the H2 snapshot, fixed per profile and admitted by the
  server.
- **Lifecycle.** A gate script for one analysis turn (prompt, any number of
  bounded tool rounds, final answer, settlement, close) and the matching
  grammar in `lifecycle-store`.
- **Admission and preparation.** A server admission class and job preparation
  for the analysis profile.
- **Qualification.** Offline observer scenarios (tool rounds, long answer,
  tools-digest refusal, body retention under failover, crash and cancel) and
  one live run.

The model for an H2 arm is not chosen here (R6); the profile pins the model
named in the job packet, from a closed set.

### 4. Task-host connection

Start with the read-only sizing the route comparison calls P0, done with the
research-lifecycle lane: how a Pi realization under `lifecycle-service` maps
native turns, retains request bytes as native input, produces the delivery
causal record and passes the preparation fence and install predicate. Code in
those crates waits for their owners' architecture acceptance. Production
prepare/adopt (G4) is shared with the Codex route and remains theirs.

## Reviews and billing

Slice reviews run as Pi subagents on the Claude subscription
(`claude-sdk/` namespace): Opus at xhigh for slices 1 and 2, Fable for the
analysis profile's evidence and admission design. Live runs use the hosted
routes (subscription first, API key on failover), within the operator's
2026-11-17 authorization.
