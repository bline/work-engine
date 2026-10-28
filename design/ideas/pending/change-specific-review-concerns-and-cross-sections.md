# Change-Specific Review Concerns and Cross-Sections

## Status and authority

Exploratory architectural idea, synthesized from an operator-supplied chat turn
on 2026-10-01 and compared with the current App Server architecture and pending
ideas. This document does not accept a design, assign a permanent owner, create
a review gate, authorize implementation or provider spending, or decide how
many reviewers an episode requires.

```yaml
idea_provenance:
  origin: direct_capture_from_operator_chat_turn
  captured_on: 2026-10-01
  related_sources:
    - ../../architecture/review.md
    - ../../architecture/implementation-contract-compilation.md
    - ../../docs/review-subject-service.md
    - structural-plan-ir-for-capability-aware-multi-model-execution.md
    - evidence-basis-topology-for-bounded-inference.md
    - ../../../proposals/adaptive-specialized-review/concern-scoped-review-judgments/proposal.md
    - ../../../proposals/adaptive-specialized-review/adaptive-review-panel-coordination/proposal.md
```

The candidate consequence is:

> For one exact implementation subject, select a bounded set of falsifiable
> semantic properties whose failure could change the owning decision. Give
> each judgment enough revision-bound code, contracts, state relationships,
> counterevidence, and verification evidence to challenge that property, while
> keeping omissions and unknowns visible.

This is a candidate connection among existing owners, not a proposed new
architectural Review dimension or a universal quality rubric. The term
**review concern** follows the already-formed concern-scoped review proposal;
its exact identity and derivation contract remain open.

## 1. Problem

A general request to review correctness, security, reliability, architecture,
performance, and maintainability asks a reviewer to infer what matters for this
particular change before it can test anything. A flat file list or diff gives
the reviewer a subject, but does not identify the semantic properties the
change was meant to establish or could have broken.

Existing work supplies important but separate pieces:

- [Implementation-Contract Compilation](../../architecture/implementation-contract-compilation.md)
  describes accepted obligations, a bounded implementation basis, expected
  evidence, and the post-execution conformance question owned by Review.
- [Structural Plan IR](structural-plan-ir-for-capability-aware-multi-model-execution.md)
  proposes stable semantic addresses and a reviewer projection of accepted
  plan meaning. Its representation is pending, not an implemented prerequisite.
- The [review-subject service](../../../app-server/docs/review-subject-service.md) provides
  an immutable checkpoint and deterministic *physical* change profile. It
  expressly does not infer semantic risk or select reviewers.
- [Concern-Scoped Review Judgments](../../../proposals/adaptive-specialized-review/concern-scoped-review-judgments/proposal.md)
  proposes exact concern, subject, evidence, and judgment identities; it does
  not by itself establish which concerns this change warrants.
- [Adaptive Review-Panel Coordination](../../../proposals/adaptive-specialized-review/adaptive-review-panel-coordination/proposal.md)
  proposes consequence-sensitive selection and truthful omission accounting.
  Its placement and implementation remain unsettled.
- [Evidence-Basis Topology](evidence-basis-topology-for-bounded-inference.md)
  explores decision-relative evidence relationships and projections. Its
  architectural classification and inference benefit remain unproven.

The unresolved question here is how to derive *this change's* bounded
questions and assemble evidence adequate to challenge them without giving a
diff, graph, profile, or coordinator authority to decide semantic relevance or
review completeness on its own.

## 2. Candidate distinctions

**Review concern:** a bounded, falsifiable semantic property selected because
the change is supposed to establish it, modifies it, depends on it, or may
violate it. It has a question and potential counterexample, not merely a topic
label such as “reliability.” A concern may cross files, services, persistent
state, execution paths, tests, and other evidence kinds.

**Review cross-section:** a revision-bound selection of the code, state,
contracts, callers, tests, observations, contrary evidence, and explicit gaps
needed to judge one concern under a declared evidence cutoff. It is an
evidence projection for a question, not a second source of truth or a claim of
whole-repository completeness.

**Selection origin:** the attributed reason a concern matters. Candidate
origins include an accepted implementation obligation, a protected existing
property affected by the change, a required verification consequence, or an
observed discrepancy between intended scope and repository evidence. These
origins can overlap. They are not three mandatory classes of review work.

For example, “transition interruption proof” may be required evidence for
the concern “closed-admission transition integrity.” It becomes a separate
concern only if the verification mechanism has an independently consequential
property to judge.

## 3. Candidate formation route

```text
accepted intent and governing contracts
              +
exact candidate and observed physical change
              +
affected ownership, dependency, and execution relationships
              +
verification obligations and known uncertainty
              |
              v
attributed concern nominations
              |
              v
semantic selection, merge/split, and omission judgment
              |
              v
bounded cross-sections for selected concerns
              |
              v
mechanical checks where sufficient; scoped semantic review where needed
```

The physical profile, repository graph, and dependency observers can nominate
possible effects and provide exact observations. They cannot conclude that a
property is semantically affected, that all production routes were discovered,
or that a reviewer should be invoked. A selection owner must preserve why each
concern was selected, why a material nomination was omitted or left uncertain,
and what evidence would reopen that decision. A review contract may independently
require particular concerns or coverage; adaptive selection cannot omit them.

