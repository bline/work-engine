# Repository and Runtime Observations

These observations support intake of the exact Semantic Context Manager source
revision. They nominate relationships and boundaries; they do not decide
proposal quality, implementation authority, or permanent placement.

## Existing semantic owners

- `proposals/agent-state/semantic-durability-context-lifetime/` contains a
  formed proposal for context as conditionally durable working memory,
  continuation safety, preservation economics, and context-lifetime judgment.
  It does not allocate those judgments to a host-invoked specialist role.
- `proposals/agent-state/context-decision-observability/` contains the approved
  candidate observation contract. It exposes context pressure, lifecycle,
  checkpoint, and rehydration evidence while explicitly withholding safety,
  benefit judgment, role-state ownership, and effect authority.
- `proposals/agent-state/role-owned-durable-operational-state/` owns publication
  and recovery of resume-critical role state. A context manager cannot defer or
  replace publication required by that stronger owner.
- `ideas/intake/semantic-durability-context-lifetime/record.json` already
  separates semantic judgment from empirical calibration and bounded memento
  questions. The current source appears to refine the judgment candidate by
  allocating runtime responsibilities rather than duplicating its doctrine.

## Runtime and environment seams

- `ideas/runtime-adapter.md` already assigns provider actor enumeration,
  creation or binding, lifecycle observation, child topology, and capability
  reporting to a provider-neutral adapter. It prevents provider subagents from
  automatically becoming Work Engine roles.
- `ideas/environment-adapter-and-host-provided-runtime-services.md` assigns host
  capability discovery and realization selection to environment machinery
  without transferring logical identity or semantic authority.
- The context-manager source can therefore depend on one provider-neutral
  runtime-observation port. Its hooks and app-server profiles are enabling
  realizations, not independently owned context-manager semantics.

## Existing mechanical precedent

- `skills/slice-supervisor/scripts/live_slice_state.py` publishes phase
  consequences against the current durable revision.
- `skills/slice-supervisor/scripts/manage_active_slice.py` fails when an
  expected durable revision no longer matches current state.
- `skills/slice-supervisor/scripts/resume_active_slice.py` verifies exact slice
  identity before recovery and separates historical observation from effectful
  resumption.

These mechanisms do not implement context snapshots, but they demonstrate
repository-local compare-and-swap and exact-identity precedents for the proposed
snapshot fence.

## Current Codex runtime surfaces

The official Codex Hooks documentation observed on 2026-08-24 describes
`SessionStart`, `SubagentStart`, `PreCompact`, `PostCompact`, `Stop`, and
`SubagentStop` lifecycle events. Hooks remain a possible compatibility or
bounded observation surface, but the newly adopted direction no longer uses
them as the first runtime realization.

The official Codex App Server documentation observed on 2026-08-24 describes
thread start, resume, fork, read, enumeration, loaded-thread listing, lifecycle
notifications, token-usage updates, `collabToolCall` child-thread identities,
and `contextCompaction` items. Exact-turn and ephemeral fork behavior exists,
but some fields and methods remain experimental.

The local Codex CLI unexpectedly exposed `codex-cli 0.150.0-alpha.5`. A bounded
experiment recorded in `app-server-runtime-evidence.md` observed one App Server
thread retain its identity while automatic token-budget exhaustion moved from
one context-window identity to another. The user then confirmed that manual
`thread/compact/start` under `token_budget` produced the same semantic shape: a
fresh context-window transition on the same durable thread, without legacy
summarizing-compaction behavior. The runtime emitted a `contextCompaction` item
for the automatic transition, so this supports App Server as the first scaffold
and falsifies classification from event or method names alone. It does not
establish behavior for other versions, a separate direct `new_context` host
method, or a stable source for complete context-window identity.

## App Server scaffold boundary

`ideas/codex-app-server-scaffold-and-role-port.md` records the neighboring raw
direction for a parallel runtime scaffold. It keeps provider mechanics behind
the existing runtime-adapter boundary, logical roles distinct from thread
bindings, skills explicit and reusable, and canonical domain state outside
thread history. It also records which current packages are capabilities that
can move nearly unchanged and which role-like skills require thread templates,
lifecycle policy, and integration testing.

`claim-evidence` is evidence for capability portability, not for the cost of a
full role port. The first `strategic-planner` vertical is the intended test of
the reusable role shape.

## Neighboring proposal conflict audit

- `work-engine.semantic-durability-context-lifetime` remains the stronger owner
  of continuation safety and economic context-lifetime judgment. Its compact
  Wind Walker projection aligns with the newly specialized role; the new
  intake must not absorb that doctrine.
- `work-engine.context-decision-observability` remains the owner of
  authenticated pressure, lifecycle, checkpoint, and rehydration observations
  without replacement authority. Its explicit reopening condition for changed
  provider behavior covers the newly observed thread/window distinction.
- `work-engine.role-owned-durable-operational-state` remains the owner of
  resume-critical role meaning and writer fencing. Wind Walker may request or
  validate preservation but cannot replace required role-owned publication.
- `work-engine.operational-state-transition-history` may supply reusable
  retained-history mechanics. It does not become the owner of the Wind Walker
  lifecycle ledger or App Server runtime bindings merely because those records
  have transitions.
- `ideas/runtime-adapter.md` remains compatible: it already treats App Server
  as a provider realization and threads as runtime rather than semantic
  identity. The neighboring scaffold idea selects an implementation direction
  without replacing that boundary.

No formed proposal currently requires a thread, transcript, generated summary,
or observer ledger to become canonical workflow state. The conflict is instead
with the revision-1 intake assumption that hooks should precede App Server and
with any future reading of `contextCompaction` as sufficient transition proof.
Those statements are superseded in revision 2. Formed proposals retain their
own authority and should be reopened only if implementation evidence later
violates their explicit boundaries.

## Intake consequence

One independently useful candidate survives: a durable Wind Walker Context
Manager role that performs bounded semantic-difference inspection for
non-human-facing roles while preserving domain ownership, target-role bounded
correction or veto, snapshot fencing, native fresh-context readiness,
transition classification, rehydration, and truthful runtime capability
limits.

The runtime adapter and parallel App Server scaffold remain enabling neighbors
rather than second Context Manager candidates. The first credible vertical is
one read-only strategic-planner thread paired with Wind Walker to exercise
observe, preserve, request manual fresh-context replacement, rehydrate, and
reconcile. Automatic runtime migration, calibrated thresholds, and broad
human-facing applicability remain outside the candidate boundary.
