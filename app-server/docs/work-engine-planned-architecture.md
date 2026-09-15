# Work Engine: Planned Architecture

## Status

This document assembles the settled and explicitly unresolved facts produced
by this session's work into one coherent planned architecture: the earlier
broader-family synthesis
([`architecture-direction-synthesis.md`](architecture-direction-synthesis.md),
[`architecture-direction-seam-map.md`](architecture-direction-seam-map.md)),
the service-plane baseline
([`service-plane-inventory.md`](service-plane-inventory.md),
[`service-plane-reconciliation.md`](service-plane-reconciliation.md)), and
the twelve-item, four-wave sequel reconciliation queue that followed
(thirteen idea files, twelve of which were previously "exploratory" and one
of which — `agent-instruction-structure-and-placement-review` — was found
already built and dogfooded).

**Methodological rule this document holds itself to:** it contains no new
architectural decisions. Where assembling the reconciled sources would
require inventing connective tissue two source documents do not themselves
state, this document says so explicitly in §13 rather than filling the gap
with prose. A new reader should be able to understand the intended machine
without knowing how it was discovered — but nothing here is authoritative
merely because it is written down here. Every claim traces to a specific
reconciliation document, an accepted design, or real implementation
evidence, named at the point of use.

**Authority:** Exploratory synthesis only. This document does not accept any
idea, authorize implementation, amend the migration roadmap, or select a
permanent schema. §12 states, subsystem by subsystem, what already has
production evidence versus what remains a document.

## 1. Purpose and thesis

> Work Engine is a governed execution substrate for model-mediated semantic
> work. It externalizes mechanically knowable state, preserves the authority
> and consequences of semantic judgment, and reconstructs the smallest
> sufficient world for each bounded decision.

The principles below are not new — each is a restatement of a rule this
session found independently, repeatedly, and by name across unrelated
documents and reconciliations:

```text
Contracts constrain what must remain true.
Models choose how to make it true.
    -- service-plane's deterministic/semantic/hybrid split; claim-evidence's
       "a role remains responsible for its domain judgment; the claims
       service owns the mechanical preservation... consequences of that
       judgment"

Define the space, not the solution.
Expose the machine, not the route.
    -- agent-instruction-structure-and-placement-review's two-question
       diagnostic; Plan IR's resolution-level ladder; decision-gated
       compilation's mechanically-constrained-or-bounded-discretion split

Nothing derived becomes authoritative merely by being derived.
    -- evidence-anchor's may_affect ("mechanical suspicion is never
       authority to declare something false"); claim-evidence's
       may_affect/refresh-episode split; every reconciliation's own
       repeated correction of "X resembles Y, therefore X retires to Y"

Nothing owned is reconstructed instead of read.
    -- incremental-terminal-accounting-projection's incremental-projection
       rule; control-plane-causal-observability-ui's "join existing owners
       by immutable or revision-bound references" table

Mechanically knowable work belongs in deterministic machinery.
Inference is reserved for unresolved semantic consequence.
    -- service-plane's determinism classes; claim-evidence's deterministic-
       rules vs. semantic-candidate-compiler split; the mechanical/semantic
       split this session applied to evidence-anchor, seam review, and
       closed-loop calibration alike
```

## 2. The architectural model

The primary map, reconciled and unchanged from the pre-queue synthesis, is
two orthogonal axes, not a layered stack:

```text
three responsibility planes                four functional systems
(who owns a piece of responsibility)        (what part of governed
                                              execution is being implemented)

    KERNEL                                      REALIZATION
    SERVICE            x                        PLANNING / COMPILATION
    WORKFLOW                                     ORCHESTRATION
                                                  EVIDENCE / HISTORY
```

```text
                       FUNCTION

              Realization   Planning/     Orchestration   Evidence
                            Compilation                   /History
          +--------------------------------------------------------
KERNEL    | workspace-      --              --              --
          | coordination
SERVICE   | RoleRealization  Plan IR,        durable          claim-evidence,
          | ports            decision-       branch/plan      evidence-anchor,
          |                  gated           state            review-finding
          |                  compilation
WORKFLOW  | OperatorProjection Structural    orchestrator/    reviewer/
          |                    Plan IR       supervisor       operator
          |                    consumers     topology         judgment
          +--------------------------------------------------------

DOMAIN (orthogonal to both axes):
    software engineering (code, review, Candidate Trajectory)
    browser / UI (AI-Accessible Browser, UI Experience Evidence, UI review)
    proposal/portfolio (research maturity, evaluation, portfolio selection)
    research (revisioned coordinates, capability learning)
    review (seam review, architectural review, agent-instruction review,
        UI review — one ownership topology, many concerns)
    future domains
```

A subsystem has a *location* across these dimensions rather than belonging
to one monolithic layer. The reconciliation queue added an entire domain
(proposal/portfolio decision-making) and an entire ownership topology
(review) without requiring a third axis — both compose onto the existing
grid.

## 3. End-to-end execution story

Before the components, what happens to one piece of work, with every arrow
resolved to a named owner (or explicitly marked unresolved):