Absence can be evidence. If an accepted contract names all production inference
routes as in scope, but the candidate modifies only some observed routes, that
discrepancy can nominate **production-route closure**. The negative claim that
all routes are covered still needs a declared search universe, freshness, and
coverage evidence; a graph query or changed-path count alone cannot establish
it.

## 4. Illustrative concern, not a schema commitment

```yaml
concern: transition-finalization-gate
question: >-
  Can finalization become eligible while transfer for this transition
  remains incomplete?
origin:
  - accepted-contract obligation at an exact revision
subject: exact transition implementation candidate
cross_section:
  required_relationships:
    - transition-state producer to persisted transfer state
    - transfer state to finalizer eligibility predicate
    - predicate to every admitted finalizer caller
    - relevant interruption and restart observations
  evidence_cutoff: exact repository and evidence revisions
  observed_coverage: attributed observations with limitations
  explicit_gaps: []
falsifier: >-
  Any admitted execution route can observe finalizer eligible while
  transfer_complete is false.
disposition: selected_for_semantic_judgment
```

The example names *what would falsify the property*. It does not assert that a
particular implementation is correct, that the listed cross-section is already
complete, or that a test alone proves the absence of every violating route.

## 5. Ownership boundaries to preserve

- The accepted plan or governing domain contract owns required properties and
  any mandatory concern set. Review selection may derive a projection of
  those properties, but cannot revise their meaning or weaken an obligation.
- The exact review subject and physical change observations belong to their
  existing checkpoint/profile producers. Semantic characterization needs its
  own attribution and revision.
- Evidence sources and deterministic comparators own their observations.
  Declared-versus-observed coverage, freshness, unsupported ranges, and source
  limitations remain visible. Semantic sufficiency and consequence stay with
  the relevant judgment owner.
- A selected reviewer owns only a bounded semantic judgment. Findings and
  positive judgments bind exact concern and subject revisions, relied-on
  evidence, limitations, and producer. Context presented to a reviewer does
  not mechanically prove attention, inspection, or reliance.
- Coordination may choose zero, one, or several perspectives according to the
  review contract and credible decision value. One reviewer per concern is an
  available route, not an invariant; neither a fixed panel nor a fixed count
  of active concerns follows from this idea.
- A completed review judgment does not accept the implementation, publish a
  successor, or grant itself blocking authority. Those consequences remain
  with the existing decision and publication owners.

## 6. Failure modes and pressure tests

1. **Plan echo:** concerns reproduce only written obligations and miss a
   property the change could break. Does affected-system evidence add a
   material concern that intent alone did not nominate?
2. **Diff tunnel vision:** an untouched caller or route is excluded because
   its file is absent from the patch. Can the cross-section expose the
   producer-to-consumer path and its coverage limits?
3. **False closure:** a packet labels an incomplete graph traversal or a few
   tests “proof.” Does it distinguish observed coverage, missing evidence,
   and semantic sufficiency?
4. **Concern inflation:** every acceptance bullet or test becomes a separate
   reviewer task. Can related obligations collapse without losing a distinct
   falsifier or decision consequence?
5. **Frame capture:** narrow questions prevent reporting a new material issue
   outside the selected set. Can the reviewer nominate an unexpected concern
   without silently expanding its own acceptance authority?
6. **Premature automation:** deterministic evidence nominates “all routes
   covered” although route enumeration is not a closed, current universe.
   What exact bound would make the mechanical conclusion valid?
7. **Duplicated ownership:** a new object or service redefines concern
   identity, evidence topology, panel selection, or finding lineage already
   proposed elsewhere. Which existing owner can express each consequence,
   and what genuinely remains unowned after that mapping?

## 7. Bounded evaluation before a design commitment

Use one immutable completed change with an accepted contract and enough known
execution relationships to make omissions observable. Record the current
review boundary, candidate concerns, their derivation reasons, cross-section
inputs and gaps, reviewer route, findings, missed material issues, elapsed
time, and inference cost. Compare with the current review presentation under
the same subject and an agreed evaluation owner. A useful second case should
contain an obligation absent from the diff's obvious changed paths.

This comparison must separate gains from better question selection, better
evidence selection, clearer prose, extra evidence, provider variation, and
reviewer framing. The pending [Evidence-Topology Inference-Fitness Pilot](evidence-topology-inference-fitness-pilot.md)
tests a different, narrower representation hypothesis; its result should
inform but cannot substitute for this concern-selection test.

The idea is weakened if bounded concerns systematically miss material
unanticipated findings, if cross-sections conceal consequential gaps, or if
similar quality requires more authoring and inference work than the current
review route. A positive result would support further design of the
selection/projection seam; it would not itself accept a universal compiler,
canonical schema, panel topology, or mandatory review count.

## 8. Open ownership questions

1. Is concern selection part of the adaptive review coordinator's eventual
   judgment, a review-contract compiler owned elsewhere, or a composition of
   already-owned nominations and a domain review profile?
2. How does a change-specific concern relate to the concern-scoped proposal's
   stable, versioned concern definitions? Which meaning is reusable, and which
   is exact to this implementation subject?
3. Who declares the required relationships and sufficiency bar for a
   cross-section when the accepted contract is incomplete or repository
   evidence reveals a new affected boundary?
4. What compact evidence projection helps reviewers without treating the
   pending evidence-topology representation as an established prerequisite?
5. When must a missing or contradicted concern return to planning or another
   semantic owner, rather than becoming a reviewer-only uncertainty?
