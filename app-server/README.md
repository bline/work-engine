# Work Engine App Server Runtime

This boundary is the provider-specific runtime realization for Codex App
Server. It translates authorized Work Engine delivery intent into App Server
protocol effects while keeping role identity, authority, and canonical workflow
state outside provider threads.

The first vertical provides:

- a JSON-lines stdio transport for App Server;
- initialization and fail-closed capability/version negotiation;
- pinned provider-runtime negotiation for model-context replacement, requiring
  explicit `token_budget` configuration and target-model invocation semantics;
- a durable logical-role-to-thread binding registry;
- a closed, digest-attributed runtime manifest that projects logical role
  templates, thread settings, and exact skill inputs;
- a durable workspace-development runtime that assigns operation-namespaced
  local Git worktrees, persists expiring resource leases and monotonically
  increasing fencing generations, and confines accepted-checkpoint publication
  to configured canonical branches through isolated integrated-tree validation;
- exact `SKILL.md` input resolution within configured roots;
- thread-scoped dynamic-tool declaration and dispatch;
- idempotent turn delivery through client message IDs;
- bounded consumption of stable `turn/completed` notifications, including
  terminal failure propagation and final agent-output extraction;
- a protocol-compatible observable transport wrapper that emits bounded,
  content-free request, notification, server-request, token, and close events
  without changing downstream App Server behavior;
- a bounded provider-neutral lifecycle evidence collector fed by a
  version-pinned Codex notification normalizer for token usage and context
  transition signals;
- a provider-neutral pressure controller that requires an explicit
  revision-bound basis-point policy and applies deterministic hysteresis without
  invoking inference or acquiring checkpoint or retirement authority;
- a shadow lifecycle coordinator that composes pressure scheduling, semantic
  inspection, and optional accepted-checkpoint publication while making context
  retirement mechanically unreachable;
- integrity-bound shadow episode receipts, an injected append boundary, and
  revision-homogeneous summaries for later policy comparison without treating
  missing telemetry as zero;
- a schema-migrated SQLite state adapter for restart-safe lifecycle episodes,
  checkpoint fences, checkpoint publications, and lifecycle-ledger heads with
  transactional compare-and-swap semantics;
- deterministic `observed-context-v1` construction with closed trust and
  instruction-applicability vocabulary, explicit completeness limits, and
  Ed25519 host attestation;
- closed `continuation-state-v1` candidate validation with exact source and
  authority references, deterministic candidate revisions, and explicit human
  interaction state;
- a storage-neutral lifecycle-ledger record contract with constrained event
  truth statuses and a verifiable SHA-256 predecessor chain;
- a revision-fenced checkpoint publisher that requires accepted semantic
  verification and exact authority revalidation, then atomically compares
  source, binding, authority, checkpoint-predecessor, and ledger-predecessor
  revisions before storing a checkpoint and its publication evidence;
- a revision-bound in-process transition lease that validates the published
  checkpoint and ledger fence, records readiness, and gates adapter turn,
  dynamic-tool, and runtime-binding ingress around one sterile retirement
  control-turn request; and
- an event-driven transition/reconciliation path that subscribes before
  retirement, admits only a pinned matching compaction signal as a wake-up,
  injects the exact checkpoint once, and keeps domain work fenced until an
  exact predecessor-linked reconciliation receipt is accepted;
- a provider-neutral semantic inference harness that verifies projected source
  bytes, runs distinct compiler and verifier capabilities, and derives the
  verification disposition in host code; and
- deterministic parsing and request-binding validation of the strategic
  planner's version-1 YAML handoff.

Request-bound Work Engine context is compiled into a deterministic text input
item carried by the pinned `TurnStartParams.input` field. It is included in the
idempotency fingerprint. The runtime does not send an unpinned
`additionalContext` extension.

`threadId` is always a replaceable runtime binding. It is never used as the
logical role identity or as canonical workflow state.

Completion results are runtime observations, not canonical workflow state. A
live adapter retains a bounded result cache so fast notifications cannot race
their consumer. A replay whose completion is no longer retained fails with an
explicit reconciliation requirement instead of waiting for an event that has
already occurred.

## Isolated development workspaces and canonical publication