```text
idea (ideas/, root substrate)
    |
    v
intake / proposal formation
    -> product-development's delivery services (mechanical packet
       validation only, real)
    |
    v
proposal research maturity / evidence state
    -> claim-evidence-service.md's proposal-research-v1 profile (real,
       implemented); decision-specific readiness contract UNRESOLVED
       (`proposal-research-maturity-and-freshness-reconciliation.md`)
    |
    v
proposal evaluation
    -> typed evaluation estimates (claim-evidence substrate, real);
       comparison-contract/dominance mechanism UNRESOLVED
       (`evidence-backed-proposal-evaluation-reconciliation.md`)
    |
    v
portfolio / strategic selection
    -> proposal packets + typed relationships (real); PortfolioDecision
       record UNRESOLVED (`proposal-backed-portfolio-selection-reconciliation.md`)
    |
    v
planning (Plan IR / evidence-calibrated resolution)
    -> AUTHORIZED_DIRECTION, not built (§12)
    |
    v
material decision closure (sealed decision set)
    -> decision-gated-implementation-compilation.md, AUTHORIZED_DIRECTION
    |
    v
implementation compilation (implementation contract)
    -> same document; Plan IR is its candidate representation, not a
       competing owner
    |
    v
organizational execution envelope
    -> `organizational-execution-envelopes.md`'s ExecutionEnvelope; the
       compiler itself is UNRESOLVED — the queue's one genuinely new
       architectural survivor (`organizational-execution-envelopes-reconciliation.md`).
       NOTE: reconciliation established what feeds this stage and what it
       produces (§5), not its temporal position relative to implementation
       compilation — shown here for narrative continuity across one piece
       of work, not as a decided compile order.
    |
    v
runtime realization + admission
    -> RoleRealization (`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`,
       AUTHORIZED_DIRECTION); role-active-binding fencing UNRESOLVED
       (`control-plane-and-client-protocol-reconciliation.md`)
    |
    v
execution
    -> ProviderTurnPort / HarnessRuntimePort (AUTHORIZED_DIRECTION);
       workspace-coordination fencing/CAS (IMPLEMENTED)
    |
    v
review / verification
    -> review-finding-bridge.mjs (IMPLEMENTED, one domain profile);
       agent-instruction-review (IMPLEMENTED, fully dogfooded); seam
       review / architectural review specialists UNRESOLVED
       (joint reconciliation)
    |
    v
implementation acceptance
    -> `slice-campaign`'s deterministic gate + completion offer/commit
       (IMPLEMENTED: `completion-publication.mjs`, accepted-checkpoint
       pipeline) — distinct from, and not resolved by, the proposal-level
       `decision.json` pattern. §4 already establishes proposal acceptance
       != implementation acceptance; the proposal schema is evidence that
       acceptance authority is separately represented (independently-false
       authority flags), not the owner of *this* transition. Portfolio
       exclusion is likewise explicitly kept distinct from proposal
       rejection.
    |
    v
revisioned history + evidence
    -> claim-evidence-service.md (AUTHORIZED_DIRECTION, partially
       IMPLEMENTED); evidence-anchor-observation-and-impact-nomination.md
       (RECONCILED_PENDING)
    |
    v
future impact / refresh / research
    -> claim-evidence's may_affect/refresh pipeline; attributed
       calibration diagnosis UNRESOLVED
       (`closed-loop-engineering-learning-reconciliation.md`)
```

Every arrow resolves to a named document or real code. Where the resolution
is "UNRESOLVED," that is not a gap this document is filling — it is the
queue's own finding, cited at the point of use, and restated in full in §13.

## 4. Planning and decision architecture

The upstream chain, with ownership kept unmerged even where the objects
sit adjacent to each other:

```text
idea
    v
proposal (proposal packets: identity, lifecycle_state, placement, decision)
    v
research maturity / evidence state (claim-evidence substrate + a
    decision-specific readiness contract, UNRESOLVED)
    v
proposal evaluation (typed estimates + a comparison-contract/dominance
    mechanism, UNRESOLVED)
    v
portfolio selection (a PortfolioDecision record, UNRESOLVED)
    v
accepted objective
    v
material decision surface
    v
sealed decision set (decision-gated-implementation-compilation.md)
    v
Structural Plan IR / implementation basis
    v
implementation contract
```

Two distinctions the queue repeatedly found and repeatedly had to defend
against collapse:

```text
claim  !=  decision  !=  plan

    claim: an evidence-backed statement about reality (claim-evidence)
    decision: a selection or delegation among materially different routes
        (decision-gated-compilation's sealed decision set)
    plan: a compiled, mechanically-constrained-or-bounded-discretion
        artifact derived from an accepted decision (Plan IR / the
        implementation contract)
```

```text
proposal meaning  !=  proposal acceptance  !=  decision authority
    !=  implementation authority  !=  implementation acceptance

    Confirmed by the actual authority-flag schema
    (`agent-instruction-review`'s bindAgentInstructionReviewResult):
    mutationAuthorized, architectureChoiceAuthorized,
    proposalAcceptanceAuthorized, reviewerSelectionAuthorized,
    implementationAcceptanceAuthorized, selfCertificationAuthorized,
    humanAuthorityConferred, independenceClaimed — eight independently
    false-by-default flags, not one authority bit.

    Confirmed again by portfolio selection's own residue: a
    PortfolioDecision's "exclude_from_current_portfolio" must never
    silently mutate a proposal's own separately-owned lifecycle/acceptance
    state (`decision.json`).
```

Readiness (item 5) and evaluation (item 6) each supply evidence to a
decision; neither becomes the decision. This is the same non-authority
boundary evidence-anchor enforces for architecture facts, restated at every
layer of the planning chain independently.

## 5. Organizational and orchestration architecture

The queue's one genuinely new architectural survivor gets first-class
treatment here, distinguished from its nearest neighbor. Reconciliation
established what feeds `ExecutionEnvelope` and what it produces — it did
not establish a compile-time sequence with hierarchical orchestration, and
this document does not invent one. `ExecutionEnvelope` *composes* role
contracts; it cannot sit upstream of the contracts it consumes, and
hierarchical decomposition was found `CORRESPONDS`/adjacent — a plausible
future *consumer* of a built envelope, not a required prior compilation
stage:

```text
                    admitted problem / work

individual role contracts -----------+
    (Agent Environment Graph /       |
     role-compiler-proposal.md,      |
     IMPLEMENTED for single-role     |
     structural shape)               |
                                      |
hierarchical work decomposition -----+--> organizational composition
    (hierarchical-planning-and-      |         |
     multi-supervisor-orchestration  |         v
     .md -- a plausible consumer,    |    ExecutionEnvelope
     not an established prior        |    (organizational-execution-
     compilation stage)              |     envelopes.md)
                                      |    durable, provenance-bearing,
capability / delegation needs -------+    problem-level multi-role
                                           organization: instantiated
                                           roles, ownership/delegation,
                                           information-flow boundaries,
                                           capability requirements and
                                           selected realizations
                                           -- UNRESOLVED: no compiler
                                              exists anywhere, even for
                                              the idea's own minimal
                                              first vertical; reusable
                                              role-profile composition
                                              is coupled to, not owned
                                              by, this idea
                                              (role-compiler-proposal.md's
                                              own deferred question)
                                                |
                                                v
                                           individual RoleRealizations
                                           (pre-indexed-capability-
                                            resolution-and-frozen-
                                            runtime-realization.md)
```

