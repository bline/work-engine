# Transition Fencing and Leases

> **Question:** How do concurrently prepared transitions, across independently owned dimensions, stay coherent against an authoritative world that keeps changing underneath them while they are being prepared?

## Purpose

This is a **mechanism view**, not a dimension view. It describes a reusable preparation-and-activation discipline that Context Lifecycle, Organizational Compilation, and (confirmed 2026-09-16) Runtime Realization's own executable-generation substrate all need and none owns, not a class of architectural truth this page owns itself.

> **This mechanism protects *concurrency* — whether a transition being prepared is still valid when it activates. It is not a second Revision/CAS page.**

`mechanisms/revision-cas-and-publication.md` already answers whether a given publish is safe against the current head. This mechanism answers a different question: whether a transition that takes real preparation time (compiling a checkpoint candidate, compiling an admitted organization) is still preparing against a world that is still authoritative by the time it's ready to activate.

```text
REVISION / CAS / PUBLICATION                TRANSITION FENCING / LEASES
    Is this successor based on the              May this transition proceed
    authoritative predecessor/head?              concurrently with other
    Can it publish atomically?                   transitions?
    Is this write stale?                        What world revision is it
                                                  preparing against?
                                                 What must remain stable
                                                  until activation?
```

---

## Preparation vs Publication — the Central Distinction

Confirmed 2026-09-16 directly against `deterministic-authority-projection-and-adaptive-organizational-topology.md` Part 7 and `semantic-context-lifecycle-manager.md`'s own implemented transition-lease sequence, not assumed from the mechanism's name alone:

```text
authoritative revision R
        ↓
transition preparation binds to R
        ↓
acquire the appropriate fence/lease
        ↓
prepare the successor (compile checkpoint candidate, or
compile admitted organization — domain-specific work)
        ↓
revalidate the bound revision(s) / lease, immediately before activation
        ├── changed  → stale; abort or recompute against the successor world
        └── unchanged
                ↓
        publish through Revision/CAS
                ↓
            activate
                ↓
          release the fence
```

Preparation is the expensive, time-consuming part (compilation, verification). Publication is a single atomic CAS check (owned by Revision/CAS). This mechanism exists specifically to protect the *gap* between them — the window during which the world the preparation was based on could stop being authoritative before activation.

---

## The Governing Invariant

Stated once, in `deterministic-authority-projection-and-adaptive-organizational-topology.md` Part 7.3, general enough to cover every instance of this mechanism:

> **No transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared.**

**Revision binding, not service signaling, is the actual mechanism.** Preparation binds to an exact pair (or set) of authoritative revisions. If the authoritative source commits a new revision during that window, the prepared transition fails promotion as stale and must recompute — mechanically, from the changed revision itself, not because any service told it something happened. Nothing here is a new concept; it is the same CAS/fencing discipline `mechanisms/revision-cas-and-publication.md` already establishes, extended to cover a preparation interval rather than a single instantaneous publish.

---

## Two Fence Classes, Deliberately Not Collapsed Into One Lock Abstraction

```text
decision-episode fence
    protects an unresolved semantic judgment in general — not only
    organizational ones, any event-scoped semantic decision whose result
    would be lost by context replacement. While the judgment remains
    active, retirement admission stays closed, even though observation
    and preparatory compilation may continue.

topology-transition fence
    protects the actual organizational state change itself: accept split
    → acquire topology transition → compile authority/contracts/
    projections → publish coherent successor organization → activate →
    release. During this window, lifecycle cannot checkpoint or replace
    the parent against a stale topology revision.
```

They share the generic revision-binding discipline above, but protect different things — one protects a judgment in progress from being lost, the other protects an in-progress world-changing transition from activating against a world that moved. Collapsing them into one generic "lock" would lose exactly the distinction that makes each fence's own release condition meaningful.

**A third, real instance protects a different kind of thing than either named class, found 2026-09-16 — noted here as an open naming question, not resolved.** The executable-generation host-process reload (Confirmed Instances, below) fences a preparation interval the same way these two do, but what it protects is neither an unresolved semantic judgment nor an organizational topology change — it is which concrete executable/host-process generation currently holds authority to realize a role's `harness_runtime`. Whether this warrants naming a third fence class (an "executable-substrate fence") or whether "two fence classes" should simply become "at least two, not exhaustive" is left open rather than decided by this edit.

