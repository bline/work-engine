# Service-Plane Reconciliation

## Status

Steps 3–4 (seam map, reconciliation) of the bounded architecture-intake
session that began with
[`service-plane-inventory.md`](service-plane-inventory.md) (step 1, the
implemented baseline) and
[`service-plane-and-kernel-domain-boundary.md`](../ideas/pending/service-plane-and-kernel-domain-boundary.md)
(step 2, the prospective architecture). This document runs the 20 units the
baseline identified against the prospective three-plane architecture and the
`ServiceOperation` grammar hypothesis, records where they align, and records
where they don't.

This document is expected to change the prospective architecture, not merely
confirm it. If every unit fit the hypothesis cleanly on first contact, that
would be reason to suspect the classification was fitted to the theory rather
than the reverse. Several do not fit cleanly. Those are the interesting
findings, not defects in the audit.

Reconciliation states below use
`incremental-architecture-intake-and-seam-reconciliation.md`'s vocabulary
(§7.2): `ALIGNED`, `TERMINOLOGY`, `OWNERSHIP`, `REPRESENTATION`, `LIFECYCLE`,
`ORDERING`, `COUPLED_DECISION`, `FUNDAMENTAL_CONFLICT`, `UNRESOLVED`.

**Authority:** Exploratory only. This document does not accept any
constituent idea, authorize implementation, or freeze the prospective
architecture. Per that intake method's own §8, it resolves only the structure
required for coherent proposal formation later — it does not specify exact
repository transformations, file placement, or implementation routes.

---

## Part 1 — Per-unit plane assignment and grammar fit

Each entry is deliberately compact. Depth is spent where a real seam exists,
not spread evenly.

**`agent-instruction-review`** — SERVICE. Grammar fit: `ALIGNED`. Ten fields
populate honestly; determinism is correctly `hybrid`.

**`implementation-review`** — SERVICE. `ALIGNED`. Notably the cleanest
"admits/validates only" shape in the inventory alongside `review-subject`.

**`review-episode`** — SERVICE, but flagged `OWNERSHIP`-adjacent. Its
mechanics (identity, revision, authority-grant, writer-generation takeover,
CAS) contain nothing specific to "review." Nothing here mentions Git, code,
or review semantics — only lifecycle transitions over an opaque payload. That
makes it a candidate for the **kernel** plane as a generic admission-gated
durable-episode primitive, not a workflow-private service. Currently it has
exactly one consumer and is described as "workflow-private review-lifecycle
choreography," so promoting it would require evidence a second consumer
actually needs the same shape — evidence this document does not have.
Recorded as `UNRESOLVED`, not `ALIGNED`: is this kernel-shaped machinery that
happens to have one caller today, or a workflow-specific service that merely
looks generic?

**`review-bench`** — does not fit any of the three planes, but this is not
evidence for a fourth plane. Kernel/service/workflow describe **architectural
responsibility** within production, governed execution. `review-bench` is
explicitly non-production tooling (`production_review_authority: false`, no
coordinate relevance by design) — a matter of **scope**, not responsibility.
An evaluation system could itself someday have internal kernel/service/workflow
structure; "evaluation" is not a peer of the three planes, it is a boundary
around which units are subject to plane classification at all:

```text
production / governed execution substrate
    KERNEL, SERVICE, WORKFLOW

out-of-band / auxiliary environments
    evaluation tooling, developer tooling, migration tooling, diagnostics, ...
```

Recorded as `TERMINOLOGY`: not a conflict, and not a missing plane — a missing
scope boundary in section 1, which currently only distinguishes planes within
production execution without saying so.

**`reviewer-runtime`** — SERVICE, and the concrete instance of section 8's
"semantic/hybrid operations require a realization" claim. Grammar mostly
fits, but `realization_coupling: direct — two hard-bound adapters selected by
the caller, not by an admitted realization` means it does not yet match the
target state `provider-turn-harness-runtime-and-operator-projection.md`
describes. This is `ORDERING`, not `FUNDAMENTAL_CONFLICT`: the admission
mechanism that document requires must exist before this unit's coupling can
be evaluated against the target, and until then this unit is honestly
described using `authority-backed-architecture-directions-as-workflow-inputs.md`'s
adoption vocabulary as `inherited_transitional_state`, not as a defect
introduced by the service-plane idea.

**`review-subject`** — SERVICE. `ALIGNED` with no reservation. The strongest
existing exemplar of a pure, domain-rich, kernel-independent service.

**`claim-evidence` (ledger core)** — this is the most consequential
reclassification in this pass. The inventory called it "kernel-adjacent"; on
reconciliation it should be split:

```text
ledger mechanics (identity, revision, CAS, idempotency-by-operation_id)
    -> KERNEL

each profile (proposal-research-v1, revision-bound-review-finding-v1,
production-path-v1)
    -> SERVICE, hosted on the kernel-owned ledger mechanism
```

Nothing in the ledger's transition/authority/CAS machinery is specific to
claims, proposals, or reviews — it is a generic revisioned-state-with-grant
primitive. The three profiles are what make it domain-rich.

Precision matters here: this is the claim that **these mechanics semantically
belong in the kernel**, not the claim that **the existing `claim-evidence`
implementation is now a kernel component**. This document does not propose
moving, renaming, or extracting any code. A generic revisioned-state primitive
may eventually be extracted from `claim-evidence`; that is a future
implementation question this reconciliation does not decide. `OWNERSHIP`,
resolved at the semantic level only: kernel owns the envelope mechanics
conceptually, service plane owns each profile's semantics.

**A repeated kernel-primitive family, noticed rather than designed.** Three
independent units in this pass share the same shape:

```text
claim-evidence ledger    revision + CAS + idempotency-by-operation_id
review-episode           revision + CAS + writer-generation takeover
slice-campaign state     revision + CAS + digest chain
```

All three are: durable revisioned state, plus an admitted transition, plus
optimistic concurrency (CAS/fencing), plus an authoritative successor
identity. `review-episode` is already flagged above as kernel-shaped but
under-evidenced (one consumer); `slice-campaign`'s campaign state is the
subject of the `COUPLED_DECISION` below. This is not a proposal to design a
generic revisioned-episode primitive now — three independent implementations
converging on the same shape is exactly the kind of evidence such an
abstraction should eventually emerge from, rather than being designed
prospectively and imposed on all three. Recorded as an observation for the
next synthesis, not a decision.

**`claim-evidence` (Git-checkpoint observation)** — SERVICE. `ALIGNED`.

**`operational-coordination`** — same shape question as `workspace-coordination`
(both are domain-neutral coordination primitives), but weaker evidence: its
recovery is a heuristic regex match, not a durable CAS index. This is
`LIFECYCLE`, not `OWNERSHIP`: its *conceptual* place is likely alongside
`workspace-coordination` in the kernel, but it cannot be admitted there on its
current recovery mechanism. Strengthen first; reclassify after.

**`product-development/artifact-root`** — SERVICE. `ALIGNED`. Domain-neutral
filesystem publication with clean digest evidence; nothing code-specific.

**`product-development/delivery-adapters`** — SERVICE, domain `product-development`
— not `code`. The inventory's `semantic_owner: code domain` label is too
coarse here. This surfaces a real refinement to section 1: `domain` needs to
be a namespaced tag (`product-development`, `code.review`,
`code.candidate-trajectory`) rather than the four-way
kernel/runtime/code/other split used for triage in the inventory. `TERMINOLOGY`.

**`skills-migration-integrity`** — no plane. Confirmed as ordinary utility
code, not a service-plane participant at all, matching section 6's working
conclusion exactly. `ALIGNED` with the prospective document's own prediction.

