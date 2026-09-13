# Architecture Direction: Work Engine's Prospective System

## Status

Frozen prospective architecture (intake method §12.1) for the family of
pending ideas integrated in
[`architecture-direction-seam-map.md`](architecture-direction-seam-map.md).
This is a synthesis of how those documents compose, not a restatement of each
one. Every claim below traces to a specific document and, where a
relationship required inference, to a specific seam record in that map.

An earlier pass at this document organized its subject as "five layers"
(substrate, realization, planning/compilation, orchestration,
evidence/history), stacked sequentially with substrate at the bottom. On
review this was a category error: planning/compilation and orchestration are
themselves largely workflow-plane activities, so they cannot coherently sit
"above" a substrate that already contains the workflow plane as one of its
three planes. This revision replaces the five-layer stack with two orthogonal
axes — see the Summary below.

**Authority:** Exploratory only. This document does not accept any
constituent idea, authorize implementation, amend the migration roadmap, or
select a permanent schema, provider, or routing policy. It states what the
architecture is intended to become if its constituent ideas are accepted and
successfully realized (per the intake method's §11), not what currently
exists — see [`service-plane-inventory.md`](service-plane-inventory.md) for
current implemented reality.

**Scope:** Runtime realization, capability-aware planning and compilation,
multi-branch orchestration, and evidence/history — as they compose with the
already-reconciled kernel/service/workflow substrate.

## Summary

Work Engine's prospective architecture is not five sequential layers. It is
two orthogonal axes crossing one small substrate:

```text
three responsibility planes                four functional systems
(who owns what kind of responsibility)      (what part of governed execution
                                              we are implementing)

    KERNEL                                      REALIZATION
    SERVICE           x                         PLANNING / COMPILATION
    WORKFLOW                                     ORCHESTRATION
                                                  EVIDENCE / HISTORY
```

The three planes (`service-plane-and-kernel-domain-boundary.md`, already
reconciled) answer *who owns a given piece of responsibility* — kernel,
service, or workflow — independent of what functional concern it serves. The
four functional systems answer *what part of governed execution is being
implemented*, and each one draws on more than one plane rather than living
entirely inside one:

- **Realization** is primarily service-plane-facing: semantic/hybrid service
  operations consume materialized realizations assembled from admitted
  provider/harness mechanisms — ports advertise mechanisms, resolution/
  admission chooses a compatible composition, and the resulting immutable
  realization is what a service operation actually binds to. Ports are not
  themselves service operations.
- **Planning/compilation** is workflow-plane behavior (deciding how much
  structure an executor needs, and routing to one) that increasingly wants to
  render results as deterministic service operations rather than inference.
- **Orchestration** is workflow-plane behavior (a preplanner/orchestrator and
  per-branch planner/supervisor pairs) that depends on service-owned durable
  state and, per §3 below, on the same kernel-shaped revisioned-state pattern
  found elsewhere in the substrate.
- **Evidence/history** is not one plane at all — it is the durable record
  every plane and every functional system writes into and reads from, which
  is why it does not fit as "layer 5 on top of layer 1" any better than
  orchestration does.

The code domain remains orthogonal to both axes, exactly as
`service-plane-and-kernel-domain-boundary.md` §1 establishes: it supplies
domain-specific services (`review-subject`, `candidate-trajectory`) and
workflows inside this structure, without the kernel needing to know what a
Git tree or a Python symbol is.

The seam map found these four systems compose without a fundamental
conflict, though three points show two documents in this family reaching for
the same underlying decision — recorded as open `COUPLED_DECISION`/
`UNRESOLVED` seams, not silently resolved here.

## 1. Three responsibility planes: who owns what

`service-plane-and-kernel-domain-boundary.md`, reconciled against 20 actually
implemented units, is the fixed baseline every functional system below
composes with. Its governing rule carries through everything that follows:

> A service may expose domain semantics. A workflow may compose domain
> semantics. The kernel should only need to understand the service contract
> around them.

Concretely, kernel-shaped primitives found in the current implementation
(`workspace-coordination`'s fencing/CAS admission, `claim-evidence`'s ledger
mechanics) are domain-neutral and reusable; `review-subject`'s deterministic
candidate/profile mediation is a genuinely domain-rich service that stays
code-specific without needing the kernel to know what a Git tree is. Every
functional system in §2 is written, in this family's own documents, to
respect exactly this boundary — none of them propose new kernel-level domain
concepts.

## 2. Four functional systems

Each of these draws on more than one responsibility plane. None of them is
"the workflow plane" or "the service plane" by itself — each states which
plane(s) it primarily depends on as it's introduced.

### 2.1 Realization: executing semantic work

Primarily service-plane (materialization) with a workflow-plane consumer
(binding). Two documents jointly own "how does semantic work actually run":

- `provider-turn-harness-runtime-and-operator-projection.md` defines three
  independently replaceable ports (`ProviderTurnPort`, `HarnessRuntimePort`,
  `OperatorProjection`) and states plainly that Work Engine retains
  orchestration, authority, and workflow truth while ports supply mechanism
  only.
- `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` is
  its explicit companion (seam `ports.companion-binding`, `ALIGNED`): it owns
  capability observation, the operator policy overlay, admission, and
  materialization of an immutable `RoleRealization` from those inputs.

Neither document conflates "capability" (a role/realization's granted
permission — confirmed `ALIGNED` in seam `capability.sense-confirmed-aligned`
against service-plane's post-reconciliation vocabulary) with "service
operation" (a callable identity). This family already uses the vocabulary
service-plane's reconciliation settled on, without having to be corrected.

Service-plane's own §8 adds one refinement the other three systems depend on:
realization is required only for `semantic` or `hybrid` service operations. A
`deterministic` operation (like `review-subject`'s profile derivation) needs
no model/provider/harness binding at all — it needs an **implementation
revision** instead (seam `realization.scope-gap`), a concept
capability-resolution's own scope does not cover and should not be expected
to. This distinction — semantic realization vs. implementation revision vs.
contract revision — is load-bearing for §2.4's history model below.

**Open seam, with a candidate direction worth recording (not resolved here):**
decision-gated compilation's "execution model routing" (choosing Sol vs.
Spark as an executor class) and capability-resolution's "resolution and
admission" (choosing a concrete provider/harness binding) are both claims on
"which mechanism does this work," in incompatible vocabularies, with no
stated composition order (seam `routing.vs.admission`, `UNRESOLVED`). A
plausible shape — not adopted by either document, and not adopted here —
separates the two questions rather than merging them:

```text
semantic requirement / strategy (which executor class does this work warrant?)
            |
            v
resolution / admission (given that class + current capabilities + policy,
                         which exact model/provider/harness/tool realization
                         is admitted?)
            |
            v
concrete realization
```

Decision-gated compilation would own the first question; capability-resolution
the second. This preserves the distinction between *what kind of execution
the work warrants* and *how that requirement is concretely realized* — but it
is a candidate composition, not something either source document currently
states, and this synthesis does not adopt it on their behalf.

### 2.2 Planning and compilation: how much structure, and for whom

Primarily workflow-plane, with an increasing pull toward rendering results as
service-plane operations. Four documents form one coherent (if not yet fully
cross-referenced) picture:

- **Plan IR** (`structural-plan-ir-for-capability-aware-multi-model-execution.md`)
  proposes one canonical structural representation of implementation
  intent, rendered at different resolution levels (P0–P3) for different
  builder capability profiles, explicitly to avoid entangling plan semantics
  with model identity.
- **Evidence-calibrated plan resolution**
  (`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`)
  generalizes Plan IR's scalar P0–P3 ladder into a dimension-specific
  `projection_profile` vector (dependency structure, authority boundaries,
  judgment boundaries, and others) — and says so itself ("the existing P0–P3
  levels may remain useful even if named presets eventually map to
  profiles," seam `resolution-ladder.vs.projection-profile`, `ALIGNED`).
- **The execution-profile-scoring and planner-owned-characterization addenda**
  each independently characterize the *work itself* (semantic novelty,
  repository breadth, oracle strength, reversibility, and others) as input
  to strategy selection. These are distinct from the projection-resolution
  vector above — characterization informs how much structure to render, it
  is not the same list (seam `characterization.vs.projection-profile`,
  `ALIGNED` as related-but-distinct). The two addenda also duplicate most of
  their own dimension list against each other without cross-reference (seam
  `execution-profile.addenda-duplication`, `TERMINOLOGY`) — a real, cheap
  fix this family should make before either is formalized.
- **Decision-gated implementation compilation**
  (`proposal-decision-gated-implementation-compilation.md`) owns the actual
  workflow: a material decision surface, a sealed decision set, a compiled
  implementation contract, a plan-conformance gate, and staged evidence
  (Stages 0–6) for whether a cheaper executor (Spark) can safely realize a
  compiled contract. It explicitly treats Plan IR as "a candidate
  representation of the compiled implementation contract rather than a
  parallel semantic or authority owner" — the two documents already state
  their own relative ownership correctly.

**High-value connection this synthesis surfaces — corrected from an earlier,
overstated version.** Decision-gated compilation's Stage 3 requires comparing
a compiled contract against "the actual patch and implementation reasoning"
for one implementation pass and classifying each divergence, without
specifying how "the actual patch" is derived. An earlier pass at this
document credited Candidate Trajectory as "exactly" this mechanism; that
overstated its scope, verified by rereading Stage 3's own text (it describes
one contract, one implementation pass, one comparison). For the *first*
implementation candidate, "the actual patch" is baseline→C1 — already
review-subject/code-change-profile's existing deterministic diff, not
Candidate Trajectory's distinctive contribution. Candidate Trajectory's own
primitive (predecessor→successor deltas: C1→C2, C2→C3, …) supplies the
mechanism specifically once remediation begins (seam
`physical-evidence.supplies.plan-deviations`, `ALIGNED`, corrected scope) —
and there it may be a *stronger* fit than the original claim, not a weaker
one: Stage 3's classification (valid discretion vs. mechanical error vs.
unauthorized drift) is easier to make against the exact sequence of
review-driven revisions than against one collapsed final diff. Candidate
Trajectory does not and should not perform the semantic classification either
way — that remains decision-gated-compilation's judgment, consistent with
Candidate Trajectory's own host-owned-facts / model-owned-judgment
discipline. Both halves (review-subject for the first pass, Candidate
Trajectory for remediation) are genuine, previously unstated `SUPPLIES`
relations; neither document currently claims either one.

### 2.3 Orchestration: work at portfolio scale

Primarily workflow-plane, depending on service-owned durable state.
`hierarchical-planning-and-multi-supervisor-orchestration.md` proposes the
same planner/executor relationship recursively at two levels
(preplanner:orchestrator :: planner:supervisor), decomposing one campaign into
coherent branches, each with its own bounded planner, supervisor, and
builder, integrated through explicit dependency-release evidence and
declared integration workstreams rather than an orchestrator that infers
readiness.

This document correctly declines to own realization ("provider choice does
not redefine role semantics or authority... the exact provider-routing
policy is a replaceable runtime concern" — seam
`hierarchical-orchestration.defers-realization-correctly`, `ALIGNED`),
deferring cleanly to §2.1.

**Open seam, with a candidate direction worth recording (not resolved here):**
it does not state whether its own "branch plan" artifact is the same object
as decision-gated compilation's "sealed decision set" + "implementation
contract," or a distinct artifact those nest inside (seam
`decision-gated.vs.hierarchical-orchestration`, `UNRESOLVED`). A plausible
shape treats the branch plan as strictly upstream, not identical:

```text
orchestration plan
      |
      v
branch objective / branch plan
      |
      v
decision surface -> sealed decisions -> implementation contract(s)
      |
      v
slice execution
```

Under this shape a branch may contain multiple implementation contracts,
preserving hierarchical orchestration's actual concern — what coherent
workstream exists and how it relates to others — without giving it
implementation-compilation's authority. This is a candidate composition, not
adopted by either source document, and this synthesis does not resolve the
seam on their behalf.

**A structural observation, not yet acted on:** its own durable-state list
(§16 — orchestration-plan revision, branch-plan revisions, dependency state,
released artifacts, ...) is a closed field enumeration doing exactly what
§2.4's open service-state map already generalizes (seam
`orchestration-state.converges-on-coordinate`, `ALIGNED`). This is the third
independent convergence on the coordinate pattern this session has found —
after `strategic-reconciliation.mjs`'s `continuity` vocabulary and the
`claim-evidence`/`review-episode`/campaign-state kernel-primitive family —
strengthening the case that §2.4's coordinate model, not a bespoke list per
subsystem, is the right generalization for orchestration state too. It is
also the **fourth** independent instance of a more specific pattern worth
naming directly: durable revisioned state, an admitted transition, optimistic
concurrency (CAS/fencing), and an authoritative successor identity, now found
in `claim-evidence`'s ledger, `review-episode`, `slice-campaign`'s campaign
state, and decision-gated compilation's decision set (§2.2). Four independent
implementations converging on one shape is evidence a kernel-level primitive
exists here, not a reason to design one prospectively — matching the
discipline already applied to the first three instances in
`service-plane-reconciliation.md`.

