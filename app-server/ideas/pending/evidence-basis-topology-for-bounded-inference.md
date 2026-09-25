# Evidence-Basis Topology for Bounded Inference

## Status and authority

Exploratory architectural idea, directly captured from operator discussion on
2026-09-22.

This document does not accept a design, establish a new architectural
dimension, assign a permanent owner, authorize implementation, change the
roadmap, grant evidence access, determine evidence sufficiency, or authorize a
judgment or decision. It records a candidate representation and relationship
among evidence, bounded inference, active inquiry, readiness, and causal
diagnosis for later intake and reconciliation.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-22
```

The candidate novelty is:

> A bounded inference may need an explicit, decision-relative evidence
> topology whose anchors, relationships, represented boundaries,
> contradictions, resolution, and visible gaps are preserved independently
> from its model-facing projection. Evidence acquisition may refine that
> topology locally; readiness judges its sufficiency; inference traverses it;
> and causal diagnosis may attribute topology, projection, inquiry, or
> traversal failures without collapsing them.

Whether this is a new dimension, a reusable mechanism, a representation over
existing owners, or only a useful explanatory view remains unresolved.

---

## Recognition: evidence as a scaffold

Evidence is often described as input to inference. That description is true
but incomplete.

Evidence does not normally contain the whole solution. It constrains the space
in which a sound solution can be formed. An intuitive metaphor is an evidence
scaffold:

```text
sparse or broken scaffold

●          ●
      ?
                ●          ●

Inference has too many unconstrained routes.
Priors may silently complete the missing shape.


structurally useful scaffold

●──────────●
     │
     ●────────●
              │
              ●

Important regions, boundaries, and relationships are visible.
Inference bridges bounded gaps and can identify where another anchor is needed.


indiscriminately dense projection

●●●●●●●●●●●●●●●
●●●●●●●●●●●●●●●
●●●●●●●●●●●●●●●

The important topology may exist but lose salience in the rendering.
Attention, not canonical evidence custody, becomes the scarce resource.
```

The scaffold is not the answer. Inference still connects, explains, proposes,
tests, and sometimes discovers that the scaffold itself is wrong or
insufficient.

The architectural question is whether Work Engine needs an explicit object or
contract for the structure that constrains this movement.

---

## Equal evidence volume can expose different problem shapes

Two evidence bases with the same number of items can differ materially:

```text
clustered basis
    many observations about one local implementation
    no ownership boundary, caller, integration seam, or acceptance consequence

structurally distributed basis
    implementation
    one caller
    one interface
    one ownership boundary
    one acceptance test
```

The second basis may constrain the consequential solution space more reliably
despite containing fewer bytes, files, or observations.

This suggests evidence-basis quality may depend on more than:

- quantity;
- individual source quality;
- provenance;
- freshness;
- subject identity;
- conventional collection coverage;
- semantic resolution.

Candidate additional properties include:

- structural coverage;
- preservation of consequential relationships;
- boundary representation;
- counterevidence and conflict visibility;
- gap visibility;
- resolution transitions across regions;
- model-facing salience distribution.

These properties are not assumed to have one owner or one scalar score.

---

## The scaffold is a typed topology, not a homogeneous matrix

The dot-matrix metaphor is useful intuition. A candidate architectural
representation is closer to a typed graph or hypergraph.

### Candidate node kinds

```text
observation
claim revision
accepted decision
plan or contract invariant
test or validation result
implementation subject
authority or ownership boundary
explicit unknown
contradiction
unavailable evidence
```

### Candidate relationship kinds

```text
supports
constrains
contradicts
depends_on
derived_from
applies_to
verifies
invalidates
corresponds_to
leaves_unresolved
```

### Candidate regions or concerns

```text
architecture boundary
subsystem
integration seam
implementation surface
recovery path
acceptance consequence
authority domain
```

These lists are illustrative. A universal closed taxonomy would be premature.

Most importantly, the topology may not invent an edge merely to make the
scaffold connected. A relationship must retain its provenance, revision,
trust class, semantic owner, and uncertainty. Missing relationships may be
more consequential than missing nodes.

---

## Decision-relative, not universally complete

There is no single ideal scaffold for a subject independent of the judgment
being made.

The same repository region may require different evidence topology for:

```text
architecture placement
implementation mechanism selection
security-boundary review
compatibility diagnosis
acceptance of a completed slice
```

A decision or judgment contract may declare:

- the exact subject and evidence cutoff;
- the consequence being protected;
- required regions, boundaries, and relationship classes;
- required counterevidence or independence;
- tolerated gaps and uncertainty;
- required resolution by region;
- prohibited inference or authority crossings;
- sufficiency and escalation owner.

The scaffold is evaluated relative to that contract. It is never evidence of
universal completeness.

---

## Three different structural-coverage facts

"Structural coverage" should not collapse three separately owned facts.

### Declared structural coverage

What consequential regions, boundaries, relationship classes, or alternatives
does the evidence basis claim to represent?

This belongs to whichever artifact or contract declares the basis and its
intended use.

### Observed structural coverage

Which declared regions and relationships are actually backed by current,
revision-bound evidence, and which are missing, stale, contradicted,
unsupported, or unavailable?

Where mappings and requirements are explicit, part of this may be mechanically
derived. Semantic relevance must not be smuggled into a generic observer.

### Decision-relative sufficiency

Is the observed topology sufficient for decision or judgment `D` under
readiness contract `C`?

This is already adjacent to, and potentially owned by,
[`../../docs/architecture/decision-specific-readiness.md`](../../docs/architecture/decision-specific-readiness.md).
The scaffold candidate must not create a competing sufficiency owner.

The separation follows Work Engine's recurring discipline:

```text
declaration / observation
        -> deterministic derivation where possible
        -> bounded semantic judgment by the actual owner
