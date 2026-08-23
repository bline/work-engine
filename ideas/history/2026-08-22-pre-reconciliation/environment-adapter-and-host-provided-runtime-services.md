# Environment Adapter and Host-Provided Runtime Services

## Status

Future architecture idea / proposal seed

## Core idea

Work Engine should distinguish between semantic ownership, which remains inside Work Engine, and environment-provided service realizations, which may vary depending on where Work Engine is running.

The runtime integration layer may therefore be broader than a provider-specific execution adapter such as Codex App Server.

A more general abstraction is an **Environment Adapter**.

The Environment Adapter represents the host environment in which Work Engine is running and exposes provider-neutral service contracts for things such as:

- agent/runtime execution;
- durable storage;
- secrets;
- notifications;
- scheduling hooks;
- filesystem integration;
- UI hosting;
- synchronization;
- environment identity and capabilities.

Codex App Server would then be one realization beneath the execution portion of the Environment Adapter rather than the definition of the environment itself.

## Motivation

Work Engine increasingly owns semantic state that must survive model context, process, provider, and host lifecycle changes.

Examples include:

- workflow-owned live role state;
- proposal state;
- research claims and evidence;
- decisions;
- receipts;
- scheduler obligations;
- execution envelopes;
- runtime bindings;
- control-plane state.

Those semantic objects should not depend directly on one hosting environment such as VS Code, a terminal/CLI process, a local daemon, a future desktop application, or a remote Work Engine service.

At the same time, different host environments may provide materially better implementations for infrastructure services.

For example, a VS Code host might provide extension-managed storage, sync where appropriate, SecretStorage, native notifications, workspace identity, extension lifecycle hooks, and integrated UI/webviews.

A CLI host may instead use SQLite, filesystem state, OS keyrings, desktop or terminal notifications, and process supervision.

Work Engine should be able to exploit these differences without moving semantic ownership into the host.

## Central distinction

```text
WORK ENGINE
owns semantic meaning

HOST ENVIRONMENT
provides service realizations
```

For example:

```text
Role state
    owns:
        unfinished semantic work
        role-specific recovery state
        current obligations

Durable storage provider
    owns:
        persistence mechanics
        atomicity
        revisions
        integrity
        synchronization behavior
```

The storage provider does not decide what the role state means.

Likewise:

```text
Runtime binding
    owns:
        logical actor → provider runtime relationship

Execution provider
    owns:
        threads
        turns
        processes
        runtime events
```

The execution provider does not decide workflow authority or acceptance.

## Environment Adapter

Conceptually:

```text
                    WORK ENGINE
                         │
                         │ abstract service contracts
                         ▼
                ENVIRONMENT ADAPTER
                         │
       ┌─────────────────┼──────────────────┐
       │                 │                  │
       ▼                 ▼                  ▼
Execution service   Durable storage     Secret service
       │                 │                  │
       ▼                 ▼                  ▼
Codex App Server    VS Code / SQLite    SecretStorage /
future provider     filesystem/service   OS keyring

       ┌─────────────────┼──────────────────┐
       │                 │                  │
       ▼                 ▼                  ▼
Notifications       UI host            Filesystem/workspace
```

The adapter exposes what the environment can do.

Work Engine decides what those capabilities mean and when they may be used.

## Environment service contracts

### Runtime execution

Conceptual capability:

```text
RuntimeExecution
```

May provide:

- bind/create runtime actor;
- resume runtime actor;
- deliver input;
- observe status;
- receive lifecycle events;
- interrupt;
- inspect runtime topology.

Possible realization: Codex App Server.

### Durable storage

Conceptual capability:

```text
DurableStore
```

May provide:

- get;
- compare-and-swap write;
- revision identity;
- atomic publication;
- integrity verification;
- optional subscriptions;
- optional synchronization.

Possible realizations:

```text
SQLite
filesystem
VS Code-backed storage
Git-backed storage
remote service
```

The Work Engine storage contract should describe required semantics. A host implementation may provide additional optional capabilities.

### Secret storage

