# Provider-Neutral Runtime Adapter

## Status

Exploratory runtime integration idea.

## Idea

Introduce a provider-neutral boundary between Work Engine logical actors and provider execution runtimes, with Codex App Server as the first likely implementation.

## Current evidence

Work Engine already distinguishes:

- logical actor identity from provider/session identity;
- semantic workflow state from runtime continuity;
- scheduler recipient identity from provider thread identity;
- telemetry provenance from semantic ownership.

Codex App Server research indicates a rich runtime surface for persistent threads, turns, events, resume, steering, runtime topology, and local control transport.

No Work Engine runtime-adapter layer is implemented today.

## Runtime binding

A binding records where a logical actor is currently embodied:

```text
logical actor
    ↕
provider runtime identity
```

A logical actor may survive replacement of its provider runtime.

A provider runtime actor may exist without becoming a durable Work Engine actor.

## Adapter responsibilities

A runtime adapter may:

- enumerate/inspect provider actors;
- bind or create an execution actor;
- resume;
- deliver authorized input;
- observe status and lifecycle events;
- interrupt where supported;
- expose runtime child topology;
- report provider capabilities/availability.

It does not:

- grant Work Engine authority;
- advance workflow semantic state directly;
- accept work;
- own proposal/review/receipt truth;
- automatically promote runtime subagents into Work Engine roles.

## Runtime projection

Provider events become runtime observations. Workflow owners decide whether those observations have semantic consequences.

## Persistence boundary

Runtime-binding records may use shared durable storage, but execution adapters do not become the owner of general Work Engine semantic durability.

## Relationship to control plane

The control plane resolves authority and delivery intent.

The runtime adapter performs the provider-specific execution consequence after authorization.

## Compact statement

> Runtime adapters answer where and how computation is executing; Work Engine remains the owner of what that computation means.