The distinction that prevents future drift:

```text
ExecutionEnvelope
    WHO needs to exist for this problem and how they relate.

RoleRealization
    HOW one already-admitted role is concretely executed
    (provider, harness, tools, policy overlay).
```

Neither owns the other's question. `organizational-execution-envelopes-reconciliation.md`
confirms this explicitly as a `SUPPLIES` relation
(`ExecutionEnvelope` → `RoleRealization`), not competing ownership.

## 6. Runtime realization and control

```text
role contract
+ requested capability subset
+ policy overlay
+ capability observations
    v
resolution
    v
admission
    v
immutable RoleRealization
    v
ProviderTurnPort / HarnessRuntimePort
```

`OperatorProjection` sits deliberately outside this dependency path — it
observes and controls Work Engine but is not a dependency of role
execution (`provider-turn-harness-runtime-and-operator-projection.md` §4).

This is also where the control-plane reconciliation now lets the following
be stated precisely, which it could not be before this session:

- **Scheduling** — `role-scheduler` (root substrate, real prototype, not
  yet ported to app-server); delivers, does not activate or authorize.
- **Role activation / binding** — `operator-switchboard.mjs` (IMPLEMENTED,
  attach/detach registry); no fencing or expiry on the active binding.
- **Workspace coordination / fencing** — `workspace-coordination`
  (IMPLEMENTED): typed resource leases, fencing tokens, CAS-protected
  `admitMutation` — the strongest kernel-shaped primitive found anywhere in
  the inventory.
- **Context lifecycle** — semantic context lifecycle management (referenced
  by claim-evidence-service.md as the precedent for its own placement
  rule); `context-transition-lease.mjs` (real).
- **Provider/harness invalidation** — `pre-indexed-capability-resolution-and-frozen-runtime-realization.md`
  §6 ("Invalidation"): observations may invalidate realization dependencies
  but do not mutate the realization or authorize its successor.
- **Control-plane commands** — `OperatorProjection`'s control-packet model
  (action/subject/revision/authority/expected consequence), confirmed
  `CORRESPONDS` with `control-plane-and-client-protocol.md`'s own framing.

**The one unowned piece**, found by `control-plane-and-client-protocol-reconciliation.md`
and independently re-surfaced by the Studio reconciliation: **fenced
active-binding coordination for logical role instances** — ensuring at most
one runtime realization generation holds the authoritative active binding
for a role instance. `workspace-coordination`'s existing `RESOURCE_TYPES`
enum has no role-active-binding-slot kind. This is distinct from
`OperatorProjection`'s already-correct invariant that a UI session must
never become the canonical role identity — that is about client
multiplicity, not realization exclusivity.

## 7. Evidence and semantic-state architecture

Arguably the deepest backbone this session found. The distinctions that
must not collapse into each other:

```text
observation  !=  deterministic derivation  !=  claim  !=  judgment/decision
    != plan != authority
```

The evidence substrate converges through one shared mechanical pipeline;
semantic consequences then branch by owner rather than continuing through
one more shared stage. Not every semantic object is a claim-evidence
domain profile — this is deliberately not "the pipeline every domain
profile instantiates":

```text
external / derived reality
    v
observation (EvidenceAnchorObserver family — one adapter per anchor kind:
    TextAnchor, CodeStructureAnchor, ServiceStateAnchor,
    ImplementationRevisionAnchor; extensible, not a closed set)
    v
comparison (exactly one of: matches / differs / unknown / unsupported /
    failed — never a judgment about whether a difference matters)
    v
may_affect nomination (mechanical, "beautifully weak": a report that a
    declared relationship no longer holds, nothing about whether it
    matters)
    v
possible semantic consumer -- branches by owner, not one further stage
    |
    +-- claim (claim-evidence-service.md: stable identity, immutable
    |         revisions, evidence references, confidence, limitations,
    |         reopening_conditions)
    |         v
    |     authorized transition (refresh episode: retained_unchanged /
    |         changed / inapplicable / insufficient / contested /
    |         deferred / superseded)
    |         v
    |     durable consequence (exact reverse reliance projection ->
    |         downstream-owner applicability and reopening judgment)
    |
    +-- bounded judgment / decision (owning domain -- e.g. a sealed
              decision set, an architectural finding, a review judgment;
              generic durable judgment identity/ancestry remains
              UNRESOLVED, per role-decision-trace's own finding that this
              is not a claim-evidence domain profile)
```

Located against this pipeline:

- **`claim-evidence`** (IMPLEMENTED substrate; `proposal-research-v1` and
  `revision-bound-review-finding-v1` are real, implemented vertical
  profiles in `authorized-vertical.mjs`) owns identity, evidence,
  provenance, and revision/lineage mechanics — not sufficiency, not
  decisions, not applicability judgments.
- **`evidence-anchor-observation-and-impact-nomination.md`** (RECONCILED_PENDING)
  owns the observation/comparison boundary feeding `may_affect` —
  explicitly not claim lifecycle or refresh judgment.
- **`may_affect` / refresh episodes / exact reliance** — claim-evidence's
  own authorized design, proven in part by root `ARCHITECTURE.md`'s
  claim-lineage backbone dogfood.
- **Candidate Trajectory** (RECONCILED_PENDING idea family: durable
  foundation, remediation-delta, builder-side-consumption) — the first
  fully-worked proposed code-domain service family against this substrate;
  supplies decision-gated-compilation's Stage 3 comparison for the
  remediation case specifically, and closed-loop-engineering-learning's
  "route revisions" observation source.
