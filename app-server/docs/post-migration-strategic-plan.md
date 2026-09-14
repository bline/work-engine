# Post-migration strategic plan

## Status and decision basis

Central planning index, initially drafted 2026-09-12. The operator subsequently
approved the bounded migration-exit amendment and inclusion of IR with its
required pilot, subject to the supplied review clarifications. The
[approved migration amendment](skills-migration-plan.md#approved-pre-s14-roadmap-amendment--2026-09-12)
owns that change: it separates status from priority, preserves the pause and
exact acceptance gate, and exposes S17's exit-obligation register. Detailed
successor staging below remains a planning hypothesis, not an accepted pilot or
implementation plan. Source ideas and separately authorized repairs retain
their meaning, status, and owners.

The operator's priorities for this planning session are:

- Finish the App Server migration, with context-lifecycle repairs dogfooded
  through ongoing work.
- Prepare to use substantially more subscription capacity next month. The
  operator described an intended $200 Pro 20x purchase; availability and usable
  capacity are planning assumptions supplied by the operator, not verified
  product facts or spending authorization.
- Advance toward replaceable provider/harness/operator boundaries and revisioned
  research and execution.
- Consider hierarchical concurrent workflows as an accelerator.
- Reconcile relevant pending and legacy ideas before choosing implementation
  stages, including the unimplemented architecture-intake direction.
- Treat Structural Plan IR and its required pilot as first-class roadmap work,
  including serious consideration of the two uncommitted evidence-calibrated
  plan-resolution documents. This direction does not itself accept their designs
  or authorize pilot execution.

**Updated 2026-09-14, after the sequel reconciliation queue completed. The
operator now expects the Pro 20x capacity increase imminently — a planning
assumption, unchanged in evidentiary status from the original 2026-09-12
framing above, not a verified product fact (see "Priority:
hierarchical planning and the claim-evidence impact/refresh vertical" below
and the new strategic planning handoff at the end of this document).** The
operator's stated priority is now explicit: move to hierarchical
planning/workflow and complete the claim-evidence impact/refresh vertical as
basic working blocks, so that once expanded Codex
capacity is available, there is enough well-scoped, parallelizable,
ready-to-execute work to sustain development and expend the full weekly
session quota — not be capacity-constrained by a lack of ready work or by a
single-threaded execution model. This does not replace the priorities above;
it sequences and sharpens them.

**Original 2026-09-12 evidence cutoff** (superseded for anything touched by
the 2026-09-14 revision — see that revision's own evidence cutoff at the end
of this document, under "Strategic planning handoff" → "Revision
2026-09-14"): repository `714741ad4b02157b38c2339931d176043d2e846b` and
directly inspected documentation on 2026-09-12. This pass establishes documented
intent and candidate relationships, not implementation completeness or current
campaign terminal status. No campaign terminal receipts were audited. Graph
coverage metadata generation `2026-09-12T14:36:35Z` reported no recorded issues
and matching metadata for the source documents checked; that is best-effort
coverage, not proof of completeness. Do not read this document as a single,
uniformly-714741-based assessment — sections marked 2026-09-14 rest on the
later cutoff.

Follow-up reconciliation: the operator confirms migration is paused before S14
while approved PPCE-03/04 remediation and ongoing context repairs proceed. The
[post-S13 handoff](../../planning/production-path-claim-evidence-strategic-handoff.md)
records S13 accepted and published at `8abc57ba9b140b1c30367364572c8f10b55da842`
with research residuals remaining; the Git commit exists. It preserves S13's
acceptance and places PPCE-03/04 before S14 acceptance. The
[plan decision](../../planning/production-path-claim-evidence-plan-decision.json)
records exact PPCE plan acceptance. These are historical decision records, not
a fresh audit of the active repair campaign's terminal state.

## The destination and its enabling work

[Provider/harness/operator separation](../ideas/pending/provider-turn-harness-runtime-and-operator-projection.md)
describes how execution mechanisms can be independently composed while Work
Engine retains authority. Its companion
[runtime-realization idea](../ideas/pending/pre-indexed-capability-resolution-and-frozen-runtime-realization.md)
owns capability observations, operator policy, admission, invalidation, and
immutable realizations. These are one architectural family with distinct owners.

[Revisioned research and execution](revisioned-research-and-execution-architecture.md)
describes the larger consequence: externally owned decision state can support
role reconstruction, then separately governed experiments. It remains a
conceptual reference. Recovery coordinates do not alone establish experimental
validity, and context-lifecycle repairs do not alone implement coordinates.

[Hierarchical orchestration](../ideas/pending/hierarchical-planning-and-multi-supervisor-orchestration.md)
is an accelerator for coherent independent workstreams. It can reuse the
migrated execution substrate; the full provider-neutral architecture need not
be complete before a bounded concurrency pilot. Conversely, implementing the
entire orchestration hierarchy before useful product work would incur cost
before establishing its throughput benefit.

## Priority: hierarchical planning and the claim-evidence impact/refresh vertical as basic working blocks

**Added 2026-09-14.** The operator's priority for the post-migration period is
explicit: move to hierarchical planning/workflow and complete the
claim-evidence impact/refresh vertical, ahead of other reconciled residues.
"Complete the impact/refresh vertical" is deliberately narrower than "finish
the claims system" — the claims system's core substrate is already built;
what remains is one specific vertical within it, detailed below. The
reason is a capacity argument, not a pure architecture preference — a Pro 20x
Codex account is expected to be available by then, and the goal is to sustain
development and expend the full weekly session quota rather than be
bottlenecked by a single sequential workflow with too little ready,
well-scoped work queued behind it. These two priorities were chosen because
each directly targets one half of that bottleneck:

```text
hierarchical planning/workflow
    -> the mechanism for running many concurrent, independently-authorized
       branches instead of one sequential thread -- the actual lever for
       consuming a large capacity increase

claim-evidence impact/refresh vertical
    -> the shared substrate a large fraction of the queue's reconciled
       residues already depend on (review findings, proposal-research
       evidence, evaluation estimates, readiness contracts, evidence-anchor
       nomination) -- completing its one still-missing operation unblocks
       the largest number of downstream units of work at once
```

**What "complete the impact/refresh vertical" concretely means.**
`claim-evidence`'s core substrate (identity, evidence, revision/lineage, the
`proposal-research-v1` and `revision-bound-review-finding-v1` domain
profiles) is already real, implemented code — see
[`work-engine-planned-architecture.md`](work-engine-planned-architecture.md)
§12. What is not built is the one piece
[`claim-evidence-service.md`](claim-evidence-service.md)'s own design
identifies as its next step: the `nominate_impact`/refresh-episode operation
(no such operation exists yet in `contract.mjs`, confirmed directly by
[`evidence-anchor-observation-and-impact-nomination.md`](../ideas/pending/evidence-anchor-observation-and-impact-nomination.md)
§6). Its own §6 distinguishes what that gap actually blocks from what does
not need to wait for it:

```text
independently buildable now, not blocked
    EvidenceAnchorObserver family (one adapter per anchor kind)
    AnchorObservation contract
    shadow observations against explicit test/input anchors -- may run
        report/shadow-only until the operation below lands

blocked on the nominate_impact/refresh-episode operation landing
    the PRODUCTION anchor registry (durable, revision-bound dependency
        declarations)
    durable ImpactNomination records
```

This sharpens the scheduling argument rather than weakening it: there is
real, parallel work available on the observer/registry side right now, not
a single hard dependency chain. The same `nominate_impact` gap also sits
underneath the readiness contract (item 5) and the comparison-contract
mechanism (item 6) treating freshness as an input. Completing this one
operation remains disproportionately high-leverage relative to its size.

**Hierarchical planning: what stands in the way.** Unlike the impact/refresh
vertical's gap, this is not a small missing piece — it is a full idea in
`app-server/ideas/pending/`, not yet formed as a proposal, with two open
seams the pre-queue synthesis already found and did not resolve
(`architecture-direction-synthesis.md` §5, items 1 and 2): whether
executor-class routing sits inside or above realization admission, and
whether hierarchical orchestration's "branch plan" is the same artifact as
decision-gated compilation's sealed decision set. Both need an explicit
answer before hierarchical orchestration can be built without creating a
second, competing authority for the same question. This is a genuine
prerequisite, not busywork: running many concurrent branches without a
settled admission/routing boundary risks exactly the kind of authority
collapse this session's reconciliation work spent all day preventing
elsewhere.

**Sequencing implication.** Neither priority is blocked on the other, so
they can proceed in parallel: the impact/refresh vertical is a bounded,
specified build (with the observer/registry sub-piece already available in
parallel, per above); hierarchical planning needs its two open seams
resolved (a design/reconciliation task, not yet a build) before
implementation starts. Neither priority is itself one of the reconciliation
queue's eleven queue items with a surviving residue (per the corrected
accounting in "Legacy ideas" above — twelve queue items, one with no
residue, eleven with one) — all eleven remain real, accepted, authorized
work, but are secondary to these two until the operator's capacity increase
is closer.

