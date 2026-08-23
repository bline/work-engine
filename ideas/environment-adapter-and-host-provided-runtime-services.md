# Environment Adapter and Host-Provided Runtime Services

## Status

Exploratory host-integration architecture.

## Idea

Separate Work Engine's semantic ownership from the infrastructure services
provided by its current host. A provider-neutral **environment adapter** exposes
host capabilities without allowing a terminal, editor, desktop application, or
remote service to become the owner of workflow meaning.

```text
Work Engine semantic owners
        ↓ service requirements
Environment Adapter
        ↓ provider-neutral contracts
host service realizations
```

## Current evidence

Work Engine currently uses several local realizations directly:

- opaque compare-and-swap durable state;
- SQLite-backed scheduled obligations;
- filesystem and Git-backed artifacts;
- provider/runtime session bindings;
- terminal or host notifications;
- local processes and sockets.

These demonstrate useful service classes, but no general environment adapter or
capability-negotiation contract exists today.

## Central ownership boundary

Work Engine owns:

- role and workflow meaning;
- proposal, review, decision, receipt, and checkpoint semantics;
- authority and acceptance boundaries;
- logical actor identity;
- portability and retention requirements.

The environment owns the mechanics it truthfully provides:

- persistence and synchronization behavior;
- process/runtime execution;
- secret custody;
- notification delivery;
- wake-up and scheduling hooks;
- filesystem/workspace integration;
- UI hosting;
- environment identity and capability discovery.

A storage provider preserves bytes and revisions; it does not decide what role
state means. An execution provider runs actors; it does not decide workflow
authority.

## Candidate service contracts

An environment may expose some combination of:

### Runtime execution

- create, bind, resume, observe, interrupt, and retire runtime actors;
- deliver input and lifecycle events;
- report provider/runtime identity and limitations.

Provider-specific runtime adapters, including a possible Codex App Server
adapter, are realizations beneath this service.

### Durable storage

- read and integrity-check a value;
- compare-and-swap publication;
- stable revision identity;
- atomicity and crash guarantees;
- optional history, subscriptions, or synchronization.

The abstract contract should not require SQLite, Git, filesystem storage, editor
storage, or a remote service specifically.

### Secrets

- environment-appropriate secret lookup and custody;
- non-exportability or redaction constraints;
- explicit absence and access failures.

### Notifications and wake-up

- human notification delivery;
- role or client wake-up hooks;
- scheduling integration;
- acknowledgement or delivery evidence where supported.

Scheduling state remains owned by the scheduler or governing workflow. A host
wake-up mechanism does not create an obligation, activate a role, or grant
authority.

### UI hosting

- render or host a client surface;
- bind authenticated user interaction to the control protocol;
- expose host-native affordances without making the UI authoritative.

### Workspace and filesystem integration

- workspace identity and lifecycle;
- bounded filesystem access;
- repository discovery;
- host-specific change and lifecycle notifications.

## Required, optional, and selected capabilities

An execution envelope should be able to distinguish:

- capabilities a host advertises as **available**;
- capabilities the governing role/problem contract **requires**;
- effects that contract and human authority **authorize**;
- realizations the runtime linker **selects**; and
- capabilities that are absent, degraded, or emulated.

The adapter must fail clearly when a required consequence is unreachable. An
optional host feature may improve a route without becoming a universal product
requirement.

## Environment selection and provenance

Environment selection is variant machinery. It must not silently change semantic
identity or authority.

Material bindings should preserve:

- environment and adapter identity;
- service implementation and revision;
- advertised guarantees and limitations;
- which realization was selected;
- actual fallback or migration events;
- portability and synchronization scope.

Moving between hosts must not imply that a workflow restarted, advanced, or
became accepted.

## Portability and synchronization

Not all durable state is portable, and not all portable state is safe to
synchronize.

Each semantic owner should define the consequence it needs: durability,
availability, confidentiality, locality, portability, or cross-device
coordination. The selected environment realization then states which guarantees
it can establish.

Synchronization is therefore an optional host capability, not a universal
semantic guarantee. Conflicts or unavailable replicas must be represented
truthfully rather than resolved by pretending one host is globally authoritative.

## Relationships

- **Runtime adapter:** realizes provider-specific actor execution beneath the
  environment's runtime service.
- **Control plane:** routes authorized intent and discovers environment/client
  capabilities; it does not absorb host mechanics or domain truth.
- **Execution envelope:** declares capability requirements before selecting host
  implementations.
- **Role-owned durable state:** owns recovery semantics and consumes an
  environment storage realization.
- **Studio:** is one possible hosted client over the control protocol.

## Adoption boundary

A first vertical should adapt one existing local service behind a small explicit
contract while preserving its current semantic owner and observable behavior.
Durable storage or notification delivery are plausible candidates.

It should not initially:

- replace every local mechanism;
- require cross-host synchronization;
- make a particular editor or provider canonical;
- create one universal service interface;
- hide materially different guarantees behind identical names; or
- infer authority from host capability availability.

## Does not own

This idea does not own:

- domain schemas or lifecycle meaning;
- role and workflow state;
- control-plane activation policy;
- provider-specific runtime protocol details;
- UI product design;
- secret values;
- universal deployment topology.

## Compact statement

> The environment adapter tells Work Engine what its host can faithfully do;
> Work Engine's semantic owners still decide what those services mean and when
> their use is authorized.