`openWorkspaceDevelopmentRuntime` is the App Server-owned composition boundary
for development workspace allocation and publication. It opens a private
SQLite coordination store, creates local worktrees from exact commits in the
existing repository, and performs no implicit clone, fetch, or push. Builders
receive only their assigned checkout and index; an accepted checkpoint remains
a publication input rather than branch-mutation authority.

The runtime is configured with an explicit canonical-branch allowlist.
`publishAcceptedCheckpoint` acquires the corresponding Git-ref lease inside the
host boundary, reconciles the accepted checkpoint against the observed branch
tip in a separate integration worktree, validates that exact integrated tree,
and admits `update-ref` atomically with the current fencing generation and
expected parent. It refuses checked-out canonical targets, unmanifested
content, semantic conflicts, expired or superseded admissions, validation
mutations, and targets outside the configured allowlist. Publication receipts
bind the starting tip, accepted checkpoint, resulting commit and tree,
validation receipt, resource generation, and authorization reference.

Cleanup preserves the latest private commit under a namespaced Git ref. A
dirty worktree is retained for recovery; a clean worktree can be removed after
restart using its allocation receipt. The user's ordinary checkout, index, and
checked-out branch are never used as agent staging or integration state.

Live integration tests can expose the observable transport in real time:

```bash
WORK_ENGINE_APP_SERVER_INTEGRATION=1 \
WORK_ENGINE_APP_SERVER_TRACE=1 \
node --test app-server/tests/integration.test.mjs
```

Trace events retain method, operation correlation, thread/turn/item identity,
duration, failure type, and bounded token counts. They intentionally exclude
prompts, model output, developer instructions, request context, tool arguments,
and error messages. Observer failures are retained separately and cannot alter
protocol delivery. This is the observability foundation used by the local
`:we command` switchboard, not a workflow or authority surface.

The first local operator switchboard is available with:

```bash
npm run app-server:switchboard -- --trace
```

It starts the pinned App Server, loads `runtime-manifest.yaml`, and preserves
logical role bindings under the user's local state directory by default. Use
`--bindings PATH` to select an isolated registry for a test. The optional
`--enable-token-budget` flag starts App Server with `token_budget` and records
that provider capability in retained environment bindings.

To run manifest roles through the retained semantic lifecycle harness, use:

```bash
npm run app-server:switchboard -- \
  --semantic-shadow \
  --trace \
  --enable-token-budget
```

[`semantic-context-profile.yaml`](semantic-context-profile.yaml) owns the local
pressure bands, hysteresis exits, inspection schedule, and non-publication
setting. `--semantic-profile PATH` selects an alternate closed profile, and
`--semantic-state PATH` selects an isolated SQLite episode store. This makes it
possible to lower thresholds for a bounded live compiler/verifier experiment
without changing the default profile.

Semantic-shadow mode records each completed role turn against matching token
usage, persists the resulting episode, and invokes separate ephemeral compiler
and verifier turns only at configured pressure dispositions. The model response
is followed by a compact lifecycle line, while `:we status` retains the complete
most recent shadow result for inspection.

This mode deliberately cannot publish checkpoints or request context
replacement. Its projection signer is run-scoped, so persisted shadow episodes
are diagnostic evidence rather than reusable checkpoint authority. Enabling
`token_budget` negotiates the future target-model capability but does not make
the shadow harness call `new_context`.

The administrative command surface is deliberately thin:

```text
:we agents
:we attach strategic-planner:main
:we detach
:we status
:we threads
```

Attaching selects where subsequent ordinary lines are delivered; it does not
start a thread or grant workflow authority. The first ordinary line creates or
resumes the manifest-bound role thread and waits for its final model output.
Every line beginning with `:we` is intercepted locally, including unknown
commands, and is never sent to the model. Handoff meaning, packet validation,
acceptance, receipts, and workflow transitions remain owned by roles, skills,
and their deterministic boundaries rather than the switchboard.

Codex TUI reserves the leading `::` form for its own prompt expansion before
`turn/start`, so the switchboard uses the non-reserved `:we` namespace. This
keeps administrative input visible to the proxy instead of consuming it in the
client composer.

The protocol-facing proxy provides a Unix-socket App Server endpoint compatible
with the pinned Codex TUI:

```bash
npm run app-server:proxy -- --socket /tmp/work-engine-app-server.sock --trace
codex --remote unix:///tmp/work-engine-app-server.sock
```

It relays the complete bidirectional App Server JSON-RPC protocol between one
remote client and a private observable stdio App Server. The socket is created
with owner-only permissions, an existing path fails closed, and `/rpc` is the
only accepted WebSocket route. `--enable-token-budget` passes that provider
feature to the private App Server. The proxy also starts one validated
executable-generation worker from an immutable source snapshot. Use
`--generation-state PATH` to isolate its snapshots and durable generation
receipt during testing; otherwise state is scoped by the socket identity under
the user's local state directory.

Hosted manifest-role turns pass through the semantic lifecycle in shadow mode
by default. The proxy loads `semantic-context-profile.yaml`; use
`--semantic-profile PATH` to select another closed profile. The checked-in
`semantic-context-live-profile.yaml` selects the live host and must be paired
with `--enable-token-budget`; use it only after the gated live transition test
passes on the installed Codex version. Completed role
turns are joined only to token telemetry carrying the same turn ID, and their
episodes are stored at `semantic-context.sqlite3` under the proxy's stable
state directory. The SQLite path therefore survives executable-generation
replacement and host restart, while the profile and lifecycle implementation
remain part of the immutable generation and its environment fingerprint.
Comfortable turns persist without semantic inference. Configured inspection
bands may run the ephemeral compiler and verifier. Shadow mode cannot publish
a checkpoint or invoke `new_context`. Live mode keeps the operator turn pending
through checkpoint publication, model-requested context replacement,
reconciliation, and ordered release of input received while admission was
closed.

The proxy uses the connected Codex thread as a UI shell. It intercepts
single-text `turn/start` requests inside the active executable generation:
administrative `:we command` lines are handled locally by the switchboard, while
ordinary lines are delivered to the attached manifest role and returned through
a synthetic shell-turn lifecycle. Commands never enter a role thread. The initial command
surface is `:we agents`, `:we attach role:instance`, `:we detach`, `:we status`, and
`:we threads`; attachments persist outside replaceable generations so a
compatible reload does not silently detach the operator.

The current shell projection is intentionally narrow. A switchboard turn must
contain exactly one text input. The proxy acknowledges an accepted role turn
with a synthetic in-progress shell turn, then forwards the matching completion
when the role turn terminates so the TUI can display progress and keep its
composer lifecycle accurate. Provider role-thread notifications and tool
requests re-enter the generation that admitted the parent dispatch; they are
consumed there and are not forwarded to the UI shell. Rich input, incremental
role-item streaming, and synthetic shell-history reconciliation remain later
presentation work.

The executable-generation foundation now provides the lifecycle kernel needed
for bounded development-time reloads. A reload request made by an admitted turn
establishes an admission fence synchronously, drains all generation-bound work,
captures a closed file inventory into a content-addressed immutable snapshot,
builds and validates an injected candidate, and compares its environment and
bootstrap fingerprints before activation. Active work is always tagged with
the generation that admitted it. The durable file-backed store records the
state sequence, active-generation compare-and-swap, startup reconciliation,
first successor turn, effects, and predecessor retirement; it never treats
activation as proof that the successor has exercised real work successfully.

Candidate classification is closed: unchanged semantic and bootstrap
fingerprints are `implementation_compatible`; a changed role, authority,
capability, skill, or tool environment is `environment_migration_required`; a
changed stable transport/bootstrap boundary is `bootstrap_restart_required`;
and snapshot, build, validation, or activation failure is `candidate_invalid`.
A shared replaceable-substrate arbiter prevents an executable-generation
transition from overlapping a model-context transition when both hosts use the
same arbiter.

