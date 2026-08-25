# Codex App Server Scaffold and Role Port

## Status

Adopted raw architectural direction awaiting independent intake and proposal
formation. This document does not authorize implementation or change roadmap
priority.

## Objective

Build a parallel Codex App Server scaffold for Work Engine rather than porting
the current Codex harness in place. Preserve Work Engine's semantic
architecture while replacing harness-specific execution machinery.

The governing model is:

- role definitions describe behavior, authority, and responsibilities;
- durable logical role instances bind to App Server threads;
- a thread is a durable reasoning runtime and replaceable runtime binding, not
  the canonical owner of workflow state;
- each thread may contain successive model context windows;
- skills remain reusable capabilities explicitly supplied to role threads;
- provider mechanics remain behind runtime adapters; and
- proposal packets, authority records, checkpoints, claims, findings,
  schedules, contracts, receipts, and exact review subjects remain separately
  durable and authoritative.

Thread history, native notes, and bounded history lookup may support recovery.
They do not replace canonical domain state.

The precise shorthand is:

> Logical role instances bind to threads, and threads contain successive
> context windows.

## Why a parallel scaffold

App Server provides native machinery for persistent thread start, resume,
read, fork, archive, and history; retained goals and settings; turn steering
and follow-up queues; exact skill injection; runtime skill invalidation;
thread-scoped dynamic tools; token-usage events; explicit context controls; and
independent thread, turn, and context lifecycles.

These capabilities can replace or reduce current dependence on harness
subagent primitives, session-ID conventions, mailbox handoffs, implicit skill
placement, context-recovery instructions, and some MCP indirection.

A parallel scaffold avoids making the old orchestration topology the default
shape of the new runtime. Existing behavior remains available while one real
vertical constrains the new boundary.

## Provider boundary

Codex App Server is the first realization, not the semantic interface. The
runtime adapter owns connection, protocol translation, capability observation,
and execution effects. Work Engine owners retain role identity, authority,
workflow meaning, durable state, acceptance, and receipts.

The scaffold must capability-negotiate experimental APIs and pin generated
protocol bindings to supported Codex versions. It must not put unstable App
Server method names into durable role contracts.

Process-scoped skill roots and apparently global skill enablement are not a
safe concurrent isolation mechanism. Role threads should receive exact skill
input items and thread-scoped dynamic tools. The stable separation is:

```text
role contract       → behavior, authority, responsibilities
skill               → reusable semantic capability
dynamic tool        → runtime-selected implementation
thread environment  → model, workspace, permission, skill, and tool bindings
runtime adapter     → provider protocol and lifecycle effects
```

## Scaffold ownership

A top-level `app-server/` boundary should own:

- connection, initialization, reconnection, and event streaming;
- generated protocol bindings for supported Codex versions;
- capability negotiation and integration probes;
- a logical-role registry mapping role instances to current thread bindings;
- thread start, resume, fork, and fresh-entry policies;
- role developer instructions;
- exact skill resolution and per-turn injection;
- thread-scoped dynamic-tool dispatch;
- model, permission, workspace, and environment bindings;
- idempotent turn delivery using client message identities;
- token and context-window observation;
- context-lifecycle and replacement controls;
- queue, goal, setting, and thread-status observation;
- references to canonical Work Engine artifacts; and
- lifecycle ledgers and transition classification.

Thread IDs remain runtime-binding identities. They must not become stable
semantic identifiers for supervisors, planners, reviewers, proposal formers,
builders, or Wind Walker.

## Role migration

Current role-like skills become App Server thread templates or
provider-backed roles:

- `slice-supervisor`: one durable supervisor instance per campaign;
- `slice-builder`: one builder per slice, retained through planning,
  implementation, gating, and remediation;
- `strategic-planner`: a long-lived advisory planning instance;
- `proposal-former`: one instance per proposal-formation stream;
- `idea-intake`: one instance per bounded intake or raw-idea reconciliation;
- `agent-instruction-review`: a bounded specialist retained through
  remediation;
- `wind-walker`: the centralized context-lifecycle observer; and
- `claude-recon-implementation`: split into an independent-review role, a
  provider adapter, and a disposable reconnaissance capability.

A Codex reviewer that claims fresh-entry independence must start in a fresh
thread. `thread/fork` copies history and cannot establish that boundary. If
Claude remains necessary for cross-provider review, it stays behind the same
provider-neutral runtime contract.

## Capability migration

