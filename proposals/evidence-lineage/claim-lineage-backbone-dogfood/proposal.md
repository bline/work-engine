# Proposal: Claim-Lineage Backbone Dogfood

## Identity and state

- Proposal ID: `work-engine.claim-lineage-backbone-dogfood`
- Family ID: `work-engine.evidence-lineage`
- State: proposal meaning approved with probable experimental placement; not
  prioritized or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle projection is in [`packet.json`](packet.json), and the
authority-authored disposition is in [`decision.json`](decision.json). This
narrative owns the proposal's current meaning. Supporting artifacts describe
its [experimental placement](placement.md) and [relationships](relationships.md).

The user authorized formation of this packet after semantic review closed on
the parent candidate. That instruction did not accept the proposal, change the
parent's `defer_for_dogfooding` decision, or authorize implementation.
Following specialized review and retained remediation, the user separately
approved this dogfood proposal's meaning. That decision did not accept the
parent proposal, settle permanent placement, change roadmap priority, or
authorize dogfood execution.

## Candidate and consequence

Run one controlled, reversible dogfood exercise that demonstrates how the
smallest claim-lineage backbone represents real proposal-research and
specialist-review evidence and can expose a semantic mismatch when one occurs.
A fresh consumer should be able to reconstruct
what proposition persisted, which immutable evidence world each revision
examined, which implementation or change event merely nominated possible
impact, what authorized refresh judgment retained or changed the conclusion,
and which exact revision a downstream artifact relied on.

The exercise tests a semantic hypothesis. Durable records, passing validation,
or successful queries do not establish that the claims are true, that the
shared abstraction is architecturally correct, or that the parent proposal is
accepted.

## Why this is independently decidable

The parent proposal defines a broad candidate event model and a much larger
evidence boundary. This proposal authorizes no part of that model. It isolates
one reversible experiment whose result can be accepted, revised, rejected, or
repeated without deciding permanent evidence-lineage placement or building the
full lifecycle, delivery, recovery, or monitoring system.

The output is evidence for a later proposal or portfolio decision. It is not a
first implementation slice smuggled into formation.

## Valid execution and positive demonstration

A dogfood run is valid when it preserves its pre-bound subjects, authority
state, observed outcomes, unresolved gaps, exclusions, projection limits, and
proof failures without changing the criteria after seeing the result. A valid
run may produce negative, incomplete, ambiguous, or authority-blocked evidence.
Those outcomes must not be repaired by selecting replacement fixtures or
manufacturing a missing judgment.

A positive backbone demonstration is a narrower result claim. It requires one
coherent valid run to establish all four consequences below across both the
research and review fixtures. Failure to obtain a positive result does not make
the run invalid.

## Four backbone proofs

### 1. Stable claim identity survives revision

Each fixture has a stable claim identity with two immutable revisions whose
bounded subject, question, quantification, and authority domain remain constant
while the evidence baseline or supported conclusion changes. One bounded
counterfixture changes exactly one identity-defining field and must create a
new stable identity with an explicit cross-identity relationship rather than
hiding the change as another revision.

### 2. Implementation can nominate `may_affect`

An immutable implementation-completion record, bound to its exact repository
change event, can source an independently identified impact nomination against
an exact claim revision. The nomination records observed change evidence and
why impact is plausible. It cannot declare the claim stale, false,
inapplicable, refreshed, or causally changed.

### 3. Refresh can retain or revise the conclusion

The exercise contains both terminal paths:

- an authorized, attributed refresh judgment creates a new evidence-bound
  revision with `retained_unchanged` and no `changed_because_of` edge; and
- an authorized refresh judgment creates a revised conclusion and records only
  the causal events it actually adjudicated through `changed_because_of`.

Opening an episode, investigating it, and reaching a terminal judgment remain
separate durable facts. Both terminal judgments must bind authority effective
for that exact claim, domain, and outcome. The implementation producer does not
acquire refresh authority merely because it nominated the impact.

### 4. Reliance targets an exact revision

