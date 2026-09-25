# Active Evidence Acquisition During Execution

## Status and authority

Exploratory architectural idea, directly captured from operator discussion on
2026-09-22.

This document does not accept a design, assign a permanent semantic owner,
change the roadmap, authorize implementation, grant evidence access, increase
an execution role's authority, or establish a new architectural dimension. It
records a candidate execution-time control loop for later intake and
reconciliation.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-22
```

The candidate novelty is narrow:

> Treat execution-time evidence acquisition as an explicit, governed control
> loop in which an executor selects admissible probes for their expected value
> to a pending consequential decision, updates an externalized evidence basis,
> and stops, acts, blocks, or escalates without requiring either omniscience or
> a fixed evidence quota.

---

## Recognition

An implementation plan and its initial evidence projection cannot contain every
fact that will become relevant during execution.

Some facts do not exist until an action is taken. Others exist but cannot be
recognized as relevant until an earlier observation exposes a particular
ambiguity. The effective evidence basis is therefore path-dependent:

```text
E0 = initial evidence basis
A1 = first execution or inquiry action
O1 = attributed observation produced by A1

E1 = E0 + admitted consequence of O1

judgment(E1) -> A2
O2 = attributed observation produced by A2

E2 = E1 + admitted consequence of O2
...
```

This does not mean every observation becomes canonical evidence or that the
executor may revise the accepted plan. It means the executor's legitimate work
contains a recurring inquiry function:

```text
hypothesis or unresolved question
        -> admissible probe
        -> observation
        -> updated explicit uncertainty
        -> implement, investigate again, block, or escalate
```

An initial-context optimizer can improve `E0`. It cannot eliminate the online
loop.

---

## Four temporal questions

This idea depends on separating four questions around an execution episode:

| Position | Question | Candidate owner relationship |
| --- | --- | --- |
| Before | What established structure and evidence should the executor initially receive? | Adaptive plan resolution and contract compilation |
| During | Given the current evidence basis and pending decision, what should be investigated next? | This candidate execution-time inquiry loop |
| After | What competing explanations best account for the observed episode? | Governed causal diagnosis, separately explored |
| Across episodes | What durable capability or policy change is warranted? | Existing policy authority informed by learning evidence |

The four questions exchange evidence but do not acquire one another's
authority.

---

## The executor performs active epistemic control

Calling the loop "evidence gathering" is incomplete. Some execution actions
exist primarily to change the executor's decision-relevant knowledge.

For example:

```text
read implementation
    -> ambiguous ownership discovered

inspect callers and contracts
    -> two interpretations remain

run a focused test
    -> one interpretation falsified

inspect the accepted architecture boundary
    -> remaining route appears delegated

modify implementation
    -> broader validation exposes a distant failure

trace the failure
    -> accepted plan premise may be incomplete
```

The candidate control question is:

> Which currently admissible action is likely to reduce uncertainty that can
> change the next protected consequence?

"Likely" need not imply a numerical information-value score. It may remain a
bounded semantic judgment over the question, available probes, costs, effects,
and possible consequences.

---

## Externalized epistemic state, not private reasoning

The architecture should not require access to, publication of, or persistence
of a model's hidden reasoning or internal belief state.

The durable or host-visible boundary may instead preserve only the
decision-relevant epistemic consequences:

```text
current evidence basis and exact subject revisions
explicit unresolved question
why the question can affect a protected consequence
candidate interpretations, where material
known coverage, omissions, conflicts, and limitations
selected probe and its authority/effect basis
attributed observation and integrity reference
disposition of the uncertainty
next allowed consequence
```

A concise inquiry record might have the following illustrative shape:

```yaml
inquiry_step:
  episode: execution-episode-id
  subject: exact-artifact-or-state-revision
  question: bounded-unresolved-question
  consequence_at_risk: named-contract-consequence
  current_basis: [exact-evidence-references]
  known_unknowns: []
  selected_probe:
    kind: repository_query | inspection | deterministic_check | test | runtime_probe | experiment
    expected_discrimination: qualitative-description
    authority_reference: exact-reference
    effect_class: read_only | sandbox_mutation | governed_external_effect
  observation:
    reference: exact-observation-reference
    coverage: explicit
    limitations: []
  disposition: continue_inquiry | sufficient_for_action | tolerate_uncertainty | block | escalate
