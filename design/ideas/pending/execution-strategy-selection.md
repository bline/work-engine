# Execution Strategy Selection — Candidate Dimension (ADMITTED)

**Admitted 2026-09-27** as the 14th Work Engine truth dimension, by explicit
operator decision, from this candidate at commit `f566c47`. The canonical
view is now [`app-server/docs/architecture/execution-strategy-selection.md`](../../docs/architecture/execution-strategy-selection.md)
— read that page for the current, canonical statement of this dimension.
This file is retained as the historical candidate record: five original
bounding rounds, one independent adversarial review (`REVISE_CANDIDATE`,
incorporated), two reconciliation rounds settling the predicate boundary and
`RoutingPolicy` ownership, a second independent adversarial review
(`REVISE_CANDIDATE`, four small defects, all repaired), and a final
admission-verification pass (`ACCEPT_CANDIDATE`). Its body below is left
unedited as that record; where it says "candidate," "not yet architecturally
homed," or similar, read that as accurate *at the time this candidate was
reviewed*, not as the current state — the canonical page above is current.

## Identity and state

- Candidate ID: `work-engine.execution-strategy-selection`
- State: **admitted 2026-09-27** — canonicalized as `docs/architecture/execution-strategy-selection.md`; this file is now the historical candidate record, not the current specification
- Decision owner: user or future explicitly authorized architecture owner
- Primary consumers: whichever authority performs the acceptance act on a
  projected routing nomination (advisory until accepted, per
  `implementation-contract-compilation.md` §6), the operator or explicitly
  authorized authority who authors or adopts `RoutingPolicy` revisions
  (Settled Finding B, below), and, downstream of acceptance,
  `runtime-realization.md`'s own stage-3 admission

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/architecture/execution-strategy-selection.md
  superseded_by: app-server/docs/architecture/execution-strategy-selection.md
  status_as_of: 2026-09-27
```

```yaml
idea_provenance:
  origin: reconciliation_synthesis
  predecessor: app-server/ideas/pending/execution-strategy-selection.md @ 2d00d31
  related_reconciliations:
    - app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md
    - app-server/ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md
    - app-server/ideas/pending/structural-plan-ir-addendum-planner-owned-execution-characterization.md
    - app-server/ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md
    - app-server/docs/architecture/implementation-contract-compilation.md
    - app-server/docs/architecture/runtime-realization.md
    - app-server/docs/architecture/role-and-contract-structure.md
    - app-server/docs/architecture/decision-specific-readiness.md
    - app-server/docs/architecture/authority-and-ownership.md
    - app-server/docs/architecture/mechanisms/candidate-resolution-and-admission.md
