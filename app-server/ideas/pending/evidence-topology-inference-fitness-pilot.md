# Evidence-Topology Inference-Fitness Pilot

## Status and authority

Exploratory pilot design, directly captured from operator discussion on
2026-09-22.

This document does not authorize pilot execution, paid inference, provider
access, repository mutation beyond fixture authoring, architectural acceptance,
production integration, roadmap priority, or a new canonical evidence object.
It defines a bounded experiment that may help classify the candidate described
in
[Evidence-Basis Topology for Bounded Inference](evidence-basis-topology-for-bounded-inference.md).

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-22
  source_hypothesis: evidence-basis-topology-for-bounded-inference.md
```

Any executable pilot must be separately admitted with an immutable subject,
frozen fixtures, declared models and routes, an inference budget, evaluation
authority, and durable receipts. Results may supply evidence for later intake
or proposal formation. They do not accept an architecture or authorize an
implementation.

---

## Decision this pilot informs

The evidence-topology idea currently leaves four classifications open:

```text
new dimension
shared mechanism
representation
pure composition
```

The pilot does not select among them directly. It tests whether the proposed
topological representation does explanatory work beyond evidence selection,
evidence volume, semantic resolution, and editorial quality.

The narrow research question is:

> Can explicit preservation of consequential relationships, boundaries,
> conflicts, and gaps improve bounded inference independently of evidence
> quantity, semantic content, and editorial clarity?

A positive result would justify further investigation of a shared
representation or protocol. It would not establish a new architectural
dimension, service, graph store, compiler, or semantic owner.

A null or adverse result would be useful. It could indicate that ordinary
well-formed projections, readiness contracts, claim lineage, and explicit
unknowns already provide the required constraint structure.

---

## Scope of the inference this pilot can support

The primary C-versus-B comparison tests a **model-facing representation**:

```text
B
    consequential structure expressed in equally clear prose

C
    the same consequential structure expressed as independently addressable,
    typed topology
```

It does not test every possible value of an upstream topology. The common-case
ledger already provides a structured source from which A, B, and C are formed.
That ledger may prove useful for projection compilation, relationship
preservation, auditing, revision comparison, or answer-leakage detection even
if a reasoner performs equally well on B and C.

Therefore:

```text
B ≈ C
    explicit topology has not shown superiority as model-facing presentation

does not establish
    an upstream topology is useless for formation, preservation, or auditing
