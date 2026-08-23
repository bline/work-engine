# Proposal: Code Change Characterization Profile

## Identity and state

- Proposal ID: `work-engine.code-change-characterization-profile`
- Family ID: `work-engine.empirical-agent-research`
- State: formed; placement probable; not reviewed, evaluated, accepted, prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. The original exploratory source remains
[`ideas/code-change-characterization-framework.md`](../../../ideas/code-change-characterization-framework.md).
[`implementation-plan.md`](implementation-plan.md) describes a possible
delivery shape without implementation authority.

## Candidate and consequence

Define a reusable, provenance-bearing profile that describes independently
meaningful properties of an exact repository change subject without collapsing
them into a universal complexity score. Planning, testing, review, risk,
architecture, ownership, release, cost, history, and agent research can consume
the same recomputable observations while deciding independently which
properties matter for their own outcomes.

For Review Bench, the profile supplies the missing control variable needed to
ask which reviewer configurations perform well on which kinds of changes rather
than treating heterogeneous cases as interchangeable.

## Exact subject

A profile binds more than two branch names or a final commit. Its subject
identifies, where applicable:

- repository identity;
- base commit and tree;
- result commit and tree;
- exact task-owned path/scope manifest;
- patch or diff digest;
- run, slice, attempt, and plan/scope revision;
- candidate, repair, accepted, or completion disposition;
- evidence cutoff; and
- subject-construction method and limitations.

Branch and timestamp remain context, not immutable subject identity. A final
accepted profile does not overwrite the original review candidate or a
materially revised remediation candidate.

## Independent dimensions

Candidate dimension families include:

- physical diff shape;
- symbols and structural relationships;
- control flow and failure paths;
- state, persistence, recovery, and concurrency;
- interfaces, schemas, compatibility, and contracts;
- semantic responsibilities, architecture, roles, authority, and invariants;
- data models;
- test and validation surface;
- history and novelty as of an explicit cutoff;
- dependency radius and change topology;
- claimed or inferred behavioral surface; and
- coherence or independent concern count.

The common contract should be extensible. It need not permanently enumerate
every future dimension or force every analyzer and subject to populate the same
fields.

## Evidence state and provenance

Every observation records its epistemic and measurement state. Candidate
evidence classes include:

```text
direct
structurally_derived
classified
inferred
self_reported
authority_decision
later_outcome
```

Candidate measurement states include:

```text
observed
zero
unknown
not_measured
unsupported
not_applicable
failed
coverage_incomplete
```

Zero never substitutes for missing measurement. Each derived observation binds
its source subject, analyzer and version, graph/invariant/classifier revisions
where applicable, timestamp, evidence cutoff, coverage, limitations, and exact
source references. Derived profiles remain replaceable.

## Five meanings remain separate

The profile must not flatten:

```text
declared intent
observed code transformation
claimed semantic impact
demonstrated runtime behavior
later outcome
```

For example, “UI behavior changed” can be proposal intent, a diff classifier's
inference, a test result, a Chrome observation, or later production evidence.
Each retains its owner and evidence class. The characterization profile may
reference and project them but cannot turn one into another.

## Entity and profile lineage

Structural comparison must preserve before/after entity relationships for
renames, moves, splits, merges, signature changes, and ambiguous matches. Match
method, confidence, and alternatives remain visible.

Profiles for candidate attempts, remediation, accepted checkpoints, completion
commits, slices, and campaigns retain explicit relationships such as:

```text
derived_from
revises
remediates
accepted_as
composes
supersedes
```

Aggregate profiles are not formed by blindly adding metrics. Dependency radius,
topology, novelty, coherence, and semantic breadth are generally non-additive.

## Pre-outcome and post-outcome separation

A predictive profile freezes only evidence available at its declared cutoff.
Review findings, remediation, deployment results, later defects, and corrections
are linked labels or outcomes, not retroactive pre-review inputs. Historical
features such as prior defects are valid only when computed as of the frozen
cutoff.

## Review Bench use

Review Bench may join:

```text
task/problem profile
+ exact change profile
+ reviewer environment profile
+ review output and adjudicated truth
+ later outcomes
+ resource use
```

The change profile does not own benchmark truth. Reviewer success remains
multidimensional: confirmed detection, missed known defects, false positives,
unique confirmed contribution, calibration, verification quality, remediation
usefulness, stability, latency, token use, and lifecycle cost.

Early comparisons should prefer the same immutable subject, prompt, evidence
packet, truth adjudication, and protocol with only the reviewer configuration
changed. Sparse results remain confidence-qualified by change class rather than
becoming a universal model leaderboard.

Task/problem and evidence environment remain necessary because an identical
diff can be easier or harder to review depending on the bug report, acceptance
oracle, claims, tests, repository access, Codebase Memory, browser evidence,
and prior judgments supplied. A UI miss without rendered-state evidence does
not by itself prove weak UI-review capability.

## Finding normalization

Finding unions, intersections, entropy, and unique-contribution measures require
stable normalized finding or claim identity. Raw findings, equivalence or
overlap judgments, adjudication, exact reviewed subject, reproduction status,
authority, later evidence, and normalization version remain retained. Writing
granularity must not masquerade as reviewer uniqueness.

## Boundary and placement

The probable semantic owner is a shared derived-evidence capability. It owns an
attributed, recomputable description of an exact change subject. It does not
own:

- Git commits, trees, patches, or checkpoint acceptance;
- proposal intent or implementation authority;
- architectural or invariant truth;
- claim truth or authority;
- tests and runtime observations;
- review findings or benchmark adjudication;
- later outcomes; or
- reviewer-routing policy.

It references those owners and preserves derivation provenance.

## Out of scope

- one universal change-complexity score;
- hard-coded reviewer routing or release policy;
- benchmark truth or case admission;
- environmental assessment implementation;
- redefining Codebase Memory, checkpoints, claims, review artifacts, or metrics;
- treating model classification as deterministic structure;
- causal claims from historical correlation alone; and
- implementation authorization.

## Acceptance consequence

If later accepted and implemented, a fresh consumer can retrieve a truthful,
recomputable, evidence-qualified description of an exact change subject and
join it with task, environment, behavior, review, outcome, and resource evidence
without inventing its own incompatible approximation or accepting a universal
difficulty score.