```

## Revision History

- **Revision 1** (2026-09-26, `2d00d31`) — initial candidate, five bounding
  rounds (§ How This Candidate Was Bounded, items 1–5). Independently
  adversarially reviewed; verdict `REVISE_CANDIDATE`.
- **Revision 2** (2026-09-27, this revision) — incorporates that review's
  findings plus two further falsifier-test reconciliation rounds (items
  6–7, below), settled through direct, session-level discussion rather than
  a formal `*-reconciliation.md` document of their own — the same
  evidentiary bar `authority-and-ownership.md` §12 and `mechanisms/
  candidate-resolution-and-admission.md` were each held to when first
  named. Independent adversarial review of Revision 2 itself has not yet
  occurred — it is a successor produced *from* Revision 1's review, not a
  revision that has itself been reviewed. Settles:
  - **Finding A — predicate boundary.** `Supported(C,X)` (ICC's own Stage-1
    fact) is necessary but never sufficient for `Warranted(C,X|E,P)` (this
    candidate's own Stage-2 judgment). The boundary is a **revision-cadence**
    distinction, not an evidence-coarseness one: the same underlying
    observations may legitimately inform both. `N=1` (ICC's current
    single-proposed-class representation) leaves this candidate non-vacuous;
    `N>1` only adds comparative selection on top of an already-real judgment.
  - **Finding B — `RoutingPolicy` ownership.** Following `runtime-
    realization.md`'s own already-accepted Operator Runtime Policy Overlay
    precedent exactly — that page's own Purpose line states it owns "the
    concrete policy content being projected" — this candidate owns *what
    `RoutingPolicy@P` says* once an authorized revision is adopted (the
    content as domain state), together with its identity, lineage, schema,
    and application semantics. The appropriate operator or explicitly
    authorized authority owns a different thing: the decision to make `P`
    say that. This candidate hosts and applies adopted content; it never
    exercises the authority to choose that content.

  Also fixes, independent of either finding: an honest three-outcome
  disposition grammar (previously missing the case where supported
  candidates exist but none is currently warranted); consistent citation of
  `execution_profile` everywhere this candidate's exact basis is described;
  historical (never voided) preservation of a superseded contract-subject's
  prior judgment; demotion of ICC's own Stage-1 representation tension to an
  independent upstream question, not an admission prerequisite for this
  candidate; and a round-count inconsistency between this document's own
  intro paragraph and its enumerated bounding list.

  Still candidate status throughout: not accepted, not authorized for
  implementation, not part of the canonical atlas. Independent adversarial
  review of this revision and an explicit operator admission decision remain
  the two gates before any canonical-atlas edit — neither has occurred.

---

Not a source-document transcription like most files in this directory. This
page is a fresh synthesis, produced through five rounds of falsifier/grain
testing at first commit (2026-09-26) against the `routing.vs.admission`
ruling's own named-but-architecturally-unhomed Stage 2
(`implementation-contract-compilation.md` §6; `runtime-realization.md` §5),
plus two further adversarial reconciliation rounds for this revision
(2026-09-27) — seven rounds in total, enumerated in full below. It is
recorded here, in `ideas/pending/`, rather than in `docs/architecture/`,
precisely because independent adversarial review of *this* revision and an
operator admission decision are both still pending.

---

## Candidate question

> Given a bounded implementation contract, its Stage-1
> (`implementation-contract-compilation.md`) characterization, revision-bound
> capability/outcome evidence, and an applicable `RoutingPolicy` revision,
> which of the semantically supported executor classes — if any — is
> warranted for this subject now?

`Supported(C,X)` — ICC's own Stage-1 fact, established and revision-bound at
Stage 1 — is **necessary but never sufficient** for `Warranted(C,X|E,P)`, this candidate's
own, independently revisioned Stage-2 judgment (Settled Finding A). This
candidate owns exactly the residual judgment of whether an already-supported
class remains the right one **now**, never whether a class is supported in
the first place.

This question is held deliberately narrow. Five rounds of falsifier testing
at first commit progressively removed content that looked plausible at first
pass but either duplicated an existing owner or would have silently widened
the already-settled 2026-09-15 `routing.vs.admission` ruling, which names
Stage 2 specifically as **executor-class routing/acceptance** and nothing
broader. Two further reconciliation rounds in this revision (items 6–7,
below) sharpened this candidate's own boundary against ICC and settled its
own ownership of the `RoutingPolicy` artifact — neither round widened this
section's own scope.

---

## How This Candidate Was Bounded

**Original bounding, five rounds (2026-09-26), each removing content the
prior pass had wrongly folded in:**

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
   respectively). Neither was actually settled at the time: `evidence-and-
   claims.md`'s own §10 profile pattern is, per the evidence-calibrated
   document's own corrected finding, "a real but compositional relationship,
   not a claim already stated" — still true, still carried as an open
   dependency (§ What Remains Explicitly Unresolved). Routing-policy
   authorship *was* genuinely unresolved at first commit; Settled Finding B
   (item 7, below) resolves it for this revision.
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
   tradeoff — the addendum's own joint economic framing survives intact as
   this candidate's own internal evaluation method. It merely keeps
   ownership where it already sits, or is already named to eventually sit.
   Joint evaluation is not joint ownership.

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
   "Stage-1 Representation Tension" below, now demoted to an independent
   upstream question rather than a precondition for *this* candidate (item
   6, below resolves why). This candidate therefore binds to a
   **representation-neutral Stage-1 support basis**: whatever ICC-owned
   record(s) establish a given class's semantic support and readiness,
   however ICC eventually represents them. It owns only the residual
   judgment of which already-established-supported class is warranted now,
   under current evidence and policy — never which classes are semantically
   supported in the first place, and never a specific storage shape for
   that fact.

**Revision 2 reconciliation, two further rounds (2026-09-27):**

6. **Predicate boundary reconciliation.** An adversarial pressure test asked
   directly whether `Supported(C,X)` (ICC) and `Warranted(C,X|E,P)` (this
   candidate) name the same fact under different words — the primary
   falsifier any admission review of this candidate must clear. Traced
   against the original `routing.vs.admission` ruling's own two-ingredient
   Stage 6 text ("allow the supervisor to nominate an executor class from
   contract characteristics **and historical outcomes**"), both stages'
   consulted evidence, and a working counterexample — a class can remain
   `Supported=true` (established and revision-bound to an exact ICC
   Stage-1 characterization revision — this candidate does not decide
   whether that establishment happens literally the instant contract bytes
   are emitted or somewhere within ICC's own compiler/plan-conformance
   boundary; that sequencing is ICC's own unresolved internal question, not
   this candidate's to settle) while later becoming `Warranted=false` under
   a later capability-evidence or `RoutingPolicy` revision, with no
   contradiction, because `Supported` is bound to one ICC Stage-1
   characterization revision while `Warranted` is independently re-askable
   under later `E`/`P` revisions — the distinction was confirmed
   real and settled **necessary-but-not-sufficient**. Critically, the
   boundary is **not** that Stage 1 consumes coarse evidence and Stage 2
   consumes exact evidence: the same underlying observation may legitimately
   ground both facts, for different reasons, at different times — the
   operative distinction is **revision cadence**, the same
   freshness-independent-of-evaluation pattern `decision-specific-
   readiness.md`'s own Key Invariant 3 ("a freshness change can invalidate
   a readiness assessment without evaluation itself re-running") already
   establishes for its own dimension against `proposal-evaluation.md`.
   This also confirmed the candidate is non-vacuous
   even while ICC exposes exactly one proposed class (`N=1`): "is it *still*
   warranted *now*" is a real, independently re-askable question regardless
   of how many classes ICC currently exposes. `N>1` (once ICC's own
   representation exposes it) adds comparative selection on top of an
   already-real judgment — a difference of richness, not of kind.
7. **`RoutingPolicy` ownership reconciliation.** A second pressure test
   asked who authors and owns the standing policy `Warranted` is evaluated
   against, explicitly refusing to accept the superficially attractive
   `decision-specific-readiness.md` "contract + assessment" analogy without
   falsifying it first — that dimension's own `ReadinessContract` is a
   small, largely static, decision-type-keyed taxonomy, not a continuously
   revised, preference-bearing policy, and is the wrong-shaped precedent.
   The correct, closer precedent is `runtime-realization.md`'s own already-
   accepted Operator Runtime Policy Overlay: that dimension explicitly owns
   a policy overlay at its own Stage-3 grain (its own Purpose line: "This
   dimension owns capability observation, **operator runtime policy**,
   resolution of concrete runtime composition, the immutable `RoleRealization`
   artifact itself..." — and, precisely on point, its own §11 states it
   "owns the concrete policy content being projected; the mechanism owns
   the discipline that keeps the projection from becoming a second
   resolver"), and its own Key Invariant 10 ("a manipulable control
   surface... not canonical workflow or runtime truth") and Key Invariant
   13 (a confirmed, partial instance of `mechanisms/authority-preserving-
   intent-projection.md`) establish that owning that content as domain
   state never means holding the *authority* to choose it. Following that
   precedent exactly rather than inventing a new shape: this candidate owns
   *what `RoutingPolicy@P` says* once an authorized revision is adopted —
   the content as domain state, together with its identity, lineage,
   schema, and application to Stage-2 candidates. The appropriate operator
   or explicitly authorized authority owns a distinct thing: the decision
   to make `P` say that — the authoring or adoption act itself. This
   candidate hosts and applies adopted content; it does not exercise the
   authority to choose it, and stating the boundary as "never owns the
   content" would overstate the gap and risk making the artifact look
   ownerless once a revision is actually adopted — it does not; it becomes
   this candidate's own domain state at that point, exactly as `runtime-
   realization.md`'s overlay content becomes that dimension's own domain
   state once an operator sets it. Three adjacent mechanisms were
   checked directly and confirmed not to absorb any part of this: `mechanisms/
   authority-preserving-intent-projection.md` owns only the projection/edit
   discipline through which authored content reaches the artifact, never
   policy meaning; `mechanisms/candidate-resolution-and-admission.md` owns
   only the generic 0/1/N reduction this candidate's own residual
   class-selection step reuses when ICC exposes `N>1`, never routing
   meaning; `evidence-and-claims.md` owns capability-evidence and freshness
   mechanics, never routing policy. Exploration-versus-exploitation
   governance is deliberately left **outside** this candidate's own scope:
   the evidence-calibrated source's own text treats it as "one possible
   mechanism," not a settled Stage-2 responsibility, and claiming it here
   would assert more than that source itself does.

---

## Boundary

```text
OWNS
    subject-specific warranted-executor-class judgment
    disposition grammar (warranted / no_candidate_warranted /
        insufficient_basis -- see "Disposition: Three Honest
        Outcomes" below)
    confidence / unresolved uncertainty
    exact basis binding (contract revision, Stage-1 support reference,
        execution-profile reference, capability-evidence reference,
        RoutingPolicy revision)
    reopening / applicability semantics, including historical
        preservation of a superseded subject's prior judgment (never
        deletion or retroactive rewriting)
    the RoutingPolicy domain artifact -- what an adopted revision says
        (content as domain state), identity, lineage, schema, revision
        semantics, scoping, and application to Stage-2 candidates
        (Settled Finding B) -- never the authority to choose that
        content, which remains the operator's or an explicitly
        authorized authority's own decision

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
        unresolved -- see "Stage-1 Representation Tension," now an
        independent upstream question, not an admission prerequisite for
        this candidate -- Settled Finding A)
    execution-profile characterization (a derived representation,
        self-disclaimed as non-owner by its own source; bound as part of
        this candidate's own exact basis wherever that basis is recorded)
    capability/outcome evidence                    [owner unresolved]

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
    the authority to choose RoutingPolicy content (the operator's or
        explicitly authorized authority's own decision act -- this
        candidate owns the artifact, what an adopted revision says as
        domain state, revision semantics, and application mechanics,
        but never the authoring/adoption decision itself -- Settled
        Finding B)
    the projection/edit discipline through which policy content is
        authored or adopted (mechanisms/authority-preserving-intent-
        projection.md)
    the generic 0/1/N candidate-reduction mechanic itself
        (mechanisms/candidate-resolution-and-admission.md) -- reused
        for residual class selection when ICC exposes N>1, never owned
    exploration-versus-exploitation governance (deliberately left
        outside this candidate's scope pending separate establishment
        -- Settled Finding B)
    executor-class acceptance (an authority consequence -- the
        appropriate authority's own act, never this candidate's)
    concrete runtime realization (runtime-realization.md)
    implementation or review acceptance (review.md +
        evidence-and-claims.md + authority-and-ownership.md +
        mechanisms/revision-cas-and-publication.md's own composition)
```

---

## Disposition: Three Honest Outcomes

Mirroring `implementation-contract-compilation.md` §3's own
`compiled`/`returned_for_decision`/`blocked_by_evidence` honesty, this
candidate's own judgment step returns exactly one of:

```text
warranted                 a specific already-ICC-supported executor
                           class is warranted for this subject now,
                           under the exact capability-evidence and
                           RoutingPolicy revision bound to the judgment.
                           warranted_executor_class is populated.

no_candidate_warranted    one or more classes are ICC-supported for
                           this subject, but current evidence and
                           policy jointly warrant none of them -- a
                           real, evaluated negative, never silently
                           defaulted, hidden, or resolved by picking
                           the least-bad candidate. Returns to the
                           authorized routing authority for
                           disposition; never blocks, reopens, or
                           second-guesses ICC's own Stage-1 fact.

insufficient_basis        one or more required judgment inputs cannot
                           truthfully support a warrant -- capability
                           evidence unavailable/stale/contested,
                           RoutingPolicy missing/unresolved/inapplicable,
                           the required Stage-1 support reference
                           unavailable, or the required execution-profile
                           basis unavailable. A RoutingPolicy is not
                           itself evidence, so this outcome is named for
                           the basis as a whole rather than folded under
                           an evidence-only label; distinct from
                           no_candidate_warranted, which is a real,
                           fully-evidenced, fully-policy-bound negative,
                           not a gap in the basis itself.
```

This is the case revision 1 omitted entirely: a subject with genuinely
supported candidates for which none currently clears policy or evidence
requirements is a real, nameable outcome, not an absence this candidate
should paper over by warranting the least objectionable class anyway.

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
      # Representation Tension," an independent upstream question, not a
      # precondition for this candidate (Settled Finding A). ESS binds to,
      # and cites, whichever concrete record(s) ICC actually exposes for
      # the class in question.
      class_support_reference: <pointer to ICC's own record(s) establishing
          this specific class's supportedness + readiness>
    execution_profile: <reference -- derived representation, non-owner;
        bound here as part of this candidate's own exact basis>
    capability_evidence: <reference -- ownership unresolved>
    routing_policy_revision: <exact RoutingPolicy artifact revision (see
        below) this judgment was evaluated against -- owned by this
        candidate; content authored by the operator or an explicitly
        authorized authority, never by this candidate itself>
  disposition: warranted | no_candidate_warranted | insufficient_basis
  warranted_executor_class: <populated only when disposition = warranted;
      must be a class whose semantic support is established by
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

```yaml
RoutingPolicy:
  # This candidate's own domain artifact, per Settled Finding B, following
  # runtime-realization.md's own Operator Runtime Policy Overlay precedent
  # exactly (that page's own §11: "owns the concrete policy content being
  # projected"). This candidate owns what an adopted revision says (content
  # as domain state), together with identity, lineage, schema, revision
  # semantics, scoping, and application mechanics below. It does not
  # exercise the authority to choose that content -- the decision to make a
  # revision say what it says belongs to the operator or an explicitly
  # authorized authority, exercised through mechanisms/authority-preserving-
  # intent-projection.md's own projection/edit discipline.
  revision: <content-digest identity, predecessor-chained -- same
      Revision/CAS shape as every other durable artifact in this
      architecture; mechanisms/revision-cas-and-publication.md>
  scope: <global | campaign | role-class | subject-class | one-slice --
      the same hierarchical scoping runtime-realization.md §11's own
      overlay already uses; a narrower scope may override a broader
      preference, never a role contract, an ICC-established support fact,
      or an authority boundary>
  class_exclusions: [...]        # administrative prohibitions, independent
                                  # of ICC's own Supported(C,X) fact -- see
                                  # "Distinguishing Policy Exclusion From
                                  # ICC's Own Negative," below
  class_preferences: [...]       # ranking among supported classes
  cost_objective: <reference or weighting, if authored -- narrowly scoped
      to whatever the operator/authority actually specifies; this
      candidate does not itself define an economic model>
  confidence_thresholds: {...}
  freshness_requirements: {...}  # this field and its shape are this
                                  # candidate's own schema (decision-
                                  # specific-readiness.md §4's own
                                  # precedent for this exact split); the
                                  # actual threshold value in any adopted
                                  # revision is authored/adopted content,
                                  # never set by this candidate itself; the
                                  # freshness mechanism itself remains
                                  # evidence-and-claims.md's own
  severe_failure_treatment: {...}  # this field and its shape (e.g.
                                  # "exclude a class with an unresolved
                                  # severe-failure flag") are this
                                  # candidate's own schema; the actual rule
                                  # in any adopted revision is authored/
                                  # adopted content, never set by this
                                  # candidate itself; preservation of the
                                  # underlying evidence distinction remains
                                  # evidence-and-claims.md's own
                                  # representation concern
  escalation_triggers: [...]     # systemic policy-level triggers, distinct
                                  # from a single judgment's own subject-
                                  # specific reopening_conditions
  pins: [...]
  class_limitations: [...]
  authored_by: <the operator or explicitly authorized authority who
      authored or adopted this revision -- never this candidate's own
      producer identity>
  predecessor: ...
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
> `implementation-contract-compilation.md` §8, which mirrors
> `material-decision-selection.md` §9's own invariant from this dimension's
> own side). Symmetrically, per Settled Finding A, `Supported=true` alone
> never obligates `warranted`: `disposition = no_candidate_warranted` is the
> correct, honest outcome when every supported class fails current evidence
> or policy — never a silent default to whichever supported class looks
> least objectionable.

### Distinguishing Policy Exclusion From ICC's Own Negative

`RoutingPolicy`'s own `class_exclusions` must never become a second name for
`Supported(C,X) = false`. ICC's negative is a **structural/mechanical
incapability fact**, established and revision-bound to an exact ICC Stage-1
characterization revision, carrying no preference or administrative
content. A policy exclusion is an **administrative/preference prohibition**
that can hold even when `Supported(C,X) = true` — an operator may exclude an
otherwise fully capable class from security-adjacent work for compliance
reasons unrelated to whether that class can technically do the work. These
answer different questions, about different things, on different revision
cadences, and must never be collapsed into one field or one negative.

`warranted_executor_class` names the semantic judgment this candidate
actually owns. It is deliberately not called `selected_executor_class` or
described as authoritative on its own: a warrant is a judgment about what the
evidence and policy basis supports, not yet a consequence. The chain from
judgment to effect stays strictly one-directional and never collapses:

```text
warranted_executor_class          (THIS CANDIDATE's own judgment, produced
                                    against its own RoutingPolicy revision)
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
which contract revision, Stage-1 support reference, execution-profile
reference, capability-evidence reference, and `RoutingPolicy` revision — the
same discipline `runtime-realization.md` §6 already holds for
`RoleRealization`.

Two distinct classes of change carry two distinct consequences, and must not
be conflated:

```text
CONTRACT-SUBJECT REOPENING
    implementation-contract-compilation.md's own returned_for_decision /
    blocked_by_evidence outcome, or material-decision-selection.md §6's
    decision-set successor reopening every implementation contract that
    relied on the superseded revision
        -> the prior ExecutionStrategySelection's subject (the exact
           contract revision it bound to) is superseded, never deleted
           and never retroactively rewritten. Its own applicability
           transitions to `historical` -- it remains a true, permanently
           inspectable record of what was warranted, against exactly
           which contract revision, evidence reference, and
           RoutingPolicy revision, at the time it was produced. A new
           contract revision requires an entirely new judgment against a
           new subject identity; the prior judgment's historical truth
           is preserved, not erased -- the same discipline every other
           Revision/CAS instance in this architecture already holds
           (mechanisms/revision-cas-and-publication.md), and the same
           correction "invalidation never mints authority"
           (authority-and-ownership.md §12) already requires: a
           superseded subject losing current applicability is not
           license to treat its own historical truth as if it had never
           existed.