```

---

## Canonical topology, projection, and realized salience

Three more facts must remain separate.

### Canonical or durable evidence topology

What evidence anchors, relationships, gaps, conflicts, and subject bindings are
durably known or materialized.

### Model-facing projection

Which bounded subset and representation are presented for one inference
episode, with what omissions, ordering, compression, and transformation.

### Realized salience

How the projection interacts with a particular model, context state, ordering,
task frame, and provider behavior.

A durable evidence universe may contain the correct topology while a poor
projection buries its decisive structure through repetition, verbose
rendering, irrelevant neighboring material, ordering, or context pressure.

Therefore:

> "Too dense" is ordinarily not a reason to delete canonical evidence. It is a
> candidate defect in selection, transformation, or model-facing rendering.

Salience is not an intrinsic property of an evidence item. It is a property of
an item within a realized projection and inference episode.

---

## Candidate evidence-topology artifact

An illustrative artifact might look like:

```yaml
evidence_topology:
  identity: stable-basis-identity
  subject: exact-subject-and-revision
  judgment_contract: exact-reference
  evidence_cutoff: exact-reference
  anchors:
    - identity: evidence-reference
      kind: observation | claim | invariant | test | unknown
      owner: exact-owner-reference
      subject_binding: exact
      freshness: explicit
      trust_class: explicit
      limitations: []
  relationships:
    - from: anchor-id
      type: supports | constrains | contradicts | depends_on | derived_from
      to: anchor-or-required-region-id
      provenance: exact-reference
      owner: exact-owner-reference
      confidence_or_state: explicit
  required_regions: []
  represented_regions: []
  explicit_gaps: []
  unresolved_conflicts: []
  resolution_by_region: []
  projection_lineage: []
  known_omissions: []
```

This is not a proposed schema. It demonstrates the candidate semantic residue
that ordinary evidence lists may fail to express.

The artifact must remain subordinate to the authoritative evidence, decisions,
plans, contracts, and relationships it references. It cannot become a new
owner merely by joining them.

---

## Formation and transformation

The scaffold may pass through several transformations:

```text
durable evidence universe
        +
decision or judgment contract
        |
        v
initial evidence-topology compilation
        |
        v
model-facing bounded projection
        |
        v
inference encounters a gap, conflict, or resolution mismatch
        |
        v
active evidence acquisition
        |
        v
successor scaffold / projection
        |
        v
conclusion and outcome
        |
        v
causal diagnosis of topology, projection, inquiry, or traversal failure
```

Each transformation should preserve:

- exact predecessor and subject identity;
- included and excluded evidence;
- relationship provenance;
- gaps and contradictions;
- compression or abstraction performed;
- authority and confidentiality boundaries;
- the distinction between canonical evidence and episode-local projection.

Whether these transformations require a reusable mechanism or are ordinary
Revision/CAS publication plus domain-owned compilation remains open.

---

## Adaptive scaffold refinement

[`active-evidence-acquisition-during-execution.md`](active-evidence-acquisition-during-execution.md)
asks which lawful probe is likely to reduce uncertainty that can change the
next protected consequence.

In scaffold terms:

```text
traverse current supported topology
        |
        v