### 2.4 Evidence and history: coordinates over owned service state

Spans all three planes — this is why it was miscast as "layer 5 on top of
layer 1" in the earlier version of this document; it is the durable record
every plane and every other functional system writes into and reads from,
grounded in the substrate rather than stacked above it.

`revisioned-research-and-execution-architecture.md` proposes the
**reproducible reasoning coordinate**: enough durable state to reconstruct
the decision environment a model's next judgment was made from, with
adequacy judged claim-relative rather than by one intrinsic fidelity score
(§27). Its own coordinate field list includes a deliberately vague
"world-state basis and coverage" entry.

`service-plane-and-kernel-domain-boundary.md` §5 (already reconciled)
concretizes that field as an **open map of revision-bound service-state
references** — `service_state: {claim-evidence: {...}, workspace-coordination:
{...}, code.review-subject: {...}}` — so a future domain can add entries
without the coordinate schema itself changing. §2.1's realization/
implementation-revision/contract-revision distinction is what lets a
coordinate answer three separate questions about one result: what was
promised, what deterministic machinery produced it, and what model performed
any judgment involved.

The **Candidate Trajectory family**
(`candidate-trajectory-durable-foundation.md` and its two consumers) is the
first fully-worked *proposed* code-domain service family designed against
this substrate — an architecture idea, not implemented runtime behavior: a
durable predecessor/successor binding between candidate checkpoints, with
a profile delta and a candidate delta as two distinct deterministic
products — neither of which requires the kernel to understand Git, symbols,
or AST diffing. Its own bindCandidate-time event ("candidate binding") is
already named, independently, as an example committed semantic event in
`scoped-workflow-event-surface.md` (seam
`candidate-binding.already-named-event`, `ALIGNED`) — a connection recorded
but deliberately not drafted further, per that family's own amendment sheet.