Conceptual capability:

```text
SecretStore
```

May provide:

- read secret by stable logical key;
- write/update where authorized;
- delete;
- environment/provider isolation;
- secure-at-rest behavior.

Possible realizations include VS Code SecretStorage, OS keyrings, and external vaults.

### Notification service

Conceptual capability:

```text
Notifier
```

May provide:

- user notification;
- severity/category;
- activation link or context reference;
- acknowledgement where supported.

### Scheduling / wake integration

Conceptual capability:

```text
WakeBridge
```

This is distinct from the Work Engine scheduler.

The scheduler owns obligation identity, due time, recipient, acknowledgement, and semantic delivery state.

The host environment may provide timers, process wake-up, background service activation, and host lifecycle wake events.

The host bridge does not own scheduled-work semantics.

### UI hosting

Conceptual capability:

```text
UIHost
```

May provide role/environment pages, organization graphs, runtime inspection, proposal views, control surfaces, and forensics views.

### Workspace / filesystem integration

Conceptual capability:

```text
WorkspaceHost
```

May expose repository/workspace identity, paths, workspace lifecycle, project root, and environment metadata.

It should not silently become authority over repository mutation.

## VS Code as a host environment

A future VS Code realization could look like:

```text
VSCodeEnvironmentAdapter

execution
    → Codex App Server

durable_storage
    → selected VS Code / local durable provider

secrets
    → VS Code SecretStorage

notifications
    → VS Code notification API

ui
    → extension + webviews

workspace
    → VS Code workspace API

wake_bridge
    → extension / daemon integration
```

This allows Work Engine to use host-native affordances without coupling semantic state directly to VS Code.

## CLI as another host environment

```text
CLIEnvironmentAdapter

execution
    → Codex App Server / local process

durable_storage
    → SQLite or filesystem

secrets
    → OS keyring / configured vault

notifications
    → terminal or desktop

ui
    → terminal rendering

workspace
    → current repository

wake_bridge
    → scheduler daemon / OS service
```

The same Work Engine workflows should remain semantically valid across both environments.

## Required versus optional environment capabilities

Not every host implementation needs identical machinery.

The environment should advertise capabilities.

Example:

```yaml
environment:
  id: vscode-local

  services:
    durable_storage:
      provider: vscode_local
      capabilities:
        - atomic_write
        - compare_and_swap
        - revisions
        - integrity

    execution:
      provider: codex_app_server
      capabilities:
        - persistent_threads
        - lifecycle_events
        - direct_input
        - runtime_topology

    secrets:
      provider: vscode_secret_storage
      capabilities:
        - secure_read
        - secure_write

    notifications:
      provider: vscode
      capabilities:
        - user_notification
```

Work Engine can then distinguish a service required by semantic contract from a service implementation currently available.

## Environment selection is variant machinery

The host/environment implementation belongs in variant structure.

For example:

```text
DurableStore
    SQLite implementation

DurableStore
    VS Code implementation
```

Both may satisfy the same Work Engine semantic contract.

Changing the storage realization should not automatically change proposal semantics, role semantics, workflow state ownership, scheduler meaning, or decision meaning.

## Semantic ownership remains above the adapter

The Environment Adapter must not become a universal state owner.

Examples:

```text
Proposal workflow
    owns proposal meaning

Slice workflow
    owns slice state

Reviewer workflow
    owns live findings

Planner workflow
    owns planner state

Scheduler
    owns scheduled obligations

Runtime-binding layer
    owns logical actor/runtime binding

Environment Adapter
    provides infrastructure services used by all of them
```

The adapter should not merge these semantic domains merely because they share a storage backend.

## Durable storage abstraction

A useful conceptual storage contract might be:

```text
DurableStore

read(key)
publish(key, payload, expected_revision)
delete/retire where contract permits
inspect_revision(key)
```

The exact API should be driven by existing Work Engine needs.

The important invariants are more fundamental:

- durable publication is atomic;
- concurrent updates do not silently overwrite one another;
- revisions are attributable;
- integrity is detectable;
- semantic payload remains opaque to the storage provider;
- provider replacement does not redefine domain ownership.

