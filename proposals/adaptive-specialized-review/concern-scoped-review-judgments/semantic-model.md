# Semantic Model: Concern-Scoped Review Judgments

This document elaborates the record and lineage model owned in compressed form
by [`proposal.md`](proposal.md). It does not independently own proposal state,
acceptance, implementation authority, or final schema decisions.

## Record topology

```text
ReviewEpisode
  points_to -> current ReviewResult
  preserves -> session/generation/continuation/remediation state

ReviewResult
  reviews -> exact candidate
  realizes -> perspective + reviewer configuration
  contains -> ReviewJudgment
  produced -> Finding
  records -> declared scope + completion + limitations

ReviewJudgment
  reviewed_for -> concern revision
  about -> exact subject revision
  relied_on -> exact evidence revision or evidence set
  produced -> zero or more Findings
  succeeds -> zero or more predecessor judgments

Finding
  finding_about -> exact subject revision
  arose_from -> ReviewJudgment
  remediated_by -> exact candidate delta
  verified_resolution_of -> predecessor finding revision
```

The topology is many-to-many. A result may contain many judgments; one judgment
may cover an explicitly declared set of concern-subject pairs; one judgment may
produce multiple findings; and one finding may require evidence spanning
several subjects. Convenience serialization must not erase these identities.

## ReviewResult

A result minimally needs:

- stable result identity and schema version;
- exact episode, reviewer generation, and runtime execution reference;
- exact perspective and reviewer-configuration revisions;
- exact candidate and evidence cutoff;
- declared concern and subject scope;
- applicability and `complete_for_declared_scope`, incomplete, failed,
  unavailable, or uncertain outcome;
- referenced judgments and findings;
- unresolved questions, exclusions, and limitations;
- publication authority and immutable predecessor or correction lineage; and
- observed-versus-requested provider identity and authorized omission state.

Publication closes only this result revision. It does not retire the episode,
prevent later exact-subject continuation, synthesize a panel, or accept the
candidate.

## ReviewJudgment

A judgment minimally needs:

- stable judgment identity and schema version;
- exact result, episode, generation, perspective, and candidate references;
- exact concern identity and definition revision;
- exact subject identity and revision, with subject-kind-specific addressing;
- asserted inspection and reliance references;
- a bounded disposition such as `no_material_issue_observed`,
  `finding_produced`, `incomplete`, `not_applicable`, `uncertain`, `retained`,
  `superseded`, or `invalidated`;
- findings produced, if any;
- limitations, exclusions, confidence or uncertainty where the domain contract
  gives those fields meaning;
- producer, authority, and evidence cutoff; and
- predecessor, successor, correction, composition, or carry-forward lineage.

The final vocabulary is a contract decision. A disposition label never widens
the exact concern, subject, evidence, or authority to which it is bound.

## Compact declarations and atomic identity

A reviewer may declare a judgment group to avoid repetitive output:

```yaml
judgment_group:
  mapping: cartesian
  concerns:
    - concern:ui-truth/epistemic-class-legibility@sha256:...
    - concern:ui-truth/uncertainty-preservation@sha256:...
  subjects:
    - ui:timeline@checkpoint:C2
    - ui:anomaly@checkpoint:C2
  evidence_set: evidence-set:truth-17@sha256:...
  disposition: no_material_issue_observed
  limitations:
    - mobile-layout-not-inspected
```

`mapping: cartesian` explicitly asserts every concern-to-subject combination.
An `explicit` mapping lists each pair or bounded group independently. Parallel
arrays without mapping semantics are invalid. Host validation may derive stable
atomic judgment identities only from declared mappings and exact source
revisions. It cannot infer missing combinations or reviewer intent.

The canonical realization may retain the group plus deterministic atomic
projections or publish atomic records directly. Acceptance should choose the
smallest representation that preserves independent succession, correction,
carry-forward, query, and integrity consequences without semantic duplication.

## Observed activity and asserted review meaning

Observed activity may include:

- `presented_to_runtime` for exact context or evidence projections;
- `retrieved` for files, symbols, graph neighborhoods, claims, or artifacts;
- `executed` for commands, tests, probes, or interactions;
- `captured` for browser, pixel, accessibility, geometry, or provider events;
  and
- explicit unavailable, redacted, omitted, failed, stale, or partial states.

Asserted review meaning may include:

- `asserted_inspected`;
- `relied_on`;
- `reviewed_for`;
- `finding_about`; and
- `verified_resolution_of`.

The word `inspected` is never used without provenance identifying whether it is
a mechanical activity observation or an attributed reviewer assertion. Context
delivery and retrieval may support but cannot manufacture inspection, reliance,
or coverage.

## Concern identity

A concern definition identifies the protected distinction, applicability
boundary, expected evidence consequence, possible dispositions, limitations,
and owning review-contract revision. A perspective composes exact concern
revisions. A profile binds provider and execution realization to a perspective
without acquiring concern-definition authority.

Names and display versions are not sufficient identity. Runtime and durable
records bind an immutable concern revision or digest. A concern split, merge,
semantic correction, or supersession retains explicit lineage rather than
silently reusing a label.

## Delta nomination and carry-forward

The lifecycle is:

```text
exact candidate change
  -> structural and evidence delta
  -> possibly affected judgment nominations
  -> attributed relevance assessment
     -> retained by successor judgment
     -> superseded by new judgment
     -> invalidated with reason
     -> incomplete or unresolved
```

Mechanical evidence uses bounded relations such as
`unchanged_under_comparator`, naming the comparator, version, inputs,
coverage, exclusions, and result. It does not use a semantic relation such as
`coverage_remains_valid`.

A carry-forward successor binds:

- predecessor judgment;
- successor candidate and exact delta;
- reused and newly observed evidence;
- relevance rationale;
- retained reviewer or authorized policy identity;
- scope and limitations; and
- new evidence cutoff.

An authorized deterministic policy is permissible only for concerns whose
contract makes the decisive consequence mechanically closed. Policy identity,
version, admission authority, and proof remain explicit. Cost or convenience
cannot silently convert a semantic concern into a mechanical one.

## Current coverage projection

Current coverage is a named projection over:

- exact candidate and projection cutoff;
- selected perspective and concern obligations;
- immutable judgment lineage;
- unresolved possibly affected nominations;
- supersession, correction, and conflict state;
- evidence freshness, completeness, exclusions, and failures; and
- the authority that selected the review obligation.

Useful projected states include current, possibly affected, awaiting relevance
judgment, carried forward, superseded, invalidated, incomplete, unresolved, and
not selected. `Not selected` and absent provenance are not failed review.

The projection cannot accept a candidate, choose a reviewer, create an
obligation, or infer complete coverage from graph reachability.

## Recovery and concurrency

Result and judgment publication is idempotent and immutable. Corrections and
successors add records. Episode state may point to the current published result
while retaining compare-and-swap continuation and writer fencing. A result
publication that succeeds while the episode pointer update is uncertain must
be reconcilable without duplicate semantic effect or loss of the published
result.

Concurrent reviewer generations, replacements, and reconstructed continuation
remain distinct. A successor generation cannot rewrite judgments attributed to
its predecessor. Retirement of an episode prevents operational continuation
but does not delete or invalidate published results.