- **Execution observations** — `incremental-terminal-accounting-projection.md`
  (RECONCILED_PENDING): incremental receipt projection folding
  provider/retry/recovery/review-gate events; also the closed-loop
  reconciliation's corrected owner for "review burden" (not
  `review-finding-bridge.mjs`, which owns finding semantics, not
  execution burden).
- **Review findings** — `review-finding-bridge.mjs` (IMPLEMENTED).
- **Decision lineage residue** — `role-decision-trace.md`'s reconciliation
  found this is *not* a claim-evidence domain profile: durable semantic
  judgment identity/ancestry (of which material decisions are one
  subtype) remains genuinely unowned, with all seven proposed relation
  types (`PREMISE_FOR`/`SUPERSEDES`/`WEAKENS`/`CONTRADICTS`/`AFFECTS`/
  `REOPENED_BY`/`CHANGED_BECAUSE_OF`) unresolved by endpoint type against
  claim-evidence's own revision-lineage-only relationship topology.
- **Coverage semantics** — `AnchorObservation`'s `bound_observation` /
  `observed_observation` split, explicitly guarding against comparing a
  declared dependency against evidence whose repository/service/
  implementation revision is merely assumed to be the requested one.

## 8. Revisioned state, coordinates, reconstruction, and branching

This session repeatedly found the same shape without naming a universal
primitive to build — that restraint is itself load-bearing and is preserved
here rather than resolved:

> Multiple independently owned subsystems instantiate a recurring
> revisioned-state pattern.

```text
stable identity
    v
exact revision
    v
admitted transition
    v
authoritative successor
    v
predecessor / lineage
    v
exact downstream reliance
```

The evidence is strong precisely because it spans two different evidence
classes, not because four equivalent instances exist — typed accurately
rather than flattened:

```text
implemented instances (real code, IMPLEMENTED)
    claim-evidence's ledger
    review-episode (skills/independent-review-state, root substrate)
    slice-campaign's campaign state

authorized design convergence (AUTHORIZED_DIRECTION, not built)
    decision-gated-compilation's sealed decision set

pending fifth convergence (found this session, not yet built)
    organizational-execution-envelopes' orchestration-state field list
```

A pattern appearing independently across both real implementation and
prospective design, without anyone having deliberately shared code between
them, is evidence to keep watching, not a primitive to build now — the
same discipline `service-plane-reconciliation.md` already held to.

Coordinates, over this substrate:

```text
Coordinate =
    exact owned revisions
  + realization identity
  + implementation / repository identity
  + coverage
  + frontier
```

`service-plane-and-kernel-domain-boundary.md` §5 concretizes the vague
"world-state basis and coverage" field `revisioned-research-and-execution-architecture.md`
leaves open as an **open map of revision-bound service-state references**
(`service_state: {claim-evidence: {...}, workspace-coordination: {...}}`),
so a future domain can add entries without the coordinate schema itself
changing. Consequences this enables: recovery, continuation, historical
reconstruction, branching, replay, model comparison, and experiments —
"semantic Git" behavior without claiming the kernel abstraction is already
implemented.

## 9. Domain systems

The common architecture supports rich domain semantics without absorbing
them. Three domains this session grounded directly:

### Software engineering

```text
code evidence / codebase-memory-mcp (confirmed permanently read-only)
    v
CodeEvidenceAdapter (Work-Engine-owned port, evidence-anchor §4)
    v
review-subject / Code Change Profile (IMPLEMENTED — digest-verified,
    already an App Server service, not an aspiration)
    v
Candidate Trajectory (RECONCILED_PENDING — predecessor/successor deltas)
    v
implementation compilation (Plan IR / decision-gated compilation)
    v
review scope / architecture conformance
    (review-scope-coordination.md's residue; the joint seam-review/
     architectural-review reconciliation)
```

### Browser / UI

The cleanest example this session produced of general infrastructure
underneath, domain-specific meaning above:

```text
Chrome
    v
AI-Accessible Browser (capture / normalize / index / project)
    v
UI Experience Evidence Interface (formed, undecided proposal —
    product-view semantics)
    v
claim-evidence (claim lifecycle) + UI review
    (UIReviewProfile — RECONCILED_PENDING, coupled to
     adaptive-review-panel-coordination + concern-scoped-review-judgments;
     Site2JSON's Truth/Maintainability/Explainability/Aesthetics is the
     first demonstrated profile, not a universal ontology)
```

The UI review Truth concern is a named candidate domain owner (for the
UI-specific case only) of the seam-review "UI-representation ↔ state"
correspondence judgment — a partial, not generic, resolution.

### Research

```text
historical execution coordinates (revisioned-research-and-execution-architecture.md)
    v
curated experimental coordinates
    v
controlled branch realizations
    v
execution observations
    v
capability evidence (evidence-calibrated-plan-resolution-and-continuous-capability-learning.md
    -- execution/provider-strategy learning)
    v
routing / planning hypotheses
```

Explicitly distinct from `closed-loop-engineering-learning.md`'s
proposal-prediction-accuracy calibration (was this proposal's expected
value/risk/complexity correct) — `candidate-trajectory-upstream-amendments.md`
already keeps these two calibration targets separate. Research consumes
production history without gaining production authority, the same
non-authority rule found everywhere else.

## 10. Review architecture

Now that every review-shaped idea in the queue has been reconciled, the
ownership topology — not a catalog of reviewer types — is:

```text
deterministic correspondence / impact detection
    (evidence-anchor's comparator: matches/differs/unknown/unsupported/failed)
    v
review evidence

semantic reviewer judgment
    (a domain-owner judgment evidence-anchor's own design reserves but
     does not make)
    v
review finding
    (claim-evidence's review-finding-bridge.mjs, IMPLEMENTED; or a
     concern-scoped ReviewJudgment, RECONCILED_PENDING)

coordination
    (adaptive-review-panel-coordination: an open, model-interpreted
     specialist registry — coordination executes specialists; it does
     not decide which concerns are mandatory for a given review profile)
    v
panel / synthesis / applicability

acceptance authority
    (a separate, named owner — never the reviewer, never the coordinator)
```

