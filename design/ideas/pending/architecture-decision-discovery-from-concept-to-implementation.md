# Architecture Decision Discovery: From Concept to Implementation

## Status and intent

Exploratory architectural idea, synthesized from an operator-supplied chat turn
and its follow-up on 2026-10-01. **“From Concept to Implementation” is a
candidate product framing, not an adopted tagline or a claim that Work Engine
can autonomously settle every product choice.**

This document does not accept an architecture, create a permanent decision
ledger, designate a decision owner, revise proposal authority, require a fixed
human approval sequence, or authorize implementation or provider spending. It
leaves representation, lifecycle, completeness, and service placement open.

```yaml
idea_provenance:
  origin: direct_capture_from_operator_chat_turn
  captured_on: 2026-10-01
  follow_up: >-
    The operator identified “From Concept to Implementation” as both a
    marketing framing and a real missing product path, conditional on
    human-in-the-loop possibilities at consequential decisions.
  related_sources:
    - ../../architecture/material-decision-selection.md
    - ../../architecture/semantic-planning-hierarchy.md
    - ../../architecture/implementation-contract-compilation.md
    - ../../docs/role-decision-trace-reconciliation.md
    - authority-backed-architecture-directions-as-workflow-inputs.md
    - incremental-architecture-intake-and-seam-reconciliation.md
    - resolution-layered-semantic-refinement.md
    - change-specific-review-concerns-and-cross-sections.md
```

## Candidate thesis

An application concept can become a governed implementation when Work Engine
first identifies the architectural questions whose answers materially shape
the system, bounds and investigates those questions, obtains decisions from
the proper human or delegated authority, and carries their accepted meaning
through proposals, architecture, planning, implementation, verification, and
review. The origins, alternatives, assumptions, dependencies, and reopening
conditions of consequential decisions remain recoverable.

The immediate candidate novelty is **pre-proposal architectural decision
discovery**: recognizing and tracking an unresolved question before a
particular idea or proposal answers it. The downstream workflow should compose
with existing owners rather than acquire their authority by being presented as
one end-to-end path.

## 1. Why the product path matters

Work Engine has substantial prospective structure for proposal formation,
decision-specific readiness, material route selection, implementation-contract
compilation, execution, and review. A person can still begin with something
much less formed:

> Build a local personal task manager.

That concept leaves consequential questions open. What owns durable task
state? What is task identity? Which mutations must be atomic? Can work execute
outside an active request? What happens after interruption or schema change?
Which boundaries are product requirements, which are preferences, and which
may be chosen during implementation?

A direct jump from this concept to a service diagram or implementation plan
can silently choose answers. It can hide product tradeoffs inside generated
mechanisms, make preferences look binding, couple unrelated choices, and leave
reviewers unable to identify the architectural property the code was meant
to realize. “From Concept to Implementation” is a meaningful product promise
only if these choices can be surfaced, governed, revisited, and traced to the
resulting implementation.

The promise is not uninterrupted automation. A truthful path can ask for a
human decision, proceed under an explicit delegation, defer a branch, or stop
when required evidence or authority is missing.

## 2. Decision questions precede answers

The proposed initial output is a **provisional architectural decision
surface**: bounded questions whose answers may materially constrain the
system. A question exists independently of any particular answer.

```yaml
# Illustrative question nomination, not a canonical schema.
question_id: durable-task-state-authority
question: What owns authoritative durable task state?
origin:
  - task state must survive process restart
current_context:
  - local-first application
  - single-user assumption
preferences:
  - SQLite preferred where sufficient
related_questions:
  - task identity
  - transaction semantics
  - recovery behavior
state: nominated
answer: unresolved
```

This is a *decision question* or *decision need*, not yet an authority-bound
decision selecting a route. Current [Material Decision Selection](../../architecture/material-decision-selection.md)
uses “decision” for unresolved choices within an already accepted proposal,
with admissible routes, material consequences, and reserved or delegated
authority. The earlier question may be material before anyone knows whether
zero, one, or several routes are admissible. The relationship between these
two identities needs explicit design; identical vocabulary must not silently
merge their lifecycle points.

The first pass should prefer a question such as “What owns durable state?”
over an unexamined answer such as “Use a SQLite repository abstraction.”
It may also discover an invariant or a missing fact rather than a choice.
“Committed task state must survive restart” can be a requirement; whether
SQLite, a journal, or documents own it is a candidate decision question.

## 3. Consequence-based significance

The surface should contain questions whose alternative answers would change
a durable or externally meaningful consequence: authority, ownership,
identity, state, lifecycle, public interaction, persistence, concurrency,
failure and recovery, security, operational footprint, migration, or another
property on which a consumer or future decision depends.