**The reverse race matters too, and is resolved the same way, not by inventing a new rule:** topology should not inject a new organizational judgment while a lifecycle retirement is already in progress for that context. It either defers the organizational decision until the successor context reconciles, or — if policy ranks it higher — invalidates and aborts the lifecycle preparation and keeps the context. That arbitration may itself be partly deterministic (critical provider pressure with no safe deferral outranking an ordinary in-progress topology episode) without being permanently hard-coded.

---

## Neither Consumer Owns This Mechanism

```text
Context Lifecycle
    consumes fencing — its own transition-lease sequence (below) is a
    concrete instance, not an invention of the mechanism itself

Organizational Compilation / topology transition
    consumes fencing — the topology-transition fence above is the
    concrete instance this dimension would use, once implemented

Runtime Realization / executable-generation substrate
    consumes fencing — a real, implemented instance (below), one layer
    beneath RoleRealization itself, protecting a third kind of thing
    neither named fence class above describes

none of the three owns fencing
```

This is the entire reason this page exists rather than living inside `context-lifecycle.md`. That page's own §6 already states it explicitly, correcting an earlier informal characterization that risked implying ownership: all three dimensions are independent, equally-ranked consumers of one shared layer, none senior to the others.

---

## Confirmed Instances

### Context Lifecycle — implemented

Re-verified directly against `semantic-context-lifecycle-manager.md`'s own text, not restated from memory: "begin a revision-bound preparation fence → ... → atomically publish checkpoint H → reread raw thread snapshot S' → if S' differs, retain the delta and recompile; repeat until stable → promote preparation to the final transition lease → start a semantically sterile retirement control turn → target model calls `new_context`..." Its own stated transition-lease invariant: "Retirement readiness and actuator delivery occur under one revision-bound transition lease. Any competing input, effect, or runtime-binding change revokes the lease before actuation." This is real, live machinery — the preparation-fence-then-transition-lease sequence above is not a hypothetical shape, it is this dimension's own implemented reality, generalized.

### Organizational Compilation / topology transition — named, not implemented

`deterministic-authority-projection-and-adaptive-organizational-topology.md` Part 7.3 names the topology-transition fence's own sequence (accept split → acquire transition → compile → publish → activate → release) conceptually. No organizational compiler exists yet (`organizational-compilation.md`'s own status), so this instance has no implementation evidence — the fence type is named and reasoned about, not built.

### Runtime Realization / executable-generation substrate — implemented, protecting a third kind of thing

