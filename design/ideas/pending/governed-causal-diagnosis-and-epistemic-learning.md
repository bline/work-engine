# Governed Causal Diagnosis and Epistemic Learning

## Status and authority

Exploratory architectural idea, directly captured from operator discussion on
2026-09-22.

This document does not accept a design, assign a permanent owner, authorize an
experiment or implementation, change production policy, grant research or
provider-spending authority, or establish that causal diagnosis is a new
architectural dimension. It records a candidate relationship among execution
traces, competing explanations, controlled comparison, and governed policy
learning.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-22
```

The candidate novelty is:

> Preserve the distinction between observing an episode, diagnosing competing
> contributing explanations, acquiring stronger causal evidence, proposing a
> future policy change, and authoritatively admitting that change.

---

## Recognition

Once execution-time evidence acquisition becomes adaptive, an observed outcome
does not reveal which upstream control was defective.

An implementation failure might reflect:

```text
an insufficient initial evidence projection
an adequate initial projection followed by weak inquiry
a poorly chosen probe
an ambiguous observation interpreted too strongly
a false accepted premise
lost salience under context pressure
incorrect implementation judgment despite adequate evidence
correct local results that fail during integration
an environmental or instrumentation defect
```

These explanations imply different remedies. Treating the outcome itself as
proof of one cause can make the system worse. For example:

```text
failure observed
    -> assume Builder lacked initial context
    -> increase plan detail
```

may be the wrong correction when the Builder already had adequate initial
context but failed to investigate a novel local ambiguity. The change would
increase context burden without repairing inquiry behavior.

---

## Four temporal questions

This idea is downstream of, but distinct from, the other episode-control loops:

```text
BEFORE
    adaptive plan resolution
        what should the executor initially receive?

DURING
    active evidence acquisition
        what should the executor investigate next?

AFTER
    causal diagnosis
        what competing explanations fit the episode and evidence?

ACROSS EPISODES
    epistemic learning and policy admission
        what future change, if any, is warranted?
```

The loops form a feedback system, not a single authority:

```text
historical capability evidence
        -> adaptive plan-resolution policy
        -> initial execution basis
        -> active evidence-acquisition trajectory
        -> bounded implementation judgments
        -> execution and outcome trace
        -> causal diagnosis
        -> candidate capability or policy update
        -> authority-bearing admission decision
        -> future resolution and inquiry policy
```

---

## Observation, diagnosis, discovery, and learning are distinct

### Observation

Records what happened, under which subject, revisions, actors, tools,
providers, environments, and evidence cutoffs.

### Mechanically established causality

Records a direct system-owned relation such as one admitted operation
publishing state that deterministically triggered a named transition. This is
stronger than temporal proximity because the owning operation or contract
establishes the edge.

### Causal diagnosis

Produces attributed competing or contributing explanations for an observed
outcome, each with its own evidence, counterevidence, confidence, limitations,
and causal strength. It need not establish one true cause.

### Causal discovery

Acquires stronger discriminating evidence through controlled comparison,
intervention, replay, natural experiment, or another governed research design.
A single observational trace rarely establishes this level on its own.

### Epistemic learning

Determines what capability or policy candidate the accumulated evidence
warrants proposing. It does not itself admit or activate the proposal.

### Admission

The existing authority owner decides whether the candidate becomes future
policy. Observation, diagnosis, discovery, and recommendation never acquire
that authority merely by producing evidence.

---

## Candidate diagnosis pipeline

```text
frozen execution episode
    exact plan, projection, inquiry, runtime, implementation,
    validation, review, and acceptance identities
        +
observed outcome and terminal accounting
        |
        v
mechanical correspondence and comparability checks
        |
        +-- noncomparable / incomplete -> unresolved
        |
        v
candidate contributing explanations
        |
        v
attributed diagnosis
    evidence and counterevidence per explanation
    confidence and limitations per explanation
    causal-strength qualification
    missing discriminating evidence
        |
        +-- adequate only for recommendation -> policy candidate
        +-- inadequate but consequential -> research candidate
        +-- insufficient and low value -> remain unresolved
        |
        v