## Recommended staging hypothesis

These stages express useful release consequences, not a compulsory execution
procedure. Exact implementation dependencies remain to be verified.

**2026-09-14 qualifier:** this table remains a release-consequence hypothesis
from the original 2026-09-12 plan and is not rewritten here. Under the
2026-09-14 revision, claim-evidence impact/refresh completion and
hierarchical-orchestration seam resolution are priority workstreams that may
proceed alongside the relevant stages below; the table does not override
that reprioritization, and the table's own stage sequence does not place the
impact/refresh vertical anywhere within it.

The current migration pause remains effective. PPCE-03/04 gates S14 acceptance,
not automatically all S14 planning or implementation. Each lifecycle repair
blocks the specific activity whose required behavior it prevents. Planning for
concurrent workflows and the pilot follows its actual evidence and authority
requirements rather than a blanket dependency on the preceding table row.

| Stage | Intended result | Evidence that would justify advancing |
| --- | --- | --- |
| 0. Reconcile the migration exit | An exact accepted App Server baseline, repair dispositions, and a short remaining operational backlog | Migration terminal evidence plus references to actual lifecycle dogfood results; unresolved cases remain visible |
| 1. Establish the minimum shared boundaries | Existing runtime paths described through explicit ownership and realization identity, with attributable workflow progress | A current role runs under an exact realization and its activity can be followed through the operator projection without changing workflow authority |
| 2. Prove useful concurrency | Two bounded workflows with separate supervisors, explicit dependencies, resource admission, and an integration owner | Both complete under isolated authority; a wait or failure in one does not corrupt the other; recovery and operator decisions remain attributable |
| 3. Prove reconstructable execution | One real role can resume from revisioned owned state with later effects and queued input reconciled | A bounded recovery vertical establishes continuity and exposes missing state without depending on an improvised transcript summary |
| 4. Turn a qualified coordinate into research | One controlled comparison over isolated descendants | A separately owned experiment specification, research authority, adequate fixture coverage, and interpretable outcome evidence |