Seam review and architectural review sit inside this topology as two
distinct semantic-judgment layers, not two competing mechanisms:

```text
cross-cutting seam review
    "do two independently-valid things still correspond, truthfully and
    proportionately?" -- mechanical half retires to evidence-anchor;
    the semantic correspondence judgment for non-mechanical seams
    (UI<->state, authority<->exposed-control) remains UNRESOLVED generically

architectural review
    "is the system MODEL, ownership, decomposition, or placement wrong?"
    -- its diagnostic-finding shape is coupled to claim-evidence's
    domain-profile pattern (RECONCILED_PENDING); its "blocking consequence
    remains separately owned" boundary is confirmed achievable via
    strategic-planning-handoff.mjs's real campaign-level verdict authority
    -- one concrete consumer among several named, not the universal owner
```

`agent-instruction-structure-and-placement-review` is the one fully
IMPLEMENTED and dogfooded instance of this entire topology: a distinct
specialist skill, delivered read-only with a zero-effect boundary,
registered in the open registry, with its own authority flags mechanically
fixed `false` — including `selfCertificationAuthorized: false`, the
mechanical enforcement of its own explicitly-named self-referential blind
spot.

## 11. Operator and Studio boundary

> Work Engine Studio is a projection and interaction surface over Work
> Engine-owned state. It does not become the owner of the domain semantics
> it presents.

That conclusion is far stronger after the reconciliation queue than it
would have been before it. Studio's five originally-proposed views resolve
almost entirely to owners this queue already found real, formed, or
reconciled:

```text
design / contracts
    -> Agent Environment Graph / role-compiler-proposal.md (read side,
       IMPLEMENTED); an authoring/admission path back to those owners
       remains UNRESOLVED

organization
    -> organizational-execution-envelopes.md's ExecutionEnvelope
       (blocked on that idea's own still-unbuilt compiler — not a new
       Studio-specific gap)

runtime
    -> control-plane-causal-observability-ui.md's Operational view
       (AUTHORIZED_DIRECTION, an already-accepted design found during
       this session's own investigation, previously uncross-referenced
       with Studio in either direction — now cross-referenced)

control
    -> the control plane / OperatorProjection already own which
       operations exist and their authorization; unified with the
       design/contract authoring path above into one open question below

evidence / claims, history / reconstruction
    -> claim-evidence, evidence-anchor, the coordinate model (§8)

review, forensics / replay
    -> control-plane-causal-observability-ui.md's causal graph
       (execution-history half, AUTHORIZED_DIRECTION); the
       design/organization <-> runtime historical correlation half
       remains blocked on the same execution-envelope compiler as
       "organization" above
```

**The one genuinely open design question, unifying what first looked like
two separate gaps (Control view, Contract/design authoring):** an
authority-preserving interactive command/edit projection.

```text
owned state
    v
discoverable projections + admitted operations
    v
Studio: display / edit / request
    v
identity + revision + authority-bound intent
    v
owning transition
    v
authoritative result
    v
Studio feedback
```

Studio does not decide which operations exist or whether they are
authorized — both are already owned elsewhere. Its own missing capability
is only the discovery/rendering/submission/lifecycle-feedback mechanism
itself.

## 12. Implementation-state map

"Implemented" means production evidence exists — real code, exported,
tested, or dogfooded — not that a document describes it.

On 2026-09-14 the user explicitly reviewed and accepted the reconciliation
findings for all thirteen queue idea files, and authorized their residues
to proceed — distinguishing two states this table now uses in addition to
the original six. `ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION` means the
residue is specified enough to build against directly. 
`ACCEPTED_AUTHORIZED_FOR_DESIGN` means the reconciliation's meaning is
accepted but the residue itself names an open design question (an owner
choice, a foundational primitive decision, or a contract that must exist
before code referencing it can be written) — implementation authorization
for those is explicitly deferred until that design question is resolved.
Acceptance of meaning does not retroactively make an `UNRESOLVED_SEAM`
architecturally solved; it is a decision to proceed, not a claim that the
work is done. See each idea file's own Status section for the exact
authorization language.

