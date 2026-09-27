# Evidence-Calibrated Plan Resolution and Continuous Capability Learning

## Status

Conceptual architectural direction. Not accepted, prioritized, or authorized
for implementation. The user or an explicitly authorized portfolio owner
retains decision authority.

This direction explores how execution evidence could inform the amount of
structure exposed to an executor. It extends the Structural Plan IR family
without taking ownership of planning, routing, admission, or context lifecycle.
The proposed mechanisms and economic effects need pilot evidence.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Corrected 2026-09-17, then verified again per review: not_applicable was wrong (the prior note's own words -- 'new, unaccepted, speculative MECHANISM' -- already admitted real architectural content; 'nothing owns this because it doesn't exist yet' would make all genuinely new architecture disappear from this axis). The immediately-following 'partial' finding is now ALSO corrected to none: citing evidence-and-claims.md SS10 was composition, not supersession. SS10's specific representation pattern (correspondence basis, competing/contributing explanations, evidence/confidence/limitations PER explanation, consequence linkage) does not actually appear anywhere in this document's own text -- checked directly. This document's own closest passage ('Continuous learning and authority': 'fifteen first-pass acceptances, one repair, and one authority-boundary violation... would not establish a universal capability claim or permit the severe failure to disappear inside an average acceptance rate') describes evidentiary-class PRESERVATION (don't average distinct outcome types together), a narrower and different claim from SS10's multi-explanation diagnostic-finding shape. SS10 would supply infrastructure this document's evidence COULD flow through if the mechanism were ever built and materialized as durable claims -- a real but compositional relationship, not a claim already stated. none is correct: a real architectural claim (evidence-calibrated resolution, continuous capability learning), checked against the most plausible canonical owner, with zero actual content match found."
  residue: none
  residue_note: "Corrected 2026-09-17, per re-audit using the normalized-truth/route-invariance test: item 12 (previously the sole RESIDUE survivor) is retracted and reclassified BACKLOG -- see inline tag for the split-and-recheck reasoning. Zero RESIDUE-kind items now exist in this ledger."
  backlog: present
  backlog_ledger: "12 KIND: BACKLOG items after reclassification (11 and, as of this correction, 12). Both confirmed OPEN -> backlog: present. Items 1-10 remain UNCHECKED (empirical/measurement-design questions not specifically verified this pass); no dedicated staged-plan section in this document itself (it references Structural Plan IR's own SS20 pilot and decision-gated compilation's own Stage 5, both belonging to those documents)."
  audit_scope:
    - open-question-ledger
  audit_scope_completeness: partial
  status_as_of: 2026-09-27
```

**Relationship table updated 2026-09-27** (below) following `execution-strategy-selection.md`'s admission — a pointer to that dimension's own new ownership of the "routing or policy candidate" step this document's own authority-flow diagram already named without assigning an owner, not a supersession of anything this document itself claims. `architectural_supersession` and `residue` above are unchanged and remain accurate.

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - evidence-calibrated-plan-resolution-upstream-amendments.md
    - structural-plan-ir-for-capability-aware-multi-model-execution.md
    - proposal-decision-gated-implementation-compilation.md
```

Related changes are collected in the
[upstream amendments](evidence-calibrated-plan-resolution-upstream-amendments.md).

A separately proposed Candidate Trajectory primitive may also be relevant as a
concrete observation source for this document's continuous-learning loop, at
three distinct levels (measurement, capability learning, and capability
modification). Candidate connections are collected in section B of the
[Candidate Trajectory upstream amendments](candidate-trajectory-upstream-amendments.md),
including a correction to how closure-cost evidence should be read from it.
Not accepted or applied.

## Motivation

A realization may handle local code transformation and explicit dependencies
well while struggling with repository discovery, authority boundaries,
escalation, or integration judgment. A single plan-resolution level may expose
structure the executor already handles while leaving its actual weaknesses
unaddressed.

The hypothesis is that dimension-specific projections can reduce total
accepted-work cost while preserving the implementation contract and its quality
requirements. More detail is useful only where it improves the required
consequence enough to justify its cost.

## Canonical contract and projection

The proposed relationship is:

```text
canonical implementation contract
+ execution characterization
+ realization capability evidence
+ governing routing and admission policy
→ required projection profile
→ executor-specific Plan IR projection
```