```

This is an illustrative semantic shape, not a proposed canonical schema.

---

## Consequence-sensitive inquiry

Neither of these policies is sufficient:

```text
collect until confident
collect N files / N tokens / N tests
```

Confidence may be poorly calibrated. Fixed quotas are not decision-relative.

An unresolved uncertainty is a candidate for further investigation when it
could:

- change an authorized implementation choice;
- expose a material decision outside the executor's delegation;
- invalidate an accepted plan, architecture, or dependency premise;
- change the validation needed to establish completion;
- alter an integration or recovery consequence;
- expose an authority, security, confidentiality, or effect-boundary conflict;
- prevent a consequential failure at proportionate acquisition cost.

Otherwise the uncertainty may be recorded and tolerated if the governing
contract permits action without resolving it.

The executor can therefore stop while uncertainty remains. The objective is
not omniscience. It is sufficient evidence for the consequences the executor
is authorized to realize.

---

## Two stopping bounds

### Sufficiency floor

Do not act when a material uncertainty lacks evidence required by the contract
or when the available basis cannot truthfully support the intended consequence.

### Marginal-value ceiling

Do not continue gathering evidence when another admissible probe is unlikely
to alter an authorized action, protected consequence, required validation, or
legitimate escalation.

The ceiling remains subordinate to mandatory evidence and safety requirements.
A cheap next probe cannot justify stopping below the sufficiency floor, and an
expensive probe is not required merely because it might satisfy curiosity.

Candidate terminal outcomes include:

```text
sufficient_for_delegated_action
uncertainty_tolerated_by_contract
requires_more_evidence
returned_for_material_decision
accepted_premise_falsified
blocked_by_unavailable_evidence
inquiry_budget_exhausted
```

Whether these names correspond to a new mechanism, domain-local statuses, or
existing outcome vocabularies is unresolved.

---

## An acquisition profile, not a universal depth ladder

Evidence-acquisition actions vary along several independent dimensions:

- direct and indirect monetary cost;
- latency and context burden;
- subject breadth;
- expected discriminating value;
- evidentiary strength and reproducibility;
- environment fidelity;
- mutation or environmental effect;
- reversibility;
- authority and access required;
- confidentiality and evidence-custody consequences.

An illustrative progression remains useful:

```text
consume supplied projection
inspect already indexed evidence
query repository or dependency surfaces
run deterministic checks or tests
create a discriminating probe
vary implementation or environment under a governed experiment
```

But this is not a monotonic quality or cost scale. A focused test may be cheaper
and more discriminating than a broad search. A small runtime probe may be more
intrusive than a large read-only inspection. The governing representation is
therefore more plausibly a multidimensional acquisition profile or inquiry
envelope than one scalar `depth`.

---

## Candidate execution evidence contract

Adaptive plan resolution or implementation-contract compilation might provide
the executor's initial epistemic conditions without owning the later inquiry
trajectory:

```yaml
execution_evidence_contract:
  initial_basis: []
  known_uncertainties: []
  delegated_questions: []
  required_evidence_classes: []
  admissible_sources: []
  admissible_probe_profiles: []
  evidence_custody_and_confidentiality: []
  must_escalate_if: []
  required_completion_evidence: []
  inquiry_cost_and_effect_ceilings: []
  tolerated_uncertainty: []
```

`admissible_sources` alone is insufficient. Permission to read a source is not
permission to mutate an environment, run an external experiment, incur new
cost, retain confidential material, or publish a semantic conclusion.

The contract provides the initial envelope. The executor selects among lawful
actions inside it. Anything outside it requires the authority already assigned
to that effect or decision; evidence need never mints authority.

---

## Relationship to Builder and Supervisor

Reconnaissance is a recurring function, not inherently a dedicated role.

The retained Builder or Supervisor context often has the best vantage for local
inquiry because it can carry accumulated understanding directly into execution.
Separating reconnaissance into a disposable scout may force the executor to
reconstruct the very understanding the probe produced.

Organizational Compilation may still derive a separate reconnaissance vantage
when a stable boundary and protected consequence justify it, for example:

- independent evidence is required;
- the evidence surface would overwhelm the execution context;
- different access or effect authority is required;
- investigation can be partitioned cleanly;
- continuity requirements differ;
- several concerns need independent inquiry and conflict-preserving fan-in.

The role name does not decide the inquiry function. The compiled contract does.

---

## Relationship to adaptive plan resolution

[`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md)
asks how much established structure should be exposed to a particular executor
before execution. It already distinguishes production cost, ingestion cost,
and attention burden, and seeks a minimum sufficient resolution by dimension.

