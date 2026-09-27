# Execution Strategy Selection — Candidate Dimension

## Identity and state

- Candidate ID: `work-engine.execution-strategy-selection`
- State: candidate dimension specification; not accepted, prioritized, or
  authorized for implementation; not part of the canonical atlas
- Decision owner: user or future explicitly authorized architecture owner
- Primary consumers: whichever authority performs the acceptance act on a
  projected routing nomination (advisory until accepted, per
  `implementation-contract-compilation.md` §6), and, downstream of
  acceptance, `runtime-realization.md`'s own stage-3 admission

```yaml
architecture_status:
  design: proposed
  reconciliation: unreconciled
  authorization: unrecorded
  implementation: none
  owner: app-server/ideas/pending/execution-strategy-selection.md
  status_as_of: 2026-09-26
```

```yaml
idea_provenance:
  origin: reconciliation_synthesis
  related_reconciliations:
    - app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md
    - app-server/ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md
    - app-server/ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md
    - app-server/docs/architecture/implementation-contract-compilation.md
    - app-server/docs/architecture/runtime-realization.md
```

Not a source-document transcription like most files in this directory. This
page is a fresh synthesis, produced this session through a four-round
falsifier/grain test against the `routing.vs.admission` ruling's own named-but-
architecturally-unhomed Stage 2 (`implementation-contract-compilation.md` §6;
`runtime-realization.md` §5). It is recorded here, in `ideas/pending/`, rather
than in `docs/architecture/`, precisely because it has not yet been through an
independent adversarial pass or an operator admission decision — both
explicitly still pending.

---

## Candidate question

> Given a bounded implementation contract, its Stage-1 (`implementation-contract-compilation.md`) characterization, revision-bound capability/outcome evidence, and an applicable routing-policy revision, which of the semantically supported executor classes is warranted for this subject?

This question is held deliberately narrow. Five rounds of falsifier testing
this session progressively removed content that looked plausible at first
pass but either duplicated an existing owner or would have silently widened
the already-settled 2026-09-15 `routing.vs.admission` ruling, which names
Stage 2 specifically as **executor-class routing/acceptance** and nothing
broader.

---

## How This Candidate Was Bounded

Four corrections, in order, each removing content the prior pass had wrongly
folded in:

1. **Temporal ordering.** Stage 2 runs strictly after Stage 1 has already
   compiled the contract. `planning/compilation effort` — how much upstream
   inference the Sol-class compiler spent — is a sunk cost by the time this
   candidate's judgment runs, not a quantity it selects. Removed entirely; it
   belongs to a separate, currently unowned question about how much
   upstream planning `implementation-contract-compilation.md`'s own compiler
   should invest, not this candidate's concern.
2. **Ownership honesty.** Capability/outcome evidence and routing-policy
   authorship were initially treated as already settled elsewhere
   (`evidence-and-claims.md`; a self-owning standing policy artifact,
   respectively). Neither is actually settled: `evidence-and-claims.md`'s own
   §10 profile pattern is, per the evidence-calibrated document's own
   corrected finding, "a real but compositional relationship, not a claim
   already stated"; no canonical page names an owner for routing-policy
   authorship at all. Both are carried here as **open dependencies**, not
   assumed.
3. **Artifact primacy.** The candidate owns a durable judgment artifact.
   Routing nomination is a *projection* of that artifact's own judgment,
   never a second truth class — the artifact records what was warranted and
   why; the nomination is only the advisory hand-off of one field from it.
4. **Component-by-component ownership test.** The most consequential
   correction. An earlier pass treated the addendum's own four-part strategy
   tuple (`plan_ir_projection_resolution + executor_capability_class +
   execution_autonomy + verification_review_strength`) as one jointly
   *owned* judgment. Tested component-by-component against exact existing
   and named-pending owners, only `executor_capability_class` survives as
   something this candidate may hold authoritatively:
   - `plan_ir_projection_resolution` — unclaimed by any existing page, but
     never named by the 2026-09-15 ruling either. Owning it here would widen
     the ruling, not fulfill it. Demoted to a joint-evaluation input.
   - `execution_autonomy` — substantially pre-owned upstream:
     `implementation-contract-compilation.md` §4 already authors an
     "explicit implementation-discretion envelope" as a compiled-contract
     field at Stage 1, before Stage 2 runs; the specific grant within that
     ceiling is an authority-projection act
     (`authority-and-ownership.md` §8–9), the same act-class already
     excluded from this candidate for acceptance itself. Demoted to a
     joint-evaluation input.
   - `verification_review_strength` — `review.md` explicitly disclaims
     "specialist selection, execution, or independence mechanics," naming
     `adaptive-review-panel-coordination` — a distinct, still-unresolved
     mechanism — as its own intended owner. Claiming it here would preempt
     an already-named future owner. Demoted to a joint-evaluation input.