The canonical Plan IR remains the semantic source of truth. A projection
materializes established structure; it cannot silently change the contract,
invent a material decision, or remove legitimate implementation discretion.
These boundaries preserve agreement among planner, executor, reviewer, and
acceptance authority about what work was authorized.

The [planner-owned characterization addendum](structural-plan-ir-addendum-planner-owned-execution-characterization.md)
distinguishes properties of the work from executor selection. Characterization
should describe the judgment surface without encoding model names or presumed
capability. Capability evidence describes what a particular realization has
demonstrated under particular conditions.

## Resolution profiles

A profile could represent several dimensions instead of one scalar resolution:

```yaml
projection_profile:
  dependency_structure: 1
  repository_localization: 2
  invariant_explicitness: 2
  judgment_boundaries: 3
  authority_boundaries: 3
  implementation_discretion: 1
  sequencing: 1
  verification_obligations: 2
  acceptance_conditions: 3
  integration_contracts: 2
```

These dimensions and values are illustrative. Evidence should determine which
are independently useful, which interact, and whether a vector improves on
simple presets. The existing `P0`–`P3` levels may remain useful even if named
presets eventually map to profiles.

Capability evidence could be indexed by execution realization, problem
characterization, judgment dimension, projection resolution, and verification
environment. It should retain model and provider identity, harness and role,
context and projection, capability grant, reasoning controls, and evidence time
where those affect the claim.

A valid finding is that no demonstrated projection makes a realization reliable
enough for particular work. More elaboration need not compensate for every
capability limit. A stronger executor is an alternative, but agreement with
that executor is not itself an acceptance oracle.

## Cost and reliability

Three effects need separate accounting:

| Effect | What changes | Implication |
| --- | --- | --- |
| Artifact production | Model reasoning needed to form or revise a plan or projection | Deterministic rendering needs no model inference; semantic decisions may require it. |
| Context ingestion | Tokens consumed by the executor, with provider-specific cache treatment | Cheap rendering can still produce expensive input. |
| Context burden | Competition for attention, reduced capability salience, drift, and compaction risk | Token discounts do not establish that a larger context is behaviorally harmless. |

These are analytical categories, not three monetary totals to add together.
Inference is itself billed through provider usage, so economic accounting must
avoid counting the same tokens twice. Deterministic work also has runtime and
maintenance costs even when its model-inference cost is zero.

[Slice-bounded builder context](../../../ideas/slice-bounded-builder-context.md)
discusses the behavioral risks of large persistent contexts. Their magnitude
and relationship to projection resolution remain measurement questions; more
structure can also reduce reconstruction and improve comprehension.

The proposed objective is to reduce total accepted-work cost within the owning
reliability and authority requirements. Context burden needs separate quality
observations rather than an invented monetary weight. A cost saving cannot
license planner overreach or relax an acceptance condition.

## Decision closure and rendering

[Structural Plan IR](structural-plan-ir-for-capability-aware-multi-model-execution.md),
sections 4 and 9, proposes stating semantic facts once and rendering more
explicit projections from the relationships already encoded. If much of that
rendering is deterministic, the expensive variable may be additional decision
closure rather than projection resolution itself.

The relevant economic question is:

> What additional decisions must be resolved for this realization to execute
> within its contract, and does that work cost less than the expected saving
> in accepted implementation?

Canonical planning also supports review, repair, traceability, orchestration,
and plan-quality checks. Comparisons should distinguish that shared value from
additional planning performed specifically to enable a different executor.
Charging all plan formation to delegation can overstate its marginal cost.

The [decision-gated compilation proposal](proposal-decision-gated-implementation-compilation.md)
owns the material decision surface and reserved or delegated decision
authority. A closure-cost estimate would inform its routing judgment; the
estimate would not select an executor or acquire decision authority.

### Renderability as a candidate check

A projection check could identify which dimensions can be expanded from the
canonical plan without making new material decisions. Deterministic expansion
is a useful implementation where the output is fully determined.

If rendering reveals an unresolved material decision, the decision-gated
compiler's `returned_for_decision` outcome provides the relevant boundary.
Missing evidence may instead require `blocked_by_evidence`. A renderer cannot
hide either condition by silently choosing an answer.

