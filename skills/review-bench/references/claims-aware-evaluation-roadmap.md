# Claims-Aware Review Evaluation Roadmap

Status: staged evaluation direction; not a change to
`evidence-calibrated-review/1.2`

## Purpose

The system is being reviewed while several intended evidence capabilities are
still under construction. Current attempts therefore measure a reviewer
working primarily from an immutable snapshot, a case contract, repository
inspection, and deterministic checks. They do not measure the eventual Work
Engine review environment in which proposal packets, versioned claims,
reliance, reopening, and Codebase Memory jointly organize the evidence world.

Preserve current results as the pre-claims baseline. Do not use them alone to
select the eventual production reviewer or harness.

## Continuous candidate capture during development

Each sufficiently identified implementation slice should become a potential
baseline review candidate. Capture should happen at the slice boundary while
the evidence is cheapest and strongest to preserve:

- immutable candidate and repaired or accepted checkpoints;
- effective objective, acceptance criteria, exclusions, and authority scope;
- task patch, manifest, tree, gate, and deterministic-test identities;
- original review prompt, provider configuration, result, and remediation
  lineage when available; and
- known missing evidence and reconstruction confidence.

This is candidate inventory, not automatic corpus growth. A later admission
decision selects cases for representativeness, truth quality, route coverage,
and independence. Passing tests, accepted slice status, or a historical review
does not establish benchmark truth.

## Same-snapshot harness ladder

After the relevant capabilities exist, replay selected immutable cases through
an additive harness ladder:

1. snapshot and frozen contract only;
2. snapshot plus proposal packet;
3. snapshot plus exact claim revisions and reliance records;
4. snapshot plus claims and a frozen Codebase Memory projection;
5. optionally, a fresh contradictory-path pass that does not inherit prior
   semantic judgments.

Each attempt must identify the exact supplied artifacts, their revisions and
digests, graph project and generation, protocol, tool access, and whether prior
judgments were visible. Adding claims or graph evidence changes the harness and
usually the protocol; it is not a reinterpretation of an earlier attempt.

Claims should reduce broad discovery by naming propositions, evidence
baselines, assumptions, sensitivity surfaces, and downstream reliance. They do
not become an answer key. Codebase Memory can establish structural reach,
callers, dependencies, and candidate impact; it cannot establish semantic
truth or causality. Proposal-packet validation establishes mechanical
consistency, not proposal quality, acceptance, priority, or implementation
authority.

## Prospective production evidence

Once the system is sufficiently complete, ordinary slices should produce
prospective review evidence through the live process:

- claims exist before the reviewer judgment they inform;
- change events may nominate exact revisions as `may_affect`;
- reviewers verify or falsify claims against the current evidence world;
- authorized judgments retain, revise, contest, or invalidate claim revisions;
- downstream reliance binds the exact revision actually used; and
- reopening remains owned by the appropriate consumer or authority.

Production evidence is stronger for operational realism than reconstructed
development cases, but it has different selection effects and must remain a
separate evidence class. Naturally occurring accepted and rejected reviews,
provider failures, blocked outcomes, and no-finding cases should all remain
visible.

## Required measurements

Compare complete reviewer configurations and report at least:

- defect recall, precision, blocking calibration, and severity calibration;
- false and correctly verified claims;
- acceptance-criterion coverage and unsupported acceptance;
- stale or inapplicable claim detection;
- correct `may_affect` nomination versus unsupported semantic causality;
- authority, provenance, lineage, and exact-revision reliance correctness;
- evidence acquisition cost, latency, and blocked-unverified rate; and
- anchoring or correlated error introduced by inherited claims.

The current scoring schema does not adjudicate false verified claims. A future
claim-aware result and scoring protocol must close that gap before benchmark
scores influence reviewer routing.

## Leakage and independence controls

For retrospective cases, freeze claim-aware packets before revealing sealed
truth or later remediation. Claims derived from knowledge of the expected
defect measure answer reconstruction, not review effectiveness.

Where independence matters, distinguish:

- fresh discovery without prior semantic judgments;
- claim-directed verification with exact provenance;
- retained-context remediation review; and
- same-model contradictory-path or aggregation passes.

These are complementary harnesses, not interchangeable claims of independent
review.

## Decision boundary

The staged evidence can support reviewer and harness policy only after enough
representative cases exist across route classes and evidence eras. Until then:

- current reviews are an unaided baseline;
- claim-aware replays measure the value and risks of the evidence system;
- production runs measure real operational performance; and
- no benchmark result substitutes for acceptance of the reviewed slice.