```

Testing upstream value would require a different experiment: form or maintain
projections with and without a topology ledger, then compare relationship
loss, gap destruction, answer leakage, revision fidelity, and authoring cost.
Those questions are outside this first pilot.

---

## Why a pilot is warranted now

The candidate is specific enough to be falsifiable:

- evidence items may be held constant;
- model-facing organization can be varied;
- consequential edges and gap states can be represented or withheld;
- outputs can be scored without inspecting private reasoning;
- competing explanations can be assigned separate conditions;
- topology-sensitive cases can be paired with cases where topology should add
  little or no value.

Without a controlled comparison, improvements could be attributed
incorrectly to any of the following:

```text
better evidence selection
more evidence
more tokens
clearer prose
answer-bearing labels
more semantic guidance
task-specific prompting
model or sampling variation
evaluator preference for diagrams
```

The pilot exists to separate those explanations before Work Engine designs a
production mechanism around the metaphor.

---

## Hypotheses and competing explanations

### H1 — topology-fitness hypothesis

For judgments whose correctness depends materially on relationships,
boundaries, conflicts, or explicit gaps, a faithful topology projection will
improve bounded inference relative to both a flat evidence presentation and an
equally clear narrative presentation of the same admitted evidence.

Expected effects include:

- fewer unsupported material commitments;
- better preservation of consequential unknowns;
- better recognition of ownership or authority boundaries;
- fewer omissions of material relationships;
- more accurate identification of when another evidence anchor is required;
- equal or better decision correctness without indiscriminate context growth.

### H0 — editorial-clarity explanation

Any apparent benefit comes from clearer writing, grouping, or emphasis rather
than from an explicit topology. A well-edited narrative should perform as well
as the topology condition.

### H0a — answer-encoding explanation

Topology appears to help only because its relationship labels or arrangement
silently encode the desired conclusion. If an edge cannot be warranted
independently of the answer being evaluated, that edge is not a lawful pilot
input.

### H0b — ordinary-projection explanation

Existing projection practices already preserve everything material. A flat or
narrative basis with evidence identifiers, provenance, limitations, and
explicit unknowns performs equivalently.

### H0c — presentation-cost explanation

Topology increases token or attention cost, creates false salience, or leads
the reasoner to over-trust represented edges. It performs worse on correctness,
calibration, or efficiency.

### Perturbation prediction

If represented relationships are causally relevant, removing, demoting, or
marking one consequential edge as unknown should produce a predictable change
on topology-dependent cases. If the perturbation has no effect, either the
case was not topology-dependent, the reasoner ignored the topology, or the
proposed representation added no usable constraint.

---

## Pilot boundary

The first pilot is a **static bounded-inference experiment**.

The reasoner receives one frozen evidence projection and returns a structured
judgment. It does not search the repository, run tests, call tools, acquire new
evidence, or refine the scaffold during the episode.

This boundary deliberately excludes the online loop described in
[Active Evidence Acquisition During Execution](active-evidence-acquisition-during-execution.md).
Tool-enabled acquisition would introduce additional variables:

- whether the reasoner notices a gap;
- which probe it chooses;
- tool and retrieval quality;
- evidence availability;
- stopping policy;
- cost and latency of inquiry.

If the static pilot supports the topology hypothesis, a later pilot may test
whether topology improves local refinement. The first pilot should establish
whether the represented scaffold changes bounded inference at all.

---

## Experimental conditions

Every case is rendered into four matched conditions.

### Condition A — flat evidence basis

The reasoner receives the admitted evidence items as an intentionally plain,
well-formed list.

The list preserves:

- evidence identifiers;
- exact factual content;
- every independently warranted relationship from the case ledger, expressed
  as an ordinary list proposition rather than as a traversable structure;
- provenance and subject identity where material;
- explicit unknowns and limitations;
- the common task and response contract.

It does not suppress relationships in order to make the topology condition
win. It withholds only the relationship-oriented organization under test.

This is not a deliberately bad control. A confusing or careless flat list
would make the comparison meaningless.

### Condition B — edited narrative control

The reasoner receives the same admitted facts, relationship meanings, gap
states, and limitations in concise, professionally edited prose. The narrative
may group related material and improve flow, but it may not introduce facts,
conclusions, relationship warrant, priority claims, or answer-bearing emphasis
absent from the common evidence basis. Conversely, it may not omit a ledger
relationship merely because prose makes it awkward to express.

This condition tests whether any gain attributed to topology is merely a gain
from editorial clarity.

### Condition C — explicit topology

The reasoner receives the same admitted facts, relationship meanings, gap
states, and limitations through an explicit topology projection containing
only independently warranted structure, such as:

```text
evidence anchors
typed relationships
represented ownership or authority boundaries
contradictions
explicit gaps and their epistemic states
resolution transitions where material
projection limitations
```

Relationship meaning and establishment basis must remain orthogonal. For
example:

```yaml
relationship:
  meaning: constrains
  establishment_basis: owner_declared
  source: E4
  subject_revision: case-revision
  status: established
```

Allowed establishment bases should be frozen before execution, for example:

```text
directly observed
owner declared
deterministically derived
bounded semantic judgment
hypothesized
```

Hypothesized relationships must remain visibly hypothesized. The topology may
not promote them to established merely to make the scaffold connected.

### Edge-establishment strata

The primary scored cohort should use relationships with unusually clean
establishment bases:

```text
directly observed
owner declared
deterministically derived
```

Edges established through `bounded semantic judgment` should be excluded from
the primary cohort or placed in a separately preregistered exploratory stratum
whose outcomes are not pooled with the primary comparison. Hypothesized edges
should not appear as established structure in the primary cohort.

This prevents the pilot from crediting topology for semantic work that was
actually performed while authoring the edge. A later stratum may ask whether
explicit topology helps when a relationship itself depends on bounded semantic
judgment, but it cannot provide the clean initial causal test.

### Per-fixture representation-difference rationale

Every B/C pair must declare what is being varied and what is held constant.
The rationale should use a structure such as:

```yaml
representation_difference:
  semantic_content_held_constant: true
  relationship_availability_held_constant: true
  b_expression: ordinary relational prose
  c_expression: independently addressable typed nodes, edges, and gaps
  claimed_manipulation: structural explicitness and topology preservation
  b_artificially_degraded: false
  c_contains_unique_semantic_guidance: false
  reviewer_finding: pending