separate authority-bearing policy decision
```

The diagnostic role does not rewrite the historical episode, alter the source
claims it examines, or select future production policy on its own standing.

---

## Candidate diagnosis record

An illustrative record could contain:

```yaml
diagnosis:
  subject_episode: exact-immutable-reference
  evidence_cutoff: exact-reference
  comparison_contract: exact-reference-or-unresolved
  observed_outcome: exact-reference
  explanations:
    - proposition: bounded-candidate-explanation
      evidence: []
      counterevidence: []
      confidence: qualified
      causal_strength: observational | mechanistic | discriminated | interventional
      limitations: []
      missing_discriminating_evidence: []
  unresolved_conflicts: []
  possible_consequences:
    - no_change
    - gather_more_evidence
    - propose_resolution_policy_change
    - propose_inquiry_policy_change
    - propose_runtime_or_role_change
    - propose_experiment
  authority_owner_for_each_consequence: []
  reopening_conditions: []
```

The causal-strength vocabulary is illustrative. It must not be adopted without
reconciliation against research and claim-evidence owners.

---

## Evidence required from the execution episode

A useful diagnosis needs more than a terminal success or failure label.

Candidate inputs include:

- exact accepted plan and implementation-contract revisions;
- initial plan and evidence projection identity;
- explicit coverage, omissions, and known uncertainties;
- active inquiry questions, probes, observations, and dispositions;
- provider, model, role, context, tool, and runtime realization identities;
- context-lifecycle and projection changes;
- implementation changes and immutable checkpoints;
- validation and review subjects, findings, and repairs;
- integration and acceptance consequences;
- cost, time, token, retry, and failure measurements;
- environmental state and instrumentation limitations;
- relevant alternatives that were available, prohibited, or unknown.

This does not require retaining private reasoning. It requires an attributed
causal surface over externally visible decisions, observations, effects, and
state transitions.

---

## Epistemic-control failure surfaces

Four failure surfaces are especially useful:

| Surface | Typical defect | Candidate remediation direction |
| --- | --- | --- |
| Initial representation | Required structure or evidence was not projected | Resolution or projection policy |
| Active inquiry | Available discriminating evidence was not acquired | Inquiry policy, capability, or contract |
| Judgment | Adequate evidence was present but interpreted incorrectly | Model, decomposition, independent review, or judgment contract |
| Causal learning | The outcome was attributed to the wrong upstream condition | Diagnosis, comparison, or research design |

These are not mutually exclusive and do not classify every implementation
failure. Observation defects, actuation errors, environmental failures,
integration failures, and authority failures remain separately possible.

The taxonomy exists to prevent every adverse outcome from collapsing into
"more context," "a smarter model," or "a stronger prompt."

---

## Comparison before causal explanation

Observed and predicted quantities are not automatically comparable.

Before diagnosis, a correspondence contract may need to establish:

- which observation answers which prediction or policy claim;
- subject and revision compatibility;
- units, scale, directionality, and aggregation window;
- evidence cutoff and environment;
- treatment of missing, censored, or partial observations;
- tolerance or interval semantics;
- whether one realized outcome can evaluate a probabilistic prediction;
- whether selected-strategy production evidence can support an alternative
  strategy claim.

Where the relationship is mechanically defined, the result may be derived as
`matched`, `diverged`, or `unresolved`. The reason for divergence remains a
separate semantic question.

---

## Controlled evidence and intervention

Production observes the strategy that was actually selected. It does not
directly reveal the counterfactual result of an unselected plan resolution,
inquiry policy, executor, or runtime realization.

Stronger causal evidence may require:

- a controlled pilot varying one projection or inquiry policy;
- a replay with declared reconstruction coverage;
- a deliberately discriminating probe inside an authorized sandbox;
- matched cohorts with explicit comparability limits;
- a natural experiment whose assignment process is understood;
- an intervention that changes one candidate cause while preserving named
  invariants.

Each intervention requires its own research, mutation, cost, provider, and
evidence-custody authority. A diagnosis cannot authorize its own experiment.

---

## Learning without silent optimization

Causal diagnosis may produce a candidate such as:

```yaml
learning_candidate:
  hypothesis: insufficient initial subsystem evidence increased reconstruction work
  applicable_scope: characterized-work-class
  evidence: []
  counterevidence: []
  causal_strength: provisional
  uncertainty: []
  candidate_change: raise subsystem projection resolution for this class
  expected_consequence: reduce repeated repository reconstruction
  possible_quality_loss: increased context burden and salience competition
  validation_required: controlled comparison
  authority_owner: named-policy-owner