The protocol proxy now runs through this stable/reloadable boundary. Its Unix
socket, remote-client correlation, private App Server transport, and server
request correlation remain in the stable process. Client requests and
notifications are admitted against one executable generation, dispatched to
that generation's child worker, and forwarded to App Server by the stable host.
On restart, the host captures the current executable workspace before opening
the proxy socket. An unchanged snapshot resumes the durable active generation.
A changed snapshot is built and validated as a startup candidate. When its
environment fingerprint still matches, one atomic startup reconciliation
receipt advances the durable active pointer to the current workspace
generation. A changed bootstrap fingerprint is recorded as
`bootstrap_restart_completed`, because process startup is the required stable
host restart rather than an in-process generation swap. Capture, build,
validation, or environment-compatibility failure leaves the predecessor record
unchanged and refuses startup instead of silently serving stale code. Startup
reports the resolved state directory, workspace generation, selected
generation, outcome, and reconciliation identity when one was written.

New and resumed proxy threads receive the experimental dynamic-tool namespace
`environment`. `environment.status` reads the active generation, reload fence,
and admitted work without exposing internal admission tokens.
`environment.reload` captures the configured executable inventory, validates a
candidate child, and stages compatible activation. The provider turn remains
admitted until its exact `turn/completed` notification; the proxy delays that
notification while the successor settles, so a later turn cannot race the
swap. Invalid or environment-incompatible candidates remain inactive.

Editing files still creates no activation event. The current worker may
forward App Server protocol operations, apply the thin administrative
switchboard, deliver ordinary input through an already projected manifest
role, or request the two closed lifecycle controls. It cannot grant workflow
authority or reinterpret handoffs. Dynamic tools use the pinned App Server experimental API,
so the worker upgrades the private `initialize` request to opt into that API
and injects the namespace on `thread/start` or `thread/resume`.

Each manifest-bound generation contains the raw runtime manifest, semantic
context profile, exact skill bytes, required role/lifecycle runtime modules,
and closed runtime package dependencies, plus a generated projection whose
bytes participate in the content-addressed snapshot. Manifest, skill, and
semantic-profile digests also participate in the environment fingerprint.
Snapshot-local delivery paths do not change canonical role-environment
identity, while a semantic manifest, skill, profile, or tool-surface change is
classified as an environment migration rather than implementation-compatible
reload.

Reloadable dispatch code also has one bounded route back to the stable host. It
may request the same App Server effect owned by its currently admitted dispatch,
for example issuing a role-thread request while handling an intercepted client
request. Each effect is correlated to the parent worker request, inherits that
request's generation and admission fence, and becomes unavailable as soon as
the dispatch settles. An effect request without a live parent dispatch closes
the worker as a protocol violation. This is the prerequisite for moving
switchboard routing into the replaceable generation without giving the child a
second permanent transport or independent effect authority.

Lifecycle evidence follows the same ownership boundary. The Codex notification
source validates supported provider payloads and projects them into immutable
token-usage or context-transition observations. The in-memory collector assigns
a run-local sequence, bounds retention, and produces per-thread snapshots.
Provider `contextCompaction` items remain explicitly `unclassified`; neither
their names nor their presence establish semantic context replacement. This
collector is an observation surface, not the future durable lifecycle ledger.

Pressure disposition is a separate scheduling surface. The controller accepts
host-projected utilization rather than interpreting provider token telemetry,
and it requires callers to supply every entry and exit threshold. It returns
`comfortable`, `approaching`, `replacement_candidate`, or `critical` with the
policy revision, exact observation, prior disposition, and transition reason.
Each observation names its measurement source and SHA-256-bound source revision;
sequences must increase monotonically. Hysteresis prevents repeated boundary
crossings, while exact immediate replay is idempotent and conflicting reuse
fails. The controller has no default production values and cannot compile
state, validate checkpoint freshness, quarantine a role, notify a user, or
authorize context replacement.

The shadow coordinator consumes an explicit revision-bound schedule declaring
which pressure dispositions justify inspection and whether an accepted result
should be offered to the checkpoint publisher. Every observation produces an
integrity-bound episode receipt, including skipped inspections and failures.
The receipt binds the logical role, runtime binding, pressure policy, schedule,
measurement source, observed-context source when present, semantic outcome,
checkpoint outcome, and the invariant fact that no retirement was requested.
Exact request replay is idempotent; changed evidence under the same episode
identity fails.

Inference capabilities may report normalized token and cost measurements, while
the host independently measures call duration. Episode telemetry can also carry
attributed retained-context and counterfactual avoided-token estimates. Missing
measurements remain `null`. Summary projection is allowed only across one exact
pressure-policy and shadow-schedule revision and reports coverage alongside
totals. The lightweight reference episode store is in-memory. The SQLite
adapter below provides restart-safe local persistence. Outcome follow-up after
a later real transition and policy recommendation or adoption remain outside
this foundation.