```

B is allowed to express topology very clearly in prose. That is the hard
control, not a defect. Fixture authors must avoid making B artificially clumsy,
but they must also ensure that B and C differ by more than punctuation or
cosmetic delimiters. If no meaningful representation difference can be stated
without changing semantics, the fixture cannot test the primary hypothesis.

### Condition D — topology perturbation

Condition D starts from Condition C and changes exactly one consequential
relationship or gap state according to a preregistered perturbation.

Candidate perturbations include:

- omit one critical edge;
- demote an established relationship to `unknown`;
- preserve both nodes but remove their represented dependency;
- replace `not_required` with `unresolved` where the distinction matters;
- remove a boundary marker without removing either adjacent fact.

The perturbation must not insert a false relationship. It tests sensitivity to
missing or weakened structure, not susceptibility to fabricated evidence.

Condition D is a diagnostic negative control. Because it intentionally changes
the represented basis, it is not part of the strict same-content comparison
between A, B, and C.

Each D condition must preregister a case-specific directional prediction rather
than merely asserting that performance should become worse:

```yaml
perturbation_prediction:
  target: R3
  change: established boundary becomes unrepresented
  predicted_behavioral_transition: route_to_owner -> unauthorized_proceed
  predicted_outcomes:
    - ownership_boundary_error increases
  explicitly_not_predicted:
    - evidence_request_rate changes
    - unrelated_unknown G2 is completed
  interpretation_if_absent:
    - edge was not decision-material
    - edge was redundant with other content
    - reasoner ignored the representation
    - fixture lacked sensitivity
```

An arbitrary behavioral change does not validate D. The observed transition
must correspond to the preregistered consequence of the exact relationship or
gap state that was weakened.

---

## Common-content invariant

Conditions A, B, and C must draw from one frozen case truth table.

For each case, authors should maintain a content ledger:

```yaml
case_content:
  evidence_items:
    - id: E1
      proposition: ...
      provenance: ...
      epistemic_status: established
  relationships:
    - id: R1
      endpoints: [E1, E3]
      meaning: ...
      establishment_basis: ...
      source_support: [E1, E3]
  explicit_gaps:
    - id: G1
      status: unknown
      required_for_judgment: false
  prohibited_additions:
    - ...
```

The ledger is not shown identically in every condition. It is the source used
to verify that A, B, and C preserve common semantic content while varying how
that content is organized. A relationship may appear as a flat proposition in
A, an ordinary sentence in B, and a typed edge in C; its meaning and warrant
must remain the same.

An independent fixture reviewer should compare all three renderings against
the ledger before inference begins. If Condition C contains a relationship
whose warrant cannot be found in the ledger and source material, the fixture
fails admission because topology has become a disguised answer key.

Token counts need not be byte-identical, but they must remain within a frozen
tolerance. Any systematic token advantage must be reported and analyzed as a
possible explanation.

---

## Case portfolio

An initial scored pilot should contain approximately 8–12 cases. That range is
large enough to prevent one attractive example from dominating the result but
small enough to respect inference and fixture-authoring cost.

The portfolio should contain both topology-sensitive and topology-insensitive
cases.

### Candidate topology-sensitive case families

1. **Ownership boundary** — all local facts support a change, but one
   represented owner or authority boundary requires routing rather than
   commitment.
2. **Dependency constraint** — implementation and interface facts are present,
   but correctness depends on their relationship to an invariant.
3. **Conflict** — individually credible observations disagree across revisions
   or subjects.
4. **Unnecessary completion** — an identity or value is unknown and not
   required for the judgment; the correct behavior is to preserve the hole.
5. **Missing relationship** — every important node is present, but a critical
   edge determines whether the conclusion is supported.
6. **Resolution seam** — architecture-level evidence is sufficient for one
   decision but insufficient for a local implementation commitment.
7. **Counterevidence placement** — decisive counterevidence is present but
   disconnected or buried in one rendering.
8. **Conditional applicability** — a claim is true but applies only when a
   represented precondition or subject relationship holds.

### Topology-insensitive controls

Some cases should be answerable from one or two direct facts with no material
relationship reconstruction. Condition C should not receive credit merely for
adding ceremony, and it should not impose material cost or error on these
cases.

### Controlled and repository-derived cohorts

Where practical, use two small cohorts:

- controlled cases with minimal facts and unambiguous gold judgments, used for
  causal isolation;
- frozen repository-derived cases, used for ecological validity.

Repository-derived cases must bind exact source revisions and be reduced into
the same fixture ledger. They must not depend on the evaluator's unstated
repository knowledge.

Case authors should avoid famous examples and familiar benchmark wording that
could allow memorized answers to substitute for evidence-bounded judgment.

---

## Task and response contract

Every condition for one case receives the same task statement and response
schema.

The schema should request externally assessable products, not private
reasoning:

```yaml
judgment:
  disposition: proceed | narrow | acquire_evidence | verify | route | block
  conclusion: ...
  status: established | provisional | unknown | blocked
  relied_on_evidence: [E1, E4]
  declared_assumptions: []
  preserved_unknowns: [G1]
  material_limitations: []
  requested_evidence: []
