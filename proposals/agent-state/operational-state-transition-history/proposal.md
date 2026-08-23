# Proposal: Operational State Transition History

## Identity and current state

- Proposal ID: `work-engine.operational-state-transition-history`
- Family ID: `work-engine.agent-state`
- State: placement uncertain; not evaluated or accepted
- Decision owner: user or future explicitly authorized architecture owner
- Evidence cutoff: repository
  `23f2e44dcb43421330c979525234b31ddbf54b93`
- Human formation decision: operational history must be listable sequentially
  by time so a future UI can decide how to visualize what happened

The canonical lifecycle and placement metadata is in [`packet.json`](packet.json).
This narrative owns the candidate's current meaning. It records a formed
direction, not implementation authority.

## Independently decidable consequence

Add a reusable transition-history capability beneath role-owned operational
state profiles. Each workflow continues to own the meaning and authorization of
its transitions. The shared capability preserves immutable revision lineage,
supports reconstruction at a retained revision, and serves bounded,
deterministically ordered history queries. MCP and UI consumers may project
that sequence without changing the history contract.

This candidate does not make operational state, claim history, decisions,
receipts, or runtime observations one semantic domain. It supplies compatible
lineage and query mechanics while their existing owners retain authority.

## Problem

The current durable-state primitive atomically publishes an opaque value under
a stable key using an expected revision. Its private ref names only the current
value. Older Git objects may remain physically present, but they have no
application-owned reachability, ordering, retention, or supported query
contract.

The current slice profile retains handled consequence identities and the latest
full phase consequence. It cannot reconstruct every prior full state or provide
a trustworthy transition timeline. That is sufficient for the first recovery
vertical, but insufficient for:

- sequential retrieval of how a workflow moved between historical states;
- selection and reconstruction of the semantic projection at a retained
  revision;
- replacement-role diagnosis of corrections, supersession, and uncertainty;
- Claude or other MCP consumers retrieving prior role-owned state; and
- comparison of claim changes, implementation events, and workflow responses
  without treating nearby events as semantic causes.

## Ownership and boundaries

```text
role workflow
  owns transition meaning, validity, authority, and current semantic projection

shared transition-history capability
  owns atomic lineage mechanics, revision reachability, integrity, retention
  enforcement, reconstruction, and bounded query behavior

claim/evidence subsystem
  owns epistemic claim identities, revisions, refresh judgments, and causality

MCP and UI projections
  read authorized views; they do not become semantic or transition owners
```

A history record must never gain authority merely because it is immutable or
durable. A persisted prior judgment is historical evidence of that judgment,
not new evidence that the judgment remains correct.

## Required transition identity

Every retained operational transition must bind at least:

- stable role and workflow/attempt identity;
- transition identity and resulting state revision;
- predecessor revision or another explicit, validated ordering relation;
- transition kind and protected consequence identity;
- attributed transition owner and writer generation;
- authority and governing-boundary references where applicable;
- evidence and stronger-artifact references rather than copied domain facts;
- publication time and integrity identity;
- uncertainty, correction, supersession, or retirement consequence when
  applicable; and
- schema/profile version needed to reconstruct its semantic projection.

Physical adapters may add fields, but they must not silently invent missing
semantic ownership or authority.

## Current projection and history consistency

The current projection and history must be published or reconciled under a
protocol that cannot silently expose mutually inconsistent truths. Acceptable
designs may include:

- an atomic append plus current-pointer update;
- a predecessor-linked immutable record whose accepted head is the current
  revision;
- an append-only journal with transactional current projection; or
- snapshot plus ordered tail with integrity-bound roots.

The first implementation must compare viable realizations against exercised
failure and query requirements. This proposal does not permanently select a
database, graph store, event-sourcing architecture, or Git representation.

Crash recovery must distinguish:

- no transition was durably published;
- a transition is durable but its acknowledgement was lost;
- history advanced but a cached projection did not;
- a projection advanced without a reachable accepted transition;
- an old writer attempted to append after replacement; and
- history or projection integrity cannot be established.

Uncertainty is a first-class result. Recovery must not repair an ambiguous
history by guessing from context or runtime liveness.

## Sequential history contract

The history capability returns retained transitions in deterministic temporal
order. Each transition exposes enough identity and references for a consumer to
understand:

- its position in the transition sequence;
- before/after projections or semantic deltas;
- actor, authority, evidence, claim, receipt, and checkpoint references;
- pending obligations and their later dispositions;
- corrections, supersession, route revision, and retirement; and
- limitations or unavailable referenced evidence.

A future UI may arrange this sequence as selectable visual time slices. That
presentation is owned by the UI, not by the history capability. Listing or
reading a historical revision must not:

- invoke a model or tool;
- redeliver an event or scheduled obligation;
- repeat a repository or external mutation;
- reactivate a role or provider session;
- alter current state; or
- convert historical authority into current authority.

Any future effectful simulation or re-execution requires a separate proposal,
authority model, and isolation contract.

## Query surface

The capability should support a storage-independent semantic surface equivalent
to:

```text
read_current(role_workflow_key)
read_revision(role_workflow_key, revision)
list_transitions(role_workflow_key, cursor, limit, filters)
reconstruct(role_workflow_key, revision)
compare(role_workflow_key, earlier_revision, later_revision)
```

Every result reports its owner, exact revision, integrity status, freshness or
availability limitations, and authorization scope. Pagination must be stable.
Queries must not depend on a Git reflog or enumeration of unreachable objects.

The Claude MCP access plane and UI may expose projections of this surface. They
must enforce role and consumer visibility and preserve exact revision identity.
The history contract does not prescribe their visual or interaction model.

## Retention, snapshots, and deletion

Each role profile declares which transitions remain reachable and for how long.
The shared mechanism enforces or reports that contract; it does not invent one
universal retention period.

Snapshots may bound reconstruction cost. A snapshot records its covered history
root, profile/schema version, and integrity identity. Deleting compacted detail
requires explicit profile authority and must preserve every lineage consequence
the retention contract still requires. Git garbage-collection accidents and
reflog expiry are not valid retention policies.

Retirement and deletion remain separate. A retired workflow keeps the
tombstone, suppression, and audit consequences required by its profile even if
other state is later eligible for physical deletion.

## Relationship to claim history

Claim lineage provides the closest exercised evidence: stable claim identity,
independent revisions, unchanged and changed refresh outcomes, typed lineage,
and exact-revision reliance. Operational history should reuse compatible
identity and edge concepts where they reduce consumer cost.

It must not collapse the semantic distinction:

- claim history answers what was believed, supported, refreshed, or changed;
- operational history answers what a role considered current, pending,
  authorized, uncertain, superseded, or complete; and
- a typed reference records when an operational state relied on an exact claim
  revision.

Claim-refresh causality remains owned by the evidence subsystem. An operational
transition may nominate impact or record reliance; it cannot adjudicate a claim
change merely by being later in the timeline.

## First dogfood slice

Exercise the capability on the slice-supervisor active-state profile because it
already has stable identity, CAS publication, phase consequences, recovery
tests, and a real UI/MCP consumer direction.

The bounded slice should prove:

1. implementation and gate publications create ordered immutable transitions;
2. current-state recovery remains behaviorally correct;
3. an earlier state can be reconstructed after later publication;
4. a fresh consumer can list transitions sequentially and reconstruct selected
   states without causing an effect;
5. stale-writer, replay, lost-acknowledgement, and projection/history mismatch
   cases fail closed or reconcile truthfully; and
6. snapshot or checkpoint use bounds reconstruction without erasing required
   lineage.

The slice should instrument transition count, stored bytes, reconstruction
work, query volume, replay-projection size, and any avoided reviewer or builder
re-synthesis. Cost evidence cannot establish correctness.

## Evidence required before an authority decision

- Compare at least two history/publication realizations against the slice's
  atomicity, recovery, retention, and query requirements.
- Specify the exact transition and snapshot schemas for the exercised profile.
- Demonstrate current/history consistency across injected interruption points.
- Reconstruct and compare retained revisions through a fresh process.
- Produce a bounded, deterministic sequential-history projection that a future
  UI or MCP adapter can consume.
- Demonstrate that claims, receipts, checkpoints, schedules, and runtime
  observations remain references to their stronger owners.
- Obtain specialized architecture, lifecycle/evidence, authority/doctrine, and
  security/visibility review; UI review applies to the eventual visual surface,
  not the storage-only slice.

## Out of scope

- model transcripts, hidden reasoning, raw tool streams, or universal memory;
- effectful re-execution, simulation, rollback, or time travel;
- automatic activation, authority, acceptance, scheduling, or interruption;
- one universal schema for all role semantics;
- moving claim-refresh authority into operational state;
- permanent control-plane, database, graph-store, or Git placement;
- UI implementation or visual-interaction contract in the first history slice;
  and
- generalized distributed multi-writer history beyond the exercised fencing
  requirements.

## Acceptance consequence

An authority decision on this proposal decides only whether Work Engine should
add reusable operational transition history beneath role-owned state and
require deterministic sequential queries plus read-only reconstruction of a
selected retained revision.

It does not authorize implementation, choose whether the contract is hosted by
individual workflows, a shared durability layer, or a future control plane,
change claim ownership, authorize effectful replay, or prioritize the Claude
MCP or UI roadmap.