A versioned downstream reliance record for each fixture identifies the exact
consumer revision, decision scope, and claim revision used. The record must
either cite direct evidence of historical reliance or represent a newly
versioned dogfood consumer that prospectively adopts the exact claim revision.
Retrospective correspondence without direct reliance evidence is labeled as an
attributed inference and cannot satisfy this proof. A `may_affect` nomination
produces at most a derived candidate-impact warning. Neither the warning nor a
later claim revision silently advances the reliance, mutates the consumer, or
authorizes reopening.

## Controlled subject selection

Selecting subjects after seeing the produced dogfood outcomes would permit
post-encoding substitution and make the exercise weak evidence. Historical
source outcomes may already be known, so pre-binding alone cannot make their
selection outcome-independent. The executing plan must therefore durably bind
a small fixture-selection artifact to immutable source revisions before it
produces dogfood records. The selection must contain:

- one material proposal-research claim that existed before the chosen source
  event;
- one material specialist-review finding that existed before a remediation or
  other relevant source event;
- the exact enclosing artifact revision and evidence baseline for each;
- the producer and actual authority scope of the original judgment;
- the exact immutable implementation-completion evidence and its bound
  checkpoint, commit, or repository-change event proposed as the source of
  candidate impact; and
- an explanation of why the two examples exercise shared semantics rather than
  merely sharing field names;
- the bounded candidate population considered, the selector and its authority,
  the outcome information available during selection, excluded candidates and
  reasons, and whether selection was purposive or outcome-independent; and
- one identity-fork counterfixture that changes exactly one identity-defining
  field.

The examples must be real repository judgments, not claims authored solely to
make the expected outcome pass. A fixture may use historical immutable Git and
review artifacts, so the experiment need not modify production code merely to
manufacture a change event. A purposively selected historical pair can support
a two-case representability demonstration and expose counterexamples; it cannot
be reported as outcome-independent falsification.

Selection is invalid if the same dogfood output is treated as new evidence for
its own architectural correctness. A role must not treat its own prior judgment
as independent evidence merely because that judgment became durable.

## Fixture authority and proof coverage

Before producing a terminal refresh or reliance record, the fixture contract
must bind the authority actually available for each semantic act. Execution
authority, attribution, and possession of an artifact are insufficient. For
each research and review fixture it records:

| Semantic act | Required binding |
| --- | --- |
| Original judgment | source owner, producer, authority scope, and evidence of the grant |
| Dogfood claim-boundary interpretation | role allowed to interpret the source judgment into the experimental claim boundary, with scope and limitations |
| Terminal refresh judgment | domain owner or explicitly experimental refresh authority effective for the exact claim and outcome |
| Reliance | consumer and decision owner authorized to adopt the exact revision for the declared scope |
| Consumer reopening | downstream owner that may retain, supersede, or reopen the reliance |

Experimental refresh authority may establish only a dogfood-local judgment. It
does not change the canonical proposal or review conclusion unless the source
domain's owner separately authorizes that consequence. A missing authority
binding remains an explicit authority-blocked result and prevents a positive
proof for that fixture.

The fixture-selection artifact also owns a closed proof-coverage matrix:

| Fixture | Stable identity | Provenance and authority | `may_affect` | Authorized refresh | Exact-revision reliance | Terminal path |
| --- | --- | --- | --- | --- | --- | --- |
| Proposal-research claim | required | required | required | required | required | unchanged or changed |
| Specialist-review finding | required | required | required | required | required | the other terminal path |

One of the two fixtures additionally supplies the bounded identity-fork
counterfixture. Domain-specific fields and every unexercised or
authority-blocked cell remain visible. Sharing field names does not satisfy a
cell.

## Candidate artifact boundary

The probable first realization is a dogfood-local, Git-backed set of canonical
records plus a replaceable query projection:

```text
immutable source artifacts and events
  -> dogfood-local claims, revisions, nominations, judgments, and reliance
  -> rebuildable local projection
  -> bounded proof queries and evidence report
```

The canonical dogfood records must be independently readable and retain stable,
collision-safe identities, schema versions, endpoint types, evidence baselines,
producer provenance, authority references, and non-destructive predecessors.
The projection may accelerate queries but must rebuild from empty state and
must expose its source identities and completeness boundary.

This experiment may add only the minimal schema, validator, fixtures, and query
adapter needed to establish the four proofs. Their location and vocabulary are
experimental. Existing proposal packets, review artifacts, decisions,
completion receipts, checkpoints, and Git objects remain their own canonical
owners and are referenced rather than copied or reinterpreted.