**`workspace-coordination` (core)** — KERNEL, and evidence toward
[refinement 12](../ideas/pending/service-plane-and-kernel-domain-boundary.md#10-open-reconciliation-questions),
though not for the conclusion an earlier pass at this document drew from it.
`admitMutation` is a kernel-owned, fencing-checked admission operation that
wraps a **caller-supplied mutation callback**. That is fully consistent with
**exclusive kernel ownership plus a boundary**, not with kernel and
service-plane membership overlapping:

```text
SERVICE
    domain mutation semantics
         |
         v
KERNEL OPERATION
    admitMutation(resource, fencingToken, mutation)
    validates fencing / CAS
         |
         v
authorized execution of the callback
```

The kernel does not become part of the service plane merely because services
call into it. The corrected reading of refinement 12:

> **Kernel primitives may expose bounded operations consumed by services
> without becoming service-plane operations themselves.**

This also reopens something about the `ServiceOperation` grammar (section 3):
`admitMutation` is a real operation with identity, authority, effects, and
recovery semantics, but it is not a *service* operation — it is a kernel
operation. That suggests the shared grammar this whole exercise is looking
for may not ultimately be called `ServiceOperation` at all, but something
broader that both kernel and service operations instantiate as distinct
kinds:

```text
OwnedOperation
    owner plane, contract revision, implementation revision, authority,
    effects, evidence, recovery, ...

    KernelOperation   (e.g. admitMutation)
    ServiceOperation  (e.g. review-subject.create_physical_profile)
```

This is not a schema proposal — it is a single data point (`admitMutation`)
suggesting the ten-field grammar may describe operations beyond the service
plane, which is exactly the kind of thing reconciliation exists to surface,
not resolve. `ALIGNED` for the kernel-ownership reading; `UNRESOLVED` for
whether `ServiceOperation` needs a more general supertype.

**`workspace-coordination` (Git realization)** — SERVICE, and a direct
instance of the pattern above: `git-publisher.mjs`/`git-worktree.mjs` supply
the domain-specific mutation callback that the kernel's `admitMutation` wraps.
`ALIGNED`.

**`slice-campaign` (campaign state)** — WORKFLOW by its own description ("this
unit *is* the workflow"), but its durable state machinery (CAS, digest,
revision chain) is service-shaped in the same way `review-episode`'s is. This
is a `COUPLED_DECISION`: whether campaign-state persistence should be factored
out as its own service (mirroring `review-episode`'s separation of state from
whatever consumes it) is a decision that affects both this unit and the
Candidate Trajectory family, which already needs exactly this unit to retain
a predecessor/successor relationship it currently discards
(`service.mjs:255-278`). Neither this document nor
`candidate-trajectory-durable-foundation.md` should resolve this
independently — it is one decision with two dependents.

**`slice-campaign` (native-review host)** — WORKFLOW. Confirms the inventory's
own note that it deserves top-level standing rather than nesting. No new
tension beyond what was already recorded.

**`slice-campaign` (capability dispatch)** — the clearest `OWNERSHIP` seam in
the whole pass, and it names itself: the inventory already wrote "should be
kernel; is actually workflow-private." The evidence supports one conclusion
only: do not promote this specific dispatcher to kernel scope on the strength
of its shape alone, since it is proven across exactly one consumer. It does
**not** support templating the eventual common grammar on `claim-evidence`
either — the prospective document's own original instinct was correct here:
extract the grammar from underneath all the precedents (`claim-evidence`,
`workspace-coordination`'s `admitMutation`, and this dispatcher), not by
copying whichever one currently looks most generic. This is now more
important than it was before reconciliation: `admitMutation` suggests the
common grammar may cross the kernel/service boundary (see the
`workspace-coordination` entry above), which a `claim-evidence`-shaped
template would not naturally capture. This unit can be re-hosted on whatever
envelope eventually emerges, if and when that migration is worth its cost;
nothing requires resolving that now.

**`slice-campaign` (legacy adapters)** — SERVICE (realization adapter), same
`ORDERING` seam as `reviewer-runtime`: both are honestly
`inherited_transitional_state` relative to
`provider-turn-harness-runtime-and-operator-projection.md`'s target, pending
the same admission mechanism. Grouping these two under one seam avoids
treating them as two separate problems.

**`slice-campaign` (completion-publication)** — SERVICE (state owner), with
the `REPRESENTATION` seam `service-plane-inventory.md` already found:
independently redeclared `ACCEPTED_CHECKPOINT_FIELDS`/`CHECKPOINT_ATTRIBUTIONS`
rather than importing them from `slice-checkpoint`/`review-subject`. This
reconciliation adds nothing new here beyond confirming the seam persists and
naming its resolution direction: establish `slice-checkpoint` (or
`review-subject`) as the canonical owner and make this unit consume that
definition, rather than deciding a new third owner.

**`slice-campaign` (strategic-reconciliation)** — WORKFLOW (thin projection).
`ALIGNED`, and its `continuity` vocabulary remains the strongest independent
corroboration of the coordinate/continuation concept found in this pass.

---

## Part 2 — Answering the twelve reconciliation questions

Numbered 1–8 as in `service-plane-and-kernel-domain-boundary.md` §10, 9–12 as
added there under "Refinements surfaced by review."

**1. What belongs in kernel/control versus an ordinary service?**
Partially answered by Part 1: `workspace-coordination`'s core and
`claim-evidence`'s ledger mechanics belong in the kernel *semantically*
(neither is a proposal to move code — see the `claim-evidence` entry's
precision note); `review-episode` and `operational-coordination` are
plausible candidates pending more evidence (one consumer; weak recovery,
respectively). A repeated shape emerged across three units
(`claim-evidence`, `review-episode`, `slice-campaign` state — see the
repeated-primitive-family note above): durable revisioned state, an admitted
transition, optimistic concurrency, and an authoritative successor identity.
That is the closest this pass came to a general rule, and it is an
observation to carry forward, not a settled boundary test. `UNRESOLVED` as a
general rule; `ALIGNED` for the two confirmed cases.

**2. What is the minimal common operation envelope?**
Still open. This pass did not find a reason to add fields beyond section 3's
ten, but refinement 9 (below) shows at least one is missing
(`implementation revision`). `UNRESOLVED`.

**3. Which existing services are true semantic owners versus derivations,
adapters, or coordinators?**
Answered unit by unit in Part 1. No unit resisted this classification once
plane assignment was separated from it — the ambiguity in this pass was
always about *plane*, not about *role* (owner/derivation/adapter/coordinator
labels held up).

**4. Which workflow-private units should remain workflow-local rather than
become services?**
`slice-campaign`'s native-review host and legacy adapters, confirmed. New
finding: `review-bench` is not a "workflow-private unit deciding whether to
become a service" question at all — it's outside the three-plane model
entirely (Part 1). The question's framing assumed every unit was a candidate
service; `review-bench` shows that's not universal.

**5. How does service state participate in coordinates and reconstruction?**
Refinement 11 (coverage vocabulary) is the concrete gap this pass surfaces.
The four coordinate fragments named in
`service-plane-and-kernel-domain-boundary.md` §5 remain accurate; none of the
20 units contradicted them. `UNRESOLVED` pending the coverage-state design.

**6. How are provider/harness realization services represented without
leaking implementations upward?**
`reviewer-runtime` and the `slice-campaign` legacy adapters are both
`ORDERING` seams against the same target architecture, not two separate
problems. `UNRESOLVED` — depends on
`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`'s
admission mechanism landing first, which is outside this document's scope.

**7. How do capability grants authorize service operations without
reintroducing the naming conflation?**
Not resolved by this pass — no unit's *implementation* forced a decision here
one way or the other, because none of them currently distinguish the two
senses in code (the conflation is in the word "capability" itself, not in any
single unit's design). `UNRESOLVED`, unchanged from the prospective document.

**8. Which current representations duplicate another owner's state?**
One confirmed instance (`completion-publication.mjs`, Part 1). This pass
found no second instance among the 20 units, but did not exhaustively check
every cross-service field for duplication — that would require a dedicated
pass, not a byproduct of this one. `UNRESOLVED` as a general sweep;
`ALIGNED` for the one instance already found (resolution direction named).

**9. Realization vs. implementation revision.**
Confirmed necessary by this pass, not merely hypothetical: `review-subject`
already depends on a pinned analyzer/checkpoint-validator digest before every
invocation (`legacy-backend-adapter.mjs:57-106`) — that digest pair *is*
implementation revision, already present in the code, just not named as a
`ServiceOperation` field. Section 3 should add it. `ALIGNED` — this is not a
speculative gap, it's an existing mechanism without a name in the grammar.

**10. A third inference case (architectural gap vs. semantic judgment).**
No unit in this pass currently implements the "temporary fallback with
provenance" state explicitly, which means it is a real gap, not merely an
absent grammar field. The closest existing precedent is `code-change-profile`'s
`unsupported`/`failed` measurement states — those are per-field, not
per-fact-ownership, but establish that Work Engine already has the discipline
this case needs. `UNRESOLVED` — needs a design, but has a clear precedent to
build from.

**11. Coverage, not just presence.**
See question 5. `UNRESOLVED`.

**12. Can kernel primitives expose bounded operations consumed by services?**
**Answered, `ALIGNED`, with the question retitled.** The original framing
("can kernel primitives expose service operations?") presumed an answer of
"yes, so kernel and service membership overlap." `workspace-coordination`'s
`admitMutation` (Part 1) shows the opposite reading is correct: kernel owns
exclusive, generic fencing/CAS admission and exposes a bounded operation; the
service supplies the mutation semantics as a caller-provided callback. The
kernel does not become part of the service plane by being called into.
Kernel primitives may expose bounded operations consumed by services without
becoming service-plane operations themselves — and this may mean the shared
grammar (question 2) needs a supertype broader than `ServiceOperation`,
covering both kernel and service operations as distinct kinds (see the
`workspace-coordination` entry in Part 1). That broader-grammar question is
left `UNRESOLVED`; the ownership-exclusivity question is `ALIGNED`.

---

## Part 3 — What this changes in the prospective architecture

Per Sol's expectation that reconciliation should change the document, not
merely confirm it, these are the concrete revisions
`service-plane-and-kernel-domain-boundary.md` should carry forward once this
reconciliation itself has been reviewed:

1. Reclassify `claim-evidence`'s ledger mechanics as belonging to the
   **kernel semantically**, with its three profiles as service-plane
   instances hosted on it — not "kernel-adjacent" as an unresolved middle
   category, and not a proposal to move, rename, or extract any code.
2. Scope the three-plane model explicitly to production/governed execution,
   and recognize auxiliary tooling (`review-bench`, and by extension
   developer/migration/diagnostic tooling) as outside plane classification
   entirely — not as a fourth plane alongside kernel/service/workflow.
3. Add `implementation revision` to the `ServiceOperation` grammar (section
   3), distinct from `contract revision` and from `semantic realization`, per
   refinement 9 — this is now confirmed against an existing mechanism
   (`review-subject`'s digest pinning), not merely hypothesized.
4. Refine `domain` from the inventory's four-way triage labels
   (kernel/runtime/code/other) to a namespaced tag
   (`product-development`, `code.review`, `code.candidate-trajectory`, …), per
   `product-development/delivery-adapters`'s misfit under the coarse "code
   domain" label.
5. Correct refinement 12's resolution: `workspace-coordination`'s
   `admitMutation` demonstrates **exclusive kernel ownership plus a boundary**,
   not overlapping kernel/service membership. Kernel primitives may expose
   bounded operations consumed by services without becoming service-plane
   operations themselves. Record as an open question whether this means the
   shared grammar needs a broader `OwnedOperation` supertype with
   `KernelOperation` and `ServiceOperation` as distinct kinds — not settled,
   only surfaced.
6. Record the `review-episode` / `slice-campaign` campaign-state
   `COUPLED_DECISION`: whether campaign-state persistence should separate from
   campaign orchestration the way `review-episode` already separates state
   from its consumer. This decision affects the Candidate Trajectory family
   directly and should not be made independently by either effort.
7. Record the repeated kernel-primitive-family observation (`claim-evidence`,
   `review-episode`, `slice-campaign` state all sharing durable revisioned
   state + admitted transition + optimistic concurrency + authoritative
   successor identity) as evidence to watch, not a primitive to design now.
8. Do not template the eventual shared envelope on `claim-evidence` alone,
   even though it is the strongest single precedent. Retain
   `claim-evidence`, `workspace-coordination`'s `admitMutation`, and
   `slice-campaign`'s capability dispatch together as evidence, per the
   prospective document's own original instinct — item 5 above is part of why
   this now matters more than it did before reconciliation.

Nothing in this pass produced a `FUNDAMENTAL_CONFLICT`. The seams found were
`OWNERSHIP` (capability dispatch, claim-evidence's true home),
`REPRESENTATION` (completion-publication), `LIFECYCLE` (operational-coordination's
recovery), `ORDERING` (the two realization-coupling seams), `TERMINOLOGY`
(review-bench's scope boundary, domain-tag granularity), and one
`COUPLED_DECISION` (campaign-state factoring). That is a healthy distribution
for a first reconciliation pass — real tensions, none of them requiring an
idea to be rejected or narrowed.

## Non-goals

- This document does not freeze the prospective architecture. Part 3's
  changes are recommendations for the next revision of
  `service-plane-and-kernel-domain-boundary.md`, not applied here.
- It does not decompose this work into implementation proposals. Per
  `incremental-architecture-intake-and-seam-reconciliation.md` §13, proposal
  decomposition follows reconciliation; it is not attempted in this document.
- It does not resolve any question marked `UNRESOLVED` above. Those remain
  open and are not silently narrowed by omission.
- It does not authorize implementation of anything, including the
  `admitMutation` pattern's generalization or the `claim-evidence`
  kernel/service split.

## Relationships

| Direction | Relationship |
| --- | --- |
| [`service-plane-inventory.md`](service-plane-inventory.md) | The baseline every unit-level claim in Part 1 traces back to. |
| [`service-plane-and-kernel-domain-boundary.md`](../ideas/pending/service-plane-and-kernel-domain-boundary.md) | The prospective architecture this document tests. Part 3 names the changes it should carry forward. |
| `incremental-architecture-intake-and-seam-reconciliation.md` | This document performs its §5–7 (seam map, reconciliation) for the service-plane family specifically, using its exact reconciliation-state vocabulary. |
| Candidate Trajectory family (`candidate-trajectory-*.md`) | The campaign-state `COUPLED_DECISION` (Part 3, item 6) affects this family directly and should be resolved jointly, not independently by either effort. |
| The broader multi-idea architecture-direction synthesis (not yet started) | Remains sequenced after this reconciliation, per the prospective document's own Relationships table. |