The need for a model does not by itself prove that the plan is defective:
faithful language realization may require judgment without changing semantics.
Whether a mechanical check can distinguish these cases is a pilot question.

### Minimum sufficient resolution

Too little structure can force an executor to reconstruct missing meaning,
leading to divergence and repair. Too much can increase context burden or make
the planner perform implementation indirectly. The useful region preserves
material decisions while leaving legitimate implementation judgment with the
executor.

That region may vary by dimension. Explicit authority and acceptance boundaries
could matter more than expanded sequencing for one realization, while another
may benefit from a different profile.

## Continuation economics

Per-turn prices do not capture the cost of a continuation. Context reuse,
remaining work, review intervals, and plan changes can affect whether retaining
or changing a realization is economical.

Cache behavior belongs to the bound provider and runtime configuration.
Relevant observations may include cache read and write usage, elapsed time
between requests, prefix changes, and observed or unknown cache state. Exact
lifetimes, eligibility, and prices need validation against the provider used
for a measurement; this direction does not define provider constants.

A pause or changed prefix may reduce cache reuse even without changing models.
Supervision can therefore affect both direct review cost and subsequent input
cost. That coupling is a hypothesis to measure, not a claim that every pause
makes a continuation cold.

The decision-gated proposal places context retention and replacement with the
external lifecycle manager. This direction supplies observations to that owner.
It does not give planners, compilers, or task executors authority to manage
their own context replacement.

Two candidate optimizations follow:

- **Projection ordering.** Stable objectives, invariants, and authority
  boundaries could precede frequently revised change nodes so some edits retain
  a reusable prefix. A pilot needs to compare that benefit with comprehension
  and actual provider behavior.
- **Closure timing.** Resolving known material decisions before a branch begins
  may avoid projection regeneration and repeated context ingestion. Later
  evidence may still require revision; branch formation is a possible economic
  opportunity, not a universal deadline.

Plan changes can affect projection identity, evidence attribution, lineage,
and cache reuse together. Those consequences make them useful observation
points without requiring a particular switching policy.

## Splitting execution across realizations

Plan IR's dependency representation exposes candidate execution regions.
Order independence alone does not establish that different executors can
implement those regions compatibly. Shared invariants, open judgments,
forbidden boundaries, or acceptance obligations can create composition risks.

One conservative separability hypothesis is to look for regions with disjoint
invariant closures, no shared open judgment, no shared forbidden boundary, and
no acceptance condition spanning both. This is a candidate filter, not a
necessary or sufficient proof of safe composition. Explicit integration
contracts may support other valid partitions.

Economic comparisons apply only within the actual correctness and authority
boundaries. Work volume cannot make an invalid partition acceptable. Where
concurrent execution is valid, latency benefits may offset coordination and
switching costs.

Per-region evidence needs exact plan and projection identity. Joint acceptance
also needs an owner and evidence: locally valid regions can fail when combined.
Composition failure deserves explicit attribution rather than being forced
into one executor's capability score. The benefit of split execution remains
uncertain until that measurement problem is addressed.

## Evidence sources and counterfactuals

The [revisioned research and execution architecture](../../docs/revisioned-research-and-execution-architecture.md)
owns experimental coordinates, claim-relative adequacy, research authority,
and reconstruction coverage.

| Source | Useful evidence | Limitation |
| --- | --- | --- |
| Controlled pilot | Effects of deliberately varied representations and realizations | Results depend on the selected tasks and controls. |
| Controlled historical replay | Targeted comparisons using coordinates from real work | Reconstruction may omit state relevant to the claim. |
| Production execution | Operational frequency, drift, and transfer to real work | Only the selected strategy is observed; selection affects the sample. |

Production does not directly reveal what another strategy would have cost.
Replay can support comparisons, but the original continuation and cache state
may be unavailable. A replay that starts cold cannot establish the cost of
retaining the original warm continuation. It may understate an incumbent's
reuse advantage and make switching appear more attractive.

This does not make all replay cold or all warm-path experiments impossible.
A controlled experiment may create and measure reuse under declared conditions.
The evidence supports those conditions, not an unreconstructed historical
state. Capability claims also require adequate reconstruction for the factors
they depend on.