encounter consequential gap, conflict, or resolution mismatch
        |
        +-- safely bridged under the judgment contract -> continue
        |
        +-- insufficiently anchored -> select lawful probe
                                             |
                                             v
                                      acquire new observation
                                             |
                                             v
                                  refine local evidence topology
```

Refinement is local and consequence-sensitive. It does not seek globally high
density.

The inquiry role may add an observation or nominate a relationship. It cannot
silently author a semantic edge, close a material decision, or upgrade its own
observation into accepted source truth.

---

## Unsupported inference exposure

Inference exists to bridge gaps. A scaffold that prohibited all bridges would
merely restate evidence and could not solve anything.

But a consequential conclusion becomes increasingly exposed when it:

- introduces material semantic content absent from its anchors;
- depends on undeclared assumptions;
- crosses an explicit unknown, contradiction, or unavailable region;
- crosses an ownership or authority boundary;
- omits relevant alternatives or counterevidence;
- lacks a bounded derivation or verification route;
- carries high consequence or low reversibility;
- remains far from any independently supported checkpoint in semantic terms.

This candidate property is better called **unsupported inference exposure**
than literal graph distance.

Ten mechanically established derivations may be safer than one imaginative
semantic leap. Node count, edge count, token distance, and confidence are not
sufficient measures.

A candidate invariant is:

> A conclusion that requires material premises no longer sufficiently anchored
> by the admitted topology must preserve the gap, acquire evidence, narrow the
> claim, obtain verification, or escalate to the appropriate owner.

No numerical threshold is proposed.

---

## Scaffold traversal without reasoning-transcript capture

Work Engine need not store chain-of-thought to make support auditable.

A consequential conclusion may instead publish or reference:

```text
exact conclusion or candidate
supporting evidence anchors
declared assumptions
typed derived relationships
explicit gaps and contradictions
scope and limitations
verification or challenge status
reopening conditions
```

This establishes conclusion-to-evidence lineage without claiming to reproduce
the model's private cognitive path.

"Traversal" is therefore a conceptual model for bounded inference, not a
requirement to observe internal token-by-token reasoning.

---

## Preserve holes instead of completing the picture

An explicit gap is useful state:

```text
known evidence          ●
declared relationship   ─
derived relationship    ···
unresolved gap          ?
contradiction           ×
```

The system should not require every gap to be filled. It should ask whether the
gap affects a protected consequence.

If a fact is neither known nor needed, the truthful result is often to preserve
`?`, not to produce a plausible value from prior knowledge.

This supplies a candidate behavioral discipline:

> Preserve holes until the judgment contract requires a bridge, and preserve
> the bridge's evidence and uncertainty when one is formed.

---

## Relationship to readiness

[`../../docs/architecture/decision-specific-readiness.md`](../../docs/architecture/decision-specific-readiness.md)
owns decision-specific requirements and the attributed judgment that an
evidence basis is `ready`, `blocked`, or `uncertain` for a particular decision.

The scaffold candidate may supply a richer subject for that judgment:

```text
evidence topology revision
        +
decision-specific readiness contract
        |
        v
readiness assessment
```

It must not duplicate:

- which evidence a decision requires;
- whether the basis is sufficient;
- what missing requirements block readiness;
- the authority to perform the decision.

If readiness contracts plus existing evidence relationships already determine
the entire scaffold meaning, this idea may collapse to a representation rather
than a new semantic owner.

---

## Relationship to implementation-basis assembly

[`../../docs/architecture/implementation-contract-compilation.md`](../../docs/architecture/implementation-contract-compilation.md)
already owns the implementation basis as a bounded dependency manifest. Every
included item must explain its implementation relevance, and missing
completeness must remain visible.

That may be a concrete domain instance of an evidence scaffold:

```text
implementation basis
    evidence anchors + accepted semantic inputs + uncertainty

candidate topology extension
    explicit consequential regions, relationships, gaps, and projection lineage