```

The candidate remains advisory until admitted. Later evidence may strengthen,
narrow, falsify, or supersede it without rewriting the historical basis of an
earlier decision.

This preserves a closed learning loop without granting the system silent
self-optimization authority.

---

## Selection bias and exploration

An admitted policy affects which future evidence becomes observable. If the
system repeatedly chooses one executor or evidence projection, alternatives
may stop accumulating evidence and the incumbent may appear increasingly
superior simply because it has more observations and reusable context.

Candidate protections include:

- preserve the selection policy and its evidence revision for each episode;
- distinguish observed production results from controlled comparisons;
- keep uncertainty regions visible rather than converting missing evidence to
  poor capability;
- permit separately governed exploration where its expected decision value
  justifies cost and risk;
- avoid averaging severe boundary failures into a favorable aggregate rate;
- retain environment, task, projection, and verification conditions on every
  capability claim.

Exploration allocation is a policy and authority question, not a consequence
of uncertainty alone.

---

## Relationship to event introspection and causal observability

[`../../../app-server/docs/control-plane-causal-observability-ui.md`](../../../app-server/docs/control-plane-causal-observability-ui.md)
already separates observed facts, mechanically derived facts, semantic
interpretations, counterfactual estimates, and recommendations. That is the
right presentation boundary for this idea.

Event introspection can provide:

- exact temporal order;
- operation and transition identities;
- subject, actor, provider, context, and runtime bindings;
- deterministic state-transition edges;
- measurements and attributable failures;
- the trace from aggregate back to source evidence.

It cannot infer semantic relevance, missing-but-unobserved causes, or the true
explanation of an outcome merely from chronology. Diagnosis consumes the trace;
it does not become part of the event observer.

---

## Relationship to claim-evidence

[`../../architecture/evidence-and-claims.md`](../../architecture/evidence-and-claims.md)
already identifies a reusable representation pattern for attributed diagnostic
findings:

```text
correspondence basis
competing or contributing explanations
evidence, confidence, and limitations per explanation
consequence linkage and reopening conditions
```

Claim-evidence may be able to materialize a diagnosis through a domain profile.
It does not thereby own which explanation is warranted. A durable diagnosis is
a materialization of another domain's judgment, never a second causal owner.

The same page also preserves a key separation: a nomination's resolution and
an episode's causal attribution are separate facts and must never be derived
from one another automatically.

---

## Relationship to closed-loop engineering learning

[`../../../app-server/docs/closed-loop-engineering-learning-reconciliation.md`](../../../app-server/docs/closed-loop-engineering-learning-reconciliation.md)
accepts, for design work only, two unresolved consequences:

- a prediction-to-outcome correspondence contract; and
- an attributed calibration diagnosis preserving multiple candidate
  explanations with their own evidence, confidence, limitations, and causal
  strength.

That reconciliation concerns proposal-evaluation prediction accuracy. This
idea asks whether the same guarded diagnosis shape also applies to execution
resolution, inquiry, runtime, and capability learning. Similar representation
does not prove one universal semantic owner.

---

## Relationship to adaptive plan resolution

[`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md)
proposes using execution evidence to calibrate projection profiles and runtime
strategy. It correctly states that production observes only the chosen
strategy, that reconstruction may be incomplete, and that evidence updates do
not authorize policy changes.

This idea supplies a possible missing bridge:

```text
execution observations
    -> episode comparability
    -> competing causal diagnosis
    -> capability or policy candidate
    -> authorized policy revision
    -> future adaptive resolution
```

It does not place causal diagnosis inside the resolution policy itself.

---

## Relationship to active evidence acquisition

[`active-evidence-acquisition-during-execution.md`](active-evidence-acquisition-during-execution.md)
describes the online inquiry trajectory whose consequences become part of the
post-episode causal surface.

Causal diagnosis may ask whether:

- the necessary question was visible;
- a discriminating probe was available and authorized;
- the selected probe could distinguish the candidate interpretations;
- the observation was complete and correctly bound;
- the result changed the explicit uncertainty state;
- stopping or escalation matched the protected consequence.

Hindsight alone is insufficient. A later-discovered perfect probe does not
prove that the executor should have selected it unless it was reasonably
available under the evidence, capability, authority, and cost state at that
time.

---

## Relationship to evidence-basis topology

[`evidence-basis-topology-for-bounded-inference.md`](evidence-basis-topology-for-bounded-inference.md)
explores the scaffold of anchors, relationships, boundaries, conflicts, and
explicit gaps that projection forms, inquiry refines, and bounded inference
traverses. This diagnosis loop may use scaffold-defect subtypes to distinguish
initial-topology, projection-salience, refinement, and unsupported-traversal
explanations without treating any subtype as the cause merely because it fits
the trace.

---

## Coupled control theory, separate owners

These ideas suggest a coherent higher-level principle:

> Allocate semantic resolution, evidence attention, execution vantages, and
> authority proportionately to the consequences and uncertainty currently
> exposed.