For restart-safe local operation, `openSqliteAppServerStateStore` opens an
explicit caller-owned path and applies the versioned App Server state schema.
The adapter implements the same episode `get`/`append`/`receipts` boundary and
the checkpoint publisher's atomic store boundary. One transaction inserts the
immutable checkpoint and ledger evidence, advances the lifecycle-ledger head,
and conditionally advances the complete checkpoint fence. A separate CAS
operation advances source, authority, or runtime-binding fence fields without
rewriting checkpoint or ledger history.

The connection enables foreign keys, WAL journaling, a configured busy timeout,
and `FULL` synchronous durability; the database file is restricted to mode
`0600`. Exact episode replay is idempotent across connections, while changed
episode evidence and stale checkpoint writers fail. Stored JSON is revalidated
against its content revision when read. The current adapter uses Node's built-in
`node:sqlite` implementation, which Node 22 still reports as experimental.
The driver is isolated behind the storage boundary for later replacement.

The SQLite schema does not yet own role bindings, delivery state, transition
leases, notification outbox state, or canonical claims. It also does not provide
database encryption, authenticated writers, backup/restore tooling, or
multi-host coordination. The database location must remain outside tracked
canonical repository content unless the operator explicitly chooses otherwise.

`TokenUsagePressureProjector` turns one retained thread's latest normalized
Codex token-usage observation into a revision-bound basis-point observation.
Its schema-version-1 profile deliberately names
`last.totalTokens / modelContextWindow`; that is an experimental scheduling
measurement, not a claim that either provider field exactly represents the
active model context. Unknown window size remains unavailable rather than
being estimated silently.

`RetainedRoleShadowLifecycleRuntime` composes normal manifest-role delivery,
turn completion, thread-specific lifecycle evidence, the pressure projector,
and a role-scoped `ShadowContextLifecycleCoordinator`. It refuses stale token
usage from an earlier turn. At inspection thresholds, construction of the
authenticated observed-context projection and its bounded source materials
remains an injected host responsibility. The wrapper has no transition lease
or context-replacement path.

Restart recovery derives two coupled values from integrity-checked lifecycle
episodes: the latest role disposition when its pressure-policy revision still
matches, and the global durable pressure-observation sequence floor. A changed policy
resets the disposition to `comfortable` but never resets the sequence floor.
The new lifecycle collector starts above that floor, and the restored pressure
controller rejects observations at or below it, preserving hysteresis without
mistaking post-restart sequence reuse for new evidence.

`createRetainedRoleShadowHost` is the supported assembly boundary. It derives
the durable sequence floor before subscribing to provider notifications, then
lazily restores one pressure controller and constructs one coordinator per
logical role. Concurrent first use shares the same coordinator construction.
Role policy, schedule, inference runtime, checkpoint publisher, and semantic
projection remain explicit injected dependencies. Closing the host detaches
only lifecycle observation; it does not close caller-owned storage or the App
Server adapter.

`projectManifestRoleObservedContext` is the source-assembly boundary used when
pressure schedules semantic inspection. The caller must classify every visible
material's origin, trust, and instruction applicability and must declare source
inventory completeness and omissions. The projector reads exact activated
skill bytes, derives all content digests itself, binds the manifest role,
runtime binding, completed turn, lifecycle snapshot, and expected next work,
then returns both the signed projection and its exact bounded source materials.
Caller-supplied digests and unknown material fields are rejected.

`CodexAppServerInferenceCapability` realizes the compiler or verifier inference
interface through one fresh ephemeral App Server thread per invocation. It
places the reviewed semantic instructions at developer precedence, serializes
only the bounded input and structural output contract into the user turn,
requests no dynamic tools, captures matching turn token usage when the provider
reports it, and returns host-labeled provenance. Separate capability instances
therefore produce separate thread and turn identities without creating durable
role bindings. A gated integration test composes two such invocations with a
real strategic-planner shadow turn; normal test runs do not claim that live
evidence.