```

The scaffold idea must not take ownership of implementation-basis assembly or
contract compilation. It may supply a representation or general relationship
that this and other domains can reuse.

---

## Relationship to claim-evidence and Evidence Anchor

[`../../docs/architecture/evidence-and-claims.md`](../../docs/architecture/evidence-and-claims.md)
owns claim identity, provenance, lineage, reliance, and refresh semantics.

[`../../docs/architecture/substrates/evidence-anchor.md`](../../docs/architecture/substrates/evidence-anchor.md)
owns exact-subject observation, mechanical correspondence, and weak
`may_affect` nomination candidates.

Neither automatically decides:

- why an item is relevant to a particular judgment;
- whether a set of anchors preserves the necessary problem topology;
- whether an absent edge is material;
- whether a model-facing projection has sufficient salience;
- whether the evidence basis is decision-ready.

The scaffold may reference claims and anchor observations. It cannot become a
second claim store, dependency registry, relevance owner, or readiness judge.

[`../../docs/claim-evidence-service.md`](../../docs/claim-evidence-service.md)
already leaves open the smallest relevant-claims projection that avoids both
omitted dependencies and indiscriminate context loading. Evidence topology may
help state that problem without deciding the answer automatically.

---

## Relationship to adaptive plan resolution

[`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md)
distinguishes resolution, context ingestion, and attention burden. It proposes
dimension-specific projection profiles rather than globally maximal detail.

Evidence topology adds a separate candidate axis:

```text
semantic or plan resolution
    how finely established meaning is rendered

evidence-topology fitness
    whether the basis preserves the consequential regions, boundaries,
    relationships, alternatives, conflicts, and gaps for the judgment

evidence-acquisition profile
    which lawful actions may refine that basis
```

Higher semantic resolution does not guarantee better topology. More evidence
does not guarantee better projection. A sparse but structurally distributed
basis may outperform a dense local cluster for one judgment.

---

## Relationship to causal diagnosis

[`governed-causal-diagnosis-and-epistemic-learning.md`](governed-causal-diagnosis-and-epistemic-learning.md)
preserves multiple explanations for an outcome and separates diagnosis from
policy admission.

Evidence-scaffold failure modes may become subtypes within its existing four
epistemic-control surfaces:

```text
initial representation
    sparse topology
    wrong topology
    resolution mismatch
    projection salience distortion

active inquiry
    missed refinement
    wrong-region refinement
    ineffective probe
    premature stopping

judgment
    unsupported traversal
    premature gap completion
    adequate scaffold misinterpreted

causal learning
    wrong scaffold defect attributed
    observational correlation overstated
    remedy selected for the wrong surface
```

These are candidate explanations, not deterministic diagnoses. Several may
coexist in one episode.

---

## Relationship to resolution-layered refinement

[`resolution-layered-semantic-refinement.md`](resolution-layered-semantic-refinement.md)
keeps semantic resolution, organizational shape, projection resolution, and
review grain independent.

Evidence topology may likewise vary by semantic layer:

```text
architecture layer
    system boundaries, ownership, invariants, competing topologies

subsystem layer
    interfaces, dependencies, seams, localized constraints

implementation layer
    exact repository state, symbols, tests, executable observations
```

Each layer may need its own decision-relative scaffold without making the
scaffold a semantic-resolution layer or granting its compiler authority over
the layer's meaning.

---

## Relationship to observation consequences

[`observation-consequences-and-evidence-basis-evolution.md`](observation-consequences-and-evidence-basis-evolution.md)
further decomposes/reconciles this document's own open question 1 ("Does
evidence topology own any truth not already declared...") by separating an
attributed observation's episode-local, durable-evidence, and
topology-nomination consequences into independently governed branches, and
by keeping a candidate durable topology revision from merging into a single
store with a claim revision. No classification is accepted there either --
it narrows the question shape, not the architecture.

---

## Candidate classification outcomes

### New dimension

Evidence-topology truth contains irreducible semantic content with a distinct
owner and lifecycle not already owned by evidence, readiness, planning, role
contracts, or inference domains.

### Shared mechanism

Several independent domains need the same basis-construction, projection,
refinement, and predecessor-binding protocol while retaining their own
topology semantics.

### Representation

A graph or manifest makes existing owner relationships, gaps, and projection
lineage visible but owns no new semantic truth.

### Pure composition

Existing claims, anchor observations, readiness contracts, implementation
bases, projections, and acquisition records already determine everything; only
a cross-view metaphor is missing.

No classification is selected by this capture.

---

## Candidate evidence for distinct treatment

Evidence supporting a distinct concept would include:

- two evidence bases with comparable item quality and quantity but materially
  different outcomes attributable to represented relationships or boundaries;
- one decision contract whose required topology cannot be expressed as a flat
  evidence list or ordinary completeness field;
- a missing relationship that matters more than any missing evidence item;
- a local refinement that changes scaffold topology without changing the
  accepted semantic contract;
- the same topology/projection/refinement discipline recurring across
  implementation, review, planning, and diagnosis domains;
