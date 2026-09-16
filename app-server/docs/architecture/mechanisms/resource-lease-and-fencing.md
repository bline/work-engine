# Resource Lease and Fencing

> **Question:** For a named resource identity, how does Work Engine ensure that authority to mutate or use a protected resource is bound to a current lease generation — so a superseded, expired, or otherwise stale holder cannot successfully exercise that authority, even if it still believes it holds the resource?

## Purpose

This is a **mechanism view**, not a dimension view. It describes a reusable mutual-exclusion-with-fenced-handoff protocol already implemented once, generically, across seven unrelated resource kinds — not a class of architectural truth this page owns itself.

> **The mechanism ensures a stale or superseded holder cannot successfully exercise protected authority. It never decides what a resource means, what authority a lease represents, or who may hold one — those are the owning domain's own questions, every time.**

**Found by architectural synthesis, not invented here** (2026-09-16): the fenced-active-binding gap named in `control-plane-and-client-protocol-reconciliation.md` turned out to be a symptom of a larger, pre-existing omission — `workspace-coordination`'s own real, already-generalized resource-lease-and-fencing discipline had simply never been given its own architecture view, despite being independently named elsewhere this session as "the strongest kernel-shaped primitive found anywhere in the inventory." **This page's definition comes from that discipline's seven existing, implemented instances — fenced active-binding is a new consumer of the mechanism, not the mechanism's own definition.**

---

## The Reusable Shape

Verified directly against `app-server/src/services/workspace-coordination/{contract.mjs,service.mjs}`:

```text
resource identity: {type, id}  -->  key = "{type}:{id}"
        ↓
acquire(resource, holder, intentId, ttlMs)
        ↓
current lease absent or expired?
    yes → new lease granted; fencingToken = current generation + 1
    no  → blocked (current lease still valid, current holder returned)
        ↓
holder later calls admitMutation(lease, operationId, mutate)
        ↓
does the presented lease match the CURRENT generation, leaseId, and holder,
and has it not expired?
    yes → mutation admitted, exactly once per operationId
    no  → rejected: "resource lease is absent or superseded" (fencingToken
          or leaseId or holder no longer matches current state), or
          "resource lease is expired"
```

The generic invariant this mechanism exists to enforce, independent of any specific resource kind:

> **For a named resource identity, authority to mutate or use the protected resource is bound to a current lease generation / fencing token. A superseded, expired, or otherwise stale holder cannot successfully exercise that protected authority, even if it still believes it holds the resource.**

Re-acquiring after expiry or release increments the fencing token monotonically — there is no way for an old holder's stale token to be mistaken for the current one, even if that old holder never learns it was superseded.

---

## What This Mechanism Does Not Decide

- **it does not decide what a resource means, or what authority a lease represents** — `directory`, `git-ref`, `port`, or a future `role-active-binding-slot` all pass through the identical shape; the owning domain supplies the meaning entirely;
- **it does not decide who may hold a lease, or under what policy** — `holder` and `intentId` are opaque strings to this mechanism; authorization to request one is the owning domain's own question (`authority-and-ownership.md`);
- **it does not decide which resource types exist** — each instance below is domain-declared, added to `RESOURCE_TYPES` when a real, evidenced need exists, never invented speculatively by this mechanism;
- **it does not mint or expand authority on acquisition** — holding a valid lease means the holder may currently exercise whatever authority the owning domain already granted over that resource, never that the lease itself is the source of that authority (see Relationship to Authority and Ownership, below);
- **it does not decide ttl policy, renewal cadence, or crash-reconciliation business logic** for any specific resource kind — those remain the calling domain's own operational concern.

---

## Confirmed Instances

### `workspace-coordination` core — implemented (7 resource types)

Verified directly against `contract.mjs:4-6`: `RESOURCE_TYPES = ["directory", "git-ref", "git-index", "port", "index", "review-budget", "database"]`. Every one of these is real, shipped, cataloged as part of the already-reconciled implementation baseline (`service-plane-inventory.md`). None of the seven has its own architecture-view page describing this shared mechanic — this page is the first.

### Fenced active-binding for logical role instances — accepted for implementation, not yet built (new instance)

The specialization, not the definition:

```text
resource:
    logical-role-instance:<id>:active-binding

holder:
    a specific runtime realization generation

invariant:
    at most one current realization generation may exercise
    authoritative execution as that logical role instance

supersession:
    successor realization acquires a newer fencing generation
        → predecessor realization may still physically exist
        → predecessor is no longer admitted to act as the role
```