Stage 1 should establish only the seams needed by the first real consumers.
Dynamic runtime selection, every alternative harness, a new UI, a generic
tool loop, and comprehensive failover are not assumed prerequisites for stage 2.
Stage 3's design and evidence gathering can proceed alongside stage 2 where
their shared contracts permit it.

The IR workstream below runs alongside these stages. Its bounded pilot is early
post-migration work, not dependent on completing stage 4 or a general replay
platform. Pilot design and source reconciliation can begin during this planning
session; execution needs its own admitted basis and authority. Production use of
resolution-based routing depends on its evidence. Basic independent workflows
need not wait for adaptive IR routing, but partitioning a single IR across
executors requires explicit composition and integration evidence.

An initial concurrency demonstration could pair a bounded operational repair
with isolated analysis for the reconstruction pilot. Two mutation-heavy branches
are useful only after their scope and integration consequences are understood.
The measure of acceleration is accepted work per unit of operator attention,
including coordination, review, recovery, and integration cost—not agent count.

## Operational backlog: reconcile before scheduling

Pending-file presence does not establish an unfixed bug. The following sources
mix incident evidence, emergency implementations, and proposed generalizations.

| Source | Candidate disposition to investigate |
| --- | --- |
| [Context emergency and writable ingress](../ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md) | Compare remaining emergency recovery, atomic ingress, and policy-conditioned tool projection against repairs actually accepted during migration; keep their separate outcomes |
| [Generation maintenance rollover](../ideas/pending/executable-generation-maintenance-rollover.md) | Identify the remaining first-class maintenance consequence after the documented emergency helper and reload repair |
| [Scoped workflow events](../ideas/pending/scoped-workflow-event-surface.md) | Prioritize the bounded observation surface needed to operate concurrent workflows |
| [Incremental terminal accounting](../ideas/pending/incremental-terminal-accounting-projection.md) | Align event identity with the runtime/event boundary; reduce repeated terminal forensic work without merging accounting and transport ownership |
| [Semantic inventory versus integrity](../ideas/pending/semantic-inventory-integrity-projection-separation.md) | Reassess after migration cutover; retain only remaining shared-artifact churn and integrity consequences |
| [Contract kit and lifecycle duplication](../ideas/pending/contract-kit-and-lifecycle-template-duplication.md) | Separate the reported correctness inconsistency from optional utility cleanup; verify both against the exit baseline |
| [OAuth credential broker](../ideas/pending/fenced-oauth-credential-tip-broker.md) | Determine whether the selected concurrent Claude realization still exposes the credential-refresh race; if so, safe credential ownership precedes that use |