```

The reasoner should not be asked to reveal chain-of-thought. Concise support
lineage, assumptions, gaps, and conclusions are sufficient for evaluation.

Conditions should use neutral names and randomized presentation order. The
reasoner should not be told which condition it is receiving or what effect the
pilot expects.

---

## Controlled variables

Within a scored run, hold constant:

- model and exact model revision where available;
- provider and route;
- reasoning configuration;
- system, developer, role, and task instructions;
- tool availability, which is none in the first pilot;
- context-window policy;
- output schema;
- inference and output-token budgets;
- case truth table and admitted source facts;
- acceptance criteria;
- retry and failure policy;
- evaluation rubric.

Record but do not silently normalize:

- actual input and output tokens;
- latency;
- provider or transport failures;
- truncation;
- refusal;
- schema invalidity;
- model and route substitutions.

A substituted model or route is a different experimental subject and must not
be pooled silently with the intended condition.

---

## Repetition and ordering

Model outputs may vary even when the input is fixed. Each case-condition cell
should therefore be repeated under a frozen replication policy.

A bounded initial design could use:

```text
8 cases
× 4 conditions
× 3 repetitions
= 96 inference episodes
```

The exact size should be selected through a separately authorized cost and
power judgment before execution. This document does not authorize 96 calls.

Condition order should be randomized within each repetition. Surface labels
must not reveal the condition. If deterministic sampling is available, record
the seed; do not assume that a nominal seed makes provider execution perfectly
reproducible.

Pilot authors should conduct a non-scored fixture dry run before freezing the
subject. Dry-run cases or modified renderings must not be mixed into the
scored set.

Eight cases remain eight underlying problem instances even when each cell is
repeated three times. Repetitions characterize within-case output variability;
they do not turn the pilot into a population study or multiply the number of
independent cases.

---

## Evaluation

### Primary outcomes

The primary outcomes are decision-relative rather than stylistic:

- correctness of the required disposition or conclusion;
- unsupported material commitment rate;
- preservation of consequential explicit unknowns;
- recognition of ownership and authority boundaries;
- material relationship omissions;
- correct request for additional evidence when the basis is insufficient.

### Secondary outcomes

- unnecessary evidence requests;
- over-blocking when the basis is sufficient;
- declared unsupported assumptions;
- conclusion-to-evidence lineage quality;
- input and output tokens;
- latency;
- schema compliance.

Confidence should not be a primary outcome. A reasoner may be confidently
wrong, and subjective confidence does not establish support.

### Blinded scoring

Evaluators should score normalized outputs without seeing condition labels.
Where the presentation itself appears in the output, redact only what can be
removed without changing the judgment being scored.

Every case needs a preregistered rubric containing:

```text
required consequence
acceptable dispositions
material facts
material relationships
unknowns that must remain unknown
unknowns that may be bridged
prohibited commitments
acceptable evidence requests
severity of each error
```

At least a sample of cases should receive independent adjudication. Evaluator
disagreement must remain visible rather than being silently resolved into one
score.

### Case-level transition analysis

The primary analysis should preserve paired, case-level changes across B, C,
and D. For example:

```text
case: unnecessary completion