The corresponding views remain distinct:

```text
semantic resolution
    at what conceptual grain judgment occurs

organizational compilation
    where and under what independence/continuity judgment occurs

evidence projection
    what the judgment initially sees

active evidence acquisition
    how that evidence basis evolves

authority
    what effects and consequences may be committed

causal diagnosis
    how outcomes are explained and uncertainty preserved

policy admission
    how future allocation rules may change
```

This may be one core theory of Work Engine. It must not become one master
optimizer, one universal score, or one semantic owner.

---

## Failure modes

### Chronology becomes causality

Events occurring near one another are treated as a causal relation without an
owning transition, comparison contract, or attributed interpretation.

### Adverse outcome becomes proof

A failure automatically proves the plan, Builder, projection, or policy was
wrong.

### One explanation wins by convenience

Competing causes and counterevidence are discarded to produce a clean story.

### Correlation silently changes policy

An observed association bypasses research, admission, cost, or product
authority.

### Selected-strategy bias

Production evidence for the incumbent is compared with missing or reconstructed
evidence for alternatives as though both were equally observed.

### Hindsight probe optimality

A probe identified after the outcome is treated as obviously selectable during
the original episode.

### Learning target collapse

Initial projection, inquiry behavior, implementation judgment, runtime
realization, and integration are collapsed into one capability score.

### Diagnostic owner collapse

The evidence store, event observer, reviewer, or policy engine acquires the
domain authority to decide which causal explanation is true.

### Silent experimentation

Production variation is introduced for learning value without separate
research and effect authority.

---

## Candidate evidence for distinct architectural treatment

Evidence supporting a distinct diagnosis/learning concept would include:

- one real episode with several plausible upstream causes requiring different
  remediations;
- a mechanically complete trace that remains insufficient to choose among
  those explanations;
- a controlled comparison that materially changes causal strength without
  changing the underlying episode history;
- the same attributed multi-explanation diagnosis shape recurring across
  proposal calibration, plan resolution, inquiry policy, and runtime strategy;
- a policy correction shown harmful because the original cause was
  misattributed;
- a durable diagnosis later revised without rewriting the evidence or policy
  decision that relied on its earlier revision.

Evidence against a distinct concept would include every candidate obligation
reducing cleanly to existing research, proposal evaluation, claim-evidence,
review, and policy-admission owners without a missing relationship.

---

## Questions for later intake and reconciliation

1. Which domain owns diagnosis truth for execution-resolution and inquiry
   outcomes?
2. Is there one reusable diagnosis mechanism, or only a common representation
   pattern across domain-owned judgments?
3. What causal-strength vocabulary is truthful across observational,
   mechanistic, replay, and intervention evidence?
4. What exact comparison contract makes two episodes, strategies, or
   predictions commensurable?
5. Which inquiry events must be durable for later diagnosis without retaining
   private reasoning?
6. How are multiple contributing explanations combined without producing a
   false singular verdict or an unhelpful list?
7. When does uncertainty justify a controlled experiment, and who owns that
   research decision?
8. How does the system distinguish an unavailable counterfactual from evidence
   that an alternative would fail?
9. What policy owner may consume diagnoses about plan resolution, inquiry,
   runtime realization, or organizational topology?
10. How are selection bias, incumbency, and exploration budgets represented?
11. Can causal learning be evaluated without the same diagnosis machinery
    grading itself?
12. Does the coupled control theory name a real architecture-level invariant,
    or only a useful explanatory view over existing owners?

---

## Non-goals

This idea does not currently propose:

- deriving causality from chronology alone;
- requiring one true cause for every outcome;
- treating one adverse outcome as proof a probabilistic prediction was wrong;
- giving event introspection semantic judgment authority;
- giving claim-evidence ownership of diagnosis truth;
- automatically changing plan resolution, routing, model, review, or inquiry
  policy;
- assigning blame to a Builder, planner, model, or provider from a terminal
  outcome;
- treating every production episode as a controlled experiment;
- authorizing exploration traffic or external effects;
- collapsing all capability evidence into one score;
- creating one master optimizer for Work Engine; or
- implementing a new diagnosis, experiment, learning, or policy service from
  this capture.

---

## Compact hypothesis

> A governed learning loop must preserve observation, mechanical causality,
> competing semantic diagnosis, discriminating intervention evidence, policy
> recommendation, and authority-bearing admission as separate acts. The loop
> can improve future plan resolution and evidence-acquisition policy without
> treating correlation as cause, diagnosis as authority, or learning as silent
> self-optimization.