`deterministic-refactor-pressure-from-work-engine-evidence.md` is the
longitudinal consumer of Candidate Trajectory's within-slice evidence,
already amended (per `candidate-trajectory-upstream-amendments.md` §A) to
source its Rework dimension from Candidate Trajectory rather than deriving it
independently, while Historical Recurrence remains a separate cross-slice
aggregation step.

`scoped-workflow-event-surface.md` proposes the transport this evidence
should move through: committed semantic events, advisory progress events,
and execution telemetry events, kept distinguishable, with authoritative
events owned by whichever service commits them — the same authority rule
service-plane and claim-evidence already enforce (seam
`event-surface.supplies.service-operation-evidence`, `ALIGNED`). A committed
`ServiceOperation`'s evidence output is naturally an instance of this
document's "committed semantic event" family, though neither document states
this yet, since the event-surface idea predates the service-plane work by
several days.

`incremental-terminal-accounting-projection.md` proposes exactly the
incremental-projection pattern this whole system depends on (admit an event
when it occurs, maintain a deterministic projection, never infer a zero for
missing instrumentation), applied specifically to terminal receipt
accounting. **This is not yet reconciled with the general event surface
above** — its own migration sketch proposes a dedicated "append-only
accounting-event store and deterministic projector," which risks becoming a
second, parallel transport mechanism if the scoped event surface is ever
built (seam `event-surface.vs.terminal-accounting`, `COUPLED_DECISION`). A
plausible shape treats the scoped event surface as the owning transport, with
terminal accounting as one projection built on top of it, rather than two
parallel append-only stores — but this family does not resolve which owns
the underlying transport, and this synthesis does not decide it here.

## 3. How the axes compose

The four functional systems do not stack in a pipeline; they share the same
substrate and write into the same evidence system. The clearest composed path
through them, for one piece of work:

```text
                              OPERATOR
                                 |
                +----------------+----------------+
                |                                 |
        ORCHESTRATION                      OperatorProjection
   preplanner/orchestrator,                        |
   branch planner/supervisor            reads realization + policy state
                |
     +----------+-----------+
     |                      |
PLANNING/COMPILATION    REALIZATION
Plan IR, decision-      ProviderTurnPort / HarnessRuntimePort
gated compilation,             |
evidence-calibrated     materialized realization
     |                         |
     +-----------+-------------+
                 |
                 v
     builder / supervisor / reviewer execution
                 |
                 v
     EVIDENCE/HISTORY
     coordinate: service_state refs into substrate services
     (claim-evidence, review-subject, candidate-trajectory,
      workspace-coordination, ...)
                 |
                 v
     SUBSTRATE: kernel / service / workflow planes
```

Planning/compilation and realization sit side by side rather than one above
the other because they answer different questions about the same piece of
work — how much structure it needs, and which mechanism executes it — and
the `routing.vs.admission` seam (§2.1) is precisely the unresolved question of
how those two questions compose. Orchestration invokes a fresh
planner/supervisor pair per branch without owning what happens inside one
(§2.3). Evidence/history is not a pipeline stage above or below anything —
every box in this diagram writes into it and can be reconstructed from it,
grounded entirely in the substrate.

