# Architecture-Direction Seam Map

## Status

Steps 3–4 (integration, seam map) of a bounded architecture-intake session
per `app-server/ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md`,
covering the family of pending ideas named in the intake directive: runtime
ports and capability realization, decision-gated compilation, Plan IR and its
two addenda, evidence-calibrated plan resolution, hierarchical orchestration,
revisioned research, the Candidate Trajectory family and Refactor Pressure,
scoped workflow events, and incremental terminal accounting — integrated
against the already-reconciled service-plane baseline
(`service-plane-inventory.md`, `service-plane-and-kernel-domain-boundary.md`,
`service-plane-reconciliation.md`).

**Authority:** Exploratory only. This document does not accept any
constituent idea, authorize implementation, or freeze the family's
architecture. It resolves only the structure required for coherent proposal
formation later, per the intake method's own §8 boundary.

**Scope note:** `structural-plan-ir-prior-art.md` is background/literature
only, per the intake directive, and is not integrated as an architectural
concern.

## Method

Ideas were integrated incrementally against the accumulated seam map, per the
method's §4, in this order: service-plane trio (already reconciled, treated
as fixed baseline) → ports + companion → decision-gated compilation → Plan IR
+ addenda → evidence-calibrated resolution → hierarchical orchestration →
revisioned research → Candidate Trajectory family + Refactor Pressure →
scoped events → terminal accounting.

Each seam record follows §6's shape. Relations use §5.2's vocabulary
(SUPPLIES, CONSTRAINS, DERIVES, BINDS, INVALIDATES, CORRESPONDS,
SHARES_DECISION, CONFLICTS). Status uses §7.2's states (ALIGNED, TERMINOLOGY,
OWNERSHIP, REPRESENTATION, LIFECYCLE, ORDERING, COUPLED_DECISION,
FUNDAMENTAL_CONFLICT, UNRESOLVED).

This is not exhaustive pairwise coverage of ~20 documents. It covers every
seam found to be load-bearing. Several pairs are noted explicitly as having
no material seam — that is a finding, not a gap.

---

## Seams

```yaml
seam:
  id: ports.companion-binding
  left: {concern: provider-turn-harness-runtime-and-operator-projection, owner: runtime-ports}
  right: {concern: pre-indexed-capability-resolution-and-frozen-runtime-realization, owner: capability-resolution}
  relation: BINDS
  shared_semantics: [ProviderTurnPort, HarnessRuntimePort, OperatorProjection, materialized realization]
  status: ALIGNED
  evidence: >
    Each document names itself as the other's companion. Ports §13: "Pre-Indexed
    Capability Cache... owns observation, policy, admission, invalidation, and
    successor-realization semantics above these ports." Capability-resolution
    §12: "This mechanism complements the independently replaceable ports
    defined in [Provider Turn...]." Confirms the pairing this session's own
    strategic-plan reading already flagged ("ports alone do not own admission").
```

```yaml
seam:
  id: capability.sense-confirmed-aligned
  left: {concern: "ports §6 + capability-resolution capability inventory", owner: runtime-ports/capability-resolution}
  right: {concern: "service-plane capability grant -> service operation split", owner: service-plane}
  relation: CORRESPONDS
  status: ALIGNED
  evidence: >
    Neither ports nor capability-resolution ever uses "capability" as an
    operation-identity dispatch key — every instance (capability inventory,
    capability observation, capability negotiation, required capabilities,
    capability ceilings) is the realization-granted-permission sense.
    `service-plane-reconciliation.md` reserved "capability" for exactly this
    sense. No reconciliation needed; explicitly confirmed rather than assumed.
```

```yaml
seam:
  id: realization.scope-gap
  left: {concern: "capability-resolution materialized realization", owner: capability-resolution}
  right: {concern: "service-plane semantic realization vs. implementation revision", owner: service-plane}
  relation: CORRESPONDS
  status: ALIGNED (partial scope)
  evidence: >
    Both mean "the concrete model/provider/harness binding used to execute
    semantic work" (capability-resolution §5's RoleRealization; service-plane
    §8's semantic realization). Capability-resolution has no concept
    corresponding to service-plane's *implementation revision* (deterministic,
    non-model machinery version, e.g. review-subject's pinned analyzer
    digest) — it is scoped to model/provider/harness realization only. This is
    a legitimate scope boundary, not a conflict: capability-resolution should
    not be expected to own deterministic-service versioning.
```