EVIDENCE / POLICY EVOLUTION
    a capability-evidence reference changes, or a new RoutingPolicy
    revision is authored or adopted, while the contract itself remains
    untouched
        -> does NOT reopen the subject and does NOT void the existing
           judgment. It marks the existing judgment's own applicability
           `stale` (never `historical` -- that transition is reserved
           for CONTRACT-SUBJECT REOPENING, above), and makes the
           judgment re-askable for a *new* attempt against the same
           still-valid contract. Any operation already relying on the
           existing judgment keeps it, per the same reliance discipline
           runtime-realization.md §9 already states for one operation
           executing under one stable realization.
```

To state the full `applicability` enum plainly, since the artifact schema
declares it but no single passage maps all three values together: `valid`
(the judgment's basis is current), `stale` (evidence or policy has moved
since the judgment was produced, but the contract subject itself has not —
re-askable, not void), and `historical` (the contract subject itself was
superseded — the judgment is permanently preserved as a true record, never
current again).

A prior `warranted_executor_class` therefore remains historically valid under
its original basis unless its own reopening rule fires — reconsideration is
never automatic on every upstream revision, and invalidation never mints
authority (`authority-and-ownership.md` §12; `runtime-realization.md` §7).

---

## Stage-1 Representation Tension (ICC's Own Open Question — Not an Admission Prerequisite for This Candidate)

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
internal storage shape.

**Settled Finding A (§ How This Candidate Was Bounded, item 6) already
confirms this candidate's own predicate is well-defined independent of how
ICC eventually resolves this tension:** `Warranted(C,X|E,P)` remains real and
non-vacuous whether ICC currently exposes one class or eventually exposes
many. Resolving ICC's own representation gap may *enrich* this candidate's
own candidate space — turning a binary warrant-or-return-for-recharacterization
judgment (today's `N=1` reality) into a genuine comparative selection among
several supported classes (`N>1`) — but it is not a precondition for this
candidate's own semantic soundness, its own admission, or its own
implementation readiness. Any eventual clarification or correction to ICC §4
belongs to that page, on its own timeline, and is out of scope here.

---

## What Remains Explicitly Unresolved

- **Capability/outcome evidence ownership.** Whether, or how,
  `evidence-and-claims.md` ever hosts this as a domain profile (its own §10
  pattern is a compositional possibility, not a settled claim) is untested by
  this page and not assumed either way.
- **Plan-IR projection-resolution ownership.** Real gap, no claimed owner
  anywhere, and deliberately not claimed here either, to avoid widening the
  2026-09-15 ruling.
- **Verification/review-strength ownership.** Pending `adaptive-review-panel-
  coordination`'s own eventual resolution — not this candidate's to
  preempt.
- **Exploration-versus-exploitation governance.** Deliberately excluded from
  this candidate's own scope (Settled Finding B, item 7). The evidence-
  calibrated source's own text treats a separately governed exploration
  allocation as "one possible mechanism," never a settled Stage-2
  responsibility — folding it into `RoutingPolicy` would assert more than
  that source itself does. May need its own future mechanism; not decided
  here.

Resolved by this revision, no longer listed here as open: **routing-policy
semantic ownership** (Settled Finding B — this candidate owns the
`RoutingPolicy` artifact, never its content authorship) and the **Stage-1/
Stage-2 predicate boundary** (Settled Finding A).

---

## What This Candidate Does Not Show

This page does not define:

- the compiled implementation contract itself (`implementation-contract-compilation.md`);
- material decision selection (`material-decision-selection.md`);
- the concrete provider/model/harness/tool realization (`runtime-realization.md`);
- review judgment or review-panel coordination (`review.md`; `adaptive-review-panel-coordination`, unresolved);
- capability-evidence truth (explicitly unresolved above);
- the authority-grant mechanics that make acceptance, autonomy grants, or admission effective (`authority-and-ownership.md`);
- the projection/edit mechanics that let an operator author or adopt a `RoutingPolicy` revision (`mechanisms/authority-preserving-intent-projection.md`);
- the generic candidate-reduction mechanic this candidate's own residual class-selection step reuses (`mechanisms/candidate-resolution-and-admission.md`).

---

## Relationship to Existing Views (if this candidate is ever admitted)

- **`implementation-contract-compilation.md`** — currently states its own
  Stage 2 as "named but not architecturally homed by this dimension" (§6,
  "What This View Does Not Show"). If admitted, this candidate is that home;
  ICC's own text would need the correction, not the reverse. Settled Finding
  A also gives ICC's own §5 plan-conformance gate criterion — "the selected
  executor class is supported by prior evidence for comparable plan entropy
  and task shape" — a precise, non-overlapping reading: a coarse, one-shot
  plausibility check, established and revision-bound at Stage 1, never
  itself a preference or currency judgment.
- **`runtime-realization.md`** — §5 and Key Invariant 11 currently name "the
  supervisor / the routing-policy authority" without a canonical page. Same
  correction, if admitted. This candidate's own `RoutingPolicy` ownership
  (Settled Finding B) is modeled directly on this page's own Operator
  Runtime Policy Overlay and Key Invariants 10/13 — the precedent this
  candidate follows, not a parallel invention.
- **`role-and-contract-structure.md`** — §6 ("The Executor-Class-Routing
  Boundary — Closed by Ruling, Not This Dimension's to Reopen") and Key
  Invariant 5 independently name the same unhomed Stage-2 seam ICC §6 and
  RR §5 name: "a contract requirement naming an executor class is not the
  same decision as routing to that class now." Same correction, if
  admitted — a fourth canonical page, not previously listed here, whose own
  text would need the identical update ICC's and RR's own text would need.
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
  producer of the judgment an acceptance act operates on, and as the
  authority whose delegation modes govern who may author a `RoutingPolicy`
  revision.
- **`mechanisms/authority-preserving-intent-projection.md`** — owns the
  projection/edit discipline through which an operator's authored
  `RoutingPolicy` content reaches this candidate's own artifact; this
  candidate never acquires that discipline itself, the same relationship
  `runtime-realization.md` §11 already holds with its own overlay.
- **`mechanisms/candidate-resolution-and-admission.md`** — this candidate's
  own residual class-selection step, once ICC exposes `N>1` supported
  classes, reuses this mechanism's generic 0/1/N reduction; the mechanism
  supplies no routing meaning of its own, the same relationship `runtime-
  realization.md` §4 and `material-decision-selection.md` §7 already hold
  with it.
- **`app-server/docs/work-engine-planned-architecture.md`** — §4's dimension
  table and §8's routing-gap language would both need updating once (and
  only once) this candidate survives an independent adversarial pass and an
  operator admission decision.
- **`app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md`**,
  **`structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md`**,
  **`structural-plan-ir-addendum-planner-owned-execution-characterization.md`**,
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
does not resolve the capability-evidence ownership question it names, and it
deliberately excludes exploration-versus-exploitation governance from its own
scope. Revision 1 was independently adversarially reviewed (verdict
`REVISE_CANDIDATE`); Revision 2 incorporates that review's findings plus two
further falsifier-test reconciliation rounds (2026-09-26/27, enumerated
above), but has not itself been independently adversarially reviewed, and
has not been through canonical admission. Both remain separate, still-pending
gates.
