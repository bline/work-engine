# Vantage composition quality cost and lifetime

## Status and provenance

Incremental understanding captured from operator discussion on 2026-10-02
and 2026-10-03. This is an exploratory idea for continued development, not an
accepted architecture or implementation plan. Operator direction and assistant
elaboration are distinguished below; the proposed dimensions and measurements
remain open to revision.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-10-03
```

## Operator direction

Vantage should be understood through the properties that produce high quality,
efficient work. A decomposition based on inherited role names such as Builder
or Reviewer is insufficient. Tokens consumed relative to useful output is one
possible dimension, alongside others needed to understand what makes a vantage
effective.

Start by describing what exists and its costs. From that description, discover
the deterministic aspects and the edge where inference is required to prescribe
a suitable vantage. The boundary is something to establish through understanding
and evidence; it is not a predetermined division of responsibilities.

Optimality concerns the shape of the evidence, the shape of the problem, and
the organization built around them. Creating a vantage creates one part of that
organization. Evaluating an isolated inference call cannot establish that the
organizational arrangement is good.

Expected lifetime and the value of accumulated context belong in this account.
Maximum context length is a limiting factor; context lifecycle mitigates that
limit. The operator identifies context lifecycle as active implementation and
wants its effects to become exposed and measurable once that system is
complete. This note records that intention, without asserting that the relevant
measurements or completed functionality already exist.

The operator further clarified that facts and understandings are already
externalized as the model works through claims and context lifecycle. Vantage
economics must account for this existing durable knowledge, including what can
be recovered without repeating discovery. The question was about that system
behavior, not an instruction to save the discussion; this note was initially
created through an assistant misreading of that question.

## Working conceptual interpretation

The following is assistant synthesis of that direction, offered for refinement.

A vantage is a situated reasoning environment within an organization. Its
composition connects a bounded subject and obligation with evidence, accumulated
understanding, capabilities, legitimate authority, and relationships to other
vantages. Familiar role names may describe recurring useful compositions, but
do not supply the decomposition itself.

Problem shape includes dependencies among decisions, separable questions, and
the points where consequences require integration. Evidence shape includes
shared and local evidence, changing facts, consequential relationships and gaps,
acquisition costs, and independence constraints. Organizational shape determines
where understanding accumulates, who owns judgments, how information moves,
and how long those arrangements remain useful.

The value of a vantage is therefore relational and temporal. Creating it incurs
the cost of acquiring evidence and developing understanding. That investment
may support one judgment or many related judgments. Separating work can reduce
context burden while introducing handoff and integration costs. Retaining work
together can preserve useful understanding while increasing capacity pressure
and interference among concerns. Neither arrangement is universally preferable.

## Candidate dimensions

These are questions for description and evaluation, not a fixed schema or
mandatory scoring procedure:

- Information sufficiency and relevance: what is needed for sound judgment,
  what is missing, and what consumes attention without contributing?
- Problem coherence: which questions benefit from shared understanding, and
  which can be separated while preserving essential relationships?
- Reasoning fitness: how well do the representation, capabilities, and model
  support the inference required?
- Error behavior: what errors are likely, detectable, or correlated across
  participating vantages?
- Authority and action sufficiency: can the vantage obtain needed evidence and
  produce its consequence within legitimate boundaries?
- Context value and lifetime: what understanding is worth accumulating, for
  how long, and with what dependence on continuity?
- Organizational cost: what acquisition, coordination, reconstruction,
  verification, and integration costs does the arrangement create?
- Useful yield: what valid progress results for the tokens, time, money, and
  human attention consumed?

Useful output still needs a definition relative to the work. Generated text
volume alone does not establish progress. A fact established, uncertainty
resolved, correct change, or decision another actor can safely consume could
be relevant output. These examples are assistant proposals, not selected units.

Quality requirements and resource costs may need to remain separately visible;
a single ratio could conceal a weakened consequence or costs shifted to another
vantage. No universal scalar objective has been selected.

## Deterministic consequences and the inference boundary

Known relationships can support mechanical conclusions, such as whether an
arrangement fits a declared capacity limit or violates a fully specified
authority or independence constraint. Observed resource use can be recorded.
Cost estimates and predictions of future quality carry uncertainty.

Inference becomes relevant where the available description leaves meaningful
alternatives unresolved: whether shared understanding will pay for its context
cost, whether a smaller projection preserves essential meaning, or whether an
additional perspective improves the organization's result enough to justify its
cost. These are illustrative boundaries, not settled routing rules.

Prescriptions could concern retention, evidence acquisition, information
projection, separation, or combination of work. Their observed consequences
could improve future prescriptions. Repetition alone would not establish that
a relationship is deterministic or transferable to a different problem shape.

## Context lifetime and observable effects

Context capacity and context value are related but distinct in this synthesis.
Occupancy shows pressure against a maximum length. Value concerns what the
accumulated understanding enables and what losing it would cost. Occupancy
alone does not measure that value.

A logical vantage may persist across context replacements. Its organizational
economics therefore include the effects of preserving, projecting, recovering,
or reconstructing understanding across those replacements. Lifecycle behavior
is an input to evaluating that economics; it does not by itself settle which
organization should exist.

Existing durability changes the baseline for this analysis. Claims materialize
attributed, evidence-backed propositions tied to their sources. Lifecycle
continuation state preserves the meaning needed to resume work, including
evidence interpretation, decisions, commitments, unresolved questions, and
references to canonical owners. The two preserve different kinds of meaning;
a continuation checkpoint does not replace the owner of a claim or domain fact.
Neither capability alone demonstrates automatic capture of every discovery.

A useful refinement is to distinguish knowledge already durable and reusable,
meaning preserved for this vantage's continuation, and useful understanding
still dependent on its live context. The marginal cost of replacement depends
on loading and re-establishing the needed relationships among those parts,
plus any discovery that must actually be repeated. Treating replacement as
loss of all accumulated knowledge would overstate that cost. How complete and
effective these preservation paths are remains a measurement question.

Candidate observations proposed by the assistant include:

- tokens, time, and retrieval required to recover useful work;
- obligations and decisions preserved, omitted, or subsequently corrected;
- repeated investigation after replacement;
- capacity recovered and material carried forward;
- quality and resource cost across multiple context lifetimes.

Attributing an effect to lifecycle behavior requires an appropriate comparison.
Cheap recovery alone would not demonstrate continuity quality; a large retained
context alone would not demonstrate valuable understanding. Measurement design,
available instrumentation, and causal attribution remain unresolved.

## Relationship to existing work

- [Organizational Compilation](../../architecture/organizational-compilation.md)
  describes formation of logical vantages and distinguishes context, vantage,
  and role boundaries. This idea explores how quality, cost, and lifetime could
  inform the organization formed around a problem and its evidence.
- [Role and Contract Structure](../../architecture/role-and-contract-structure.md)
  describes logical obligations and constraints independently of runtime.
  Evaluation dimensions here are not replacements for those contracts.
- [Context Lifecycle](../../architecture/context-lifecycle.md) is the
  architectural reference for continuation and replacement. Effects of the
  active implementation are a future evidence source for this idea.
- [Evidence Basis Topology for Bounded Inference](evidence-basis-topology-for-bounded-inference.md)
  explores consequential evidence relationships. Its
  [inference fitness pilot](evidence-topology-inference-fitness-pilot.md) asks a
  narrower question about representation; this idea includes organization and
  lifetime as well as the evidence presented to an inference.
- [Resolution Layered Semantic Refinement](resolution-layered-semantic-refinement.md)
  considers multiple vantages at different semantic resolutions. This note
  could inform their composition without deciding that proposal's open questions.

## Open questions

How should useful output and quality be defined for different kinds of work?
Which properties of problem and evidence shape predict a useful composition?
How can context value be estimated without equating it with token occupancy?
What lifetime horizon makes organizational comparisons meaningful? Which
lifecycle effects can be observed directly, and which require controlled
comparisons? Where do deterministic consequences end for a particular
prescription, and what evidence makes the remaining inference well founded?