The following non-role capabilities should move without semantic change,
including their references, scripts, schemas, and tests:

- `chrome-vision`;
- `claim-evidence`;
- `code-change-profile`;
- `durable-state`;
- `proposal-packets`;
- `repo-search`;
- `slice-checkpoint`; and
- `slice-completion-commit`.

The following should remain available but be redesigned when their first real
App Server consumer requires them:

- `agent-environment-graph`;
- `codex-adversarial-review`;
- `comparative-repository-analysis`;
- `independent-review-state`;
- `review-bench`;
- `role-scheduler`; and
- `work-engine-mcp`.

`ui-design-principles` is Site2JSON-specific and should move to that product's
plugin or package rather than the generic scaffold.

The old general-purpose Wind Walker skill should not remain loaded into every
task role. Its invariant and applicable semantics belong in the Wind Walker
role and host lifecycle manager.

## Port-cost boundary

New semantic capabilities built now do not create substantial migration debt
when their contracts, deterministic cores, state, authority, and tests remain
separate from harness mechanics. `claim-evidence` is the representative case:
its semantic package can move nearly unchanged while only its runtime exposure
changes.

Actual roles additionally require a thread template, registry entry, explicit
skill and tool bindings, model/workspace/permission configuration, canonical
state references, lifecycle behavior, and App Server integration tests. The
first role vertical establishes that reusable shape. Later clean roles should
be modest ports; roles that embed current harness orchestration will be more
expensive.

Expected relative effort is:

- non-role capabilities such as `claim-evidence`: small;
- read-only roles such as `strategic-planner`: modest, with first-port scaffold
  cost;
- stateful formation and intake roles: moderate;
- retained builder and isolated reviewer roles: moderate;
- `slice-supervisor`: substantial because it concentrates current harness
  mechanics.

This distinction prevents capability portability from being mistaken for proof
that role migration is already trivial.

## Migration order

1. Build the thin adapter, generated bindings, capability negotiation,
   logical-role registry, skill resolver, and dynamic-tool bridge.
2. Copy the semantically unchanged capability packages.
3. Port `strategic-planner` as the first read-only, continuity-sensitive role.
4. Pair it with the first Wind Walker observer vertical and prove observe →
   preserve → replace → rehydrate → reconcile.
5. Port `proposal-former` and `idea-intake`.
6. Port specialist and independent review and resolve the provider strategy.
7. Port `slice-builder`.
8. Port `slice-supervisor` last.
9. Pull redesigned capabilities into the scaffold only when a real consumer
   constrains their surface.

The supervisor is deliberately last because porting it first would encourage
reconstruction of the current harness rather than discovery of the smallest
valid App Server topology.

## Relationship to neighboring owners

- `ideas/runtime-adapter.md` owns the provider-neutral execution boundary; this
  idea selects the first concrete scaffold and role-port strategy.
- `ideas/01-semantic-context-manager-proposal.md` owns the Wind Walker
  observation, preservation, replacement, and rehydration allocation; this
  scaffold exposes its runtime machinery.
- `work-engine.semantic-durability-context-lifetime` owns the semantic-safety
  and context-lifetime judgment doctrine.
- `work-engine.context-decision-observability` owns authenticated context
  observations without effect authority.
- `work-engine.role-owned-durable-operational-state` owns role-specific
  resume-critical meaning.
- `work-engine.operational-state-transition-history` is a possible mechanism
  for retained role-state transitions, not a universal App Server ledger
  owner.

The neighboring artifacts are compatible when these ownership boundaries are
preserved. Any text that treats a provider thread, transcript, generated
summary, or observer ledger as canonical workflow state conflicts with this
direction and should be reopened at its own owner.

## Unresolved questions

- Whether manual `thread/compact/start` retains its observed fresh-context
  semantics across every supported Codex version.
- Whether a separate direct `new_context` host method exists; manual
  `thread/compact/start` is the currently observed initiation surface.
- The exact intervention and write-fencing mechanism before automatic
  replacement.
- A stable provider observation for context-window identity.
- Capability differences across supported Codex versions.
- The final thread-local skill-isolation behavior.
- The Claude-versus-Codex independent-review strategy.
- Recovery when replacement occurs before a readiness receipt exists.

## Non-authorization

This raw idea does not authorize implementation, proposal acceptance, skill
deletion or relocation, changes to existing role contracts, or roadmap
priority. Those consequences remain with their owning workflows.