The list is illustrative, not a checklist for every application. A question
does not become architectural because it mentions a database or service. A
small implementation choice may become architectural when its alternatives
change a real contract; an elaborate internal mechanism may remain delegated
implementation discretion if all valid routes preserve the same consequences.

The [route-invariance test](../../architecture/material-decision-selection.md)
is a useful downstream precedent. At concept stage, however, alternatives
may not yet be known well enough to apply its exact inclusion rule. Question
nomination can state the suspected consequence and uncertainty; later inquiry
can classify the question as material, already resolved, ordinary discretion,
unnecessary under current assumptions, or blocked by evidence.

## 4. Human participation and decision authority

Human-in-the-loop possibilities are part of the proposed path, not an
exception at its end. Work Engine may help a person clarify concept meaning,
separate hard constraints from preferences, inspect consequential alternatives,
and understand the effects of a choice. For each material question, the
applicable authority boundary determines whether a human must decide,
whether a bounded choice may be delegated, or whether known inputs already
determine the result.

Candidate interaction points include:

1. **Concept clarification:** a person confirms or corrects the intended
   capability, environment, and consequence of failure.
2. **Question bounding:** a person may admit, merge, split, defer, or reject
   nominated architectural questions where that classification changes
   product scope or other reserved meaning.
3. **Alternative selection:** the named decision owner receives the
   alternatives, evidence, tradeoffs, uncertainty, and recommendation needed
   for a reserved choice. An authorized delegation names its scope and limits.
4. **Conflict or reopening:** new evidence or changed preferences can return
   the affected question to its owner without rewriting the historical choice.

This does not imply human approval of every question, idea, proposal, or code
change. It requires a usable and truthful human decision path where authority
is reserved, plus an explicit delegation path where it is not. An unanswered
human question remains unanswered; elapsed time, model confidence, or local
repository access cannot turn it into consent.

Preferences require particular care. “Prefer single-process execution” may
guide candidate generation and comparison. It becomes a hard constraint only
when the relevant authority makes it one. If evidence shows that a required
outcome needs independent execution, the conflict should be visible to the
owner rather than silently discarding either the requirement or preference.

## 5. Ideas, proposals, and accepted architecture

A question can focus idea formation:

```text
What owns authoritative durable task state?
    -> SQLite is authoritative
    -> append-only journal is authoritative; SQLite is a projection
    -> filesystem documents are authoritative
```

Those are candidate answers, not decisions. An idea may address several
questions; several ideas may jointly answer one question; and a relevant raw
idea may arrive before its architectural question has been named. The
decision surface must remain open to that last route rather than requiring
all ideas to be generated from a prior ledger entry.

Coupled answers may need one proposal—for example, state authority,
transaction boundary, and recovery semantics. Independent questions need not
be bundled because they appeared in one application brief. The pending
[Incremental Architecture Intake](incremental-architecture-intake-and-seam-reconciliation.md)
already proposes seam reconciliation and proposal decomposition for this
problem; this idea asks how an initial application concept nominates the
questions that such reconciliation must consider.

**Proposal acceptance and architecture adoption remain separate.** A formed
or accepted proposal may supply evidence and candidate meaning for resolving
one or several questions. It does not automatically create permanent
architecture. The pending [Architecture Directions](authority-backed-architecture-directions-as-workflow-inputs.md)
idea explicitly requires an authorized architecture decision for a durable
direction. A resolution may be partial, leaving named open points, or may be
valid only under particular assumptions and a stated evidence cutoff.

The composed architecture also needs a coherence check. Several individually
sound decisions can conflict at a seam. A list of resolved questions does not
by itself establish one consistent architecture or authorize a plan.

## 6. Derived work and two-way traceability

A possible derivation is:

```text
application intent and constraints
    -> nominated decision questions
    -> investigated ideas and coupled proposals
    -> authority-backed architectural resolutions
    -> coherent applicable architecture
    -> accepted structural work and implementation contracts
    -> exact implementation subject
    -> verification evidence and bounded review judgments
```

This is a candidate high-level route, not a mandatory sequence. A concept may
already have settled architecture; one question may require no proposal;
proposal research may discover a missing question; implementation may expose
a material choice that must return upstream. Downstream work must not quietly
resolve a question merely because it encountered it.

Useful traceability works both ways:

- From a resolution: which obligations, contracts, code, and verification
  evidence realize it?
- From implementation: which accepted architectural property justifies it?
- From a changed assumption: which resolutions and dependent work may need
  reconsideration?
- From a review concern: which decision or obligation defined the property
  being challenged?

The pending [change-specific review idea](change-specific-review-concerns-and-cross-sections.md)
can consume this lineage to nominate bounded semantic review concerns. It
does not thereby accept the architecture or determine implementation
correctness.

## 7. History, negative choices, and reopening