The [migration plan's runtime-binding and progress residuals](skills-migration-plan.md#post-inhabitation-runtime-binding-and-progress-residuals)
also belong in this reconciliation. A documented desired replacement command or
progress surface is not proof that the residual remains open today.

## Bounded architecture-intake exercise — completed

**Marked complete 2026-09-14.** The exercise this section originally
proposed (2026-09-12) has since run to completion, in two passes using the
same [incremental architecture intake and seam reconciliation](../ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md)
method this section named:

- **The originally-named "first bounded family"** (runtime ports,
  materialized realization, decision-gated compilation, Plan IR and
  resolution, hierarchical orchestration, scoped events, and the recovery
  requirements of the revisioned architecture) was integrated into
  [`architecture-direction-seam-map.md`](architecture-direction-seam-map.md)
  and [`architecture-direction-synthesis.md`](architecture-direction-synthesis.md).
  Hierarchical orchestration's own open seams from that pass are exactly
  what the priority section above still needs resolved before
  implementation.
- **A separate, broader pass** reconciled the root `ideas/` legacy family
  ([`root-ideas-reconciliation.md`](root-ideas-reconciliation.md)) and then
  a twelve-item, four-wave sequel queue (the "Legacy ideas" section above),
  culminating in
  [`work-engine-planned-architecture.md`](work-engine-planned-architecture.md)
  — the disposition-and-dependency table this section asked for, in
  substance: §12 is the implementation-state map, §13 is the remaining
  open seams, each with source revision, current evidence, remaining gap,
  and what would resolve it.

A future reader should treat this section as historical record of what was
planned, not as an outstanding task — do not re-schedule this exercise.
The original text is preserved below for that record.

<details>
<summary>Original 2026-09-12 text (historical; superseded by the completed exercise above)</summary>

The likely reference for the operator's multi-session intake idea is
[incremental architecture intake and seam reconciliation](../ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md).
It proposes multiple ideas integrated within one prospective architecture and
successive sessions anchored to accepted implemented baselines. Its initial
design avoids overlapping dependent prospective baselines. This identification
remains subject to operator correction if another idea was intended.

Use that approach as a planning exercise now, without first implementing a new
intake service. Initial synthesis can identify candidate seams during migration;
an implementation-ready packet will need reconciliation against the actual exit
baseline.

The first bounded family is runtime ports, materialized realization,
decision-gated compilation, Plan IR and resolution, hierarchical orchestration,
scoped events, and the recovery requirements of the revisioned architecture.
Intake contributes the method. This broader family needs bounded projections
and staged pilots rather than one implementation batch. The revisioned
architecture contributes consumer requirements rather than turning its entire
scope into the first implementation batch.

The useful next planning artifact is a disposition-and-dependency table. Each
entry should identify its source revision, desired outcome, current evidence,
remaining gap, relevant destination, dependency consequence, shared decision,
and suggested stage. Candidate dispositions include already satisfied,
operational repair, shared foundation, bounded pilot, and deferred. These are
planning labels, not new product lifecycle states.

The central question is: which shared meanings need agreement before independent
branches can be planned? Leading candidates are realization identity, role and
workflow identity, dependency-release evidence, event ownership, and the recovery
frontier. Their implementation representation remains open.

</details>

## Legacy ideas: superseded by the completed sequel reconciliation queue

**This section is stale as originally drafted (2026-09-12) and is corrected
here, 2026-09-14, rather than rewritten from scratch — the shortlist below
predates the actual reconciliation work and both its grouping and its
coverage turned out to be wrong in places.**

The corrected accounting: **twelve queue items, thirteen source idea files**
(seam review and architectural review were reconciled jointly as one queue
item). This section named six of the twelve; the other six existed and were
not included. All twelve have since been reconciled against real app-server
implementation, one at a time, each producing its own
`app-server/docs/*-reconciliation.md` document with a disposition table and
an explicit acceptance record. Of the twelve: **one has no surviving
residue** (already built and dogfooded); **eleven have a surviving residue**.
The joint seam-review/architectural-review item is not evenly one or the
other — it contains both an implementation-authorized mechanical residue
(the seam-evidence adapter extensions) and a design-only semantic residue
(the correspondence-judgment owner and the architectural-finding profile),
so "implementation-ready" and "design-only" below are overlapping
authorization buckets across those eleven residues, not a partition of the
twelve queue items. The authoritative index is
[`work-engine-planned-architecture.md`](work-engine-planned-architecture.md)
§12 (implementation-state map) and §13 (remaining open seams) — this section
now only summarizes, and defers to that document on any conflict.

Two corrections to this section's original pairings, found during
reconciliation rather than assumed here:

- **Control plane and client protocol + review-scope coordination were
  paired by theme (both review/control-plane-adjacent), not by an actual
  discovered seam.** Reconciliation found their residues independent: item
  1's is fenced active-binding coordination for logical role instances;
  item 2's is prospective mutation-admission gating against active review
  state. Both are `ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION`, but treat them
  as two separate units of work, not one paired workstream.
- **Research maturity/freshness + closed-loop learning were paired by
  theme (both evidence-over-time), not by the actual dependency found.**
  Closed-loop learning's confirmed evidence supplier is evaluation
  (`evidence-backed-proposal-evaluation.md`), not research maturity
  directly. Research maturity's own residue (a decision-specific readiness
  contract) is `ACCEPTED_AUTHORIZED_FOR_IMPLEMENTATION`; closed-loop
  learning's residue (the prediction↔outcome correspondence contract and
  calibration diagnosis) is `ACCEPTED_AUTHORIZED_FOR_DESIGN` — it depends on
  evaluation's contract existing first, not on research maturity.