Optional host features such as synchronization should be surfaced as capabilities rather than assumed universally.

## Sync is a host capability, not a universal semantic guarantee

VS Code or another host may offer synchronization.

That can be valuable for some Work Engine data.

However, not every semantic object is necessarily safe or appropriate to sync.

Examples:

```text
user preferences
    likely sync-friendly

role/environment configuration
    potentially sync-friendly

local repository role state
    probably repository/environment scoped

runtime binding
    host-local

credentials
    secret-store scoped

large forensic traces
    likely local or repository-scoped
```

Therefore:

> **Syncability should be declared per semantic data class rather than inherited merely because the selected storage provider supports sync.**

## Environment-local versus portable state

The architecture should distinguish portable Work Engine state from environment-local state.

Portable examples:

- proposal packets;
- role contracts;
- execution-envelope definitions;
- durable decisions;
- repository-owned receipts.

Environment-local examples:

- Codex runtime thread binding;
- UI panel layout;
- transient provider health;
- local daemon PID;
- host-specific cursor/state.

Some state may be portable in meaning but cached locally for performance.

The semantic owner should declare this rather than the host deciding implicitly.

## Runtime Adapter becomes one sub-adapter

The earlier runtime-adapter architecture remains valid, but its scope becomes clearer.

```text
EnvironmentAdapter
    ├── RuntimeExecutionAdapter
    ├── DurableStorageAdapter
    ├── SecretAdapter
    ├── NotificationAdapter
    ├── WakeBridge
    ├── WorkspaceAdapter
    └── UIHost
```

This avoids overloading the runtime execution layer with unrelated responsibilities.

## Interaction with execution envelopes

Execution-envelope compilation may depend on abstract environment capabilities.

Example organizational requirement:

```yaml
required_environment_services:
  - durable_storage
  - runtime_execution
  - independent_review
```

The envelope should normally express requirements before provider-specific realizations.

Linking may then select:

```text
runtime_execution
    → Codex App Server

durable_storage
    → VS Code local storage

notifications
    → VS Code notifications
```

If a required environment capability is unavailable, the organization may be semantically valid but currently unrealizable.

That becomes a linking/runtime readiness issue rather than an automatic contract failure.

## Interaction with research maturity

Execution qualification may establish environment requirements without prematurely selecting host implementations.

For example:

```yaml
organizational_requirements:
  persistence:
    required:
      - atomic revisioned durable state

  runtime:
    required:
      - resumable logical actor execution
      - runtime lifecycle observations

  secrets:
    required:
      - protected credential retrieval
```

Activation readiness then resolves those requirements against the current host environment.

This preserves:

```text
proposal research
    determines what is needed

environment linking
    determines how the current host realizes it
```

## Interaction with the control plane

The control plane can query the Environment Adapter for runtime activation, wake delivery, notifications, persistent state, and host health.

But the control plane still owns coordination semantics only.

Example:

```text
scheduled obligation due
        ↓
control-plane authority check
        ↓
resolve execution envelope
        ↓
EnvironmentAdapter.runtime_execution
        ↓
provider delivery
```

Storage may simultaneously persist scheduler acknowledgement through `EnvironmentAdapter.durable_storage`.

The fact that both services come from one host adapter does not merge their semantic ownership.

## Interaction with the Studio UI

The Studio could expose the current environment realization.

Example:

```text
HOST ENVIRONMENT
───────────────
VS Code

Runtime execution
Codex App Server
status: healthy

Durable storage
VS Code/local provider
CAS: supported
revisions: supported
sync: optional

Secrets
VS Code SecretStorage

Notifications
VS Code native
```

This view answers:

> What infrastructure is currently realizing Work Engine's abstract services?

It is separate from:

> What organization is active?

and:

> What semantic state does each workflow own?

## Environment realization provenance

Receipts or runtime diagnostics may record the environment realization used.