| Responsibility | State | Evidence |
| --- | --- | --- |
| `claim-evidence` core substrate (identity, evidence, revision/lineage) | IMPLEMENTED | `app-server/src/services/claim-evidence/` (15 files, ~2400 lines); `proposal-research-v1` and `revision-bound-review-finding-v1` in `authorized-vertical.mjs` |
| `review-finding-bridge.mjs` | IMPLEMENTED | Real code; `revisionPayload` schema matches `revision-bound-review-artifacts` |
| `workspace-coordination` (fencing/CAS) | IMPLEMENTED | Typed `RESOURCE_TYPES`, lease/fence records, `admitMutation` |
| `review-episode` / independent review state | IMPLEMENTED at root only | `skills/independent-review-state/` (has `scripts/`, `tests/`); not yet ported to app-server |
| `slice-campaign` completion/gate (implementation acceptance) | IMPLEMENTED | `completion-publication.mjs`, accepted-checkpoint pipeline |
| `agent-instruction-structure-and-placement-review` | IMPLEMENTED, dogfooded | `skills/agent-instruction-review/`, `app-server/src/services/agent-instruction-review/`, `metrics/agent-instruction-review.jsonl` (accepted slice) |
| `operator-switchboard.mjs` (client-protocol half) | PARTIALLY_IMPLEMENTED | Attach/detach registry, binding view; no fencing/expiry on active binding |
| `strategic-planning-handoff.mjs` (campaign-level verdict) | IMPLEMENTED | Real code + dogfooded instance in `post-migration-strategic-plan.md` |
| Agent Environment Graph / skill-compiler (single-role structure) | IMPLEMENTED | `docs/agent-environments.yaml`, `app-server/src/skill-compiler.mjs`, `structural-core-ownership.md` |
| `product-development` proposal/intake delivery | IMPLEMENTED (mechanical only) | `proposal-delivery.mjs`, `intake-delivery.mjs` — packet validation, not evaluation |
| `role-scheduler` | IMPLEMENTED at root only | `skills/role-scheduler`; zero references in `app-server/src`; named migration target |
| `claim-evidence-service.md`'s impact/refresh/reliance pipeline | AUTHORIZED_DIRECTION | Design accepted; the production operation-contract surface for impact nomination and refresh-episode admission is not yet in `contract.mjs` (2026-09-15: proposal meaning approved for `claim-maintenance-and-reliance-propagation`, `implementation_authorized: false`; operation count deliberately undecided) |
| `provider-turn-harness-runtime-and-operator-projection.md` (ports) | AUTHORIZED_DIRECTION | No provider-neutral contract built yet; production builder path still statically Codex-bound |
| `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` | AUTHORIZED_DIRECTION | Partial precursors only (runtime manifest, capability negotiation) |
| `control-plane-causal-observability-ui.md` | AUTHORIZED_DIRECTION | Accepted design, phased delivery slices (UI-0..UI-4), none built |
| `service-plane-and-kernel-domain-boundary.md` | AUTHORIZED_DIRECTION | Reconciled against 20 real units; not itself an implementation |
| `role-compiler-proposal.md` | AUTHORIZED_DIRECTION | Bootstrap experiment scoped, not yet authorized beyond the sterile slice-builder migration |
| `revision-bound-review-artifacts`, `adaptive-review-panel-coordination` | FORMED_PROPOSAL | `decision.json`: `approve_proposal_meaning`, `implementation_authorized: false` |
| `concern-scoped-review-judgments` | FORMED_PROPOSAL, undecided | `lifecycle_state: "formed"`, no `decision.json` |
| `ui-experience-evidence-interface` | FORMED_PROPOSAL, undecided | No `decision.json` |
| `evidence-anchor-observation-and-impact-nomination.md` | RECONCILED_PENDING | Exploratory only; supersedes an earlier, corrected draft |
| Candidate Trajectory family (4 files) | RECONCILED_PENDING | No implementation evidence; deterministic engineering, no pilot required per this session's own assessment |
| `incremental-terminal-accounting-projection.md` | RECONCILED_PENDING | "pending post-migration proposal" |
| Fenced active-binding for role realization | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | `control-plane-and-client-protocol-reconciliation.md`; user acceptance 2026-09-14 |
| Prospective review-scope coordination before mutation admission | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | `review-scope-coordination-reconciliation.md`; user acceptance 2026-09-14 |
| Execution-envelope compiler | ACCEPTED_AUTHORIZED_FOR_DESIGN | `organizational-execution-envelopes-reconciliation.md` — the queue's one new architectural level; user acceptance 2026-09-14, implementation deferred pending reconciliation with `role-compiler-proposal.md`'s own deferred composition question |
| Durable semantic judgment identity/ancestry | ACCEPTED_AUTHORIZED_FOR_DESIGN | `role-decision-trace-reconciliation.md`; user acceptance 2026-09-14, implementation deferred pending the general-primitive-vs-sealed-decision-set choice |
| Decision-specific readiness contract | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | `proposal-research-maturity-and-freshness-reconciliation.md`; user acceptance 2026-09-14 |
| Comparison-contract / dominance mechanism | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | `evidence-backed-proposal-evaluation-reconciliation.md`; user acceptance 2026-09-14 |
| `PortfolioDecision` record | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | `proposal-backed-portfolio-selection-reconciliation.md`; user acceptance 2026-09-14 |
| Seam-evidence adapter extensions | ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION | joint seam-review/architectural-review reconciliation; user acceptance 2026-09-14 |
| Semantic correspondence-judgment owner + attributed routing/disposition concept | ACCEPTED_AUTHORIZED_FOR_DESIGN | joint seam-review/architectural-review reconciliation; user acceptance 2026-09-14, implementation deferred pending an owner decision |
| Architectural-finding domain profile + routing step | ACCEPTED_AUTHORIZED_FOR_DESIGN | joint seam-review/architectural-review reconciliation; user acceptance 2026-09-14 |
| Attributed calibration diagnosis + prediction↔outcome correspondence contract | ACCEPTED_AUTHORIZED_FOR_DESIGN | `closed-loop-engineering-learning-reconciliation.md`; user acceptance 2026-09-14, implementation deferred pending the correspondence contract's own specification |
| `UIReviewProfile` contract | ACCEPTED_AUTHORIZED_FOR_DESIGN | `ui-review-capability-reconciliation.md`; user acceptance 2026-09-14 |
| Authority-preserving interactive command/edit projection | ACCEPTED_AUTHORIZED_FOR_DESIGN | `work-engine-studio-reconciliation.md`; user acceptance 2026-09-14 |
| Dynamic team synthesis, new-role invention | DEFERRED | Explicitly self-deferred by `organizational-execution-envelopes.md`'s own "Adoption boundary" |
| `CodeStructureAnchor` locator | DEFERRED | Investigation, not invention, per evidence-anchor's own §9 |

## 13. Remaining open seams