The full, corrected set of the eleven residues (across twelve queue items,
one reconciled jointly, one with no residue and so not listed below), with
the same distinction the planned-architecture capstone uses:

**Authorized directly for implementation** (specified enough to build
against — see each reconciliation's own "## Acceptance" section):
[control-plane-and-client-protocol](control-plane-and-client-protocol-reconciliation.md),
[review-scope-coordination](review-scope-coordination-reconciliation.md),
[proposal-research-maturity-and-freshness](proposal-research-maturity-and-freshness-reconciliation.md),
[evidence-backed-proposal-evaluation](evidence-backed-proposal-evaluation-reconciliation.md),
[proposal-backed-portfolio-selection](proposal-backed-portfolio-selection-reconciliation.md),
and the mechanical seam-evidence adapter extensions from the
[seam-review/architectural-review joint reconciliation](cross-cutting-seam-review-and-architectural-review-reconciliation.md).

**Authorized for design work only**, each with a named blocking decision
(see the linked reconciliation's own "## Acceptance" section for the exact
condition):
[organizational-execution-envelopes](organizational-execution-envelopes-reconciliation.md)
(the queue's one genuinely new architectural level — see the priority
section below),
[role-decision-trace](role-decision-trace-reconciliation.md),
the semantic-judgment-owner and architectural-finding-profile halves of the
[seam-review/architectural-review joint reconciliation](cross-cutting-seam-review-and-architectural-review-reconciliation.md),
[closed-loop-engineering-learning](closed-loop-engineering-learning-reconciliation.md),
[ui-review-capability](ui-review-capability-reconciliation.md), and
[work-engine-studio](work-engine-studio-reconciliation.md).

**No residue, already built and dogfooded:**
[agent-instruction-structure-and-placement-review](agent-instruction-structure-and-placement-review-reconciliation.md)
— nothing to schedule.

## Plan IR, resolution, and capability learning

The operator's follow-up makes this a first-class roadmap workstream. It connects
the two destinations: implementation meaning and projections can be bound to
runtime realizations, and their outcomes can become evidence for revisioned
research and future admission. Its utility is broader than cheaper executors:
plan fidelity, decision ownership, review, reconstruction, and integration may
remain valuable even if a lower-cost execution strategy fails its pilot.

| Source | Distinct contribution |
| --- | --- |
| [Decision-gated compilation](../ideas/pending/proposal-decision-gated-implementation-compilation.md) | Material decision closure, reserved/delegated authority, implementation-contract workflow, conformance, and baseline measurement |
| [Structural Plan IR](../ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md) | Candidate canonical representation and faithful executor-specific projections |
| [Prior-art study](../ideas/pending/structural-plan-ir-prior-art.md) | Candidate revision/node identity, layering, lineage correspondence, and projection fidelity; not an accepted schema |
| [Planner characterization](../ideas/pending/structural-plan-ir-addendum-planner-owned-execution-characterization.md) | Derived description of the remaining judgment surface, with incremental characterization cost measured separately |
| [Execution-profile scoring](../ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md) | Evidence-driven strategy comparison by total accepted-work consequence; initial scores do not automatically route |
| [Evidence-calibrated resolution](../ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md) | Dimension-specific projections, additional decision-closure cost, context burden, continuation economics, composition, and continuous learning |
| [Upstream amendments](../ideas/pending/evidence-calibrated-plan-resolution-upstream-amendments.md) | Candidate changes across compilation, IR, runtime observations, and research coverage; not already applied to those owners |

The last two sources were untracked working-tree documents when inspected on
2026-09-12. Their complete text was read as substantive planning input. Their
status does not diminish the ideas' relevance, but exact source revisions will
need binding before a later proposal or pilot relies on them. No source document
was edited or committed by this planning pass.

Recommended workstream stages:

1. Reconcile shared meaning and pilot design: implementation contract versus
   projection, material decision versus faithful rendering, characterization
   versus executor capability, and branch independence versus safe integration.
   Use the prior-art questions to bound schema investigation rather than accept
   a general IR schema in advance.
2. Establish baseline and shadow evidence using the decision-gated track's
   measurement, decision-surface, sealed-decision, and compiler investigations.
   Preserve distinctions between prospective and outcome-informed evidence.
3. Run the required bounded representation/execution pilot. Coordinate the
   decision-gated track's direct/compiled execution comparison with Plan IR's
   current/structural and compact/elaborated comparisons. Compatible subjects
   and measurement semantics can connect separate experiments; a full Cartesian
   matrix is not presumed necessary. Preserve applicable review requirements
   from the admitted pilot, including the candidate track's initial Sol review.
4. Decide what survives: retain, revise, or reject representation, projection,
   characterization, and executor hypotheses separately. Expand task coverage
   only where results justify it. Pilot completion alone is not routing approval.
5. If admitted, collect production observations, investigate contradictions
   through targeted pilots or qualified replay, and propose evidence-backed
   policy revisions through existing authority owners.

The uncommitted extensions materially shape the pilot:

- Compare simple resolution presets with a bounded dimension-specific profile;
  do not freeze their illustrative vector into product ontology.
- Separate additional decision closure from rendering already-owned meaning.
  A renderer cannot silently resolve material choices. Test whether closure-cost
  predictions are useful before consulting them for routing.
- Measure artifact production, executor ingestion, and behavioral context burden
  separately without double-counting billed usage. Include planner,
  characterization, supervision, review, repair, latency, and operator attention.
- Bind plan/projection, realization, capabilities, and continuation observations.
  Unknown cache state remains unknown; a cold replay cannot establish the cost
  of retaining an unreconstructed warm continuation.
- Attribute plan, rendering, executor, and integration failures separately.
  Dependency independence alone does not prove regions can be assigned to
  different executors and accepted jointly.
- Keep production observations distinct from controlled comparisons. Repeatedly
  favoring an incumbent may starve alternatives of evidence; exploration remains
  separately governed rather than an automatic consequence of learning.

This changes the migration recommendation only at its boundaries: S14 should
preserve its bounded role-import scope, while its planning outputs and migration
handoff are assessed for later decision/plan identity and evidence needs. The
full IR compiler, resolution policy, and learning system remain successor work.
The pilot should inform scaled orchestration and runtime-policy design rather
than arrive after those choices have become entrenched.

## Strategic planning handoff

```yaml
schema_version: 1
strategic_objective: Establish a coherent post-migration route to replaceable runtime execution and revisioned recovery/research
evidence_cutoff:
  roadmap_revision: "714741ad4b02157b38c2339931d176043d2e846b:app-server/docs/skills-migration-plan.md"
  repository_revision: "714741ad4b02157b38c2339931d176043d2e846b"
  campaign_terminals: []
continuity: retained
verdict: reorder
current_rationale: The operator's destinations and explicit IR direction favor shared runtime boundaries, a first-class compilation/IR/resolution pilot, bounded concurrency, and recovery evidence before broad adaptive execution or experimental machinery.
assumptions:
  confirmed:
    - The named runtime and revisioned architecture documents retain exploratory/conceptual status.
  changed:
    - The operator prioritizes these two destinations and anticipates substantially greater subscription capacity next month.
    - The operator explicitly requires IR roadmap consideration and a pilot, including the uncommitted resolution extensions; the family is now first-class rather than deferred for a later pass.
  invalidated: []
route_changes:
  priorities:
    - Reconcile migration outcomes before scheduling residual work from pending filenames.
    - Evaluate a bounded hierarchical concurrency pilot as an accelerator for both destinations.
    - Coordinate the decision-gated and Plan IR pilot comparisons with the uncommitted resolution and upstream-amendment hypotheses before production strategy selection.
  dependencies:
    - Pair runtime ports with their companion materialized-realization idea; ports alone do not own admission.
    - Establish recovery adequacy before relying on a coordinate for a separately specified experiment, following the revisioned architecture reference.
    - A prospective IR pilot needs its bounded evidence and authority basis, not a completed general historical-replay platform; adaptive routing needs comparative evidence and an owning policy decision.
  newly_important:
    - Scoped workflow observation and resource coordination become more valuable under the operator's concurrency objective.
  deferred:
    - Full intake automation, broad runtime choice, and general experimental machinery until bounded consumers establish their need.
recommended_campaign:
  disposition: none
  objective: null
  work_source: null
  reason: This pass recommends staging; the exit baseline and first bounded implementation packet remain to be established.
open_uncertainties:
  - Actual migration terminal state and remaining repair consequences.
  - Exact minimum shared contract required by the first two concurrent workflows.
  - Which IR and resolution hypotheses survive the pilot, and which legacy contributions remain after migration.
  - Whether the identified architecture-intake document is the operator's intended multi-session idea.
  - Actual usable capacity and operator-attention bottlenecks after the anticipated subscription change.
authority_required:
  - The operator approved the bounded migration amendment and IR/pilot roadmap inclusion; detailed successor architecture choices and pilot execution still need their owning decisions.
  - Owning architecture and campaign decisions for later concrete implementation packets; none are supplied by this handoff.
revisit_when:
  - Migration terminal evidence and lifecycle dogfood outcomes become available.
  - Seam reconciliation changes a proposed dependency or branch boundary.
  - The concurrency pilot shows coordination or recovery cost erasing its throughput benefit.
  - A recovery attempt falsifies the adequacy of its durable state projection.
  - IR pilot evidence changes representation, resolution, composition, or accepted-work economics assumptions.
  - Actual subscription capacity differs materially from the planning assumption.
```

### Revision 2026-09-14 (appended, not a replacement)

The block above is preserved as the original 2026-09-12 handoff. This is a
new revision reflecting the sequel reconciliation queue's completion and the
operator's explicit reprioritization; it does not rewrite or invalidate the
prior one.

```yaml
schema_version: 1
strategic_objective: Complete the claim-evidence impact/refresh vertical and resolve hierarchical planning/workflow's open seams as basic working blocks, so expanded Codex capacity can be spent on ready, well-scoped parallel work rather than bottlenecked by a single sequential thread or missing shared substrate
evidence_cutoff:
  roadmap_revision: "0806de01b926e86017e045f00f538fe316cc94f1:app-server/docs/work-engine-planned-architecture.md"
  repository_revision: "0806de01b926e86017e045f00f538fe316cc94f1"
  campaign_terminals: []
continuity: retained
verdict: reorder
current_rationale: The sequel reconciliation queue (twelve queue items, thirteen source idea files, one item reconciled jointly) is complete and its findings and residues are accepted; the planned-architecture capstone now gives an accurate current/target picture. The operator now expects a Pro 20x capacity increase imminently (a planning assumption, not a verified fact). Given that, priority is to complete the claim-evidence impact/refresh vertical's one missing operation (small, high-leverage, unblocks several accepted residues; its observer/registry half is independently buildable now) and resolve hierarchical planning's two open admission/routing seams (larger, a prerequisite for safe concurrent-branch execution) ahead of the queue's other eleven accepted-but-secondary residues.
assumptions:
  confirmed:
    - The named runtime and revisioned architecture documents retain exploratory/conceptual status; hierarchical orchestration remains unbuilt and unformed as a proposal.
    - claim-evidence's core substrate (identity, evidence, revision/lineage, two domain profiles) is real, implemented code, not aspirational.
  changed:
    - All twelve reconciliation-queue items (thirteen source idea files) are now reconciled, accepted, and their residues authorized. Of the twelve: one has no surviving residue (already built and dogfooded); eleven have one, with the joint seam-review/architectural-review item contributing both an implementation-authorized mechanical residue and a design-only semantic residue to those eleven's authorization buckets.
    - The operator has explicitly named hierarchical planning/workflow and completing the claim-evidence impact/refresh vertical as the two priorities for the post-migration period, ahead of the queue's other eleven accepted residues — neither priority is itself one of those eleven.
  invalidated:
    - The original "Legacy ideas worth reconciling" shortlist's two thematic pairings (control-plane-and-client-protocol with review-scope-coordination; research-maturity-and-freshness with closed-loop-engineering-learning) — reconciliation found these were grouped by theme, not by an actual discovered seam.
route_changes:
  priorities:
    - Build claim-evidence's nominate_impact/refresh-episode operation (the one piece its own design still lacks) before other claim-evidence-dependent residues; its EvidenceAnchorObserver family, AnchorObservation contract, and shadow observations are independently buildable now and do not need to wait.
    - Resolve hierarchical orchestration's two open seams (executor-class routing vs. realization admission; branch plan vs. sealed decision set correspondence) before forming it into a proposal or beginning implementation.
    - Treat the queue's other eleven accepted residues (spanning implementation-ready and design-only authorization) as queued, not urgent, until capacity actually increases.
  dependencies:
    - evidence-anchor-observation-and-impact-nomination.md's own §6 distinguishes what is blocked from what is not: the production anchor registry and durable ImpactNomination records are blocked on the impact/refresh operation landing; the observer family and shadow/report-only observation are not.
    - Items 5 (readiness contract) and 6 (comparison-contract mechanism) both treat claim-evidence freshness as an input; the same operation strengthens both once built.
    - Hierarchical orchestration reuses workspace-coordination's fencing (already implemented) for cross-branch resource admission; that dependency is already satisfied.
  newly_important:
    - The distinction between "accepted meaning" and "authorized to implement now" (established during today's acceptance pass) becomes the actual scheduling mechanism once more parallel capacity exists — it tells future sessions which of the eleven residues are truly ready versus still gated on a design decision.
  deferred:
    - The eleven reconciled residues not named as this revision's priority, until the impact/refresh vertical and hierarchical-planning seam resolution are underway.
recommended_campaign:
  disposition: none
  objective: null
  work_source: null
  reason: This revision recommends re-sequencing priorities; the impact/refresh vertical's exact scope and hierarchical orchestration's seam resolution still need their own bounded implementation or design packets before a campaign is proposed.
open_uncertainties:
  - Exact scope and evidence requirements for the nominate_impact/refresh-episode operation.
  - Which of the two hierarchical-orchestration seams (routing-vs-admission, branch-plan-vs-decision-set) should be resolved first, and by which owning document.
  - Actual Pro 20x availability, usable capacity, and timing.
  - Whether the eleven secondary residues should be formed into proposals now (per the intake method's own §12.4) or left as reconciled-but-undecomposed until capacity increases.
authority_required:
  - Owning decision for each of the two hierarchical-orchestration seams remains with whichever document's authority the operator designates — not supplied by this handoff.
  - The impact/refresh operation's own implementation authorization is not established by this handoff. Accepted dependent work (items 5 and 6's own acceptance records, which name it as a dependency) corroborates the need and priority of that operation; it does not authorize it. A dependent work item cannot confer implementation authority on its own dependency. The operation's owning implementation unit still requires its own bounded scope and authorization.
revisit_when:
  - The impact/refresh operation lands, unblocking evidence-anchor's production registry and the readiness/comparison-contract residues.
  - Both hierarchical-orchestration seams are resolved, or evidence shows they require a different owner than currently assumed.
  - Actual Pro 20x capacity and usable throughput are confirmed, changing the urgency of the eleven secondary residues.
  - A future intake session (per incremental-architecture-intake-and-seam-reconciliation.md §3) admits new ideas that materially change this sequencing.
```
