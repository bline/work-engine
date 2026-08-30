# Control-plane causal observability UI

## Status and purpose

This document defines a read-only App Server control-plane UI for understanding
live work, historical workflow causality, reviewer economics, anomalous cost,
and evidence-backed workflow improvement. It is a product design and
architecture boundary, not an implementation plan or authority grant.

The UI exists to answer questions such as:

- What is active, waiting, retrying, or approaching a configured limit?
- Which immutable candidate, reviewer episode, finding, or remediation caused
  the current state?
- Where did provider cost and latency accrue, and which costs produced useful
  evidence?
- What changed relative to a relevant baseline?
- What might a different valid workflow have cost, under explicit assumptions?
- Which contract or policy created an avoidable transition?

Its central design rule is:

> Project causal meaning from existing owners; do not make the UI a new owner
> of workflow truth or authority.

## Product outcome

An operator can begin with an operational anomaly, traverse its complete
evidence lineage, distinguish valuable review work from avoidable workflow
amplification, inspect the exact contract responsible, and evaluate a bounded
counterfactual or recommendation without confusing any of those projections
with acceptance authority.

The primary reference case is the S5 review-amplification incident:

```text
candidate 1
  -> generic finding
  -> test remediation
candidate 2
  -> instruction findings
  -> causal-instruction remediation
candidate 3
  -> prior generic episode already reported
  -> replacement generic reviewer
  -> identity-provenance finding
candidate 4
  -> provider quota wait
  -> anomalous cost question
  -> metrics comparison
  -> reported-episode transition defect
  -> reviewer-continuity repair candidate
```

This trace contains useful findings and avoidable amplification. The UI must
preserve that distinction rather than labeling all additional review cost as
waste.

## Truth and authority model

Every displayed statement has one of five evidence classes:

| Class | Meaning | Example |
| --- | --- | --- |
| Observed | Directly published by an authoritative runtime or artifact owner | A provider call returned HTTP 429. |
| Mechanically derived | Recomputable from exact observed inputs | Cost attributed to one reviewer episode. |
| Semantic interpretation | Versioned, attributed judgment over evidence | A reported episode caused a replacement review. |
| Counterfactual estimate | Result of a named alternative policy applied to a frozen historical trace | Panel batching was projected to avoid one replacement call. |
| Recommendation | Advisory proposed change with evidence, uncertainty, and an authority owner | Separate result publication from episode closure. |

These classes must be visually and structurally distinct. A counterfactual is
not an observation. A recommendation is not an authorization. An anomaly is
not a defect until attributed evidence establishes the causal claim.

The UI may offer controls that invoke existing control-plane capabilities, but
each mutation remains owned by that capability and its authority contract.
Viewing a due retry does not authorize it; viewing an accepted checkpoint does
not create a completion commit; viewing a finding does not accept or dismiss
it.

## Existing semantic owners

The UI joins existing owners by immutable or revision-bound references:

| Meaning | Existing owner | UI responsibility |
| --- | --- | --- |
| Active slice phase, pending obligation, waiting state, runtime binding | Slice Supervisor active-slice state | Project current state and history without inferring completion from liveness. |
| Candidate and accepted subject identity | Slice Checkpoint | Display exact commit, tree, patch, manifest, plan, scope, and gate bindings. |
| Reviewer subject, findings, generation, remediation, replacement | Independent Review State | Display the episode lifecycle and preserved finding lineage without creating an independence claim. |
| Review selection and acceptance consequence | Slice Supervisor terminal receipt | Show who selected each perspective and whether execution, applicability, and closure were established. |
| Provider/model usage, cost, latency, attempts, failures | Provider events and normalized terminal metrics | Attribute measurements without merging distinct units or estimating missing values. |
| Scheduled retry and acknowledgement | Role Scheduler | Display durable due state; never treat scheduling as execution authority. |
| Revision-bound production findings and reliance | Claim Evidence | Link exact findings and limitations; do not infer applicability or acceptance. |
| Physical change characteristics | Code Change Profile | Supply deterministic subject-side facts without turning them into approval or universal difficulty scores. |
| Role composition and runtime realization | Runtime Manifest and compiled role environment | Explain which role, skills, grants, continuity, and environment revision governed a runtime turn. |
| Raw reviewer-turn behavior and normalized events | S13 reviewer event normalizer, when implemented | Preserve raw-event provenance, confidentiality, omissions, and normalization version. |

The UI projection layer owns joins, indexes, query performance, presentation,
and derived-view versioning. It does not copy an owner's payload and silently
become authoritative for it.

## Causal graph

The core projection is a directed graph over exact identities:

```text
campaign/run
  -> slice attempt
     -> accepted plan
     -> deterministic gate
     -> immutable candidate
        -> selected perspective
           -> reviewer episode/generation/session
              -> provider attempts
              -> findings
              -> remediation subject
        -> successor candidate
     -> accepted checkpoint
     -> completion offer/commit
```

Additional edges represent `caused`, `supersedes`, `reviews`, `remediates`,
`re-evaluates`, `replaces`, `waits-on`, `scheduled-as`, `supports`, and
`limited-by`. Every edge names its producing owner and evidence reference.

An event projection should minimally retain:

- stable event identity and schema version;
- event kind and evidence class;
- semantic owner and source revision;
- observed and effective timestamps when different;
- run, slice, attempt, candidate, perspective, episode, and provider identities
  when applicable;
- predecessor/successor or cause/consequence references;
- integrity-bound payload reference rather than copied domain truth;
- measurements with original units and explicit `null` for unavailable values;
- limitations, confidentiality class, and projection version.

Chronological proximity is not causality. A causal edge requires an owning
transition, an exact reference, or an attributed semantic interpretation.

## Primary views

### Operational view

The landing view answers what needs attention now.

Show:

- active runs and slices, phase, accepted boundary, and pending obligation;
- builders and reviewers by logical role, provider, model, generation, session,
  and observed liveness;
- waiting capabilities, quota resets, retries, schedules, and acknowledgement;
- configured and consumed limits for cost, time, tokens, slices, and repairs;
- immutable current subject and whether gate, selection, review, or acceptance
  remains pending;
- stale or conflicting runtime bindings separately from semantic state.

Provider/session liveness must never be presented as proof that work completed.

### Causal timeline

The timeline is an event graph rendered along a primary temporal axis. It shows
candidate creation, gates, selection, reviewer entry, findings, remediation,
re-evaluation, rebinding, replacement, quota waits, acceptance, and commit.

Collapsible lanes separate:

- candidate and gate lifecycle;
- each selected review perspective;
- provider attempts and sessions;
- findings and remediation;
- active-slice and scheduler state;
- user authority decisions.

A replacement edge must show why continuation failed and what state was
preserved, reconstructed, or lost.

### Efficiency view

Cost is attributed through non-overlapping semantic dimensions:

```text
provider attempt
  -> evidence/review role
  -> selected perspective
  -> reviewer episode and generation
  -> candidate transition
  -> finding and remediation round
  -> slice and campaign
```

The view reports cost, wall time, input, cache creation, cache reads, output,
thinking, failures, and retries without combining unlike measurements. Useful
ratios include:

- cost per accepted slice;
- cost per selected perspective;
- cost per valid finding and per verified resolution;
- cost per candidate transition;
- initial-review versus retained-remediation cost;
- replacement and reconstruction overhead;
- deterministic-gate cost versus semantic-review cost;
- avoided replacement cost after a workflow repair.

Low cost is not success, and high cost is not waste. Display useful discoveries,
false findings, unresolved findings, and avoidable amplification beside cost.

### Anomaly explanation

An anomaly compares one frozen subject with a named baseline cohort or policy.
The explanation decomposes the difference into attributed contributors such as:

- change consequence or fan-out;
- selected perspective count;
- findings and repair rounds;
- candidate churn;
- provider failure or quota wait;
- context reconstruction;
- reviewer replacement;
- repeated evidence retrieval;
- changed model/provider/profile;
- missing or noncomparable measurements.

The UI displays the observed delta first, then a versioned semantic explanation
with confidence and limitations. It must allow the operator to reject or revise
the interpretation without changing the underlying events.

### Counterfactual view

A counterfactual applies a named, versioned workflow policy to a frozen event
trace. Each estimate binds:

- the historical subject and evidence cutoff;
- baseline and alternative policy revisions;
- preserved invariants and prohibited transitions;
- assumptions about provider pricing, cache behavior, findings, and latency;
- observed events reused by the model;
- events omitted, combined, or substituted;
- estimated range, confidence, and sensitivity;
- states the alternative cannot determine from historical evidence.

Examples include panel batching, retained reported-episode continuation,
delta-composed review coverage, or a different selected perspective. The UI
must not assume that a skipped review would have produced the same findings.
Projected savings therefore separate mechanically avoidable reconstruction from
uncertain changes to discovery quality.

### Recommendation view

A recommendation connects one anomaly explanation to a proposed contract,
configuration, or policy change. It records:

- affected owner and decision authority;
- evidence and counterfactual references;
- expected consequence and savings range;
- confidence, limitations, and possible quality loss;
- validation or experiment required before adoption;
- adoption state and authorized decision, if one later exists;
- post-adoption observations used to confirm or falsify the prediction.

The recommender may propose. It cannot modify production routing, review
selection, acceptance, limits, or authority.

### Semantic drill-down