Splitting these components does not destroy an irreducible semantic
tradeoff — the addendum's own joint economic framing survives intact as this
candidate's own internal evaluation method. It merely keeps ownership where
it already sits, or is already named to eventually sit. Joint evaluation is
not joint ownership.

5. **Stage-1 cardinality narrowing.** An early adversarial pass asked
   whether `warranted_executor_class` merely restates ICC's own §4 field, "a
   proposed executor class with an evidence-backed readiness assessment" —
   singular wording. What is actually established, and no more: ICC's own
   top-level Question and §6 body use plural executor-class wording ("which
   executor classes are semantically supported, with what evidence-backed
   readiness?"); ICC §4 currently exposes exactly one proposed class with
   readiness; and Stage 1 owns *the semantic fact* of whether a compiled
   contract supports a given executor class, with evidence-backed readiness,
   for whichever class is asked about. Whether ICC stores that fact as an
   exhaustive map, as individual per-class records, or as a single
   proposed-class projection plus additional unexposed characterization is
   **not decided by this candidate and not assumed either way** — see
   "Stage-1 Representation Tension" below. This candidate therefore binds to
   a **representation-neutral Stage-1 support basis**: whatever ICC-owned
   record(s) establish a given class's semantic support and readiness,
   however ICC eventually represents them. It owns only the residual
   judgment of which already-established-supported class is warranted now,
   under current evidence and policy — never which classes are semantically
   supported in the first place, and never a specific storage shape for
   that fact.

---

## Boundary

```text
OWNS
    subject-specific warranted-executor-class judgment
    confidence / unresolved uncertainty
    exact basis binding (contract revision, evidence reference,
        policy reference)
    reopening / applicability semantics

CONSUMES
    the compiled implementation contract, and specifically Stage 1's
        own support basis for whichever executor class is in question --
        the semantic fact of whether that class is supported by the
        contract's own constraints, with evidence-backed readiness
        (implementation-contract-compilation.md §4/§6) -- not merely a
        generic "characterization," not merely ICC's own single
        proposed-class projection of it, and without assuming ICC
        represents that fact as an exhaustive map, individual records, or
        a projection plus additional characterization (representation
        unresolved -- see "Stage-1 Representation Tension")
    execution-profile characterization (a derived representation,
        self-disclaimed as non-owner by its own source)
    capability/outcome evidence                    [owner unresolved]
    routing policy                                 [owner unresolved]

DOES NOT OWN
    the implementation contract itself (implementation-contract-
        compilation.md)
    material decision selection (material-decision-selection.md)
    plan-IR projection-resolution selection (unclaimed elsewhere;
        deliberately not claimed here either — would widen the
        2026-09-15 ruling)
    the implementation-discretion ceiling or its specific grant
        (implementation-contract-compilation.md §4's envelope;
        authority-and-ownership.md §8-9's grant mechanics)
    verification/review-strength selection (review.md's own named,
        still-unresolved successor, adaptive-review-panel-coordination)
    capability-evidence truth (owner unresolved -- not assumed to be
        evidence-and-claims.md)
    routing-policy authorship (owner unresolved)
    executor-class acceptance (an authority consequence -- the
        appropriate authority's own act, never this candidate's)
    concrete runtime realization (runtime-realization.md)
    implementation or review acceptance (review.md +
        evidence-and-claims.md + authority-and-ownership.md +
        mechanisms/revision-cas-and-publication.md's own composition)
```

---

## The Owned Artifact

```yaml
ExecutionStrategySelection:
  subject: <implementation-contract identity + revision digest>
  basis:
    stage1_support_basis:
      characterization_revision: <ICC's own Stage-1 revision this binds to>
      # Representation-neutral: whatever ICC-owned record(s) establish a
      # given class's semantic support and evidence-backed readiness.
      # Whether ICC stores this as an exhaustive map, individual per-class
      # records, or a proposed-class projection plus additional
      # unexposed characterization is not decided here -- see "Stage-1
      # Representation Tension." ESS binds to, and cites, whichever
      # concrete record(s) ICC actually exposes for the class in question.
      class_support_reference: <pointer to ICC's own record(s) establishing
          this specific class's supportedness + readiness>
    execution_profile: <reference -- derived representation, non-owner>
    capability_evidence: <reference -- ownership unresolved>
    routing_policy: <reference -- ownership unresolved>
  warranted_executor_class: <the semantic judgment itself -- must be a
      class whose semantic support is established by
      basis.stage1_support_basis.class_support_reference>
  confidence: ...
  unresolved_uncertainty: ...
  rationale: ...
  advisory_recommendations:
    projection_resolution: <joint-evaluation input, not an owned field>
    execution_autonomy: <joint-evaluation input, not an owned field>
    verification_review_strength: <joint-evaluation input, not an owned field>
  applicability: valid | stale | historical
  reopening_conditions: ...
  predecessor: ...
  producer: ...
  integrity_digest: ...
```

> **Non-duplication invariant.** ESS may not warrant class C unless its
> bound ICC Stage-1 basis establishes C as semantically supported. This
> holds regardless of how ICC represents that establishment — an exhaustive
> map, an individual record, or a proposed-class projection plus additional
> characterization ICC has not yet exposed as a schema field. If C's
> semantic support is not established by whatever Stage-1 basis ESS can
> actually bind to, ESS cannot warrant C — the correct outcome is an ICC
> recharacterization or successor contract characterization, not a
> self-manufactured supportedness fact and not a `returned_for_decision`
> outcome (that outcome is reserved for ICC's own discovery of a newly
> material route choice, a different condition entirely — see
> `material-decision-selection.md` §9).

`warranted_executor_class` names the semantic judgment this candidate
actually owns. It is deliberately not called `selected_executor_class` or
described as authoritative on its own: a warrant is a judgment about what the
evidence and policy basis supports, not yet a consequence. The chain from
judgment to effect stays strictly one-directional and never collapses:

```text
warranted_executor_class          (THIS CANDIDATE's own judgment)
        ↓ projection
routing nomination                 (advisory -- ICC §6's own verbatim
                                     phrase: "Routing remains advisory
                                     until the appropriate authority
                                     accepts it for the slice")
        ↓ authority consequence
accepted_executor_class            (the appropriate authority's own act --
                                     never this candidate's)
        ↓
runtime resolution / admission     (runtime-realization.md §4)
```

The advisory recommendations block is evaluated jointly with the warranted
class (the addendum's own economic coupling is real and worth preserving as
an evaluation practice) but is never itself part of the artifact's
authoritative consequence — each recommendation is only informative input for
its own present-or-pending owner (respectively: an unnamed future
projection-generation step; `authority-and-ownership.md`'s grant machinery,
consuming ICC's own ceiling; `adaptive-review-panel-coordination`, once
built).

---

## Basis Binding, Applicability, and Reopening

An `ExecutionStrategySelection` never mutates. Exact-basis preservation means
it remains a true historical record of what was warranted against exactly
which contract revision, evidence reference, and policy reference — the same
discipline `runtime-realization.md` §6 already holds for `RoleRealization`.

Two distinct classes of change carry two distinct consequences, and must not
be conflated:

```text
CONTRACT-SUBJECT REOPENING
    implementation-contract-compilation.md's own returned_for_decision /
    blocked_by_evidence outcome, or material-decision-selection.md §6's
    decision-set successor reopening every implementation contract that
    relied on the superseded revision
        -> voids the subject itself. The existing
           ExecutionStrategySelection's subject no longer exists in that
           form. A new contract revision requires an entirely new
           judgment, new subject identity.

EVIDENCE / POLICY EVOLUTION
    a capability-evidence or routing-policy revision changes while the
    contract itself remains untouched
        -> does NOT reopen the subject and does NOT void the existing
           judgment. It only makes the judgment's applicability
           re-askable for a *new* attempt against the same still-valid
           contract. Any operation already relying on the existing
           judgment keeps it, per the same reliance discipline
           runtime-realization.md §9 already states for one operation
           executing under one stable realization.
```

A prior `warranted_executor_class` therefore remains historically valid under
its original basis unless its own reopening rule fires — reconsideration is
never automatic on every upstream revision, and invalidation never mints
authority (`authority-and-ownership.md` §12; `runtime-realization.md` §7).

---

## Stage-1 Representation Tension (Unresolved, Not This Candidate's to Settle)

`implementation-contract-compilation.md`'s own top-level Question and §6
body use plural executor-class wording ("which executor classes are
semantically supported, with what evidence-backed readiness?"). Its own §4
artifact schema currently exposes exactly one field: "a proposed executor
class with an evidence-backed readiness assessment." Whether this is a
genuine representation gap, a deliberate design in which ICC evaluates
multiple candidates internally but exposes only its own best proposal plus
readiness for that one class, or something else entirely, is an open
tension this candidate does not resolve and does not need to resolve: this
candidate's own non-duplication rests only on binding to whatever concrete
Stage-1 record(s) ICC actually exposes for a given class, per the
representation-neutral basis above — not on any assumption about ICC's
internal storage shape. Any eventual clarification or correction to ICC §4
belongs to that page, on its own timeline, and is out of scope here.

---

## What Remains Explicitly Unresolved

- **Capability/outcome evidence ownership.** Whether, or how,
  `evidence-and-claims.md` ever hosts this as a domain profile (its own §10
  pattern is a compositional possibility, not a settled claim) is untested by
  this page and not assumed either way.
- **Routing-policy semantic ownership.** Whether the routing policy this
  candidate consumes is a genuinely external standing artifact, or whether
  this candidate dimension is itself the more natural owner of that policy
  (the policy could reduce to the generalized history of this dimension's own
  past judgments), is untested and must be resolved before, or independently
  of, any atlas admission.
- **Plan-IR projection-resolution ownership.** Real gap, no claimed owner
  anywhere, and deliberately not claimed here either, to avoid widening the
  2026-09-15 ruling.
- **Verification/review-strength ownership.** Pending `adaptive-review-panel-
  coordination`'s own eventual resolution — not this candidate's to
  preempt.

---

## What This Candidate Does Not Show

This page does not define:

- the compiled implementation contract itself (`implementation-contract-compilation.md`);
- material decision selection (`material-decision-selection.md`);
- the concrete provider/model/harness/tool realization (`runtime-realization.md`);
- review judgment or review-panel coordination (`review.md`; `adaptive-review-panel-coordination`, unresolved);
- capability-evidence or routing-policy truth (both explicitly unresolved above);
- the authority-grant mechanics that make acceptance, autonomy grants, or admission effective (`authority-and-ownership.md`).

---

## Relationship to Existing Views (if this candidate is ever admitted)

- **`implementation-contract-compilation.md`** — currently states its own
  Stage 2 as "named but not architecturally homed by this dimension" (§6,
  "What This View Does Not Show"). If admitted, this candidate is that home;
  ICC's own text would need the correction, not the reverse.
- **`runtime-realization.md`** — §5 and Key Invariant 11 currently name "the
  supervisor / the routing-policy authority" without a canonical page. Same
  correction, if admitted.
- **`review.md`** — would gain an explicit note that its own "governing
  review contract" input may, in part, be informed by this candidate's
  advisory `verification_review_strength` recommendation, without this
  candidate acquiring any of `review.md`'s or
  `adaptive-review-panel-coordination`'s own authority.
- **`evidence-and-claims.md`** — would gain a relationship note for the
  consumed capability/outcome-evidence basis, contingent on that evidence's
  own ownership question (above) resolving in its favor — not assumed here.
- **`authority-and-ownership.md`** — the acceptance consequence and the
  autonomy-grant consequence both remain this page's own §7–§9 territory;
  this candidate would gain a relationship bullet noting it as the upstream
  producer of the judgment an acceptance act operates on.
- **`app-server/docs/work-engine-planned-architecture.md`** — §4's dimension
  table and §8's routing-gap language would both need updating once (and
  only once) this candidate survives an independent adversarial pass and an
  operator admission decision.
- **`app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md`**,
  **`structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md`**,
  **`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`** —
  each would need a corrected `superseded_by`/relationship-table entry citing
  this candidate as the eventual home for the narrow residue it actually
  covers, once admitted.

None of the edits in this section are made by this page. They are named so
that admission, if it happens, has an exact, pre-identified punch list rather
than a fresh audit.

---

## Authority

This candidate is a specification only. It does not amend the migration
roadmap, the routing.vs.admission ruling, any canonical architecture page, or
any idea document's own status. It does not authorize implementation. It
does not resolve the capability-evidence or routing-policy ownership
questions it names. Admission into the canonical atlas requires a separate,
independent adversarial review pass and an explicit operator decision, both
still pending.