A cost comparison should distinguish observed, controlled, inferred, and
unavailable continuation state. Unknown cache state establishes neither a cold
nor a warm comparison. Claim scope must follow the available evidence.
Research timing remains a judgment about the decision value and reconstruction
cost of the evidence needed.

## Continuous learning and authority

Controlled pilots could establish initial capability evidence. Authorized
production then supplies revision-bound observations that strengthen, weaken,
or refine that evidence. Contradictions and uncertain regions can motivate
targeted pilots or replay.

Production observations retain their evidentiary class. For example, fifteen
first-pass acceptances, one repair, and one authority-boundary violation among
seventeen executions would describe that sample. It would not establish a
universal capability claim or permit the severe failure to disappear inside an
average acceptance rate.

Uncertainty can inform research allocation. Well-supported regions may need
little additional investigation; uncertain, consequential work may call for a
better-supported realization or an isolated research branch. These are
judgment affordances within existing authorization, not a routing table.

A learning policy also needs to avoid starving alternatives merely because the
incumbent has reusable context. A separately governed exploration allocation is
one possible mechanism. Its value, budget, and treatment of continuation cost
remain policy questions.

The proposed authority flow is:

```text
execution observations
→ revisioned capability evidence
→ derived capability profile
→ routing or policy candidate
→ authorized policy revision
→ future admission
```

Evidence updates do not grant mutation rights, provider spending, relaxed
review, publication authority, or acceptance authority. Those remain with
their existing owners.

An admission should remain explainable through the policy, execution
characterization, evidence revision, projection mechanism, and runtime
realization used at the time. Cost predictions also need the provider cost
parameters used. Later evidence or pricing can change a future decision without
rewriting the basis of an earlier one.

## Relationships and boundaries

| Owner or direction | Relationship |
| --- | --- |
| Structural Plan IR | Owns the representation hypothesis, semantic layering, and projection identity. This direction proposes evidence for choosing projections. |
| Decision-gated compilation | Owns the proposed workflow, material decisions, and conformance gate. This direction supplies candidate inputs. Routing itself is now split -- see the next row, corrected 2026-09-27. |
| Execution Strategy Selection | **Added 2026-09-27**, following that dimension's admission as `app-server/docs/architecture/execution-strategy-selection.md`. Owns the residual class-warrant judgment this document's own "proposed authority flow" (Continuous learning and authority, above) names as "routing or policy candidate -> authorized policy revision -> future admission" without assigning an owner. This document's own capability-evidence and continuous-learning content remains entirely its own; that dimension consumes capability/outcome evidence by reference only, and explicitly does not resolve this document's own evidence-ownership question (its own text marks capability_evidence "[owner unresolved]" throughout). |
| Hierarchical orchestration | Owns branch partitioning. Resolution profiles may differ across branches. |
| Provider/harness runtime | Owns realization and runtime boundaries. Continuation observations need a defined relationship to those identities. |
| Context lifecycle | Owns retention and replacement decisions. Cache observations may inform them. |
| Revisioned research | Owns coordinates, adequacy, and experimental governance. Cost claims need coverage appropriate to their continuation assumptions. |

Stage 5 of decision-gated compilation includes direct implementation as a
baseline. Plan IR section 20 proposes varying plan resolution. A coordinated
comparison could cross those factors under one measurement contract; the
[upstream amendments](evidence-calibrated-plan-resolution-upstream-amendments.md)
describe that candidate change.

This document does not own routing policy, admission thresholds, context
lifecycle, decision authority, the Plan IR schema, research admission, or
implementation acceptance. It does not accept the related proposals or
establish a measurement schema.

## Failure modes

- **Premature ontology or false precision.** Imagined dimensions become a fixed
  taxonomy before evidence shows they are useful.
- **Model-specific semantics.** An executor profile changes the canonical
  contract instead of its faithful projection.
- **Capability collapse.** A single score hides task, realization, projection,
  or verification differences.
- **Under- or over-elaboration.** Missing structure causes reconstruction
  failures, or excess detail adds burden and crosses into implementation.
- **False compensation.** More detail is assumed to repair an executor limit
  that the evidence does not show can be compensated.
- **Evidence-class collapse.** Pilot, replay, and production observations are
  treated as interchangeable.
- **Cost-class collapse.** A comparison is generalized beyond its demonstrated
  cache and continuation conditions.