```yaml
seam:
  id: routing.vs.admission
  left: {concern: "decision-gated compilation §Execution model routing (Sol/Spark)", owner: decision-gated-compilation}
  right: {concern: "capability-resolution §4 Resolution, judgment, and admission", owner: capability-resolution}
  relation: SHARES_DECISION
  shared_semantics: [which concrete executor realizes a role's work]
  status: UNRESOLVED
  invariants:
    - neither document may silently let the other's authority collapse
  question: >
    Is "executor class routing" (Sol vs. Spark, chosen from plan-conformance
    evidence) a decision made *inside* realization admission, or a decision
    made *upstream* of it that admission then materializes? Neither document
    references the other. Decision-gated-compilation's routing profile
    (§Execution model routing, §Stage 6) and capability-resolution's role
    contract + policy overlay + capability inventory (§1, §4) both claim to
    determine "which mechanism does this work," described in incompatible
    vocabularies (routing vs. admission) with no stated composition order.
```

```yaml
seam:
  id: resolution-ladder.vs.projection-profile
  left: {concern: "Plan IR §10 resolution levels P0-P3", owner: structural-plan-ir}
  right: {concern: "evidence-calibrated projection_profile vector", owner: evidence-calibrated-plan-resolution}
  relation: CORRESPONDS
  status: ALIGNED
  evidence: >
    evidence-calibrated-plan-resolution-and-continuous-capability-learning.md
    explicitly states: "The existing P0-P3 levels may remain useful even if
    named presets eventually map to profiles." Evidence-calibrated's
    dimension-specific vector is a proposed generalization of Plan IR's scalar
    ladder, acknowledged by evidence-calibrated itself, not independently
    invented. No TERMINOLOGY conflict — the newer document already declares
    the relationship.
```

```yaml
seam:
  id: characterization.vs.projection-profile
  left: {concern: "execution-profile-scoring / planner-owned-characterization dimensions (intrinsic work difficulty)", owner: structural-plan-ir addenda}
  right: {concern: "evidence-calibrated projection_profile (rendering resolution)", owner: evidence-calibrated-plan-resolution}
  relation: SUPPLIES
  status: ALIGNED (distinct concepts, correctly related)
  evidence: >
    These are NOT the same list and must not be conflated. Execution-profile
    dimensions (semantic_novelty, repository_breadth, decision_closure,
    implementation_discretion, dependency_complexity, oracle_strength,
    reversibility, discovery_burden, integration_consequence) describe
    properties *of the work*. Projection_profile dimensions
    (dependency_structure, repository_localization, authority_boundaries,
    judgment_boundaries, sequencing, verification_obligations,
    acceptance_conditions, integration_contracts, ...) describe *how much
    structure to render* for an executor. The relation is producer/consumer:
    characterization should inform projection-profile selection, not equal it.
    Neither document states this explicitly; it is inferred from their
    definitions and should be made explicit before both are formalized.
```

```yaml
seam:
  id: execution-profile.addenda-duplication
  left: {concern: "structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md dimension list", owner: structural-plan-ir}
  right: {concern: "structural-plan-ir-addendum-planner-owned-execution-characterization.md dimension list", owner: structural-plan-ir}
  relation: CORRESPONDS
  status: TERMINOLOGY
  evidence: >
    Both are addenda to the SAME parent document (Plan IR) and independently
    list near-identical dimensions: semantic novelty, repository breadth,
    implementation discretion, dependency complexity, oracle strength,
    reversibility, discovery burden, and integration consequence appear in
    both, under slightly different names ("decision closure" vs. "unresolved
    material ambiguity"). Planner-owned-characterization presupposes a
    "scoring mechanism" that transforms "characterized properties" — exactly
    what execution-profile-scoring defines — but neither addendum cites the
    other by name. This is redundant, drift-prone specification of the same
    concept within one document family, not a real disagreement. Resolution
    direction: one canonical dimension list, with planner-owned-characterization
    citing it rather than re-deriving a near-duplicate.
```