## 4. What this document does not decide

- It does not resolve any `UNRESOLVED` or `COUPLED_DECISION` seam recorded in
  `architecture-direction-seam-map.md` — those remain open for their
  respective document owners. The candidate compositions sketched in §2.1,
  §2.3, and §2.4 are directions worth recording, not decisions; none of them
  are adopted by this document or by the source documents they concern.
- It does not select a permanent Plan IR schema, resolution-level count,
  execution-profile dimension list, orchestration-plan schema, or
  service-state coverage vocabulary.
- It does not authorize decision-gated compilation's Spark routing, Plan
  IR's pilot, hierarchical orchestration's incremental rollout, or any
  Candidate Trajectory consumer's default-reliance policy. Each retains its
  own acceptance criteria and evidence obligations, stated in its own
  document.
- It does not merge Plan IR, decision-gated compilation, or hierarchical
  orchestration into one artifact. Each remains independently owned; this
  document only states how they compose.
- It does not decompose this family into implementation proposals (intake
  method §12.4/§13) — that is deliberately deferred, matching the discipline
  the service-plane pass held to.
- It does not propose designing the repeated revisioned-state primitive named
  in §2.3 (durable state + admitted transition + CAS/fencing + authoritative
  successor). Four independent instances is evidence to keep watching, not a
  primitive to build now.

## 5. Open questions carried forward

These are the seam map's `UNRESOLVED`/`COUPLED_DECISION` entries, restated as
the concrete agenda for whoever picks this family up next. Where §2 above
sketches a candidate resolution, it is noted — but remains a direction, not
an answer either document has adopted.

1. Does executor-class routing (Sol/Spark) sit inside or above realization
   admission, and which document's authority governs the composition?
   Candidate direction: requirement/strategy → resolution/admission →
   realization, per §2.1.
2. Is hierarchical orchestration's "branch plan" the same artifact as
   decision-gated compilation's sealed decision set + implementation
   contract, or do these three need an explicit correspondence? Candidate
   direction: branch plan strictly upstream, with one branch potentially
   containing multiple implementation contracts, per §2.3.
3. Should the scoped workflow event surface and incremental terminal
   accounting share one underlying event-transport mechanism, and if so,
   which owns it? Candidate direction: scoped event surface as the owning
   transport, terminal accounting as a projection on top of it, per §2.4.
4. Should the execution-profile-scoring and planner-owned-characterization
   addenda be merged into one canonical dimension list?
5. All of `service-plane-and-kernel-domain-boundary.md` §10's still-open
   questions remain open and are not restated here — see that document.

## Relationships

| Direction | Relationship |
| --- | --- |
| [`architecture-direction-seam-map.md`](architecture-direction-seam-map.md) | The seam-by-seam evidence this narrative synthesizes. Every relational claim above traces to a specific seam record there. |
| [`service-plane-inventory.md`](service-plane-inventory.md), [`service-plane-and-kernel-domain-boundary.md`](../ideas/pending/service-plane-and-kernel-domain-boundary.md), [`service-plane-reconciliation.md`](service-plane-reconciliation.md) | The fixed, already-reconciled substrate §1's responsibility planes rest on. |
| Each constituent idea document (ports, capability-resolution, Plan IR + addenda, evidence-calibrated resolution, decision-gated compilation, hierarchical orchestration, revisioned research, Candidate Trajectory family, Refactor Pressure, scoped events, terminal accounting) | Each retains its own authority, acceptance criteria, and non-goals. This document does not supersede any of them; it states how they fit together. |
| `incremental-architecture-intake-and-seam-reconciliation.md` | The method this whole exercise (baseline → prospective → seam map → reconciliation) followed, for both the service-plane pass and this broader one. |

## Non-goals

- This document is not itself a proposal and authorizes no implementation.
- It does not establish decision authority, priority, or sequencing among
  the three planes or four functional systems beyond what their own
  documents already state.
- It does not claim completeness — it covers the family named in this
  intake session, not every pending idea in the repository.
- It does not treat any functional system's internal design (e.g. Plan IR's
  exact schema, the orchestration plan's exact YAML shape) as settled; those
  remain each document's own open work.
- It does not claim the three-plane / four-system framing is exhaustive or
  final. It replaces one demonstrated category error (five sequential
  layers); it may itself need revision under further reconciliation.