For a consequential resolution, useful history includes its originating
requirement or assumption, question revision, considered alternatives,
evidence and limitations, authority, disposition, rejected or deferred
routes, dependencies, downstream obligations, and reopening conditions. The
history must distinguish a question nomination, an inference about a route,
an authorized selection, and current applicability.

There is no need to record every absent technology. “No Redis” may be noise.
“No operation runs outside the active application process” may be a
meaningful negative architectural choice when scheduling, recovery, or
availability depends on it.

A change from “single user on one device” to “synchronized across devices”
can nominate reconsideration of identity, durable authority, concurrency,
conflict semantics, authentication, and recovery. A dependency relation
must state whether it was observed, inferred, or accepted; graph proximity
alone does not establish architectural causality. Impact nomination does not
reopen work or grant implementation authority on its own standing.

Existing [Architecture Directions](authority-backed-architecture-directions-as-workflow-inputs.md)
and [Role Decision Trace reconciliation](../../../app-server/docs/role-decision-trace-reconciliation.md)
already cover or expose much of the required accepted-state and judgment
lineage. This idea should not install another all-purpose ledger merely to
make the end-to-end chain easy to query.

## 8. Completeness and recursive discovery

The initial decision surface is provisional. It should be possible to ask:

> Under this concept, current constraints, assumptions, and known
> resolutions, what consequential architectural question appears missing?

The result is a *nomination*, with its origin, reason, uncertainty, and
potential consequence. It may be admitted, merged, rejected as covered,
classified as implementation discretion, declared unnecessary under current
assumptions, deferred, or left unresolved. Neither a fixed category tree nor
one model pass can certify architectural completeness.

Discovery continues during idea formation, proposal reconciliation, planning,
implementation, and review. Later evidence may reveal that a supposedly
independent pair of questions is coupled, that one question contained two
different decisions, or that a previous answer depended on a false premise.
The appropriate semantic owner handles the revision. Local discovery supplies
evidence, not inherited decision authority.

## 9. Risks and pressure tests

1. **Decision explosion:** Does the surface distinguish consequential
   architecture from product behavior and implementation discretion?
2. **Template architecture:** Are questions nominated from this application's
   actual consequences rather than filled in from a universal category tree?
3. **Premature answer:** Does a provisional question already smuggle in a
   framework, deployment topology, or storage mechanism?
4. **Preference capture:** Can an authorized owner distinguish, revise, or
   override a defeasible preference when evidence warrants it?
5. **False completeness:** Can later work nominate a missed question without
   treating the initial decomposition as exhaustive?
6. **Fragmented proposals:** Can coupled decisions be evaluated together
   while unrelated ones remain separate?
7. **Authority laundering:** Are nomination, recommendation, proposal
   acceptance, architecture adoption, delegation, and implementation
   authorization distinct recorded consequences?
8. **Retrospective rationale:** Were decisions and their evidence recorded at
   the revisions where they occurred, rather than reconstructed after code
   was chosen?
9. **Dependency overstatement:** Are observed relationships separated from
   inferred and accepted causal dependencies?
10. **Process overhead:** Does decomposition improve decisions enough to
    justify its cost for a genuinely small application?

## 10. Bounded evaluation

Use a small application concept with real but limited architectural choices,
an agreed set of hard constraints and defeasible preferences, and named human
decision authority. Compare a direct architecture-and-plan route with a
route that first nominates and bounds decision questions. Preserve exact
inputs, model/provider configuration, human interventions, artifacts, and
costs for each route. Independently assess missed consequential questions,
unnecessary mechanisms, internal contradictions, authority violations,
traceability, revision burden, plan quality, and implementation outcomes.

Then change one foundational assumption—such as adding multi-device
synchronization—and compare whether each route identifies the decisions and
downstream work requiring reconsideration. A positive result for one small
fixture would support further investigation, not a universal decision ledger
or mandatory decomposition ceremony.

## 11. Open questions

1. Who owns pre-proposal decision-question nomination and bounding?
2. What is the minimum identity and revision model for a question before an
   authorized route selection exists?
3. How does that question relate to Material Decision Selection's later
   proposal-bound decision-set record?
4. Which human decisions are reserved, which may be delegated, and what
   evidence does each need at this stage?
5. How can raw ideas arriving first nominate or revise questions without
   losing their original meaning?
6. Which coupled questions should be evaluated in one proposal, and who
   judges that coupling?
7. What exact act adopts a proposal-supported answer as architecture, and
   which open points survive that act?
8. What existing owner can preserve question-to-resolution-to-work lineage
   without duplicating claim, judgment, direction, or plan state?
9. What evidence nominates a missed question or reopens a settled one, and
   who judges the consequence?
10. What bounded human interaction makes “From Concept to Implementation”
    useful without turning routine implementation choices into approval work?
