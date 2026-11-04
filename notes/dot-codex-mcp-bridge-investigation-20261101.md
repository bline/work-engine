# Dot and local Codex MCP bridge investigation

Observation date: 2026-11-01 UTC. Status: investigation and candidate milestone,
not an accepted implementation plan or a deployment change.

The first useful increment is an authorized request to an exact local Codex
session, followed by its native turn receipt, completion event and result. The
existing Dot transport and local Codex connection already exist. The missing
boundary is durable, scoped command admission and reconciliation between them.

## Observed system

The Dot-facing endpoint is the session-events MCP service, rather than the
separate Work Engine state-projection adapter. On this machine,
`session-events-http.service`, `session-events-delivery.service` and
`tunnel-client-work-engine.service` were running. The HTTP endpoint is loopback
`http://127.0.0.1:8787/mcp`; the configured secure tunnel carries remote calls.

A live `tools/list` returned exactly:

- `list_sessions`
- `read_session`
- `test_pattern`
- `budget_status`
- `delivery_status`
- `replay_events`

The running release and repository copy of
`skills/session-events/scripts/session_events.py` both had SHA-256
`e46fcbfe4903a7284a2f0d4ffc18671892a1687bba93d5484dbbc791fdff9584`.
The release resolved under
`~/.local/share/work-engine/session-events/releases/6f0e663207201e7b0d2066516c8d419d9ba3a728/`.
Thus the checked source describes the active endpoint's implementation.

The [session-events contract](../skills/session-events/SKILL.md) also describes
`session.turn.ended`, optional board/budget/worker context, signed delivery and
replay. Board and closure context are currently embedded in enriched events;
the six tools above do not expose separate general board or closure readers.

The local transport is already implemented by
[codex_app.py](../skills/manual-orchestration/scripts/codex_app.py): WebSocket
JSON-RPC over `~/.codex/app-server-control/app-server-control.sock`.
[Codex.deliver](../skills/manual-orchestration/scripts/role_host.py#L490)
checks status, resumes the exact thread, calls `turn/start`, and returns a native
turn ID. The host also implements launch, status and interruption operations.

A direct connection using that client successfully initialized and called
`thread/loaded/list`. The response contained four loaded IDs and `nextCursor`.
This establishes read connectivity, not complete enumeration or command safety.
The installed CLI reported `codex-cli 0.164.0`. No turn was submitted and no
model worker was launched for this investigation.

## Boundaries that need implementation

**Caller and execution authority.** The current service takes one configured
`--principal` (default `tunnel`), and its HTTP handler does not extract a
per-request user identity. The upstream tunnel's identity guarantees were not
audited here. A command-capable bridge needs a trusted binding from its admitted
caller to locally issued execution authority. A caller-supplied principal,
`approved: true`, a board message or a budget observation cannot establish that
authority. Exact sessions, workspace, input, allowed operation, expiry and any
authorized budget/profile restrictions belong in that binding. Acknowledging a
decision packet remains distinct from approving it or accepting work.

**Duplicate and uncertain entry.** `Codex.deliver` writes its ledger row after
the native turn starts. An MCP timeout can therefore leave a running native
turn without a recorded acknowledgement. A thin wrapper that retries this
method would not establish safe duplicate handling. The proposed operation
owner records intent before entry, binds one request ID to immutable inputs,
and reconciles native identity after interruption. A repeated identical request
returns the same operation; a changed payload conflicts. Unknown entry remains
unknown and is not automatically resent. This is not a claim of universally
exactly-once execution or effects.

The existing
[same-session controller](../skills/manual-orchestration/scripts/auto_continue.py#L202)
has useful examples of persisted intent, native input correlation and recovery
without resending. It owns a different continuation contract; it is not a
general-purpose execution grant or a ready-made bridge backend.

**Concurrency.** Reading idle status before `turn/start` does not atomically
exclude another client. The continuation controller explicitly records this
limitation. A bridge lock coordinates bridge writers only. The first trial can
use a dedicated session with one submission owner; support for arbitrary TUI
and other concurrent writers needs native enforcement or an explicitly bounded
concurrency contract. The bridge cannot promise that its own lock fences them.

**Completion and closure.** A root turn ending does not establish worker,
process or cost closure. Current `closure_view` is a snapshot built from known
lineage sources; it does not observe every arbitrary process, and queued work
can start later. Returned state needs separate native turn status, result,
worker observations, owned-process observations and usage coverage. Missing
observations stay unavailable. Execution deadlines, admission expiry and
observed cancellation are also distinct facts.

## Candidate first milestone

Keep MCP as a typed adapter to deterministic command mechanics. New first-party
execution mechanics belong in Rust under the
[Rust development direction](../docs/rust-development.md), with the existing
Codex daemon retaining session and turn ownership. The existing event service
can continue to supply observations. The Rust service's exact package placement
and any temporary transport interoperability remain implementation decisions;
this investigation does not authorize replacing a running service.

The smallest command surface is:

| Candidate operation | Meaning |
| --- | --- |
| Submit a session request | Admit one locally authorized input for an exact existing session and return a durable operation reference. |
| Read an operation | Return its admission/entry state, native session and turn IDs, and any uncertainty. |
| Read an operation result | Return bounded result content and authorized artifact references with native completion and closure/usage evidence kept distinct. |

Existing session readers, delivery inspection and replay remain useful. Task
creation, steering an active turn, interruption, general board access and
decision-packet operations are separate additions with their respective owners.
Neither a shell command endpoint nor an arbitrary native-RPC passthrough is
needed. A future React editor can consume the same public backend operations;
it need not reproduce admission or receipt semantics in the browser.

Acceptance for this candidate milestone:

1. Offline checks establish rejection of an unauthorized caller, wrong target,
   changed input under an existing request ID, expired authority and an
   unsupported busy target.
2. Duplicate calls return the original operation. Crash/lost-ack checks preserve
   entry uncertainty and reconcile an existing native turn without resubmission.
3. In a separately authorized live trial, Dot submits one harmless bounded
   request to a dedicated local Codex session through the actual tunnel.
4. The submission receipt and completion event resolve to the same native
   session and turn. Dot retrieves the result directly, with explicit closure
   and usage coverage, and can recover delivery through replay or operation
   reads. Correlated MCP/tunnel records and the native receipt show one admitted
   Codex turn reached directly. Dot's task history establishes that no hosted
   helper executed the submission; a completion callback may still wake Dot.

The first live trial remains unperformed. The working direct read connection
does not satisfy these acceptance conditions.

## Evidence basis

Repository HEAD at observation was
`6f0e663207201e7b0d2066516c8d419d9ba3a728`, with unrelated working-tree changes
present. Retrieval used Tier 2 verification in graph project
`home-bline-code-work-engine`, indexed generation `2026-11-01T03:59:09Z`, store
generation `uf6d2e25d44647db0g8`, plus exact source and read-only local service
observations. Targeted symbol queries were exhausted; broad candidate searches
were not treated as exhaustive inventories.

The graph verified exact snippets for `Server.call_tool` and `Codex.deliver`.
Its traces did not fully resolve dictionary dispatch, and a generic `next`
reference introduced unrelated targets. Material dispatch and call relationships
therefore use the exact snippets and direct source (`graph_ambiguity` fallback).
Coverage checks on the governing documents and cited implementation files
returned content-verified metadata matches with no recorded gaps. That is a
best-effort coverage signal, not proof of completeness. The deployed service
source was checked directly by digest; it is outside this repository's graph.