Verified directly against `app-server/src/executable-generation-manager.mjs` and `executable-generation-store.mjs`, 2026-09-16. `ExecutableGenerationManager.requestReload` binds preparation to an exact predecessor generation and installs an in-memory admission fence — `this.reload = context` (`executable-generation-manager.mjs:298`), by the code's own comment "before the first durable write yields" — then calls `store.beginReload({predecessorGenerationId})` (line 300), which itself rejects the durable write if the store's own active generation no longer matches (`executable-generation-store.mjs:213`). While the fence holds, `openAdmission` refuses new generation-bound work (`reload_fence_active`, `executable-generation-manager.mjs:164-170`) and a second concurrent reload is refused outright (`reload_already_requested`) — the same "may this transition proceed concurrently with other transitions" question this mechanism exists to answer, answered: no, exactly one at a time. Preparation (snapshot → build → validate, real wall-clock time) only advances once admission drains to zero (`#advanceIfDrained`, lines 321-338), and the bound predecessor identity is revalidated immediately before publication (`store.activate`'s `expectedActiveGenerationId` check, `executable-generation-store.mjs:300-302`) — the exact bind → prepare → revalidate → publish → activate → release sequence this page's own Relationship to Resource Lease and Fencing table describes, matched step for step. Unlike the other two instances, what this fence protects is neither an unresolved semantic judgment nor an organizational topology change — see the note above.

---

## Key Invariants

1. **No transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared.**
2. **Preparation binds to an exact authoritative revision or revision set; a changed revision invalidates the prepared transition mechanically, without requiring the authoritative source to signal anything.**
3. **Decision-episode fences and topology-transition fences protect different things and are not the same lock.**
4. **A transition invalidated during preparation is not thereby granted new authority to activate anyway — it must recompute against the successor world (`authority-and-ownership.md` §12).**
5. **None of Context Lifecycle, Organizational Compilation, or Runtime Realization's executable-generation substrate owns this mechanism; all three are equally-ranked consumers.**
6. **This mechanism protects concurrency of preparation, not publication safety — that remains `mechanisms/revision-cas-and-publication.md`'s own content, not duplicated here.**

---

## What This View Does Not Show

This page does not define:

- how a publish itself is made atomic and CAS-safe (`mechanisms/revision-cas-and-publication.md`);
- the exact compiled-checkpoint schema or organizational-admission schema (each dimension's own page);
- the arbitration policy for the reverse race beyond naming that it exists and is partly deterministic;
- concrete lock, mutex, or database-transaction implementation details — this page describes the logical discipline, not a storage or concurrency-primitive implementation.

---

## Relationship to Revision/CAS/Publication

This mechanism is built directly on top of that one and does not restate its predecessor/head semantics — a revalidation check in this mechanism's own flow ("changed → stale") *is* a Revision/CAS check, just performed at the end of a preparation interval rather than at the moment of an isolated publish.

## Relationship to Authority and Ownership

A stale, invalidated preparation is exactly `authority-and-ownership.md` §12's invalidation-never-mints-authority invariant in this mechanism's own terms: losing a fence race removes validity, never grants standing to activate anyway against the authority ceiling that was already in force.

## Relationship to Resource Lease and Fencing

**Added 2026-09-16, an explicit sibling-mechanism distinction, not a merge.** `mechanisms/resource-lease-and-fencing.md` shares this mechanism's fencing-token vocabulary but protects a structurally different invariant, confirmed by direct comparison rather than assumed from the shared word "fencing":

```text
THIS MECHANISM                              RESOURCE LEASE AND FENCING

protects a one-shot preparation interval    protects a standing holder
(bind -> prepare -> revalidate -> publish   relationship (acquire -> hold,
-> activate -> release)                     renew, or release; no
                                             preparation phase, no single
staleness = the bound world revision        activation event)
changed while preparing
                                             staleness = an older
                                             holder/token no longer
                                             matches the current
                                             generation
```

The fenced-active-binding gap this session found (`runtime-realization.md` §12) was tested against both mechanisms directly and belongs to the other one, not this one — a standing "who currently holds authority to execute as this role" relationship, not a bounded transition being prepared toward one activation. Same fencing-token machinery does not mean same architectural invariant.

---

## Related Architecture Views

- **`context-lifecycle.md`** — a confirmed, implemented instance (§6, §7 there).
- **`organizational-compilation.md`** — the named, unimplemented instance (topology-transition fence).
- **`runtime-realization.md`** — a second confirmed, implemented instance, at the executable-generation substrate layer beneath `RoleRealization` itself (§9, §10 there); protects a third kind of thing neither named fence class above describes.
- **`mechanisms/revision-cas-and-publication.md`** — the mechanism this one is built on top of; defines the publication-safety half this page does not duplicate.
- **`authority-and-ownership.md`** — the general invalidation-never-mints-authority invariant this mechanism's own stale-preparation rule instantiates.
- **`mechanisms/resource-lease-and-fencing.md`** — the sibling mechanism; shares fencing-token vocabulary, protects a structurally different (standing-holder, not preparation-interval) invariant.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: partial
  owner: app-server/docs/architecture/mechanisms/transition-fencing-and-leases.md
  status_as_of: 2026-09-16
```

`design: accepted` — the mechanism's own recognition and naming (both fence classes, the shared revision-binding discipline, and the governing invariant) was explicitly settled through direct discussion, matching this architecture's own established bar. `reconciliation: reconciled` — confirmed against `semantic-context-lifecycle-manager.md`'s own implemented sequence, `deterministic-authority-projection-and-adaptive-organizational-topology.md`'s own Part 7 text, and (added 2026-09-16) direct verification of `executable-generation-manager.mjs`/`executable-generation-store.mjs`. `implementation: partial` at the mechanism level, not uniform across its three instances: the decision-episode/transition-lease sequence is real, implemented machinery in Context Lifecycle; the executable-generation host-process reload is likewise real, implemented machinery, one layer beneath `RoleRealization` itself; the topology-transition fence remains named conceptually only, with no organizational compiler yet to fence. Each consuming dimension's own page remains the authority on its own instance's status; this page does not restate or override any of them.