This idea begins where that initial projection stops:

```text
adaptive resolution
    -> initial representation and inquiry envelope

active evidence acquisition
    -> path-dependent evolution of the evidence basis during execution
```

Online inquiry may request a more explicit projection of meaning that is
already canonical. It may not silently use "more resolution" to close a new
material decision. If inquiry exposes missing semantic meaning, the executor
returns it to the existing decision owner or conflict route.

---

## Relationship to resolution-layered refinement

[`resolution-layered-semantic-refinement.md`](resolution-layered-semantic-refinement.md)
separates semantic resolution, organizational shape, projection resolution,
and review grain. This idea adds a possible fifth independent concern:

```text
evidence acquisition profile
    how the evidence basis may evolve through lawful observation actions
```

Each semantic-resolution layer may require a different initial evidence basis,
set of admissible probes, and stopping condition. That does not make evidence
acquisition itself a new semantic-resolution layer.

---

## Durability and observability

The inquiry trajectory should remain reconstructable without publishing a
reasoning transcript.

Useful attributed events or records may include:

- the exact execution episode, role instance, contract, and subject revision;
- the unresolved question and protected consequence;
- the probe selected and authority/effect envelope used;
- provider, tool, repository, test, or environment identity;
- observations, coverage, omissions, failures, and integrity references;
- whether evidence changed the implementation route, validation, or escalation;
- repeated acquisition caused by missing or stale durable evidence;
- the terminal inquiry disposition.

These records could support deterministic operational status and later causal
diagnosis. They do not themselves establish why the executor selected the
probe, whether that selection was optimal, or what caused the final outcome.

---

## Candidate quality and efficiency observations

Evidence that could later calibrate this idea includes:

- probes performed before a material route change;
- repeated queries whose result was already available durably;
- material failures preceded by an explicit unresolved question;
- failures where discriminating evidence was available but not acquired;
- evidence volume and source count by inquiry episode;
- time, tokens, provider cost, and environmental effects by probe class;
- inquiry steps producing no change in any protected consequence;
- blocked outcomes caused by unavailable evidence versus exhausted authority;
- escalations that exposed genuinely upstream material decisions;
- successful action with explicitly tolerated residual uncertainty.

No single ratio establishes inquiry quality. A costly probe may prevent a much
larger failure, while a cheap search may add noise without discrimination.

---

## Failure modes

### Search until confidence

Subjective confidence becomes the stopping owner despite weak calibration.

### Fixed evidence quotas

File, token, test, or tool-call counts substitute for decision-relative
sufficiency.

### Inquiry as hidden replanning

The executor resolves a new material decision while describing the act as
evidence gathering.

### Evidence as authority

The actor nearest an observation acquires authority over the plan or source
truth the observation may falsify.

### Probe effects hidden as reads

A test, runtime probe, experiment, or environment variation mutates state or
incurs cost outside the declared effect envelope.

### Unbounded discovery drift

Investigation expands without a named question or protected consequence.

### Context as canonical state

Evidence remains only in the retained model context and is lost or
reconstructed after replacement.

### Transcript capture as observability

The system stores private reasoning instead of the smaller attributed sequence
of questions, probes, observations, dispositions, and consequences.

### Scalar acquisition depth

Cost, intrusiveness, authority, fidelity, and evidentiary value are collapsed
into a false single ladder.

---

## Relationship to existing architecture

This idea appears to compose with:

- [`../../docs/architecture/implementation-contract-compilation.md`](../../docs/architecture/implementation-contract-compilation.md)
  for the initial implementation basis, evidence requirements, and honest
  `returned_for_decision` / `blocked_by_evidence` boundary;
- [`../../docs/claim-evidence-service.md`](../../docs/claim-evidence-service.md)
  for attributed observations, bounded evidence projections, durable claims,
  and exact-revision reliance;
- [`../../docs/architecture/evidence-and-claims.md`](../../docs/architecture/evidence-and-claims.md)
  for materialization, lineage, reliance, and the rule that evidence never
  acquires source authority;
