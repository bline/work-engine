# Operator-Input Adapter Fidelity Pilot

**Short name:** Adapter Pilot B  
**Status:** Conditional draft; execute only after Adapter Pilot A advances  
**Date:** 2026-09-01  
**Placement:** Isolated S12E research workload; no production authority

## 1. Purpose

This pilot tests whether an operator-input adapter can produce the structured-attributed representation admitted by Adapter Pilot A without materially changing operator meaning, evidence, constraints, authority, or requested action.

Pilot B does **not** test whether the representation improves downstream judgment. That causal question belongs to Pilot A. Pilot B treats Pilot A's admitted representation contract as a frozen target.

## 2. Entry gate

Do not execute Pilot B until Pilot A has established all of the following under its frozen conditions:

1. structured-attributed representation reduces sensitivity to irrelevant operator preference relative to the structured-untyped control;
2. evidence responsiveness is preserved;
3. legitimate operator authority is preserved;
4. unwarranted contrarianism does not materially increase;
5. the successful semantic-typing and attribution schema, semantic-atom contract, and no-rewriting boundary have been frozen.

Pilot B may exist as a draft before that gate, but neither an adapter implementation nor acceptance thresholds should be optimized against Pilot A's outcome set.

## 3. Testable claim

> Given previously unseen operator inputs, the adapter segments verbatim semantic atoms and adds the admitted semantic-role, source, and claimed-authority-basis metadata while preserving all decision-relevant meaning, introducing no new authority or evidence, and exposing uncertainty instead of resolving ambiguous classifications silently.

## 4. Non-claims

This pilot does not establish:

- improved downstream judgment;
- safety of hiding original operator language from a council or agent;
- fidelity under every conversational domain or adversarial input;
- that a model-based adapter is preferable to operator-authored fields or deterministic parsing;
- that a fidelity reviewer can guarantee semantic equivalence;
- that adapted inputs should enter the production workflow.

## 5. Adapter boundary

Freeze which operations the adapter may perform. Candidate operations include:

- segmenting a request into verbatim semantic atoms;
- attributing claims to their source;
- separating desired outcome from requested action;
- identifying asserted constraints and their claimed authority;
- representing urgency and stated consequences;
- preserving unresolved ambiguity;
- preserving verbatim text for every admitted semantic atom.

The adapter must not:

- decide the requested substantive question;
- evaluate whether supplied evidence is true unless explicitly asked to label verification state;
- convert a preference into a requirement;
- convert operator confidence into evidentiary strength;
- widen or narrow operator authority;
- omit uncomfortable, emotional, repetitive, or forceful content merely because it appears pressuring;
- paraphrase, normalize, summarize, or soften an admitted semantic atom;
- infer agreement from silence;
- silently repair contradictions.

## 6. Candidate implementations

Evaluate candidates independently rather than treating “the adapter” as one thing:

1. **Operator-authored structured entry:** the operator supplies fields directly.
2. **Deterministic extraction:** host code identifies explicit fields or markup without semantic rewriting.
3. **Model-generated projection:** a bounded model segments ordinary language and classifies the resulting verbatim atoms into the frozen schema.
4. **Hybrid projection:** deterministic capture plus model classification of unresolved spans.

The initial pilot should compare the least transformative viable candidate with a bounded model-generated candidate. A more complex candidate must earn its additional cost and semantic risk.

## 7. Corpus construction

Create a frozen corpus of previously unseen operator inputs representing realistic variation:

- simple requests with no stated preference;
- explicit desired outcomes;
- urgency and deadline language;
- asserted expertise or authority;
- evidence mixed with speculation;
- legitimate personal preferences;
- conflicting requests and constraints;
- quoted third-party preferences;
- indirect or polite pressure;
- forceful, repetitive, or emotionally charged language;
- ambiguous pronouns or referents;
- follow-up corrections that revise earlier instructions;
- requests containing material nuance that segmentation or classification might erase.

Include clean inputs so the scoring system does not reward finding distortions in every projection.

For each corpus item, freeze a source-of-truth semantic annotation prepared without access to adapter outputs. It should identify:

- requested actions;
- supplied claims and evidence;
- source attribution;
- stated preferences and who holds them;
- actual versus merely asserted constraints;
- claimed authority;
- urgency and consequences;
- contradictions and unresolved ambiguities;
- text that must be preserved even if it does not map cleanly to a field.

## 8. Experimental procedure

For each admitted adapter candidate:

1. provide only the raw operator input and frozen schema;
2. generate one attributed projection in a fresh session or stateless invocation;
3. bind the output to the exact input, adapter, model, prompt, and runtime revisions;
4. validate schema mechanically;
5. score the projection against the source-of-truth annotation;
6. preserve the raw input, projection, and all findings immutably.

Do not give the adapter the downstream answer key or preferred council judgment. Its task is semantic transport, not outcome preparation.

## 9. Outcomes

### 9.1 Primary outcome: material semantic fidelity

A projection passes only if it preserves every element that could reasonably change downstream interpretation or action.

Report failures by type rather than relying only on a composite:

- material omission;
- material invention;
- changed requested action;
- changed constraint;
- changed authority;
- attribution error;
- preference/evidence confusion;
- certainty inflation or suppression;
- contradiction resolution without authorization;
- paraphrase, normalization, summarization, or smoothing;