```yaml
environment:
  adapter: vscode-local
  revision: 3

services:
  runtime_execution:
    provider: codex_app_server
    version: ...

  durable_storage:
    provider: vscode_local
    capability_profile: cas-v1

  secret_storage:
    provider: vscode_secret_storage
```

This is operational provenance. It should not be mistaken for semantic ownership.

## Failure and fallback

Because environment services are abstract, Work Engine may support fallback or relinking.

Example:

```text
preferred durable store unavailable
        ↓
select compatible fallback
        ↓
verify required guarantees
        ↓
continue
```

or:

```text
no compatible implementation
        ↓
waiting_on_capability / activation blocked
```

Fallback should never silently weaken required guarantees.

Example:

```text
required:
    atomic CAS

available fallback:
    plain unversioned file write

result:
    incompatible
```

The environment adapter reports capability mismatch rather than pretending success.

## Candidate host capability negotiation

A future activation path may perform:

```text
semantic requirements
        ↓
environment capability query
        ↓
compatible realization selection
        ↓
activation
```

Conceptually:

```yaml
require:
  durable_storage:
    compare_and_swap: true
    integrity: true

host:
  vscode_local:
    compare_and_swap: true
    integrity: true
    sync: true

result:
  compatible
```

This is analogous to linking a logical capability to a runtime implementation.

## Important boundaries

- Environment Adapter does not own domain truth.
- Storage implementation does not interpret payload meaning.
- Execution provider does not grant Work Engine authority.
- Host sync does not imply universal syncability.
- Provider fallback must preserve required guarantees.
- Host-specific APIs stay behind adapter boundaries.

## Possible first implementation

Do not begin by building a universal Environment Adapter framework.

A small proof could wrap the services already required by current Work Engine code.

Candidate sequence:

```text
1. identify existing provider-dependent infrastructure calls
2. define the smallest provider-neutral service contracts
3. wrap current local implementations
4. preserve current behavior
5. add a VS Code realization only where it provides concrete value
```

A useful first target may be durable state:

```text
current durable-state contract
        ↓
provider-neutral DurableStore interface
        ↓
existing SQLite/filesystem implementation
```

Then later:

```text
VS Code-backed implementation
```

if VS Code provides a genuinely useful capability such as sync, lifecycle integration, or safer storage.

The abstraction should be justified by actual host differences rather than created only for symmetry.

## Compact model

```text
                  WORK ENGINE SEMANTICS
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   proposal state      role state         scheduler state
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │ abstract services
                           ▼
                  ENVIRONMENT ADAPTER
                           │
      ┌─────────────┬──────┼───────┬─────────────┐
      ▼             ▼      ▼       ▼             ▼
 execution       durable  secrets notifier   workspace/UI
 adapter         storage
      │             │
      ▼             ▼
 Codex App       VS Code /
 Server          SQLite /
                 other
```

## Central architectural statements

> **Work Engine owns semantic state and authority; the host environment supplies infrastructure realizations.**

> **The Environment Adapter represents the capabilities of the host, not the meaning of the work.**

> **Runtime execution is one environment service, not the entire environment abstraction.**

> **Durable storage remains a provider-neutral semantic dependency whose implementation may be selected from host-native machinery.**

> **A host may provide stronger features such as synchronization, but those features become optional capabilities rather than universal assumptions.**

> **Changing the host realization should not redefine proposal, workflow, role, scheduler, or decision semantics.**

> **Environment linking should preserve required guarantees and fail closed when no compatible realization exists.**

## Why this matters

This architecture allows Work Engine to take advantage of the environment it runs inside without binding its semantic architecture to that environment.

It makes possible:

- VS Code-native storage where useful;
- synchronized configuration where appropriate;
- host-native secret storage;
- native notifications;
- resilient background execution;
- alternate CLI/desktop/web hosts;
- provider replacement;
- explicit capability negotiation;
- cleaner runtime provenance;
- portable workflow semantics.

Most importantly, it preserves a clean ownership boundary:

```text
Work Engine says what must be true.

The environment says what machinery it can provide.

The adapter links the two without becoming the owner of either.
```