- a readiness judgment becoming substantially more precise when it consumes an
  explicit topology rather than a collection of claims;
- a failure localized to projection salience despite a correct durable
  evidence universe.

Evidence against a distinct concept would include:

- every required region and relationship being fully determined by an existing
  readiness contract plus claim/anchor references;
- every transformation reducing to ordinary claim lineage and Revision/CAS;
- every refinement reducing to Active Evidence Acquisition without an
  independently meaningful scaffold state;
- every failure subtype being representable adequately through current
  coverage, limitation, and diagnosis fields;
- no consumer being able to distinguish the topology artifact from its source
  evidence list and decision contract.

The proposed empirical comparison is specified separately in
[Evidence-Topology Inference-Fitness Pilot](evidence-topology-inference-fitness-pilot.md).
That pilot treats an equally clear narrative projection as the primary control
and a preregistered missing-edge or weakened-gap condition as a sensitivity
test. It does not authorize pilot execution or presume that evidence topology
will survive as a distinct architectural construct.

---

## Failure modes

### Scaffold becomes a second truth owner

A joined representation silently overrides the plans, claims, contracts,
decisions, or observations it references.

### Relevance edges are invented mechanically

Retrieval rank, graph reachability, file proximity, or temporal order is
treated as semantic relevance.

### Readiness is duplicated

The topology artifact decides its own sufficiency instead of supplying a basis
to the existing decision-specific judgment owner.

### Density becomes a scalar quality score

More or fewer nodes are treated as universally better without regard to
topology, decision, projection, or attention.

### Canonical evidence is deleted to improve salience

A model-facing projection concern mutates or discards the durable evidence
universe.

### Graph distance becomes semantic risk

Edge count substitutes for the materiality, authority, reversibility, and
support quality of an inference bridge.

### Gap completion becomes mandatory

Explicit unknowns are treated as defects even when the judgment does not
require them to be resolved.

### Private reasoning becomes required evidence

The system attempts to reconstruct or retain chain-of-thought rather than
publishing support, assumptions, gaps, limitations, and conclusions.

### One universal scaffold

A subject receives one supposedly complete topology independent of the
decision or judgment being made.

---

## Questions for later intake and reconciliation

1. Does evidence topology own any truth not already declared by a decision
   contract, source artifact, claim relationship, or evidence anchor?
2. Who may declare that an evidence item represents a consequential region or
   boundary?
3. Which relationship edges are mechanically derivable, and which require a
   domain-owned semantic judgment?
4. Is a scaffold a durable canonical artifact, an episode-local projection, or
   both with separate identities?
5. Does scaffold refinement require revision publication, or can it remain an
   attributed execution-episode record?
6. What minimum topology must be preserved across compression or context
   replacement?
7. How should an explicit gap refer to missing evidence that may not yet exist?
8. Can realized salience be observed without treating model attention as a
   directly inspectable fact?
9. What establishes unsupported inference exposure without creating a
   universal score or retaining private reasoning?
10. Can Decision-Specific Readiness express every scaffold-fitness judgment by
    enriching its contracts, making this only a representation?
11. Does a common topology protocol recur independently across enough domains
    to warrant a shared mechanism?
12. What empirical comparison would distinguish topology benefit from simply
    giving the model better-written context?

---

## Non-goals

This idea does not currently propose:

- a universal evidence-completeness score;
- a scalar scaffold-density or inference-distance metric;
- replacing Decision-Specific Readiness;
- replacing claim-evidence, Evidence Anchor, or implementation-basis assembly;
- granting a scaffold compiler semantic relevance authority;
- making all repository or workflow relationships evidence edges;
- requiring all canonical evidence to enter model context;
- deleting canonical evidence to reduce attention burden;
- exposing, storing, or reconstructing chain-of-thought;
- requiring every explicit gap to be filled;
- treating a connected graph as proof of a sound conclusion;
- making evidence topology the answer, decision, or acceptance authority; or
- implementing a graph store, compiler, projection engine, or inference policy
  from this capture.

---

## Compact hypothesis

> Evidence constrains inference not only through item quality and quantity but
> through a decision-relative topology of anchors, relationships, boundaries,
> conflicts, resolution, and explicit gaps. A bounded inference may consume a
> faithful projection of that topology, refine it locally through authorized
> inquiry, preserve unsupported gaps rather than complete them from priors, and
> publish conclusion-to-evidence lineage without exposing private reasoning.
> Existing owners retain evidence truth, relevance, readiness, judgment,
> diagnosis, and decision authority.