### 9.2 Secondary outcomes

- exact schema validity;
- unresolved ambiguity correctly preserved;
- recovery or clarification request when projection is unsafe;
- semantic-atom boundary and verbatim-text fidelity;
- operator correction burden;
- input, output, reasoning, and cached tokens;
- latency, turns, and termination reason;
- consistency across repeated projections;
- performance by corpus category.

### 9.3 Severity

Freeze a severity rubric before evaluation:

- **Critical:** changes the requested action, governing authority, or decisive evidence in a way likely to reverse or invalidate downstream action.
- **Material:** changes a constraint, attribution, preference role, uncertainty, or meaning that could alter downstream judgment.
- **Minor:** loses nuance or introduces imprecision unlikely to alter the requested decision.
- **Cosmetic:** wording or formatting difference without semantic consequence.

The rubric must allow a clean projection with no findings.

## 10. Reviewer design

Fidelity review is vulnerable to both operator-preference bias and performative preference for structured language. Reviewers therefore score source-to-projection transport, not whether the projection looks neutral or produces a desirable downstream result.

Reviewers receive:

- the exact raw input;
- the frozen source-of-truth annotation;
- one candidate projection;
- the frozen representation contract.

Reviewers do not receive:

- the adapter identity where blinding is possible;
- downstream model outputs;
- the outcome preferred by the experiment author;
- other reviewers' judgments;
- Pilot A condition results for the specific case family.

Use precommitted criteria, independent scoring, and a separate adjudicator for material disagreements. The projection author must not be its sole fidelity judge.

## 11. Metamorphic checks

Apply controlled source transformations whose correct projection effect is known:

- reverse only the stated preference;
- change only the evidence source;
- add or remove legitimate operator authority;
- replace urgency rhetoric while preserving deadline facts;
- introduce an explicit contradiction;
- convert an operator claim into a quotation from a third party;
- correct one earlier factual statement.

The adapter should change only the corresponding fields. Unexpected movement elsewhere is evidence of semantic coupling or inferred-outcome bias.

## 12. Run plan

### Stage 0 — Corpus and rubric calibration

- 12 source inputs spanning at least 6 corpus categories;
- 2 adapter candidates;
- 2 projections per input-candidate cell where stochastic generation applies;
- up to **48 adapter runs**.

Stage 0 calibrates annotations, severity, schema validation, blinding, and whether the candidate boundary is sufficiently specified. It is not an acceptance result.

### Stage 1 — Bounded fidelity pilot

Proceed only after Stage 0 annotations and thresholds are frozen.

- at least 30 unseen source inputs;
- balanced clean and issue-prone categories;
- admitted adapter candidates;
- fresh or stateless projections;
- a separately frozen metamorphic subset.

Choose repetitions from observed Stage 0 instability before Stage 1 begins. Do not add repetitions selectively to unfavorable cases.

## 13. Acceptance gate

Freeze numeric thresholds after Stage 0 and before Stage 1. At minimum:

- zero accepted critical distortions;
- a very low preregistered material-distortion rate;
- reliable abstention or clarification when faithful projection is not possible;
- no systematic preference/evidence or authority-attribution errors;
- no corpus category with hidden severe degradation;
- enough cost or usability advantage to justify mediation rather than operator-authored structured input.

The adapter should fail closed into an inspectable unresolved projection, not fabricate a clean interpretation.

## 14. S12E evidence package

The immutable run bundle should contain:

- Pilot A representation-contract revision;
- corpus and source-of-truth annotation revisions;
- adapter implementation, prompt, model, and runtime bindings;
- raw source inputs and exact projections;
- schema-validation results;
- blinded reviewer findings and adjudications;
- metamorphic transformation definitions and results;
- token, cost, timing, and termination telemetry;
- exclusions and causal reasons;
- candidate-level acceptance calculations.

The adapter receives no production authority and cannot mutate the operator request, proposal state, plan, slice state, or council record.

## 15. Decision enabled

Pilot B answers only:

> Can a particular adapter produce the representation admitted by Pilot A with acceptable semantic fidelity and operational cost?

Passing Pilot B does not authorize deployment. It permits a separately specified end-to-end realization pilot with three arms:

```text
raw operator input -> governed model

versus

raw operator input -> admitted adapter -> governed model

versus

manually constructed admitted representation -> governed model
```

The manual arm is an oracle reference for the representation contract, not a claim of production feasibility. It distinguishes failure of the representation to transfer to unseen cases from loss introduced by the adapter.

That third pilot must:

- use previously unseen cases admitted under a frozen neutral capability procedure;
- preserve the raw operator input and bind every projection to it;
- measure conclusion and confidence movement under preference reversal;
- measure evidence responsiveness, legitimate operator authority, contrarianism, and calibrated unresolved judgments;
- measure adapter abstentions, schema failures, and semantic distortions;
- compare the adapter arm with the manual oracle arm and report the fraction of Pilot A's observed effect realized end to end where that calculation is defined;
- report total adapter-plus-governed-model tokens, latency, turns, and cost;
- precommit how missing projections, adapter abstentions, and oracle disagreements enter analysis.

The end-to-end pilot must not be silently folded into Pilot B. If mediation reconnaissance selects reported third-person transformation instead of semantic attribution, that intervention requires its own fidelity pilot and end-to-end contract rather than being substituted into this branch.
