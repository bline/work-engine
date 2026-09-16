# Revision / CAS / Publication

> **Question:** How does authoritative state advance safely — across every dimension that owns durable state of its own — using one shared discipline, without that discipline ever becoming a shared *owner* of what the state means?

## Purpose

This is a **mechanism view**, not a dimension view. It describes a reusable invariant and protocol shape that recurs across independently owned truth domains, not a class of architectural truth this page owns itself.

> **The mechanism preserves succession safety. It never decides what a revision means, what triggered it, or who was allowed to produce it — those are the owning dimension's own questions, every time.**

Independently owned dimensions keep reaching for the identical shape without copying it from one another:

```text
Semantic Planning        — branch-plan revisions (durable state, §16)
Organizational Compilation — ExecutionEnvelope revisions (accepted admission mechanism)
Evidence/Claims           — claim revisions (implemented, real code)
Context Lifecycle         — checkpoint / lifecycle-ledger revisions (implemented, real code)
Runtime Realization       — realization lineage (proposed)
Material Decision Selection — sealed decision-set revisions (proposed)
```

Six independent instances converging on one shape is the actual evidence for documenting it once, here — not a stylistic preference for consolidation. This count was last true to the page's own body as of 2026-09-16; if it drifts again, trust the "Confirmed Instances" section below over this summary.

---

## The Reusable Shape

```text
identity
    every revision's own id is a content digest of the record, minus its
    own id field — never random, never caller-chosen

predecessor
    every revision (except the first) names the exact prior revision it
    succeeds — an explicit chain, not an inferred one

head / generation
    the current head of a revision chain is whichever revision has no
    successor yet — computed, not separately stored as a mutable pointer

expected-state comparison (compare-and-swap)
    a publish request names the exact head it expects; if the actual
    current head has moved, the request is rejected as a conflict — never
    silently applied against a stale head

atomic visibility / publication
    a revision (and anything causally produced with it — a lineage edge,
    a derived consequence) becomes visible in one indivisible step; there
    is no window where one part exists durably and the other does not

stale-write rejection
    a competing publish against an outdated head fails closed, every
    time, with no reordering, merging, or last-write-wins resolution
    invented by the mechanism itself
```

Nothing above requires knowing what a revision *is* — a claim's proposition, a branch plan's obligations, a checkpoint's continuation state. The mechanism is blind to that content by design.

---

## What This Mechanism Does Not Decide