The observed-context projector accepts only attributed content references; it
does not copy raw content or claim access to the provider's literal effective
prompt. Every projection retains mandatory unknowns for the effective model
input, provider instructions, and hidden reasoning state. Its source revision
binds the normalized role, runtime binding, visible-item inventory, governing
sources, activated skill digests, lifecycle snapshot, expected next work,
completeness state, and explicit omissions. A domain-separated Ed25519
signature authenticates the projector component and build revision. Successful
verification proves construction integrity for those declared inputs, not
projection completeness, semantic sufficiency, or retirement readiness.

Continuation validation establishes the shape and content revision of a
compiler candidate; it does not accept or publish that candidate, revalidate
its authority references, or establish retirement readiness. The lifecycle
ledger helpers distinguish observations, attempts, failures, unresolved facts,
and accepted decisions and reject event/status combinations that would turn an
attempt into a success claim. Their hash chain detects mutation of retained
records but does not provide durable storage, authenticate the writer, or make
the in-memory chain authoritative by itself.

The semantic inference harness is currently proven against recorded bounded
fixtures. It verifies the observed-context signature, admits exactly the
projected source references, checks every supplied content digest, and supplies
the same bounded material to compiler and verifier calls. The compiler can emit
only semantic continuation fields; the host attaches subject, timestamps,
inference provenance, and the candidate revision. The verifier must report all
five required checks and cited blockers or uncertainty, while the host derives
`accepted`, `rejected`, or `unresolved`. Distinct capability objects and
inference identifiers prove separate calls, not provider, model, or reasoning-
context independence. No live inference adapter, checkpoint publication,
retirement authorization, or actuation is implemented by this harness.

Human-interaction evaluation is part of the same bounded verifier pass rather
than a third role. The compiler must keep semantic status separate from loading
disposition. Host checks prevent ambiguous meaning from being compiled away,
open meaning from being omitted, active governing meaning from becoming
reference-only, and a claimed compiled consequence from lacking its durable
reference. The verifier must return one source-bound closure/loading evaluation
for every interaction, and its aggregate interaction check must agree with
those evaluations. `escalate` remains unresolved even when the underlying
classification is supported.

Checkpoint publication remains a code-owned consequence after semantic
verification. The publisher stores the complete continuation state and its
verification only after a destination authority owner returns evidence covering
the exact referenced authority set. The injected store operation must compare
the observed source revision, runtime binding, authority revision, predecessor
checkpoint, and predecessor ledger entry in one atomic write. The included
in-memory store proves that contract deterministically but is neither protected
nor durable. A rejected or failed attempt does not produce a
`checkpoint_published` ledger claim. This boundary does not establish
retirement readiness, acquire a transition lease, deliver an actuator turn, or
invoke `new_context`.

The transition-lease foundation consumes the complete published checkpoint,
its publication ledger entry, and the exact post-publication fence. Lease
acquisition appends accepted readiness evidence. Before the adapter delivers a
retirement control turn it verifies the active lease, requires the negotiated
target-model context-replacement capability, admits only the deterministic
host directive, and records an attempted actuation request. A competing domain
turn or binding change revokes readiness before proceeding; a dynamic-tool
effect is denied and revokes readiness. Once actuator delivery has begun,
competing domain turns, tool effects, and binding changes remain outside the
retiring revision. Failed delivery records failure rather than actuation
success.

The included gate serializes these boundaries only inside one process. It is
not protected durable lease storage, does not observe provider-native or
external input or tool-effect paths, and a compaction notification alone cannot
establish that `new_context` produced the required successor semantics. The
runtime subscribes before actuator delivery and waits on notifications rather
than polling. A pinned, same-thread compaction signal for the exact retirement
turn records only an unresolved transition observation and unlocks one
checkpoint-bound rehydration turn. The predecessor context-window ID is part of
the immutable lease subject; the successor receipt must report that exact
predecessor, a distinct current window, the exact lease/checkpoint/thread/nonce,
and supported continuation claims with no hidden uncertainty. Only then does
the gate record accepted reconciliation and release domain work.

The fake-transport tests prove event ordering, checkpoint injection, receipt
validation, and fail-closed admission. They do not prove live model compliance,
authenticate the model-reported context-window identifiers, or replace the
still-required live strategic-planner reconciliation probe.