```yaml
seam:
  id: decision-gated.vs.hierarchical-orchestration
  left: {concern: "decision-gated compilation workflow (decision set -> implementation contract -> builder)", owner: decision-gated-compilation}
  right: {concern: "hierarchical orchestration branch planner -> branch plan -> supervisor -> builder", owner: hierarchical-planning-and-multi-supervisor-orchestration}
  relation: DERIVES
  status: UNRESOLVED
  evidence: >
    Compatible by nesting, not conflicting: decision-gated-compilation's scope
    is explicitly "proposal workflow, claims capability, slice supervisor,
    implementation planner, builder, reviewer, and context lifecycle manager"
    — it says nothing about branches, orchestrators, or preplanners, and
    naturally operates *inside* one of hierarchical-orchestration's branches.
  question: >
    Is hierarchical-orchestration's "branch plan" artifact the same object as
    decision-gated-compilation's "sealed decision set" + "implementation
    contract," or are these three separate sequential artifacts needing an
    explicit correspondence? Neither document states this. Given both
    documents are candidate owners of "the artifact that authorizes bounded
    implementation," this needs an explicit answer before both are
    implemented, not an assumed one.
```

```yaml
seam:
  id: physical-evidence.supplies.plan-deviations
  left: {concern: "review-subject / code-change-profile baseline->candidate diff, AND candidate-trajectory-durable-foundation.md candidate delta (predecessor->successor diff)", owner: "review-subject (baseline->C1); candidate-trajectory (C1->C2->...)"}
  right: {concern: "decision-gated compilation Stage 3 (\"compare the contract with the actual patch\"), plan-deviation classification", owner: decision-gated-compilation}
  relation: SUPPLIES
  status: ALIGNED (previously unstated; corrected on review — see note)
  evidence: >
    Decision-gated-compilation's Stage 3 requires comparing a compiled
    contract against "the actual patch and implementation reasoning" for one
    implementation pass and classifying each divergence (valid discretion /
    mechanical error / stale assumption / omitted decision / unauthorized
    drift / overly prescriptive). An earlier pass at this seam credited
    Candidate Trajectory as "exactly" this mechanism; that overstated its
    scope. For the first implementation candidate, "the actual patch" is
    baseline->C1 — review-subject/code-change-profile's existing deterministic
    diff, not Candidate Trajectory's. Candidate Trajectory's distinctive
    primitive (predecessor->successor deltas: C1->C2, C2->C3, ...) becomes
    the supplying mechanism specifically once remediation begins, where it
    is arguably a stronger fit than the original claim: Stage 3's analysis
    could see not merely the final patch but the exact sequence of
    review-driven revisions (valid local discretion vs. mechanical error vs.
    unauthorized drift are easier to classify against a revision sequence
    than a single collapsed diff). Neither document currently claims either
    half of this SUPPLIES relation; both halves are genuine and previously
    unstated, but they are two different mechanisms for two different phases,
    not one mechanism for the whole stage. Candidate Trajectory does not and
    should not perform the semantic classification (why the divergence
    occurred) either way — that remains decision-gated-compilation's
    judgment, consistent with Candidate Trajectory's own host-owned-facts /
    model-owned-judgment discipline.
```

```yaml
seam:
  id: orchestration-state.converges-on-coordinate
  left: {concern: "hierarchical-orchestration §16 durable state (orchestration-plan revision, branch-plan revisions, dependency state, ...)", owner: hierarchical-planning-and-multi-supervisor-orchestration}
  right: {concern: "revisioned-research coordinate / service-plane service_state map", owner: revisioned-research + service-plane}
  relation: DERIVES
  status: ALIGNED (should be expressed via service_state, not restated)
  evidence: >
    Hierarchical-orchestration §16 enumerates a closed field list for durable
    orchestration state that is structurally identical in purpose to what
    revisioned-research's coordinate and service-plane's open service_state
    map already generalize. This is the THIRD independent convergence on the
    same coordinate pattern this session has found (after
    strategic-reconciliation.mjs's `continuity` vocabulary and the
    claim-evidence/review-episode/campaign-state kernel-primitive family) —
    strong evidence the pattern is real, not merely documented. §16's list
    should eventually be expressed as service_state references
    (`workflow.orchestration_plan: <rev>`, `workflow.branch_plan.A: <rev>`,
    ...) rather than a bespoke enumerated schema, per service-plane §5's own
    argument against closed field lists.
```