Every displayed aggregate is traversable to its exact evidence. An operator can
click an anomalous cost and follow:

```text
cost measurement
  -> provider attempt
  -> runtime session
  -> reviewer generation and episode
  -> immutable candidate
  -> finding/remediation transition
  -> active-slice or scheduler consequence
  -> owning contract and exact revision
```

The drill-down distinguishes source bytes, mechanically derived projections,
reviewer judgment, builder disposition, supervisor selection, and human
authority. Raw traces remain behind their confidentiality boundary and may be
represented by authorized projections or explicit omissions.

## S5 reference explanation

The initial golden explanation should answer the real operator question:

> Are we using more Claude tokens than before we changed remediation review?

The observed projection compares compatible historical slices and reports that
S5 consumed materially more measured Claude capacity. The causal explanation
then separates:

- valuable initial generic and instruction reviews;
- lower-cost retained remediation continuations;
- candidate changes caused by valid findings;
- the `reported` episode state preventing a later exact-subject continuation;
- the resulting fresh replacement review;
- quota failure and scheduled retry.

The explanation links the replacement to the exact review-state transition
contract. Its recommendation links to the repair that separates result
publication from episode retirement. A future post-adoption view compares
observed replacement frequency and cost against the prediction.

## Query and projection boundary

The first implementation should expose read-only query projections rather than
a general event mutation API:

- current operational projection by run, role, or repository;
- causal timeline by slice or candidate;
- cost attribution by semantic dimension;
- anomaly comparison against a named baseline;
- evidence graph traversal from any displayed identity;
- counterfactual and recommendation artifacts by exact version.

Projection refresh may be incremental, but every response names its evidence
cutoff and completeness. Partial provider telemetry, unavailable historical
metrics, missing raw traces, and stale projections remain visible.

## Delivery slices

### UI-0 — Event and owner projection

Define the provider-neutral event envelope, owner registry, integrity-bound
references, evidence classes, confidentiality handling, and deterministic joins.
Prove the S5 golden trace can be reconstructed without copied authority.

### UI-1 — Operational control plane

Render active slices, roles, reviewers, candidates, gates, limits, waiting
states, schedules, and retries. Begin read-only; invoke existing authority-bound
controls only in a later slice.

### UI-2 — Causal timeline and drill-down

Render event lanes and graph edges, then traverse every node to its exact owner,
revision, and limitations.

### UI-3 — Efficiency and anomaly explanation

Implement non-overlapping cost attribution, comparable baselines, anomaly
decomposition, and the S5 explanation fixture.

### UI-4 — Counterfactuals and recommendations

Add versioned policy simulations, uncertainty ranges, recommendation artifacts,
human decision boundaries, and post-adoption validation.

Each slice must remain independently useful. Operational visibility must not
wait for counterfactual modeling, and counterfactual machinery must not become
production routing authority.

## Acceptance evidence

- The S5 golden trace reconstructs exact candidate, gate, episode, finding,
  remediation, replacement, quota, schedule, and repair relationships.
- Every aggregate drills down to integrity-bound owner references.
- Observed, derived, interpreted, counterfactual, and recommended statements
  remain distinguishable in schema and presentation.
- Cost attribution does not double-count provider attempts across dimensions.
- Missing metrics remain unknown rather than zero.
- A provider session cannot imply semantic completion.
- A schedule cannot imply authority or execution.
- A finding cannot imply acceptance or remediation validity.
- A counterfactual cannot silently assume unchanged discovery quality.
- A recommendation cannot mutate its target owner.
- Projection cutoff, coverage, limitations, and confidentiality are visible.
- Concurrent or replaced reviewer generations remain distinct.
- Post-adoption evidence can confirm or falsify projected savings.

## Explicit exclusions

- A new canonical workflow state store owned by the UI.
- Acceptance decisions derived from cost, anomaly, or recommendation scores.
- Automatic reviewer selection or workflow mutation.
- Universal difficulty, quality, or review-coverage scores.
- Treating all additional review cost as waste.
- Inferring causal edges from timestamps alone.
- Reconstructing confidential raw traces without authorization.
- Hiding missing telemetry through estimates presented as observation.
- Coupling the first operational UI to the later counterfactual engine.

## Open implementation decisions

- The physical query index and retention policy for projections.
- Whether event normalization runs inside the App Server process or a bounded
  sibling service.
- The UI framework and transport protocol.
- Which raw provider events may be retained versus projected or omitted.
- Baseline-cohort selection and versioning for anomaly comparison.
- Counterfactual model implementation, calibration, and confidence semantics.
- Which controls, if any, graduate from read-only links to authority-bound UI
  actions in later slices.

These are mechanism choices. They must not redefine the truth classes,
authority boundaries, causal provenance, or acceptance exclusions above.