## Protocol bindings

[`protocol-bindings.lock.json`](protocol-bindings.lock.json) pins the Codex CLI
version, generator mode, and the protocol roots consumed by this boundary. Run:

```bash
npm run generate:app-server-protocol
```

The generator fails if the installed CLI version differs from the lock. It
uses the CLI's own TypeScript generator and retains the complete dependency
closure of the selected protocol types under `generated/`.

The current foundation deliberately uses only stable APIs from Codex CLI
`0.149.1`. Experimental APIs must be added to the lock and capability profile
explicitly; their presence in a newer App Server does not silently enable them.

Model-context replacement is not represented as an App Server request in the
generated protocol lock. The adapter therefore gates the provider-neutral
`model_context_replacement` capability through a separate pinned provider
runtime profile. Selection requires an explicit host declaration that
`token_budget` is configured, fixes the mechanism to model-invoked
`new_context`, and requires observed transition evidence. This declaration is
configuration evidence only: it does not prove that the tool was exposed or
that a transition occurred. Provider capabilities cannot be widened after
adapter initialization, and the selected provider profile participates in the
retained thread's environment fingerprint.

## Runtime manifest

[`runtime-manifest.yaml`](runtime-manifest.yaml) owns App Server runtime
composition for the currently ported roles. A role entry declares only variant
runtime structure:

```yaml
schema_version: 1
manifest_id: work-engine.app-server
roles:
  example-role:
    contract: ../skills/example-skill/SKILL.md
    developer_instructions: Keep the role inside its contract.
    thread_options:
      cwd: ..
      approval_policy: never
      sandbox: read-only
    skills:
      - name: example-skill
        path: ../skills/example-skill/SKILL.md
```

The loader rejects unknown fields, resolves paths relative to the manifest,
requires the named role contract to be present among the exact injected skill
inputs, binds the source bytes with SHA-256, and projects any named role plus
an instance ID into the adapter's role and skill inputs.
`ManifestRoleRuntime` provides the generic delivery boundary used to assemble
arbitrary roles for tests and future ports.

Each projected role also carries a deterministic environment revision derived
from its own contract, instructions, thread settings, and skill composition.
The binding registry refuses to resume an existing logical role thread when
that thread-scoped environment changes; the caller must perform an explicit
binding replacement. Changes to unrelated roles do not invalidate the role.

The runtime manifest does not own role authority, workflow state, or the
meaning of a skill. Those remain with their canonical contracts and durable
owners. Likewise, generated files under `docs/agent-environment-views/` remain
analysis projections and are not runtime configuration inputs.

## Tests

```bash
npm run test:app-server
```

The default suite uses a fake transport and temporary binding stores. The
optional local integration suite requires the pinned `codex` executable. It
proves initialization, model visibility of a runtime-random request-context
value through a temporary manifest role, same-thread context-window replacement
with normalized lifecycle evidence, and an exact request-bound handoff through
the production strategic-planner role:

```bash
npm run test:app-server:integration
```

## Role verticals

The first role vertical is [`roles/strategic-planner.mjs`](roles/strategic-planner.mjs).
Its runtime composition comes from the checked-in manifest. The wrapper binds
each logical planning stream to a durable thread, supplies named evidence
cutoffs and canonical references, and validates the result against the exact
request. Its output is accepted only after the terminal turn succeeds and the
YAML handoff matches the requested objective, evidence cutoff, continuity
state, and version-1 shape. The handoff remains advisory planning state;
neither the wrapper nor its thread mutates roadmap or campaign authority.

## Planning documents

- [`docs/control-plane-causal-observability-ui.md`](docs/control-plane-causal-observability-ui.md)
  defines the read-only operational, causal, efficiency, anomaly,
  counterfactual, recommendation, and evidence-drill-down views for the App
  Server control plane.
- [`docs/semantic-context-lifecycle-manager.md`](docs/semantic-context-lifecycle-manager.md)
  plans a shared code-owned lifecycle service that compiles, verifies, retires,
  and rehydrates model context across Work Engine root roles.
- [`docs/codex-agents.md`](docs/codex-agents.md) records the shared App Server
  daemon and operator-console behavior relevant to the new runtime topology.