```yaml
seam:
  id: event-surface.supplies.service-operation-evidence
  left: {concern: "scoped-workflow-event-surface committed/advisory/telemetry event families", owner: scoped-workflow-event-surface}
  right: {concern: "service-plane ServiceOperation evidence/effect fields", owner: service-plane}
  relation: CORRESPONDS
  status: ALIGNED (previously unstated)
  evidence: >
    A committed ServiceOperation's evidence output, once committed, is exactly
    an instance of scoped-workflow-event-surface's "committed semantic event"
    family ("emitted by the owner as part of, or after, its committed
    transition" — the same authority rule service-plane and claim-evidence
    already enforce). Neither document cites the other (scoped-workflow-event-surface
    predates the service-plane work, dated 2026-09-09). This is a natural
    fit, not a conflict, worth stating explicitly once both are formalized.
```

```yaml
seam:
  id: event-surface.vs.terminal-accounting
  left: {concern: scoped-workflow-event-surface.md, owner: scoped-workflow-event-surface}
  right: {concern: incremental-terminal-accounting-projection.md, owner: incremental-terminal-accounting}
  relation: SHARES_DECISION
  status: COUPLED_DECISION
  evidence: >
    incremental-terminal-accounting's "Ownership shape" (ProviderTurnPort/
    HarnessRuntimePort own observations; campaign/review services own
    semantic classification; accounting projector owns deterministic
    aggregation only) is a specific instance of exactly the general pattern
    scoped-workflow-event-surface proposes (event envelope owned by transport;
    event meaning owned by domain producers). incremental-terminal-accounting's
    own migration sketch step 2 ("Add an append-only accounting-event store
    and deterministic projector") risks building a second, parallel
    event-transport mechanism if not explicitly unified with whatever scoped
    event transport emerges. Neither document is settled (scoped-workflow-event-surface
    is "operator-originated raw architectural idea"; incremental-terminal-accounting
    is "pending post-migration proposal"), but they touch the same mechanism
    and should not be built independently without an explicit decision about
    which owns the underlying append-only/projection infrastructure.
```

```yaml
seam:
  id: candidate-binding.already-named-event
  left: {concern: "candidate-trajectory-durable-foundation.md bindCandidate-time append", owner: candidate-trajectory}
  right: {concern: "scoped-workflow-event-surface committed semantic events", owner: scoped-workflow-event-surface}
  relation: SUPPLIES
  status: ALIGNED
  evidence: >
    scoped-workflow-event-surface's own text lists "candidate binding" verbatim
    as an example of a committed semantic event. Already identified as a
    minor, deliberately-deferred connection in
    candidate-trajectory-upstream-amendments.md §E5; restated here because it
    is directly relevant to this broader family and was independently
    re-confirmed by reading the source document in full.
```

```yaml
seam:
  id: decision-set.fourth-kernel-primitive-instance
  left: {concern: "decision-gated compilation decision-set artifact (identity, revision, sealing, predecessor, integrity digest, authority)", owner: decision-gated-compilation}
  right: {concern: "service-plane repeated kernel-primitive family (claim-evidence ledger, review-episode, slice-campaign state)", owner: service-plane}
  relation: CORRESPONDS
  status: ALIGNED (reinforcing evidence, not yet acted on)
  evidence: >
    decision-gated-compilation's decision-set artifact ("stable decision
    identity and schema version... sealed... predecessor, evidence cutoff,
    integrity digest") and implementation-basis/implementation-contract
    artifacts share the exact shape service-plane-reconciliation.md already
    flagged as a repeated pattern: durable revisioned state + admitted
    transition + optimistic concurrency + authoritative successor identity.
    This is a FOURTH independent instance of that shape, from a document
    family this session had not yet integrated when that pattern was first
    noticed. Strengthens (does not settle) the case that this shape deserves
    eventual kernel-level extraction — still an observation to carry forward,
    per that finding's own explicit caution against designing it
    prospectively.
```