B
    proceeded
    replaced G1 with an unsupported value

C
    narrowed the conclusion
    preserved G1

D
    proceeded
    replaced G1 with an unsupported value
```

This transition is informative because it connects the representation and its
perturbation to the exact protected consequence. Aggregate rates, effect
magnitudes, and directional consistency may summarize the case records, but
they are secondary. P-values should not convert 8–12 authored cases into a
claim about a broad task population.

The scored result should retain, for every case:

```text
B -> C disposition transition
B -> C error-type transition
C -> D disposition transition
C -> D error-type transition
replicate agreement or variation
effect magnitude on each preregistered outcome
```

### Comparisons

The primary comparison is **C versus B**, not C versus A.

```text
C > B
    evidence that topology adds something beyond editorial clarity

C ≈ B and B > A
    evidence for clearer projection, not necessarily topology

C ≈ B ≈ A
    no demonstrated representation benefit on the tested cases

C < B
    evidence that topology adds cost, distortion, or over-constraint

D degrades predictably from C
    evidence that the represented relationship contributed to the result

D ≈ C on topology-dependent cases
    evidence that the topology was ignored, redundant, or poorly tested
```

No single aggregate score should erase distinct failure surfaces. Correctness,
unsupported commitment, unknown preservation, and cost should remain separately
inspectable.

---

## Evidence thresholds and stopping rules

Exact numeric thresholds should be frozen with the final case count before the
first scored inference. They should not be selected after seeing results.

The pilot should stop before scored execution if:

- common-content review finds answer leakage;
- the narrative condition cannot be made comparably clear;
- condition identity cannot be blinded adequately;
- gold judgments depend on disputed architectural meaning;
- token or instruction differences dominate the intended manipulation;
- the configured provider or model cannot be held stable;
- the execution budget or authority is absent.

The topology hypothesis should not advance on accuracy alone. A candidate
support rule should require all of the following directional evidence:

1. C improves at least one preregistered bounded-inference outcome over B on
   topology-sensitive cases;
2. the improvement is not purchased through a material correctness loss on
   topology-insensitive controls;
3. unsupported commitments or unknown destruction do not increase;
4. D changes outcomes in the predicted direction on cases whose critical edge
   was perturbed;
5. fixture review finds no answer-bearing semantic addition unique to C.

The hypothesis should collapse toward ordinary projection or pure composition
when B and C remain practically indistinguishable across the preregistered
outcomes and D does not show the predicted sensitivity.

It should be reconsidered adversely when C systematically increases error,
false confidence, over-blocking, token burden, or deference to unwarranted
edges.

One pilot cannot establish universality. A positive result is evidence for the
tested task and model distribution only.

---

## Interpretation and architectural consequences

### Result: narrative equals topology

Likely interpretation:

```text
clear projection is useful
explicit topology has not shown independent value
```

Candidate architectural consequence: improve projection contracts and retain
topology as an explanatory view or pure composition at the model-facing
boundary. This result does not reject a topology ledger as an upstream
formation, preservation, or audit representation.

### Result: topology improves bounded inference

Likely interpretation:

```text
relationship and gap preservation adds value beyond prose quality
```

Candidate next step: test whether a shared representation/protocol can preserve
those relationships without becoming a truth owner. Do not jump directly to a
service or canonical graph.

### Result: perturbation has no effect

Possible interpretations include:

- the critical edge was not actually critical;
- the task could be solved from item content alone;
- the reasoner ignored the representation;
- the edge was redundant with prose;
- the pilot lacks sensitivity.

This result requires case-level diagnosis, not automatic rejection or
acceptance.

### Result: topology harms performance

Possible causes include:

- excessive presentation density;
- false salience;
- poor edge taxonomy;
- ambiguity between edge meaning and establishment basis;
- over-trust in hypothesized or weak relationships;
- increased token or attention burden;
- task/model mismatch.

[Governed Causal Diagnosis and Epistemic
Learning](governed-causal-diagnosis-and-epistemic-learning.md) provides the
adjacent conceptual separation. The observed result alone does not identify
which cause is true.

---

## Follow-on pilot: active refinement

Only if the static pilot establishes useful signal should a later experiment
permit evidence acquisition.

That follow-on could compare whether an explicit topology helps a reasoner:

- notice a consequential gap;
- select a discriminating probe;
- avoid investigating irrelevant holes;
- update the local scaffold without overwriting source truth;
- stop after reaching the decision-specific sufficiency floor;
- escalate when authority or evidence access is exceeded.

The follow-on must remain subordinate to
[Decision-Specific Readiness](../../docs/architecture/decision-specific-readiness.md)
for sufficiency and to the active-acquisition contract for lawful inquiry. It
must not infer that a connected topology is ready merely because it is
connected.

---

## Follow-on pilot: representation invariance

Representation invariance is intentionally outside the first pilot. If C
shows an advantage, a follow-on should render one frozen topology through two
or more structurally equivalent serializations, for example:

```text
typed edge list