Stated explicitly, because reuse of a mechanism does not inherit the contract that governed its use in any one prior instance (the exact lesson `operation-contract-surface.md`'s own review history exists to preserve):

- **it does not decide what a revision means** — proposition identity, plan semantics, continuation meaning, all remain the owning dimension's own content;
- **it does not decide who may publish** — authority to publish belongs to `authority-and-ownership.md`'s own model, applied per dimension, never granted by this mechanism;
- **it does not decide when a revision should be superseded** — that is a domain judgment (a refresh judgment, a replan, a retirement decision), never a mechanical consequence of CAS succeeding;
- **it does not resolve branching or concurrent valid heads** — a domain profile that permits branching must define its own canonical-support-selection mechanism; this mechanism only guarantees that *whichever* head a publish targets is the one that was actually current, never that there is exactly one head at all times.

---

## Confirmed Instances

### Evidence/Claims — implemented

Verified directly against `app-server/src/services/claim-evidence/service.mjs`: `revisionHeads(store, claimId)` computes the current head set (revisions with no successor); `publish_revision` requires `revisionHeads(store, payload.claim_id).has(request.expected_state)` before minting a new revision via `makeRevision`, which sets `predecessor_revision` and derives the new revision's `id` as a content digest. A conflicting predecessor is rejected outright. Idempotency (`operation_id` + payload digest) is a *separate* concern from this CAS check — a retry with the same operation is a no-op; a competing publish against a stale head is a hard conflict.

### Context Lifecycle — implemented

`semantic-context-lifecycle-manager.md`'s own text: checkpoint publication happens "through one revision-fenced compare-and-swap boundary after exact authority revalidation," and restart reads "revalidate stored episode, checkpoint, and ledger revisions before returning them." Invariant 15 there — "final readiness comparison and actuator delivery share one revision-bound transition lease that competing input, effects, or binding changes revoke" — is this dimension's own consumption of the mechanism, not a separate invention (see `mechanisms/transition-fencing-and-leases.md`, pending, for the closely related fencing layer built on top of this same revision-binding discipline).

### Organizational Compilation — accepted design, not implemented

`organizational-compilation.md` §7: organizational authority admits a selection "as a new revision" of `ExecutionEnvelope`. Accepted 2026-09-15 as design; no compiler or durable schema exists yet.

### Semantic Planning — named, not implemented

`hierarchical-planning-and-multi-supervisor-orchestration.md` §16 names "branch-plan revisions" as required durable state directly, alongside "orchestration-plan revision." No implementation evidence has been checked for this dimension specifically — named as required, not yet verified as built.

### Runtime Realization — proposed, not implemented

`runtime-realization.md` §10: every execution record identifies its exact realization, and a successor after rematerialization names `predecessor_realization` and a `transition_reason`. Proposed shape, matching the same pattern; no implementation evidence found.

### Material Decision Selection — proposed, not implemented

`material-decision-selection.md` §6: "sealing makes that revision immutable; later changes create a successor and reopen every implementation contract that relied upon the superseded revision." The identical predecessor-chained, CAS-published shape, found independently by this dimension's own source document; no implementation evidence found.

---

## Key Invariants

1. **Every revision's identity is a content digest, never assigned or chosen.**
2. **Predecessor chains are explicit; heads are computed from the absence of a successor, never separately tracked as a mutable pointer.**
3. **Publication is compare-and-swap against an exact expected head — never against a vaguely current state.**
4. **A revision and anything causally produced with it become visible together, atomically, or not at all.**
5. **This mechanism never decides meaning, authority, supersession judgment, or branch resolution — every one of those remains the owning dimension's own content.**
6. **Reusing this mechanism for a new domain does not inherit whatever authority or scope governed its prior use — each new instance is its own contract.**

---

## What This View Does Not Show

This page does not define:

- what any specific revision means in any specific dimension (each dimension's own page);
- authority to publish (`authority-and-ownership.md`);
- the transition-fencing/lease layer built on top of this same revision-binding discipline (`mechanisms/transition-fencing-and-leases.md`, pending);
- branch representation or canonical-support selection for domain profiles that permit branching (explicitly out of scope for claim-evidence's own current vertical, per `operation-contract-surface.md`);
- concrete storage engines (SQLite, in-memory, or otherwise) — this page describes the logical discipline, not a storage implementation.

---

## Relationship to Transition Fencing and Leases

`context-lifecycle.md` §6 and §7 already note these are closely related but distinct: this mechanism defines how one authoritative revision succeeds another safely; the fencing/lease mechanism (pending) defines how *concurrently prepared* transitions across independently-owned dimensions (lifecycle replacement, organizational admission) stay coherent against a revision that changes underneath them while they're being prepared. Fencing is built on top of revision binding — defining this mechanism first, as the user's own stated sequence required, is why the fencing page can now assume rather than re-derive predecessor/head semantics.

## Relationship to Candidate Resolution and Admission

Independent mechanisms answering different questions — Candidate Resolution and Admission reduces a space of options to one lawful selection; this mechanism publishes whatever gets selected (or anything else authoritative) safely. Organizational Compilation and Runtime Realization both use the former to decide *what* to admit, then (once implemented) would use this mechanism to publish the admitted result as a durable revision.

---

## Related Architecture Views

- **`evidence-and-claims.md`**, **`context-lifecycle.md`** — the two implemented instances.
- **`organizational-compilation.md`**, **`semantic-planning-hierarchy.md`**, **`runtime-realization.md`**, **`material-decision-selection.md`** — accepted-design or proposed instances, not yet implemented.
- **`authority-and-ownership.md`** — owns who may publish; this mechanism only owns whether a given publish is safe against the current head.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: partial
  owner: app-server/docs/architecture/mechanisms/revision-cas-and-publication.md
  status_as_of: 2026-09-16
```

**Corrected 2026-09-16, same fix applied to `mechanisms/candidate-resolution-and-admission.md`:** an earlier draft named `status-grammar.md` as `owner`. That document defines what the status fields mean; it is never the semantic owner of any specific architectural claim, mechanism included. This page owns itself. `design: accepted` — the mechanism itself (as a recognized, named, cross-cutting pattern rather than one dimension's private implementation detail) was explicitly settled through direct discussion. `reconciliation: reconciled` — confirmed across four independently owned dimensions, two by direct code verification. `authorization: design_work_authorized` — documenting and naming the mechanism is what's authorized here; nothing about this page authorizes implementation in any dimension that doesn't already have its own. `implementation: partial` at the mechanism level: real in Evidence/Claims and Context Lifecycle, accepted-design-only in Organizational Compilation, merely named or proposed in Semantic Planning and Runtime Realization respectively — each dimension's own page remains the authority on its own instance's status; this page does not restate or override any of them.