```yaml
seam:
  id: hierarchical-orchestration.defers-realization-correctly
  left: {concern: "hierarchical-orchestration §15 Provider realization", owner: hierarchical-planning-and-multi-supervisor-orchestration}
  right: {concern: "capability-resolution materialized realization", owner: capability-resolution}
  relation: CONSTRAINS
  status: ALIGNED
  evidence: >
    Hierarchical-orchestration explicitly declines to own this: "Provider
    choice does not redefine role semantics or authority. The exact
    provider-routing policy is a replaceable runtime concern." This is a
    correct, positive deferral to capability-resolution's ownership — worth
    recording as a confirmed non-seam rather than leaving implicit.
```

```yaml
seam:
  id: builder-inference-boundary.corresponds
  left: {concern: "service-plane §7 sharper rule for inference (does an admitted service own this fact?)", owner: service-plane}
  right: {concern: "decision-gated compilation builder discretion envelope / plan-deviation stop conditions", owner: decision-gated-compilation}
  relation: CORRESPONDS
  status: ALIGNED
  evidence: >
    Both establish the same discipline independently: a builder should not
    have to reconstruct or infer facts a host can mechanically establish, and
    must stop rather than silently absorb an unresolved material question.
    Decision-gated-compilation's "the builder must stop and publish a
    plan-deviation report... may not reinterpret proposal meaning to make
    tests pass" is a policy-layer instance of the same principle
    Candidate Trajectory's reviewer/builder work and service-plane §7 state
    generally. Confirmed consistent, no reconciliation needed.
```

---

## Explicit no-material-seam findings

- **`structural-plan-ir-prior-art.md`** — background/literature only per
  scope; no architectural seam integrated, per the intake directive.
- **`hierarchical-planning-and-multi-supervisor-orchestration.md` ↔
  `deterministic-refactor-pressure-from-work-engine-evidence.md`** — no
  material seam found. Refactor Pressure's structural-coordinate concept
  operates at the code-region level across slices; hierarchical-orchestration
  operates at the branch/workstream level within one campaign. They may
  eventually share the coordinate/service_state substrate (see
  `orchestration-state.converges-on-coordinate` above) but neither document
  makes a claim about the other today.
- **`evidence-calibrated-plan-resolution-upstream-amendments.md` ↔
  `candidate-trajectory-upstream-amendments.md`** — both already reconcile
  independently against their respective targets (Plan IR family; Refactor
  Pressure and evidence-calibrated resolution itself, respectively — see
  `candidate-trajectory-upstream-amendments.md` §B). No unaddressed seam
  between the two amendment sheets themselves.
- **`scoped-workflow-event-surface.md` ↔ `revisioned-research-and-execution-architecture.md`**
  — no new seam beyond what scoped-workflow-event-surface's own text already
  states (it names revisioned-research directly as supplying "the authority
  and identity model the event surface must preserve"). Confirmed accurate on
  reading revisioned-research directly; not restated as a new finding.

## Reconciliation summary

No `FUNDAMENTAL_CONFLICT` was found across this family, consistent with the
service-plane pass. Distribution: 1 `TERMINOLOGY` (execution-profile addenda
duplication), 3 `UNRESOLVED`/`COUPLED_DECISION` (executor-routing-vs-admission,
branch-plan-vs-decision-set identity, event-transport duplication risk), and
the remainder `ALIGNED` — several of them previously unstated, high-value
`SUPPLIES` relations (physical-change evidence → plan deviations, split
correctly between review-subject for the first candidate and Candidate
Trajectory for remediation sequences; candidate binding → committed event)
rather than mere confirmations.

## Relationships

| Direction | Relationship |
| --- | --- |
| `incremental-architecture-intake-and-seam-reconciliation.md` | This document performs its §5–7 for the broader architecture-direction family, following the same method the service-plane pass already validated. |
| `service-plane-inventory.md`, `service-plane-and-kernel-domain-boundary.md`, `service-plane-reconciliation.md` | Treated as an already-reconciled fixed baseline; not re-litigated here. |
| `architecture-direction-synthesis.md` | The frozen prospective architecture narrative this seam map supports. |

## Non-goals

- Does not decompose this family into implementation proposals (intake
  method §12.4/§13) — that is explicitly deferred, matching the discipline
  the service-plane pass held to.
- Does not resolve any `UNRESOLVED`/`COUPLED_DECISION` seam above. Those
  remain open questions for their respective document owners.
- Does not authorize implementation of anything.
- Is not an exhaustive pairwise seam analysis of all ~20 documents; it
  covers what was found to be load-bearing.
