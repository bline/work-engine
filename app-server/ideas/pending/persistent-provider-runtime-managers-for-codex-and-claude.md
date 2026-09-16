# Persistent Provider Runtime Managers for Codex and Claude

## Status

Idea / architecture direction.

This document proposes replacing per-inference provider process launches with provider runtime managers that retain reusable harness and conversation state across Work Engine turns.

The immediate targets are Codex and Claude.

The proposal does **not** require the two providers to share the same runtime implementation. Instead, Work Engine should own a common semantic session lifecycle while each provider realizes that lifecycle using the strongest native mechanism it exposes.

**Reframed 2026-09-16, after real-code investigation (see "Real-code findings," below) and against `app-server/docs/architecture/runtime-realization.md`.** The motivating problem is not provider-specific: launching a fresh CLI process per inference pays startup/harness/auth/context-construction overhead every turn. The direction this document should now take is an **abstract, provider-neutral inference-launch efficiency port** — not "a Codex manager and a Claude manager," but one boundary that does *best-effort* launch efficiency (reuse a persistent runtime where available, fall back to a fresh launch where it isn't), with each provider realizing it however it natively can. This is not a new top-level concept: `runtime-realization.md`'s own `ProviderTurnPort`/`HarnessRuntimePort` already is that boundary. This document's own contribution is the launch-efficiency policy sitting behind those ports, not a new port above them — see "Relationship to the architecture views," below.

**Also found 2026-09-16: the two providers are not starting from the same place.** Codex already has a real, working generation-manager layer (`executable-generation-bootstrap.mjs`, `executable-generation-dispatch.mjs`) that implements most of what this document asks for, under a different name. Claude has none of it — every turn is still a fresh CLI process. The "Migration strategy" and "Initial spike questions" sections below are corrected accordingly, not left as originally drafted.

---

## Problem

Work Engine currently treats many provider invocations too much like isolated command executions.

Conceptually:

```text
work requested
    ↓
launch provider process
    ↓
initialize provider harness
    ↓
establish authentication/configuration
    ↓
construct context
    ↓
perform inference
    ↓
receive result
    ↓
process exits
```

This is simple, but increasingly mismatched with the way Work Engine operates.

A builder, reviewer, supervisor, researcher, or other role frequently performs multiple related turns within the same logical activity.

Examples:

- a reviewer examines a candidate;
- the builder addresses findings;
- the same reviewer examines the revision;
- another correction occurs;
- the reviewer performs closure review.

Launching a new provider runtime for every one of those turns throws away useful runtime state and repeatedly pays initialization costs.

It also conflates several different lifetimes:

```text
provider process lifetime
conversation lifetime
role lifetime
attempt lifetime
candidate lifetime
slice lifetime
```

Those should not necessarily be identical.

The desired direction is to make provider runtime state a managed resource rather than an incidental consequence of invoking a CLI.

**Correction, 2026-09-16:** this diagram describes Claude's current reality exactly (confirmed directly in `native-claude-code-adapter.mjs` — one `python3`→`claude` process per `execute()` call, every turn). It does **not** describe Codex's current reality — Codex already runs behind a persistent, generation-scoped app-server process (`ForkedExecutableGenerationWorker`, one worker/app-server pair per "generation," not per turn), with multi-turn/multi-thread admission already tracked in real code (`turnAdmissions`/`turnThreads`/`turnControls` in `executable-generation-dispatch.mjs`). The problem this document exists to solve is real, but it is **Claude-shaped today, not symmetric across both providers** — see "Real-code findings," below, before assuming either provider needs the same amount of new work.

---

# Core idea

Introduce a provider runtime manager between Work Engine orchestration and provider-specific harnesses.

```text
                         Work Engine

                semantic provider sessions
                          │
             ┌────────────┴────────────┐
             │                         │
             ▼                         ▼
       Codex Manager              Claude Manager
             │                         │
             ▼                         ▼
       Codex runtime              Claude runtime
```

Work Engine owns the **semantic session**.

The provider manager owns its **runtime realization**.

That distinction is central.

A logical Work Engine session might be:

```text
review:S18:candidate-4
```

Its current realization might be:

```text
Codex thread 019...
```

or:

```text
Claude session a81...
```

The provider-specific object is not authoritative workflow state. It is a replaceable runtime projection of Work Engine state.

**Reframed 2026-09-16:** the "provider runtime manager" boxes above are not a new architectural layer above `ProviderTurnPort`/`HarnessRuntimePort` (`runtime-realization.md`'s own existing boundary, already called through `ManifestRoleRuntime.deliverTurn` today). They are **best-effort launch-efficiency policy realized behind those ports** — reuse a persistent runtime where the provider supports it, fall back to a fresh launch where it doesn't, and never let a caller need to know which happened. `RoleRealization`'s own schema already carries `provider_turn`/`harness_runtime`/`dependencies` fields naming a concrete provider binding and generation (e.g. `codex.runtime @ generation 31`) — a persistent session's own identity is one more dependency entry in that same schema, not a new concept. Concretely: Codex's "manager" already exists as `executable-generation-*`; Claude's does not exist yet. This document does not need — and should not assume — that both providers arrive at an identical internal shape merely because they answer the same abstract port.

---

# Real-code findings (2026-09-16 investigation)

Grounded directly in `app-server/src`, not assumed from this document's own earlier sketch. Two independent code investigations (Codex, Claude) plus a direct check against `app-server/docs/architecture/`.

## Codex — mostly already built, under a different name

- **Launch/ownership path.** `StdioJsonRpcTransport.spawn()` (`stdio-json-rpc-transport.mjs:8-15`) runs `spawn("codex", ["app-server", "--stdio"])` as a real child process, wrapped by `GenerationBoundAppServerTransport` (`executable-generation-dispatch.mjs:196-240`), itself owned by a forked Node worker (`ForkedExecutableGenerationWorker.spawn`, `executable-generation-bootstrap.mjs:363`) — **one worker/app-server pair per generation, not per turn.** `CodexAppServerAdapter` (`codex-app-server-adapter.mjs:435-448`) receives the transport by injection; it never spawns anything itself. This is already the manager layer this document asked for.
- **Multi-thread admission** is already modeled as real `Map`s (`turnAdmissions`/`turnThreads`/`turnControls`, `executable-generation-dispatch.mjs:225-227`), demultiplexed by `turnId` (cross-checked against `notification.params?.threadId`, lines 214-216) — not a hypothetical to design, already running.
- **Generation identity and continuity across replacement is a live, documented invariant**, quoted directly from the code's own comment: *"Operator projection identity belongs to the stable transport. Executable generations are replaceable and must not forget an already-open UI thread when a successor activates"* (`executable-generation-dispatch.mjs:230-233`, backed by `this.operatorThreadIds`). This is the identical fenced-handoff shape `mechanisms/resource-lease-and-fencing.md` generalizes, already implemented specifically for Codex's operator threads.
- **Disk persistence**: Codex's own rollout file (`readCodexRolloutSnapshot`, `codex-app-server-adapter.mjs:329,600`) plus a bounded `RetainedTurnOutputStore` (`RETAINED_TURN_COMPLETION_LIMIT = 256`, line 21) — explicitly retained completions, not everything, not unbounded.
- **The redirection seam already exists**: `CodexAppServerAdapter` takes `transport` as a constructor-injected dependency rather than owning process lifecycle. Nothing about higher-level role contracts needs to change to recognize this layer formally.
- **Real gaps found, not yet addressed by any existing code**: no explicit thread archive/terminate API (termination looks implicit — a thread simply stops being referenced, not a modeled lifecycle state); process-global auth/config is inherited wholesale by the forked worker via `...process.env` spread (`executable-generation-bootstrap.mjs:369`) — not scoped per-thread; `context-transition-lease.mjs`'s exact interaction with retained Codex threads was not confirmed either way in this pass (flagged as unconfirmed, not asserted).

## Claude — none of this exists yet

- **Confirmed CLI print mode, not the Agent SDK**: `claudeArgs = ["-p", "--effort", ..., "--output-format", "json", ...]` (`native-claude-code-adapter.mjs:572-576`), spawned via a Python transport wrapper. No `@anthropic-ai/claude-agent-sdk` import found anywhere in `app-server/src`.
- **One process per turn, exactly the pattern this document exists to eliminate**: one `python3` process per `execute()` call (line 600, calling `spawn()` at line 152).
- **"Cold reuse" already works; "hot reuse" does not exist.** Continuity today is entirely via Claude's own `--session-id`/`--resume` flags (line 573) against on-disk JSONL transcripts (`<stateRoot>/native-claude/<digest(instanceId)>/<sessionUuid>.jsonl`, lines 442/490/496) — a fresh process every time, resuming prior context from disk. A live, retained process across turns does not exist anywhere in current code.
- **`CLAUDE_CONFIG_DIR` isolation is already real and per-instance** (`env.CLAUDE_CONFIG_DIR = configRoot`, lines 538/581) — any session manager must respect this existing isolation boundary, not invent a new one.
- **The credential broker this document assumes exists does not exist yet.** Today, `readClaudeLoginAccessProjection` (line 54) takes a one-time snapshot of the OAuth token at process launch (`#prepareAuthentication`, line 386) and copies it into the isolated config root. There is no mechanism for a long-lived runtime to detect or react to token rotation mid-session — consistent with `fenced-oauth-credential-tip-broker.md`'s own status as still unbuilt.
- **A real, independent gap found**: `retire(instanceId)` (line 668) exists and hard-deletes an instance directory, but a repository-wide grep found **zero callers of `.retire(` anywhere in `slice-campaign`** — defined, never invoked from any attempt-closure path today. Worth fixing regardless of whether this proposal moves forward.
- Genuinely open, not answerable from current code: when live retention should be preferred over resume (no hot-retention path exists yet to compare against); whether Work Engine should own its own `SessionStore` (today it just reads Claude's own JSONL files directly).

## Shared — answered against the settled architecture, not re-derived from scratch

- **Ownership**: not a new dimension. This is new mechanics *within* `runtime-realization.md`, which already owns "resolution of concrete runtime composition, the immutable `RoleRealization` artifact itself, its invalidation, and its rematerialization."
- **No new parallel concept needed**: `RoleRealization`'s own schema already has `provider_turn`, `harness_runtime`, and `dependencies` (with exact generation numbers) — a persistent session's identity is one more dependency entry, not a new abstraction.
- **Coordinate boundary**: the realization's own identity (role + realization generation), not role alone or attempt/slice alone — consistent with that dimension's own Key Invariant 7 ("one operation executes under exactly one stable realization"). A fresh realization after invalidation gets a fresh session.
- **Durable vs. telemetry-only events**: acquire/retain/hibernate/resume/destroy and runtime failures feed directly into `runtime-realization.md`'s own invalidation/rematerialization records and must be durable; the observability list this document already proposes (cold/warm start counts, latencies, token counts) is telemetry only, the same shape as `context-lifecycle.md`'s own shadow-mode evidence loop.
- **Minimum interface**: exactly `ProviderTurnPort`/`HarnessRuntimePort` (`deliverTurn`/`waitForTurnCompletion`/`runEphemeralTurn`) — a session manager slots in behind that interface; role code needs zero changes, confirmed directly against `claude-runtime-adapter-and-context-ownership-pilot.md`'s own citation of `ManifestRoleRuntime.deliverTurn`.
- **Session affinity**: Runtime Realization owns the actual admission decision, reusing `mechanisms/candidate-resolution-and-admission.md`'s own shape (available sessions ∩ authorized reuse ∩ satisfies current role contract → 0/1/N). Execution characterization may emit a hint (`sessionContinuity: required/preferred/irrelevant`, as this document already proposes); it does not own the decision.
- **Stale-state invariant**: `authority-and-ownership.md` §12's own generalized invalidation-never-mints-authority invariant applies directly, and points at a concrete, already-built mechanism to reuse rather than invent: `mechanisms/resource-lease-and-fencing.md`'s fencing-token pattern. A provider session should be leased the same way fenced active-binding already is, so a superseded generation cannot exercise authority after Work Engine's own realization state has moved on — this document's own credential-generation concern (see "Interaction with the Claude credential broker," below) and Codex-generation-tracking concern are both concrete instances of exactly that mechanism's shape.
- **Genuinely open, not answered here**: which current process-launch assumptions leak into supervisor/builder/reviewer role code (`role code requires zero changes` is this document's own stated design principle, not yet verified true against the actual role implementations — needs the same direct-code investigation as the provider-specific questions above, not assumed).

---

# Goals

The initial goals are:

1. Avoid launching a new provider process for every inference when the provider supports a longer-lived runtime.
2. Preserve provider context across related turns where doing so improves efficiency or continuity.
3. Allow runtime resources to be released without necessarily destroying conversational state.
4. Separate Work Engine semantic lifecycle from provider process lifecycle.
5. Give Work Engine explicit control over retention, reuse, suspension, replacement, and destruction.
6. Preserve provider-native harness behavior rather than recreating Codex or Claude behavior around raw model APIs.
7. Create a common Work Engine abstraction without pretending Codex and Claude expose identical runtime models.
8. Establish telemetry capable of determining when retention actually saves time, tokens, and repeated inference.

---

# Non-goals

This proposal does not initially attempt to:

- replace Codex or Claude harness behavior;
- call raw model APIs directly;
- maintain arbitrary provider state forever;
- force Codex and Claude into an identical internal architecture;
- implement a generalized distributed provider scheduler;
- make provider conversation history authoritative Work Engine state;
- retain every session merely because retention is possible.

Retention should remain purposeful and bounded.

---

# Semantic session

Work Engine should introduce an explicit provider-session concept.

Conceptually:

```text
ProviderSession

identity
    provider
    model
    Work Engine coordinate
    semantic purpose

lifecycle
    acquire
    turn
    interrupt
    retain
    hibernate
    resume
    fork
    release
    destroy

runtime realization
    provider-specific
```

A possible interface shape:

```ts
interface ProviderSession {
    readonly id: ProviderSessionId;
    readonly provider: ProviderId;
    readonly state: ProviderSessionState;

    turn(request: ProviderTurnRequest): Promise<ProviderTurnResult>;

    interrupt(): Promise<void>;

    fork(options?: ForkOptions): Promise<ProviderSession>;

    release(policy?: ReleasePolicy): Promise<void>;
}
```

Acquisition might look approximately like:

```ts
const reviewer = await providers.acquire({
    provider: "claude",
    model: "opus",
    role: "reviewer",
    coordinate,
    lifetime: "review-round",
});

const firstReview = await reviewer.turn(reviewRequest);

...

const closureReview = await reviewer.turn(closureRequest);

await reviewer.release();
```

Callers should not normally manipulate native Codex thread IDs or Claude session IDs directly.

Those belong to the runtime manager.

---

# Session state model

A useful semantic lifecycle is:

```text
ABSENT
   │
   │ acquire
   ▼
ACTIVE
   │
   ├──────── retain ────────► ACTIVE
   │
   ├──────── hibernate ─────► HIBERNATED
   │                           │
   │                           │ resume
   │                           ▼
   │                         ACTIVE
   │
   └──────── destroy ────────► DESTROYED
```

However, runtime liveness should be represented separately from semantic continuity.

For example:

```text
semantic state:
    active
    resumable
    destroyed

runtime state:
    live
    detached
    unavailable
```

This allows:

```text
semantic conversation exists
+
provider process does not
```

without treating the session as lost.

That distinction is especially important for Claude.

---

# Codex realization

Codex exposes a natural manager boundary through `codex app-server`.

The app-server supports thread-oriented operations including starting and resuming threads, and current Codex also has an in-process implementation that deliberately preserves app-server semantics while removing the process boundary. citeturn456341search1turn456341search4

The first Work Engine implementation does not need to embed Codex.

Instead:

```text
Work Engine
     │
     ▼
Codex Manager
     │
     │ ensure server
     ▼
codex app-server
     │
     ├── thread A
     ├── thread B
     ├── thread C
     └── thread D
```

The manager owns one long-lived app-server instance initially.

A Codex provider session is realized primarily as:

```text
ProviderSession
    ↓
Codex thread
```

Inference becomes:

```text
thread/start       once

turn/start
turn/start
turn/start
...

thread retained / resumed / released
```

rather than:

```text
launch codex
perform turn
exit

launch codex
perform turn
exit
```

Codex currently supports `thread/start` with explicit model selection and `thread/resume` of existing threads. A running thread can also be rejoined by ID. citeturn456341search4

---

## Codex Manager responsibilities

The Codex Manager should own:

```text
app-server discovery
app-server launch
app-server readiness
protocol initialization
connection ownership
reconnection
server generation
thread creation
thread lookup
thread resume
thread destruction/retention policy
turn dispatch
turn interruption
notification/event routing
thread → Work Engine coordinate mapping
server-failure reconciliation
```

Conceptually:

```text
CodexManager

server:
    state
    generation
    pid
    connection

sessions:
    ProviderSession A -> Codex thread X
    ProviderSession B -> Codex thread Y
    ProviderSession C -> Codex thread Z
```

---

# Codex server lifecycle

Start with a single managed server.

```text
request arrives
      │
      ▼
CodexManager.ensureRuntime()
      │
      ├── running + healthy
      │       └── reuse
      │
      └── absent / dead
              └── launch app-server
                      │
                      ▼
                   initialize
                      │
                      ▼
                     ready
```

Do not introduce a Codex server pool without evidence that one server creates a meaningful bottleneck or isolation problem.

Pooling introduces additional questions:

- thread placement;
- server affinity;
- generation ownership;
- failover;
- credential synchronization;
- load balancing;
- runtime isolation.

Those should be consequences of measured requirements rather than assumptions.

---

# Claude realization

Claude exposes a different runtime model.

**Dependency noted 2026-09-16: this section implicitly assumes the Agent SDK as Claude's realization path.** Confirmed directly against real code that today's production path is CLI print mode, not the Agent SDK — see "Real-code findings," above. Whether Work Engine moves to the Agent SDK at all is [`claude-runtime-adapter-and-context-ownership-pilot.md`](claude-runtime-adapter-and-context-ownership-pilot.md)'s own open question (its H0/H1/H2 conditions, gated on that document's own Pilot 0), not something this document decides or should assume settled. If that pilot recommends staying on CLI print mode, the persistence mechanics below need re-deriving from CLI-level session/resume semantics (`--session-id`/`--resume` against on-disk transcripts, already confirmed real) rather than the Agent SDK's own session model described below. The two ideas should be read together, not in either direction's isolation.

The Claude Agent SDK supports persistent conversation sessions, explicit session IDs, resume, continue, and forks. A resumed session restores previous conversational context including prior analysis and files read. citeturn802169search0turn802169search3

It also supports external `SessionStore` implementations, allowing transcript state to be persisted independently of one process or host. citeturn802169search1

Therefore:

```text
ProviderSession
    ↓
Claude conversation/session
    ↓
optional live runtime realization
```

The session and the live runtime should not be treated as the same object.

Conceptually:

```text
Claude Manager
    │
    ├── LIVE
    │     Claude session A
    │     current runtime retained
    │
    ├── LIVE
    │     Claude session B
    │     current runtime retained
    │
    └── HIBERNATED
          Claude session C
          transcript retained
          no live runtime required
```

This provides two reuse mechanisms:

### Hot reuse

Keep an active Claude SDK/client interaction alive across closely related turns.

Useful for:

```text
review
   ↓
fix
   ↓
re-review
   ↓
fix
   ↓
closure review
```

### Cold reuse

Release the runtime but retain the Claude session ID/transcript.

Later:

```text
resume session
    ↓
new runtime realization
    ↓
conversation continues
```

Claude explicitly supports resuming a specific session by ID and forking an existing session into a separate history. citeturn802169search0

---

# Claude Manager responsibilities

The Claude Manager should own:

```text
live client/runtime admission
session IDs
live session registry
session persistence
resume
fork
interrupt
runtime release
idle eviction
authentication projection
credential-broker interaction
session cleanup
runtime failure recovery
Work Engine coordinate → Claude session mapping
```

Possible state:

```text
ClaudeManager

live:
    review:S18:C4
        sessionId: abc
        runtimeGeneration: 7
        lastUsed: ...

    builder:S18:C5
        sessionId: def
        runtimeGeneration: 2
        lastUsed: ...

hibernated:
    review:S17:C2
        sessionId: xyz
        persisted: true
```

---

# Provider-neutral lifecycle, provider-specific realization

The architecture should deliberately stop at the semantic lifecycle boundary.

For example:

```text
                   ProviderSession

                        acquire
                           │
                           ▼
                         turn
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
            retain      hibernate      destroy
```

But realization differs:

```text
CODEX

retain
    thread remains active in app-server

hibernate
    thread becomes resumable/persisted

resume
    thread/resume

destroy
    release native state according to Codex semantics
```

versus:

```text
CLAUDE

retain
    preserve active client/runtime where useful

hibernate
    terminate live runtime
    preserve session/transcript

resume
    create runtime
    resume session ID

destroy
    terminate runtime
    discard/delete retained session state
```

This asymmetry is desirable.

A provider-neutral abstraction should represent common semantics, not erase important provider capabilities.

---

# Review-round reuse

Reviewer continuity is the clearest initial use case.

Current conceptual behavior:

```text
candidate
    ↓
launch reviewer
    ↓
review
    ↓
exit

builder repair

launch reviewer
    ↓
reconstruct review history
    ↓
review repair
    ↓
exit

builder repair

launch reviewer
    ↓
reconstruct history again
    ↓
closure review
```

Proposed behavior:

```text
candidate
    ↓
acquire reviewer session
    ↓
review
    │
    │ retain
    ▼
builder repair
    │
    ▼
same reviewer session
    ↓
review repair
    │
    │ retain
    ▼
builder repair
    │
    ▼
same reviewer session
    ↓
closure review
    │
    ▼
release / hibernate / destroy
```

This potentially improves two separate dimensions.

## Runtime efficiency

Avoid repeated:

```text
process startup
harness startup
configuration loading
authentication initialization
session bootstrap
```

## Inference efficiency

The reviewer already possesses contextual knowledge such as:

```text
what candidate was inspected
what findings were issued
why those findings mattered
what evidence supported them
what repair was requested
```

Work Engine therefore does not necessarily need to reconstruct the complete review narrative in every subsequent prompt.

This may reduce both latency and input-token consumption.

These benefits should be measured separately.

---

# Session lifetime policies

Provider sessions should have explicit lifetime intent.

Possible initial policies:

## One turn

```text
ONE_SHOT
```

Create session, perform task, destroy.

Appropriate for isolated work where continuation has little expected value.

## Operation

```text
OPERATION
```

Retain while one logical operation remains active.

## Review round

```text
REVIEW_ROUND
```

Retain across candidate review and repair iterations.

## Attempt

```text
ATTEMPT
```

Retain for the lifetime of one Work Engine attempt.

## Slice

```text
SLICE
```

Potentially retain across related work within a slice.

This should probably not be the default until context-growth behavior is understood.

---

# Release policy

`release()` should not automatically mean `destroy()`.

For example:

```ts
await session.release({
    disposition: "hibernate",
});
```

Possible dispositions:

```text
retain
hibernate
destroy
manager-decides
```

`manager-decides` could apply evidence-based policy using:

```text
semantic lifetime
idle duration
expected reuse
runtime pressure
context size
provider constraints
recent failure state
```

Initially, however, deterministic policy is preferable to inference.

---

# Runtime admission

Persistent sessions introduce resource ownership.

The manager therefore needs explicit admission.

Conceptually:

```text
session request
      │
      ▼
provider manager
      │
      ▼
admission
      │
      ├── reuse existing session
      │
      ├── allocate new runtime capacity
      │
      ├── hibernate idle session
      │
      └── queue request
```

For Codex, one server may support many threads.

For Claude, the number of simultaneously live runtime realizations may need tighter management.

The Work Engine-facing abstraction should not require callers to know those details.

---

# Idle retention

The manager may eventually use bounded idle retention.

Example:

```text
ACTIVE
   │
   │ no use for retention window
   ▼
HIBERNATED
```

The retention window should not initially be chosen as an arbitrary optimization constant and forgotten.

Telemetry should determine:

```text
how often sessions are reused
time between related turns
startup cost avoided
memory/resource cost
context growth
resume cost
failure rate
```

That can later justify provider-specific retention policy.

---

# Authority and ownership

Work Engine remains authoritative over semantic workflow state.

Provider sessions are runtime projections.

Therefore:

```text
Work Engine owns:

coordinate
role
attempt identity
candidate identity
authority
workflow status
evidence
claims
state packets
replacement policy
```

Provider managers own:

```text
native session/thread IDs
native runtime health
process state
transport state
provider events
session persistence mechanics
provider-specific resume behavior
```

This gives an important invariant:

> Loss of a provider runtime must not imply loss of authoritative Work Engine state.

A live provider session may contain useful context that makes continuation cheaper, but correctness cannot depend on that session being immortal.

---

# Failure and recovery

The manager boundary gives provider-runtime failure an explicit home.

## Codex example

```text
app-server generation N dies
        │
        ▼
Codex Manager marks generation unavailable
        │
        ▼
start generation N+1
        │
        ▼
for each affected semantic session:
    determine native resumability
        │
        ├── resumable → restore
        └── not resumable → reconstruct from WE state
```

Codex thread persistence should be validated carefully. Current upstream behavior has had version-specific edge cases, including a recent regression around resuming zero-turn threads, so Work Engine should not assume that receiving a thread ID is equivalent to durable resumability. citeturn456341search2

## Claude example

```text
live Claude realization dies
        │
        ▼
semantic session remains
        │
        ├── persisted transcript available
        │       └── resume session
        │
        └── persistence unavailable/corrupt
                └── reconstruct from WE state
```

Again:

```text
provider state = optimization + continuity aid
Work Engine state = authority
```

---

# Interaction with the Claude credential broker

The existing credential-tip broker remains conceptually separate from session management.

```text
Claude Manager
     │
     ├── asks credential broker for usable projection
     │
     └── owns conversation/runtime lifecycle

Credential Broker
     │
     └── owns credential generation and refresh authority
```

A Claude session must not gain refresh authority merely because it is retained longer.

The same fenced credential-generation semantics should continue to apply.

Longer-lived runtimes may actually make credential-generation boundaries more important because a runtime can outlive the credential projection with which it began.

The manager therefore needs an explicit answer for:

```text
session remains valid
credential generation changes
```

Possible behavior:

```text
next turn detects auth failure
    ↓
broker resolves credential generation
    ↓
manager replaces or rebinds runtime realization
    ↓
semantic session resumes
```

The provider session should survive runtime replacement where the provider permits it.

---

# Branching

Both Work Engine and provider harnesses increasingly have notions of continuation and branching.

These need to remain distinct.

A provider fork means:

```text
shared prior conversation
          │
          ├── conversation A
          └── conversation B
```

Claude explicitly supports session forking while keeping the original session intact. citeturn802169search0

Codex also exposes thread lifecycle primitives that should be evaluated for the same role.

But provider branching must not silently define Work Engine semantic branching.

The likely rule is:

> Work Engine authorizes the branch; the provider manager realizes it using the strongest provider-native primitive available.

Thus:

```text
WE branch
    ↓
ProviderSession.fork()
    ↓
native provider fork/resume/copy mechanism
```

rather than discovering native provider branches after the fact and trying to infer workflow semantics from them.

---

# Context lifecycle interaction

Persistent provider sessions do not replace Work Engine context lifecycle management.

They add another possible realization.

For example:

```text
context healthy
    ↓
continue native provider session

context replacement required
    ↓
produce authoritative state packet
    ↓
destroy or archive old realization
    ↓
create replacement provider session
    ↓
inject/reconcile state
```

This preserves the existing principle that context replacement is a governed Work Engine operation rather than an accidental consequence of provider compaction or process death.

Native provider resume is therefore one continuation mechanism, not the sole continuation mechanism.

---

# Observability

The managers should emit enough telemetry to answer whether this architecture actually helps.

At minimum:

```text
provider
model
semantic session id
native session/thread id
runtime generation
session lifecycle state

cold start count
warm reuse count
resume count
fork count
hibernate count
destroy count

runtime startup latency
thread/session startup latency
turn latency
resume latency

input tokens
output tokens

session age
turn count
idle duration
context/compaction events

runtime failure count
resume failure count
reconstruction count
```

This allows comparisons such as:

```text
cold reviewer turn
vs
warm reviewer turn
```

and:

```text
fresh review round
vs
retained review round
```

rather than assuming the optimization is valuable.

---

# Potential derived optimization

Once this exists, Work Engine can choose provider-session policy structurally.

For example:

```text
single isolated classifier
    → one-shot

review + expected repair cycle
    → review-round

multi-turn research activity
    → operation lifetime

builder expected to perform several edits
    → attempt lifetime
```

Much of this decision can potentially be deterministic.

The planner or execution characterization may emit:

```text
sessionContinuity:
    required | preferred | irrelevant

expectedTurns:
    1 | bounded-many | unknown

branching:
    none | possible | expected
```

Runtime realization can then be selected without requiring another model judgment.

---

# Suggested internal architecture

```text
                   Work Engine

                       │
                       ▼
               ProviderSessionService
                       │
          ┌────────────┴────────────┐
          │                         │
          ▼                         ▼
   CodexRuntimeManager       ClaudeRuntimeManager
          │                         │
          ▼                         ▼
   app-server manager        session/runtime manager
          │                         │
          ▼                         ▼
      app-server            Claude SDK / harness
          │
      many threads
```

`ProviderSessionService` should contain semantic/common policy.

Provider managers should contain native mechanics.

Avoid putting provider-specific assumptions into the common service.

---

# Possible session record

```ts
interface ProviderSessionRecord {
    id: ProviderSessionId;

    provider: ProviderId;
    model: string;

    coordinate: Coordinate;
    role: string;

    semanticState:
        | "active"
        | "resumable"
        | "destroyed";

    runtimeState:
        | "live"
        | "detached"
        | "unavailable";

    lifetime:
        | "one-shot"
        | "operation"
        | "review-round"
        | "attempt"
        | "slice";

    realization: ProviderRuntimeRealization;

    createdAt: Instant;
    lastUsedAt: Instant;
}
```

Provider realization remains discriminated:

```ts
type ProviderRuntimeRealization =
    | {
          provider: "codex";
          serverGeneration: number;
          threadId: string;
      }
    | {
          provider: "claude";
          sessionId: string;
          runtimeGeneration?: number;
          persisted: boolean;
      };
```

Exact structures should come from code evidence rather than this sketch.

---

# Migration strategy

This should probably be introduced incrementally.

**Corrected 2026-09-16, against the real-code findings above: Phase 1 and most of Phase 2 already exist for Codex.** The original phasing assumed both providers started from zero. They don't — do not schedule work that already exists.

## Phase 1 — Codex manager — **largely already built**

What this phase originally asked for is already real: `executable-generation-bootstrap.mjs`/`executable-generation-dispatch.mjs` already provide one managed app-server per generation with explicit multi-turn/multi-thread tracking. What remains of this phase is recognition and light extension, not construction:

- name and document this layer as the Codex realization of the abstract launch-efficiency port (this document's own job);
- add the missing explicit thread archive/terminate lifecycle (confirmed absent — currently implicit);
- decide whether process-global auth/config inheritance (confirmed real, `...process.env` spread) is acceptable long-term or needs per-thread scoping;
- confirm `context-transition-lease.mjs`'s actual interaction with retained threads (confirmed unchecked, not confirmed absent).

## Phase 2 — Codex session retention — **substantially already built**

Multi-thread admission against one app-server is already real code, not a narrow pilot to introduce. What remains is deciding retention *policy* (which roles/workflows should prefer reuse) using the telemetry this document's own "Observability" section proposes — the mechanism itself does not need to be built.

## Phase 3 — Claude manager — **not started, build from scratch**

Confirmed nothing here exists: every Claude turn is still one fresh `python3`→`claude` process. This phase is the real, substantial, ground-up work in this proposal — move Claude invocation behind explicit session ownership, respecting the already-real `CLAUDE_CONFIG_DIR` per-instance isolation, and decide the credential-broker interaction question (the broker itself doesn't exist yet either — see `fenced-oauth-credential-tip-broker.md`). Fixing the dead `retire(instanceId)` cleanup path (confirmed never called) belongs here regardless of the rest of this phase's fate.

## Phase 4 — Claude reviewer continuity

Unchanged in substance from the original draft — still real, still not started, still needs telemetry before defaulting to production per this document's own original caution.

## Phase 5 — Common semantic session service

Unchanged in substance, but the "common abstraction" is now confirmed to already exist at the port level (`ProviderTurnPort`/`HarnessRuntimePort` in `runtime-realization.md`) — this phase's job is extracting the shared *launch-efficiency policy* (retention lifetime, admission via Candidate Resolution and Admission, fencing via Resource Lease and Fencing) that sits behind that already-settled port, not inventing a new top-level interface.

---

# Initial spike questions

The implementation investigation should answer these with code evidence.

**All 30 questions below were investigated directly against real code and the settled architecture on 2026-09-16 — see "Real-code findings," above, for full citations. `[ANSWERED]` items should not be re-investigated from scratch; `[OPEN]` items remain genuinely unanswered.**

## Codex

1. `[ANSWERED]` What is the current app-server launch and ownership path in Work Engine? — `executable-generation-bootstrap.mjs`/`executable-generation-dispatch.mjs`, one worker/app-server pair per generation.
2. `[ANSWERED, by design not measurement]` Can one app-server safely service the expected number of simultaneous Work Engine threads? — the architecture already assumes yes (`turnThreads`/`turnAdmissions` as real `Map`s); not load-tested.
3. `[ANSWERED]` Which notifications must be demultiplexed by thread and turn? — `turn/completed` by `turnId`, cross-checked against `threadId`.
4. `[OPEN]` What native thread termination or archival semantics exist? — confirmed absent; termination is implicit today.
5. `[ANSWERED]` What survives app-server restart? — operator-facing thread identity is designed to survive generation replacement (`operatorThreadIds`); ordinary turn/thread admission state does not.
6. `[ANSWERED]` Which thread state is disk-persisted versus runtime-only? — Codex's own rollout file plus a bounded `RetainedTurnOutputStore` (256 completions); everything else is runtime-only.
7. `[ANSWERED]` How should app-server generation be represented? — already represented, exactly as "generation," matching `runtime-realization.md`'s own schema field.
8. `[ANSWERED]` What authentication/configuration state is process-global? — confirmed process-global, inherited via `...process.env` spread; not thread-scoped.
9. `[OPEN]` How does current context lifecycle behavior interact with retained threads? — not confirmed either way; needs a dedicated read of `context-transition-lease.mjs`.
10. `[ANSWERED]` Can the existing provider-turn harness be redirected to a manager without changing higher-level contracts? — yes, by construction; the seam already exists (`transport` is already constructor-injected).

## Claude

1. `[ANSWERED]` Is Work Engine currently using CLI print mode, Agent SDK, or a wrapper? — CLI print mode (`-p` flag); no Agent SDK usage anywhere.
2. `[ANSWERED]` What exact process is created for each current inference? — one `python3`→`claude` process per `execute()` call, every turn.
3. `[ANSWERED]` Can the existing transport retain a live session across turns? — no; only cold reuse via `--resume` against on-disk transcripts exists.
4. `[OPEN]` When should live retention be preferred over session resume? — no hot-retention path exists yet to compare against; needs telemetry.
5. `[ANSWERED]` Where are session transcripts currently persisted? — `<stateRoot>/native-claude/<digest(instanceId)>/<sessionUuid>.jsonl`.
6. `[OPEN]` Should Work Engine provide its own `SessionStore`? — no such abstraction exists today either way.
7. `[ANSWERED]` How does `CLAUDE_CONFIG_DIR` isolation interact with resumed sessions? — already fully isolated per review instance; `--resume` only works within the same instance's config root.
8. `[ANSWERED — the broker doesn't exist]` How does the credential-tip broker interact with a runtime that survives credential-generation changes? — it doesn't yet; today's auth is a one-time snapshot at process launch (see `fenced-oauth-credential-tip-broker.md`).
9. `[ANSWERED]` Can sessions be resumed safely after runtime replacement? — yes, via cold reconstruction from the transport receipt + session JSONL; no live-state-loss risk exists because nothing is kept live today.
10. `[ANSWERED — real gap found]` How should session deletion/retention align with Work Engine attempt closure? — `retire(instanceId)` exists but has zero callers anywhere in `slice-campaign`; a real, independent bug to fix regardless of this proposal's fate.

## Shared

1. `[ANSWERED]` Which existing service should own semantic provider sessions? — `runtime-realization.md`; not a new dimension.
2. `[ANSWERED]` Is there already a runtime-realization abstraction that should absorb this? — yes, `RoleRealization`'s own `provider_turn`/`harness_runtime`/`dependencies` fields.
3. `[ANSWERED]` What Work Engine coordinate is the correct ownership boundary? — the realization's own identity (role + generation), not role/attempt/slice alone.
4. `[ANSWERED]` Which lifecycle events must become durable? — acquire/retain/hibernate/resume/destroy and runtime failures.
5. `[ANSWERED]` Which events are telemetry only? — the observability list this document already proposes (start counts, latencies, token counts).
6. `[ANSWERED]` What is the minimum interface needed by the provider-turn harness? — `ProviderTurnPort`/`HarnessRuntimePort`, unchanged; role code needs zero changes.
7. `[ANSWERED]` Does session affinity belong in execution characterization or runtime realization? — Runtime Realization owns the admission decision; execution characterization may only emit a hint.
8. `[OPEN]` Which current process-launch assumptions leak into supervisors/builders/reviewers? — genuinely unconfirmed; needs direct investigation of role code itself, not assumed from this document's own "zero changes" design principle.
9. `[ANSWERED]` Can provider-session identity become evidence in the existing state-packet machinery? — yes, as another `dependencies` entry on `RoleRealization`, not a new concept.
10. `[ANSWERED]` What invariant prevents stale provider state from becoming authoritative? — `authority-and-ownership.md` §12's invalidation-never-mints-authority invariant, concretely realized via `mechanisms/resource-lease-and-fencing.md`'s fencing-token pattern.

**Remaining genuinely open, in priority order: (1) which role-code assumptions leak from process-launch (Shared #8) — investigate before writing any manager code, since it determines how invisible the change can actually be; (2) Claude's live-vs-resume tradeoff and `SessionStore` ownership (Claude #4, #6) — both need telemetry, not more analysis; (3) Codex's thread-archival semantics and context-lifecycle interaction (Codex #4, #9) — smaller, bounded follow-up reads.**

---

# Primary invariant

The central invariant should probably be:

> A provider session is a replaceable runtime realization of Work Engine-owned semantic state. Retaining it may improve continuity and efficiency, but correctness must remain recoverable from authoritative Work Engine state.

That lets Work Engine exploit long-lived native model harnesses without becoming dependent on their continued existence.

---

# Expected consequence

Today the effective unit of provider execution is often close to:

```text
one inference
=
one harness/process realization
```

This direction changes that toward:

```text
one logical activity
=
one semantic provider session
=
many inference turns
```

with runtime realization independently managed underneath it.

For Codex:

```text
one app-server
    → many threads
    → many turns
```

For Claude:

```text
managed sessions
    → retained or resumable runtime realizations
    → many turns
```

The immediate gain is reduced repeated startup and context reconstruction.

The larger architectural gain is that provider runtime lifecycle becomes explicit, inspectable, and governable by Work Engine rather than being an accidental property of command invocation.

---

# Relationship to the architecture views (added 2026-09-16)

This idea does not require a new dimension, mechanism, or substrate in `app-server/docs/architecture/`. It is new mechanics inside one existing dimension, reusing two existing mechanisms directly:

```text
runtime-realization.md
    owns: the RoleRealization artifact, its provider_turn/harness_runtime/
    dependencies fields, resolution and admission, invalidation and
    rematerialization -- this proposal's own "semantic session" is this
    dimension's own truth, not a competing concept

mechanisms/candidate-resolution-and-admission.md
    reused for: session admission (reuse an existing session, hibernate
    an idle one, allocate new capacity, or queue) -- the same
    AVAILABLE ∩ AUTHORIZED ∩ SATISFIES(REQUIRED) → 0/1/N shape this
    mechanism already generalizes

mechanisms/resource-lease-and-fencing.md
    reused for: preventing a superseded provider-session generation from
    exercising authority after Work Engine's own realization state has
    moved on -- the same standing mutual-exclusion-with-fenced-handoff
    shape workspace-coordination's own real resource types already use

authority-and-ownership.md §12
    the generalized invariant this proposal's own "Authority and
    ownership" section already restates independently: a provider
    session may expose or encode authority Work Engine already granted;
    it never mints authority by surviving longer or being retained
```

If this idea is formed into a proposal, it should cite these four directly rather than re-deriving the ownership split from scratch — the split is already settled, and Codex's own real `executable-generation-*` code is already a working instance of most of it.