- **Incumbency capture.** Reuse savings repeatedly favor the initial assignment
  until evidence about alternatives stops accumulating.
- **Composition attribution gaps.** Joint failures disappear between otherwise
  valid region-level observations.
- **Hidden decision-making in rendering.** A renderer silently resolves an
  upstream material decision instead of exposing it to its owner.
- **Historical amnesia or authority expansion.** Evidence loses its original
  bindings or is treated as permission to change policy.
- **Benchmark capture or excessive experimentation.** Routing overfits a narrow
  corpus, or research costs exceed its credible decision value.

## Research questions

1. [KIND: BACKLOG] [UNCHECKED — empirical question] Do dimension-specific profiles outperform scalar presets, and which
   dimensions have independently measurable effects?
2. [KIND: BACKLOG] [UNCHECKED — empirical question] Which capability limits can structural elaboration compensate for?
3. [KIND: BACKLOG] [UNCHECKED — design/schema detail] How much projection work is deterministic, and can a check distinguish
   faithful rendering from new material decisions in structured text?
4. [KIND: BACKLOG] [UNCHECKED — empirical question] Does additional decision closure predict accepted-work cost better than
   projection resolution, and can its cost be estimated before doing it?
5. [KIND: BACKLOG] [UNCHECKED — measurement design] How should context burden be measured alongside token use and quality?
6. [KIND: BACKLOG] [UNCHECKED — empirical question] How much do review intervals, prefix changes, and realization changes each
   affect observed cache reuse?
7. [KIND: BACKLOG] [UNCHECKED — empirical question] Does stability-based ordering improve reuse without harming comprehension?
8. [KIND: BACKLOG] [UNCHECKED — methodology question] How do replay and production cost comparisons differ under declared
   continuation conditions?
9. [KIND: BACKLOG] [UNCHECKED — empirical question] Does the candidate separability filter predict successful composition, and
   can failures be attributed across plan, projection, executor, harness,
   verification, and integration?
10. [KIND: BACKLOG] [UNCHECKED — empirical question] How stable are capability findings across model and harness revisions?
11. [KIND: BACKLOG, reclassified 2026-09-17 — the document's own 'Continuous learning and authority' section already settles that admission's ownership is unaffected ('an admission should remain explainable through the policy... used at the time'; 'these are judgment affordances within existing authorization, not a routing table'). This item asks for a specific evidence-sufficiency threshold and exploration budget within that already-settled authority boundary, not a new ownership assignment] [OPEN — no such threshold or budget policy is designed anywhere] What evidence would justify consulting a cost predictor during admission,
    and how should exploration allocation be governed?
12. [KIND: BACKLOG, reclassified 2026-09-17 -- retracts the 2026-09-17 term-absence finding above, which grepped this document's own local vocabulary ("continuation identity", "warmth") rather than testing the underlying architectural question against plausible owners under the route-invariance test. Split into its three actual sub-questions and re-run: (a) "What constitutes continuation identity?" -- runtime-realization.md SS10 already owns realization identity via revision/CAS lineage; the specific schema for a continuation-identity instance of that is downstream representation detail, not a new ownership question (two different schemas could both satisfy the same realization-identity architecture). (b) "Who observes/records runtime warmth?" -- runtime-realization.md SS3 ("Capability Facts Are Attributed Observations") already generalizes to any capability-adjacent observation, warmth included; which specific mechanism performs it is implementation choice under an already-settled attributed-observation contract. (c) "How do pricing changes affect use of historical predictions?" -- the already-established "invalidation never mints authority" / non-retroactive-revision pattern (authority-and-ownership.md SS12, already confirmed this session as recurring across multiple documents) directly answers this: a later cost-model change does not retroactively invalidate an earlier decision's basis. All three collapse to BACKLOG under the litmus test: two implementations choosing different answers to any of them could both still satisfy the same canonical architecture.] [OPEN — none of the three has a specific schema/mechanism/policy designed yet] Which owner should define continuation identity and warmth observations,
    and how should pricing changes affect future use of historical predictions?

These questions leave profile dimensions, renderability checks, partitioning,
projection order, closure timing, replay scheduling, and exploration policy
open. Their value is to be established through authorized evidence, not assumed
from the shape of the proposed mechanism.