- [`../../docs/architecture/substrates/evidence-anchor.md`](../../docs/architecture/substrates/evidence-anchor.md)
  for exact-subject observation, coverage, and mechanical correspondence;
- [`../../docs/architecture/organizational-compilation.md`](../../docs/architecture/organizational-compilation.md)
  for deciding when another legitimate vantage is justified;
- [`../../docs/architecture/context-lifecycle.md`](../../docs/architecture/context-lifecycle.md)
  for preserving continuation meaning while model context changes;
- [`evidence-basis-topology-for-bounded-inference.md`](evidence-basis-topology-for-bounded-inference.md)
  for the candidate decision-relative scaffold that inquiry may refine without
  acquiring evidence, relevance, or readiness authority; and
- [`governed-causal-diagnosis-and-epistemic-learning.md`](governed-causal-diagnosis-and-epistemic-learning.md)
  for the separate post-episode diagnosis and learning loop; and
- [`observation-consequences-and-evidence-basis-evolution.md`](observation-consequences-and-evidence-basis-evolution.md)
  for further reconciliation/decomposition of the "admitted consequence"
  step this document leaves undefined -- it narrows, without resolving,
  what an attributed observation may legitimately do to episode-local
  inquiry state versus durable evidence and topology; no classification is
  accepted there either.

Whether execution-time evidence acquisition is a new dimension, a reusable
mechanism, a Builder/Supervisor contract concern, or only a cross-view
description is unresolved.

---

## Candidate evidence for distinct architectural treatment

Evidence supporting a distinct concept would include:

- a real execution in which the decisive evidence did not exist or was not
  recognizable at initial projection time;
- a probe selected primarily for discriminating value rather than direct
  implementation effect;
- a consequence-sensitive stopping decision that cannot be represented as
  ordinary plan completion or generic tool use;
- the same inquiry shape recurring in Builder, Supervisor, review, or planning
  domains without sharing their semantic judgments;
- a failure correctly localized to inquiry selection rather than initial
  context, implementation judgment, or instrumentation;
- measurable benefit from preserving inquiry state across context replacement.

Evidence against a distinct concept would include every candidate obligation
reducing cleanly to existing implementation-contract fields, ordinary role
discretion, claim-evidence operations, and current stop/escalation semantics.

---

## Questions for later intake and reconciliation

1. What exact truth, if any, would this candidate own that existing role and
   contract structure does not?
2. Is an inquiry episode durable semantic state or only attributed telemetry?
3. Who declares the question and consequence against which a probe is judged
   relevant?
4. Which probe-selection facts are deterministic, and which require bounded
   semantic judgment?
5. How is evidence sufficiency distinguished from implementation confidence?
6. Can one role select a probe and judge its result without unacceptable
   self-confirmation risk?
7. When does inquiry require an independent or separately privileged vantage?
8. What preserves useful inquiry continuity without retaining private
   reasoning?
9. How do inquiry budgets compose across cost, latency, effects,
   confidentiality, and context burden?
10. Which existing terminal outcomes can represent stopping, blocking, and
    escalation without a new vocabulary?
11. How does the system prevent retrieved evidence from becoming authoritative
    merely because it appears in model context?
12. What evidence would show that a different probe should have been selected
    without relying on hindsight alone?

---

## Non-goals

This idea does not currently propose:

- exposing or retaining chain-of-thought;
- treating model confidence as evidence sufficiency;
- requiring a dedicated reconnaissance role;
- granting Builders authority to revise accepted plans;
- granting access, mutation, cost, or publication authority through an inquiry
  request;
- loading all potentially relevant evidence into model context;
- requiring numerical information-value estimates;
- imposing a universal evidence quota or acquisition-depth ladder;
- turning every tool call into a canonical claim;
- treating every implementation failure as an inquiry failure; or
- implementing a new service, schema, role, or policy from this capture.

---

## Compact hypothesis

> Execution requires an online, consequence-sensitive evidence-acquisition
> loop in addition to an initial plan projection. An executor may select lawful
> probes within an explicit inquiry envelope, externalize the resulting
> questions and observations without exposing private reasoning, and stop with
> residual uncertainty once the evidence is sufficient for its delegated
> consequence. Missing semantic meaning, unavailable evidence, exhausted
> authority, and falsified premises remain distinct outcomes routed to their
> existing owners.