## Evidence report

The dogfood result should preserve:

- the bound fixture selection and repository evidence cutoff;
- the authority and proof-coverage matrices, including blocked or unexercised
  cells;
- the fixture candidate population, selection knowledge, exclusions, and
  purposive or outcome-independent selection classification;
- every canonical record and typed relationship used in each proof;
- the authority that produced each original and refresh judgment;
- proof queries and their projection provenance;
- observed mismatches between research-claim and review-finding needs;
- failures, unresolved outcomes, and any manual interpretation required;
- whether the projection rebuilt deterministically from canonical records; and
- whether the run was valid and, separately, whether it produced a positive
  four-proof demonstration; and
- a recommendation to retain, revise, split, or reject the tested semantic
  minimum, clearly labeled as advisory.

The report may nominate parent-proposal claims as affected. It cannot revise the
parent packet, settle placement, accept a proposal, or authorize another slice.

## Explicitly deferred

This first backbone dogfood does not attempt to establish:

- automatic claim or dependency discovery;
- broad or transitive propagation into proposals, plans, architecture, or
  active roles;
- continuous freshness monitoring;
- scheduler, control-plane, inbox/outbox, wake-up, or delivery behavior;
- durable role-state integration or context-compaction recovery;
- the full adversarial matrix for concurrency, branching, duplicate and
  out-of-order delivery, authority loss, inactive recipients, or partial
  cross-owner publication;
- a production API, UI, registry, graph store, or shared database;
- permanent canonical placement, permanent projection storage, or schema
  compatibility guarantees; or
- acceptance, roadmap priority, or implementation of the parent proposal.

These are not silently satisfied by a successful backbone exercise. In
particular, the parent review's larger dogfood evidence boundary remains open.

## Current repository evidence

At repository revision `b2e4f697dd25c65c52e3fa2eca46856e346d9545`:

- the claim-centered proposal defines the candidate identity, event, refresh,
  reliance, obligation, and projection semantics, but its decision still
  defers permanent placement and records no implementation authority;
- the specialized review synthesis reports semantic review closure and
  dogfood-planning readiness while explicitly recording dogfood evidence as
  not established;
- proposal packets durably own proposal identity and lifecycle and currently
  validate packet relationships, but they do not own claim history;
- completion-commit receipts already bind immutable Git commits, parent trees,
  checkpoint trees, exact proposed paths, and provenance, making them plausible
  source-event references without granting them claim semantics; and
- Git-backed proposal and review history provides real candidate subjects for a
  controlled fixture without requiring the experiment to fabricate source
  judgments.

These are repository observations and formation inferences, not proof that the
candidate representation or placement is correct.

## Validation consequence

A later implementation should provide executable checks that determine each
proof cell from the canonical dogfood artifacts and should rebuild the query
projection from empty state. The rebuild binds an exact canonical-input
manifest, schema/build version, cutoff, exclusions or failures, and output
digest. Schema validation alone is insufficient: the proof must show the
identity, authority, outcome, and reliance distinctions in the resulting
evidence graph.

A negative, incomplete, ambiguous, or authority-blocked result is valid
dogfood evidence when preserved truthfully. The implementation must not weaken
a proof condition, substitute a fixture, or invent authority merely to obtain a
positive demonstration.

## Authority

The authority decision approves this proposal's meaning only. It does not
authorize artifact creation, schema implementation, fixture selection, semantic
refresh judgment, changes to existing packets or reviews, roadmap mutation, or
a slice-supervisor campaign. Any later execution requires separate explicit
authority bound to this decided proposal and must preserve the parent
proposal's still-open placement and evidence boundaries. Execution authority
authorizes only the work it names; it does not by itself supply
original-judgment, claim-interpretation, refresh, reliance, or reopening
authority missing from a fixture contract.

## Acceptance consequence

If separately authorized and executed, the repository gains a bounded,
inspectable piece of evidence about whether stable revision identity,
non-authoritative impact nomination, explicit unchanged-or-changed refresh, and
exact-revision reliance form a coherent shared backbone across proposal
research and review. The result makes the next semantic and placement decision
better informed without pretending to have completed the broader
evidence-lineage system.