`control-plane-and-client-protocol-reconciliation.md`'s own diagram states this exactly: "realization A ... still may physically exist but no longer possesses active-role authority" once a successor holds fence generation N+1. That document's own text already names the fork in the road this page resolves: "a future owner should decide whether it is a new `workspace-coordination` resource type (a role-active-binding slot) or a distinct mechanism." Resolved here: it is the former — a new resource type on this existing mechanism, not a new mechanism, and not a third fence class on `transition-fencing-and-leases.md` (§ Relationship to Transition Fencing and Leases, below). Accepted 2026-09-14; the resource type itself does not exist in `workspace-coordination`'s real `RESOURCE_TYPES` enum yet.

### Review-scope protection — accepted for implementation, not yet built (second new consumer)

`review.md` §7 residue item 5 and §9 (review-scope validity) hand enforcement of a declared protected scope to this mechanism. Notably, `workspace-coordination`'s real `RESOURCE_TYPES` enum already includes `review-budget` — the mechanism already reaches into review-adjacent territory — but nothing declares *which* resource key a specific active review needs protected before an incoming mutation is checked against it. `review.md` owns that declaration; this mechanism owns enforcing it once declared, exactly the same division of labor as the active-binding instance above.

---

## Key Invariants

1. **The mechanism protects exercise of already-granted authority; it never mints or determines what that authority is.**
2. **A superseded, expired, or otherwise stale holder cannot successfully exercise protected authority, even if it still believes it holds the resource.**
3. **Same fencing-token machinery does not mean same architectural invariant — reusing this mechanism's primitives never implies identity with `mechanisms/transition-fencing-and-leases.md` or any other mechanism.**
4. **Admission is single-use per operation id; replay of the same operation id is rejected outright, never silently treated as a retry.**
5. **Invalidation and supersession never mint authority — this mechanism's own concrete instance of `authority-and-ownership.md` §12's generalized invariant.**
6. **Each resource type is domain-declared; the mechanism never invents what resources exist or what they protect.**

---

## What This View Does Not Show

This page does not define:

- what authority a lease protects, or who grants it (`authority-and-ownership.md`);
- transition preparation-to-activation staleness — a structurally different invariant despite superficially similar vocabulary (`mechanisms/transition-fencing-and-leases.md`, see the explicit contrast below);
- concrete resource-type-specific business logic (what a `port` or `database` resource represents operationally, or what a review's own protected scope should actually cover);
- runtime realization's own generation/identity model (`runtime-realization.md`) — this mechanism only fences exercise of an already-existing realization's authority, it does not define what a realization or its generation identity is;
- the judgment that a scope needs protecting in the first place (`review.md`) — this mechanism only enforces a declared scope, never declares it.

---

## Relationship to Authority and Ownership

Stated as sharply as the boundary requires, because it is the single most important invariant this page exists to hold:

```text
a valid lease means:
    "this holder is currently admitted to exercise whatever authority
     the owning domain already granted over this resource"

a valid lease does NOT mean:
    "this holder owns the semantic right to act because it acquired
     a lease"
```

```text
authority-and-ownership.md
    determines what authority exists and may be delegated

THIS MECHANISM
    ensures stale or superseded holders cannot continue exercising it
```

This is a further concrete instance of `authority-and-ownership.md` §12's own generalized invariant — invalidation and supersession never mint authority — applied at the resource-exclusivity level rather than the candidate-admission level `mechanisms/candidate-resolution-and-admission.md` already instantiates it at.

## Relationship to Transition Fencing and Leases

Sibling mechanisms, sharing fencing-token vocabulary, protecting genuinely different invariants — made explicit rather than assumed compatible:

```text
RESOURCE LEASE AND FENCING (this page)          TRANSITION FENCING AND LEASES

question:                                       question:
    who is the current authoritative                is this prepared transition
    holder of resource R?                            still valid against the world
                                                       revision from which it was
                                                       prepared?

lifetime:                                       lifetime:
    standing relationship,                          bounded transition episode
    may renew over time

staleness:                                      staleness:
    an older holder/token loses                    the preparation's bound world
    exercise authority                              revision changed while
                                                     preparing

protects:                                       protects:
    resource ownership /                            coherent preparation ->
    mutation admission                              activation
```

Confirmed by direct invariant comparison, not assumed from shared vocabulary: `mechanisms/transition-fencing-and-leases.md`'s own governing invariant is "no transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared" — a one-shot preparation interval (bind → prepare → revalidate → publish → activate → release). This mechanism's own lease is a standing holder relationship with TTL renewal, no preparation phase, and no single activation event. Structurally different shapes. Fenced active-binding belongs here, not there, for exactly this reason.

## Relationship to Runtime Realization

`runtime-realization.md` owns the `RoleRealization` artifact and its generation identity; this mechanism owns protecting exercise of the active-binding slot that identity occupies:

```text
RoleRealization generation N exists
        ↓
acquire lease: resource = logical-role-instance:<id>:active-binding
        ↓
fencing generation F
        ↓
generation N may execute as the role
        ↓
generation N+1 supersedes N
        ↓
N+1 acquires fencing generation F+1
        ↓
N may remain alive physically, but F is stale
        ↓
N can no longer exercise role authority
```

This is stronger than "the old realization is killed" — it makes authority revocation host-enforced and race-safe, and composes cleanly with context-replacement work elsewhere in the architecture: a successor's activation never requires pretending its predecessor ceased to exist, only that its predecessor's authority became unenforceable.

## Relationship to Review

`review.md` decides what scope requires protection during an active review episode; this mechanism enforces possession/exclusivity for the declared resource once that decision is made; `workspace-coordination` is the current, real implementation of the mechanism itself. Review never infers a resource key from the mechanism's own shape, and the mechanism never decides review validity — each keeps strictly to its own side of the boundary.

---

## Related Architecture Views

- **`authority-and-ownership.md`** — determines what authority exists; this mechanism only protects exercise of authority already granted, never mints it.
- **`runtime-realization.md`** — the dimension whose realization-generation identity this mechanism's active-binding instance protects.
- **`review.md`** — the dimension whose protected-scope declaration (§7 residue item 5, §9) this mechanism enforces once declared.
- **`mechanisms/transition-fencing-and-leases.md`** — the sibling mechanism, sharing fencing-token vocabulary, protecting a structurally different invariant (standing exclusivity vs. preparation-to-activation staleness).
- **`mechanisms/candidate-resolution-and-admission.md`** — a different mechanism entirely; not composed with this one directly, but both are concrete instances of `authority-and-ownership.md` §12's same generalized invalidation-never-mints-authority invariant.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: unrecorded
  implementation: implemented
  owner: app-server/docs/architecture/mechanisms/resource-lease-and-fencing.md
  status_as_of: 2026-09-16
```

`design: accepted` — the generic mechanism is real, shipped, cataloged code, independently named elsewhere this session as "the cleanest kernel-shaped primitive" in `service-plane-inventory.md`'s own 20-unit implemented baseline; no document treats its existing seven resource types as contested or merely proposed. `reconciliation: reconciled` — checked directly against `app-server/src/services/workspace-coordination/contract.mjs` and `service.mjs` this session, not restated from a prior summary. `authorization: unrecorded`, **not inferred as `implementation_authorized` merely because the code exists** — this is exactly the mistake `context-lifecycle.md`'s own earlier retrofit corrected in this same session (`status-grammar.md` §8's rule: evidence supporting acceptance is not acceptance). This mechanism predates the reconciliation-queue process entirely as pre-existing kernel infrastructure; no citable "build this" authorization decision was found for the generic mechanism itself, so the honest value is silence, not a confirmed ceiling and not a confirmed authorization. `implementation: implemented` — confirmed directly against real, running code. This page owns itself.

```yaml
status_override:
  design: accepted
  reconciliation: reconciled
  authorization: implementation_authorized
  implementation: none
  source: app-server/docs/control-plane-and-client-protocol-reconciliation.md
```

Applies specifically to the fenced active-binding instance (Confirmed Instances, 2nd entry) — genuinely different maturity from the generic mechanism it specializes, not flattened to match it. That reconciliation's own Acceptance section: "Accepted 2026-09-14 ... Implementation of the stated residue — fenced active-binding coordination for logical role instances — is authorized to proceed." `implementation: none` because the `role-active-binding-slot` resource type does not exist in `workspace-coordination`'s real `RESOURCE_TYPES` enum yet — an explicit authorization to build, distinct from the generic mechanism's own unrecorded authorization.

```yaml
status_override:
  design: accepted
  reconciliation: reconciled
  authorization: implementation_authorized
  implementation: none
  source: app-server/docs/review-scope-coordination-reconciliation.md
```

Applies specifically to the review-scope protection instance (Confirmed Instances, 3rd entry) — same authorization tier as the active-binding instance, same citation already used in `review.md`'s own override for this residue item: "implementation of the stated residue — prospective review-scope coordination before mutation admission, including the design decision of which owner declares a scope protected before `workspace-coordination` enforces it — is authorized to proceed," accepted 2026-09-14. `implementation: none` because no scope-declaration mechanism has been built against this mechanism yet.