Only what is already unresolved. No brainstorming; each states known facts,
why it is unresolved, and what would resolve it. Per §12, every seam below arising from the sequel reconciliation queue
(items 1, 2, 4, 5, 6, 7, 8, 9, 10, 11) was explicitly accepted and
authorized to proceed by the user on 2026-09-14 — acceptance changes who
may act on a seam, not whether the underlying architectural question is
answered. Items 1, 4, 8, 9, 10, and 11 are authorized for design work only;
items 2, 3, 5, 6, and 7 are authorized for implementation directly (item 8
additionally includes a build-ready sub-piece, the mechanical seam-evidence
adapter extensions, per §12's table). Two further seams predating the queue
(`routing.vs.admission` and `decision-gated.vs.hierarchical-orchestration`,
listed below as items 12 and 13) were not part of the 2026-09-14 acceptance
but were separately resolved by explicit user ruling on 2026-09-15 — see the
closure notice preceding them.

**1. Execution-envelope compiler** (the queue's largest survivor)
Known: `organizational-execution-envelopes.md`'s ExecutionEnvelope concept
is well-specified; nothing compiles it, even for the idea's own minimal
first vertical (compile the topology already running). Unresolved because:
no document proposes the deterministic `system + reusable-profile + role`
composition mechanism, and reusable role-profile composition is itself
coupled to `role-compiler-proposal.md`'s own deferred question. Resolved
by: a concrete compiler proposal reconciled against `role-compiler-proposal.md`'s
eventual answer to composition/inheritance.

**2. Fenced active-binding coordination for logical role instances**
Known: `workspace-coordination` has the exact mechanism shape (leases,
fencing, CAS) but no role-active-binding-slot resource kind.
`operator-switchboard.mjs` has attach/detach with no fencing. Unresolved
because: no document assigns ownership of minting this fence. Resolved by:
a decision whether this is a new `workspace-coordination` resource type or
a distinct mechanism.

**3. Prospective review-scope coordination before mutation admission**
Known: neither `workspace-coordination.admitMutation` (blind to review
state) nor claim-evidence's reactive impact pipeline (blind to
not-yet-admitted mutations) answers whether a planned mutation threatens an
active review. Unresolved because: no review-domain owner declares which
scope needs protecting before mutation. Resolved by: a review-workflow
responsibility that declares protected scope and calls
`workspace-coordination` directly, or an equivalent explicit design.

**4. Durable semantic judgment identity and ancestry**
Known: a claim is not a decision is not a judgment; `decision-gated-compilation`'s
sealed decision set is a narrow, real precedent covering only explicit
route selection. Unresolved because: no owner exists for judgment classes,
lifecycle, active governing-judgment state, or any of seven proposed
ancestry relations (checked by endpoint type, not name, against
claim-evidence's revision-lineage-only topology — none matched). Resolved
by: an owner decision — general revisioned-state primitive vs. growing from
the sealed decision-set architecture.

**5. Decision-specific readiness contract**
Known: R0–R5 decomposes fully into existing/queued owners; the readiness
judgment itself ("is current evidence sufficient for decision D") does not.
Unresolved because: no attributed-sufficiency-judgment schema exists
anywhere, distinct from evaluation (item 6, "what do we believe") and from
authority to decide. Resolved by: a readiness-contract design, with item
6's evaluation as its confirmed evidence supplier.

**6. Comparison-contract / dominance mechanism for proposal evaluation**
Known: `claim-evidence` has no comparison mechanism at all (explicit
non-goal). Unresolved because: nothing defines typed, comparison-ready
measures or the contract (units, scale, missing/contested-evidence
handling, authority) that would make cross-proposal dominance findings
meaningful. Resolved by: a design splitting contract-schema/validation/
derivation-mechanics (evaluation's job) from which-measures/which-
proposals/which-surface (the decision or portfolio owner's job).

**7. `PortfolioDecision` record**
Known: proposal identity, typed relationships, and decisions are all real;
no priority/sequencing field exists anywhere, and strategic assumptions
have real structured precedent (`strategic-planning-handoff.mjs`) but not
bound to an exact proposal set. Unresolved because: no revision-bound
record exists combining strategic basis + proposal/evaluation/readiness
revisions + attributed cross-proposal analysis + a priority/sequencing/
exclusion disposition. Resolved by: that record's design, keeping portfolio
non-selection explicitly distinct from proposal-lifecycle rejection.

**8. Attributed routing/disposition concept + architectural-finding domain
profile**
Known: seam review's mechanical half retires to evidence-anchor; the
semantic correspondence judgment for non-mechanical seams does not.
Architectural review's diagnostic-finding shape matches claim-evidence's
existing fields closely enough to couple, not enough to assume ownership.
Unresolved because: no record shape or owner exists for either. Resolved
by: an owner decision for the semantic judgment, and a domain-profile
proposal for architectural findings, each independently.

**9. Attributed calibration diagnosis + prediction↔outcome correspondence
contract**
Known: predicted values (item 6) and observed values (two pending designs:
`incremental-terminal-accounting-projection.md`, Candidate Trajectory
remediation-delta) are not automatically comparable — many pairs (predicted
complexity vs. observed effort) are not the same variable measured twice.
Unresolved because: no typed correspondence contract exists, and no record
supports multiple contributing explanations for a miss without collapsing
into a single reflexive verdict. Resolved by: both designs, explicitly
preserving that an adverse outcome does not prove a prediction was wrong.

**10. `UIReviewProfile` contract**
Known: Site2JSON's four lenses are real but product-specific; the adaptive
review registry executes specialists but does not know which concerns are
mandatory for a given profile. Unresolved because: no project-parameterized
profile contract (required concerns, judgment semantics, per-concern
evidence requirements, completeness rules) exists. Resolved by: that
contract's design, with Site2JSON as its first demonstrated instance.

**11. Authority-preserving interactive command/edit projection**
Known: the control plane and `OperatorProjection` already own which
operations exist and their authorization. Unresolved because: no document
specifies the discovery/rendering/bounded-intent-submission/lifecycle-
feedback mechanism itself, needed by both Studio's Control view and its
Contract/design authoring path. Resolved by: one connective-layer design
covering both.

**Seams closed since publication (2026-09-15):** the two items numbered 12
and 13 below were open when this document was first published; both are now
resolved by explicit user ruling and are listed here for the historical
record, not as open seams.

**12. `routing.vs.admission`** — CLOSED 2026-09-15 (revised same day after
review).
Was: decision-gated compilation's executor-class routing and
capability-resolution's admission were both claims on "which mechanism does
this work," in incompatible vocabularies, with no stated composition order
(carried forward from the pre-queue `architecture-direction-synthesis.md`
§2.1, not part of the sequel queue's 2026-09-14 acceptance). A first ruling
collapsed this into two stages (decision-gated compilation owning "semantic
requirement/strategy," capability-resolution owning "resolution/admission");
review found this quietly gave decision-gated compilation more authority
than its own text claims, by conflating its compile-time executor-class
*characterization* (plan-readiness only — it "does not authorize the slice")
with the separate slice-level *routing decision* Stage 6 already reserves for
"the supervisor" and "the appropriate authority." Resolved: three stages, not
two —
(1) **contract characterization** (decision-gated compilation's implementation
compiler + plan-conformance gate: which executor classes are semantically
supported by a compiled contract, with what evidence-backed readiness; plan
readiness only, no slice authority);
(2) **executor-class routing / acceptance** (the supervisor / routing-policy
authority named in decision-gated-compilation's own Stage 6: which supported
class this slice actually uses, given evidence, cost, and policy — a
nomination, advisory until accepted for the slice);
(3) **runtime resolution/admission** (capability-resolution §4: given that
accepted class plus current capabilities and policy, which exact realization
is admitted now). The actor classes capable of supplying stages 2 and 3 may
overlap (the same supervisor or operator could plausibly hold both), but
that overlap carries no architectural weight: authorization to accept an
executor class does not itself authorize a concrete realization, and
realization authority does not imply authority to change the selected
class. Same actor does not mean same decision or same authority.
Recorded in both owning documents:
`proposal-decision-gated-implementation-compilation.md`'s "Relationship to
capability resolution's admission boundary" and
`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`'s own
ruling note.

**13. `decision-gated.vs.hierarchical-orchestration`** — CLOSED 2026-09-15.
Was: hierarchical orchestration's "branch plan" had no stated relationship to
decision-gated compilation's "sealed decision set" and "implementation
contract" (carried forward from the pre-queue synthesis §2.3; this document's
§5 covered hierarchical orchestration's role boundary but never carried this
seam forward as its own tracked item — a gap in this document, not a new
finding). Resolved: the user accepted the candidate composition §2.3 had
drafted but not adopted — the branch plan sits strictly upstream of, and
distinct from, the decision surface/sealed decisions/implementation
contract(s); one accepted branch plan may be realized through multiple
implementation contracts produced in successive bounded increments, not an
upstream artifact that embeds or pre-specifies contracts not yet produced.
Recorded in
`hierarchical-planning-and-multi-supervisor-orchestration.md`'s own "Branch
plan" section (`decision-gated-compilation`'s document is unaffected by this
ruling).

## 14. Architecture maintenance risks

Distinct from §13: not an unresolved architectural boundary, but a standing
process risk this session found twice — independently evolving owners can
drift into uncross-referenced overlap even when each side's own design is
individually sound.

`evidence-anchor-observation-and-impact-nomination.md` and
`ai-accessible-browser-seam-reconciliation.md` each had to add reciprocal
notes after independent development; `control-plane-causal-observability-ui.md`
and `work-engine-studio.md` were found with zero cross-reference despite
substantial view overlap between an already-accepted design and a still-
exploratory idea, now corrected. There is no single fix for this — it is a
standing risk the reconciliation program this session ran exists to keep
catching, not a one-time correction. Any future addition to
`app-server/docs/` or `app-server/ideas/pending/` should be checked against
this document's §2–§11 for overlap before being treated as independent.

## 15. Architectural dependency map

```text
proposal evidence (claim-evidence + evidence-anchor)
        v
proposal evaluation (typed estimates + comparison contract, UNRESOLVED)
        v
portfolio selection (PortfolioDecision, UNRESOLVED)
        v
planning / decision closure (Plan IR, decision-gated compilation)
        v
organizational envelope (compiler UNRESOLVED)
        v
role contracts (Agent Environment Graph, IMPLEMENTED)
        v
runtime realization (RoleRealization, AUTHORIZED_DIRECTION)
        v
execution (ports, workspace-coordination)
        v
observations / claims / reviews (claim-evidence, evidence-anchor,
    review-finding-bridge — all substantially real)
        v
history / coordinates (service_state map, AUTHORIZED_DIRECTION)
        v
future planning + research (may_affect/refresh; calibration diagnosis
    UNRESOLVED)
```

Cross-cutting infrastructure, not a pipeline stage:

```text
control plane + OperatorProjection  -- authority/discovery substrate for
    every interactive surface (Studio, agent instructions, review)
review ownership topology  -- mechanical correspondence -> semantic
    judgment -> coordination -> acceptance, instantiated by every
    reviewer this queue reconciled
coordinate model  -- service_state references every functional system
    writes into and reads from
```

## Relationships

| Direction | Relationship |
| --- | --- |
| [`architecture-direction-synthesis.md`](architecture-direction-synthesis.md), [`architecture-direction-seam-map.md`](architecture-direction-seam-map.md) | The pre-queue synthesis this document supersedes in scope, not in substance — its three-planes/four-systems model and §2's family-specific seam findings are carried forward unchanged. |
| [`service-plane-inventory.md`](service-plane-inventory.md), [`service-plane-reconciliation.md`](service-plane-reconciliation.md) | The fixed, already-reconciled implementation baseline. |
| Every `*-reconciliation.md` document in `app-server/docs/` produced by the twelve-item sequel queue | Each retains its own full disposition table and evidence. This document synthesizes their conclusions; it does not restate their reasoning. |
| `root-ideas-reconciliation.md`, `ai-accessible-browser-seam-reconciliation.md` | Earlier reconciliation passes this session, feeding §7 and §9. |
| `incremental-architecture-intake-and-seam-reconciliation.md` | The method this document and every reconciliation it draws on followed. |

## Non-goals

- This document is not itself a proposal and authorizes no implementation.
- It does not establish decision authority, priority, or sequencing beyond
  what each cited document already states.
- It does not claim completeness beyond the family of documents this
  session actually reconciled — real pending ideas outside that scope may
  exist and are not represented here.
- It does not resolve any seam listed in §13. Naming a seam is not
  deciding it.
- It does not treat the three-plane/four-system framing, or any subsystem's
  internal schema, as final. It may itself need revision under further
  reconciliation.