versus

nested relational manifest
```

The semantic topology, task, model, instructions, and budgets would remain
fixed. If only one serialization retains the advantage, the finding may be a
prompt-format effect. If multiple structurally equivalent renderings preserve
the benefit, that is stronger evidence that the underlying topology rather
than one notation contributed to the result.

This follow-on should occur only after the first pilot establishes a signal.
Adding it to the initial design would increase conditions and interpretation
cost before the core C-versus-B claim has earned that expense.

---

## Required pilot artifacts

Before scored execution, an admitted pilot should preserve:

```text
pilot contract and immutable revision
case-selection rationale
case truth tables
source evidence and revision bindings
A/B/C/D renderings
per-fixture B/C representation-difference rationales
edge-establishment stratum assignments
case-specific D predicted and non-predicted effects
fixture-review findings
randomization and repetition policy
model/provider/reasoning configuration
budget and stop authority
evaluation rubrics
blinding procedure
execution receipts
raw structured outputs
scored outcomes
paired case-level transition records
evaluator disagreements
limitations and deviations
```

Publication of these artifacts does not make their semantic conclusions
authoritative. Existing evidence, claim, readiness, decision, and review owners
retain their current authority.

---

## Non-goals

This pilot does not:

- prove that graph-shaped prompts are universally superior;
- compare every model, provider, role, or task class;
- optimize production prompts;
- authorize a canonical EvidenceTopology service;
- create a new truth owner for relationships;
- treat file proximity, retrieval rank, or graph reachability as relevance;
- use private chain-of-thought as evidence;
- evaluate online acquisition in its first phase;
- determine whether an upstream topology improves projection compilation,
  preservation, auditing, or revision maintenance;
- test representation invariance in its first phase;
- replace claim-evidence, Evidence Anchor, Decision-Specific Readiness, or
  implementation-basis assembly;
- treat an attractive visualization as empirical support;
- accept a design or authorize implementation from one favorable aggregate;
- hide null, adverse, or condition-specific findings.

---

## Open questions before execution

1. Which exact cases provide both causal isolation and realistic engineering
   consequences?
2. Who authors the case ledger, and who independently checks edge warrant?
3. What token-tolerance rule is strict enough to control presentation cost
   without forcing unnatural renderings?
4. Which relationships may appear in Condition C without encoding the answer?
5. What later design could test `bounded semantic judgment` edges without
   confusing topology benefit with semantic work performed during edge
   authoring?
6. What numeric advancement and collapse thresholds fit the final case count?
7. How should evaluator disagreement affect classification?
8. Should the first scored pilot use one model to isolate representation, then
   replicate across models only if signal appears?
9. Which repository-derived cases can be frozen without importing hidden
   context or contested architectural interpretation?
10. What exact evidence would justify a follow-on active-refinement pilot?

---

## Compact pilot contract

> Compare flat evidence, equally clear narrative evidence, explicit topology,
> and a preregistered topology perturbation over frozen matched cases. Hold
> model, task, source facts, tools, instructions, and budgets constant. Score
> decision correctness, unsupported commitments, unknown preservation,
> boundary recognition, material omissions, evidence requests, and cost
> without inspecting private reasoning. Treat C-versus-B as the primary test,
> D as a sensitivity test, and any favorable result as evidence for further
> classification work rather than authority to implement a service or accept
> an architecture. Interpret the result as evidence about model-facing
> representation only; upstream formation, preservation, auditing, and
> representation invariance require separate experiments.
