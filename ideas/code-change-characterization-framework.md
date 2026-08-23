# Code Change Characterization Framework

## Status

Promoted formation source. Current candidate meaning is owned by
[`work-engine.code-change-characterization-profile`](../proposals/empirical-agent-research/code-change-characterization-profile/proposal.md).
This document remains the original exploratory design evidence and does not
authorize implementation.

This document defines a proposed framework for representing code changes as structured, measurable objects.

The purpose is broader than review analysis, agent evaluation, or token estimation. A sufficiently rich characterization of a code change can become foundational input for many later systems concerned with:

- review;
- testing;
- planning;
- architectural analysis;
- risk;
- regression detection;
- cost estimation;
- ownership;
- security;
- release decisions;
- agent behavior;
- historical comparison;
- and future research.

The immediate goal is **not** to create a single complexity score.

The goal is to preserve and derive independently meaningful properties of a change so that downstream systems can determine which properties matter for their particular task.

The central concern is avoiding measurements that produce attractive numbers without establishing what those numbers represent.

---

## 1. Core Idea

A code change should be treated as a first-class analytical object.

Given:

```text
repository state before
        +
repository state after
        +
architecture
        +
invariants
        +
history
        +
other relevant system structure
```

the system should derive a reusable:

```text
CODE CHANGE PROFILE
```

That profile can then support many independent consumers:

```text
                     CODE CHANGE PROFILE
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
      REVIEW                TESTING               RISK
        │                     │                     │
        ▼                     ▼                     ▼
 reviewer routing      test selection       blast-radius analysis

        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   ARCHITECTURE              COST                 AGENTS
        │                     │                     │
        ▼                     ▼                     ▼
 review triggers       token estimation      behavior research

        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
    OWNERSHIP              RELEASE               HISTORY
        │                     │                     │
        ▼                     ▼                     ▼
 routing/escalation      gating decisions      comparison/trends
```

No consumer should be required to accept a universal definition of change difficulty.

---

## 2. Primary Principle

> **Preserve independent change dimensions and allow downstream evidence to determine which dimensions matter.**

A value such as:

```text
complexity = 73
```

has little meaning unless the weighting that produced it has already been validated for a specific purpose.

By contrast:

```text
files_changed: 4
lines_added: 90
lines_deleted: 30
functions_touched: 6
public_interfaces_changed: 1
state_transitions_changed: 2
invariants_affected: 2
persistence_boundaries_changed: 1
```

describes observable characteristics of the change.

A review system may eventually weight these dimensions one way.

A token estimator may weight them differently.

A release-risk model may discover that entirely different dimensions matter.

The change profile should preserve the underlying measurements rather than prematurely collapsing them.

---

## 3. Change Identity

Every profile must refer to an exact code subject.

At minimum, preserve:

- parent revision;
- resulting revision;
- diff identity;
- repository identity;
- branch where relevant;
- timestamp;
- work item or proposal lineage where available;
- generation method;
- analysis-tool version.

A change profile without an immutable subject cannot be trusted as durable evidence.

---

## 4. Physical Change

These are conventional diff-level measurements.

Possible statistics include:

- files changed;
- files added;
- files deleted;
- lines added;
- lines deleted;
- lines modified;
- diff hunks;
- functions touched;
- classes touched;
- modules touched;
- packages touched;
- tests changed;
- documentation changed;
- configuration files changed;
- generated files changed.

These measurements are useful baseline characteristics.

They should not be treated as sufficient representations of semantic importance, review difficulty, or risk.

---

## 5. Structural Change

A small textual change may affect a large structural surface.

Possible structural measurements include:

- functions added or removed;
- classes added or removed;
- interfaces added or changed;
- public APIs changed;
- call-graph nodes affected;
- call-graph edges added;
- call-graph edges removed;
- dependency edges changed;
- import relationships changed;
- module boundaries crossed;
- package boundaries crossed;
- ownership boundaries crossed;
- persistence boundaries crossed;
- external-service boundaries crossed;
- concurrency boundaries crossed;
- trust boundaries crossed.

A useful distinction is:

```text
syntactic breadth
versus
structural breadth
```

A large fixture update may have high syntactic breadth and almost no structural impact.

A six-line authorization change may have minimal syntactic breadth and major structural impact.

---

## 6. Control-Flow Change

Possible measurements include:

- branches added;
- branches removed;
- branch predicates changed;
- exception paths added or changed;
- fallback paths changed;
- retry paths changed;
- loops changed;
- asynchronous paths changed;
- lifecycle transitions changed;
- early exits added;
- failure handling changed;
- recovery behavior changed;
- alternate routes added or removed;
- ordering assumptions changed.

The system should distinguish between:

```text
more code paths
```

and:

```text
different meaning of an existing path
```

These may have very different downstream consequences.

---

## 7. State and Persistence Change

State changes may be disproportionately important relative to their physical size.

Candidate measurements include:

- state variables added or removed;
- state ownership changed;
- persisted fields changed;
- serialization formats changed;
- schema versions changed;
- migrations added;
- resumability behavior changed;
- checkpoint behavior changed;
- continuation behavior changed;
- transaction boundaries changed;
- idempotency assumptions changed;
- caching behavior changed;
- mutation ownership changed;
- recovery state changed.

Where possible, these measurements should be derived from explicit state and lifecycle models rather than syntax alone.

---

## 8. Interface and Contract Change

Interface changes can create consequences far outside the changed lines.

Possible measurements include:

- public function signatures changed;
- API request schemas changed;
- API response schemas changed;
- CLI interfaces changed;
- configuration schemas changed;
- event schemas changed;
- receipt schemas changed;
- tool schemas changed;
- persisted data contracts changed;
- validation contracts changed;
- compatibility guarantees changed;
- authority contracts changed.

Where possible, distinguish:

```text
compatible extension
breaking change
behavioral reinterpretation
validation tightening
validation loosening
```

---

## 9. Semantic and Architectural Change

This layer is likely to be one of the most valuable and one of the hardest to derive reliably.

Possible measurements include:

- responsibilities affected;
- architectural components affected;
- roles affected;
- capabilities affected;
- authority boundaries affected;
- invariants affected;
- acceptance conditions affected;
- prohibitions affected;
- lifecycle states affected;
- state transitions affected;
- evidence relationships affected;
- ownership relationships affected;
- review boundaries affected;
- security properties affected.

For systems where these concepts are explicitly represented, the analyzer should consume those representations.

For example:

```text
syntactic:
  12 lines changed

structural:
  1 function changed
  2 dependency edges affected

semantic:
  authority invariant INV-017 affected
  persistence ownership changed
  accepted→resumable transition modified
```

This is far more informative than line count alone.

---

## 10. Semantic Breadth

A separate measurement may be useful for how many independent concerns a change touches.

Candidate statistics include:

- distinct responsibilities changed;
- distinct invariant classes touched;
- distinct state domains changed;
- distinct role relationships affected;
- distinct behavioral outcomes changed;
- number of externally observable behaviors affected;
- number of independently reviewable concerns.

A change touching five lines across five unrelated behavioral domains may require more reasoning than a 300-line implementation of one coherent transformation.

---

## 11. Data-Model Change

Candidate measurements include:

- data structures changed;
- fields added;
- fields removed;
- field meaning changed;
- normalization rules changed;
- cardinality changed;
- validation rules changed;
- schema relationships changed;
- compatibility assumptions changed;
- provenance fields changed;
- identifier semantics changed;
- migration requirements introduced.

These may be particularly relevant to persistence, replay, compatibility, integration, and migration analysis.

---

## 12. Testing Surface

The test surface should be represented independently from the implementation change.

Possible measurements include:

- tests added;
- tests changed;
- existing tests implicated;
- test modules affected;
- fixture changes;
- property-based tests affected;
- integration tests affected;
- end-to-end tests affected;
- validation gates affected;
- historically relevant tests;
- expected test coverage surface.

This enables later systems to ask whether testing effort is proportionate to structural or semantic impact rather than merely to diff size.

---

## 13. Historical Characteristics

The history of affected code may contain useful information.

Possible measurements include:

- code age;
- file age;
- function age;
- change frequency;
- author count;
- prior defect count;
- prior review-finding count;
- prior revert count;
- prior remediation count;
- frequency of architectural change in the area;
- frequency of invariant violations;
- prior reviewer disagreement;
- subsystem maturity.

These are descriptive characteristics.

They should not be automatically interpreted as risk.

Frequently changed code may be unstable.

Or it may simply be actively developed.

Observed outcomes are needed to learn the difference.

---

## 14. Novelty

Novelty may matter independently of size.

Possible indicators include:

- new architectural pattern;
- new subsystem;
- new dependency;
- new tool;
- new storage mechanism;
- first use of a framework capability;
- first instance of a state relationship;
- first role or authority relationship of its kind;
- new execution-path type;
- new external integration.

Novelty should be defined relative to the repository and system architecture, not merely relative to model knowledge.

---

## 15. Dependency Radius

The analysis should characterize how far effects may propagate.

Possible measures include:

- direct dependents;
- transitive dependents;
- direct dependencies;
- transitive dependencies;
- changed public consumers;
- affected tests;
- affected workflows;
- affected roles;
- affected persisted artifacts;
- affected schemas;
- affected external systems.

Later consumers may derive classifications such as:

```text
local
component-level
cross-component
system-wide
```

but raw propagation measurements should remain available.

---

## 16. Change Topology

Two changes with the same file count may have very different shapes.

For example:

```text
Change A:
10 files inside one cohesive package

Change B:
10 files across 8 unrelated subsystems
```

Possible topology statistics include:

- file clustering;
- module clustering;
- dependency distance between changed nodes;
- number of connected change components;
- maximum graph distance between affected components;
- proportion of changed nodes in one dominant component;
- cross-layer edges affected.

This may represent breadth better than raw file count.

---

## 17. Behavioral Surface

Where the system can identify externally meaningful behavior, characterize what behavior changed.

Possible measurements include:

- behaviors introduced;
- behaviors removed;
- behaviors modified;
- error behavior changed;
- success behavior changed;
- recovery behavior changed;
- user-visible behavior changed;
- operator-visible behavior changed;
- timing behavior changed;
- persistence behavior changed;
- compatibility behavior changed;
- authorization behavior changed.

This layer should distinguish implementation structure from externally meaningful consequences.

---

## 18. Change Coherence

A potentially useful future dimension is whether the change forms one coherent transformation or several loosely related transformations.

Possible evidence includes:

- number of independent objectives represented;
- number of unrelated modules touched;
- number of distinct invariant families affected;
- number of unrelated behavior classes changed;
- clustering of changed graph nodes;
- whether changed files share one causal path.

This should initially remain a descriptive or classified property rather than a numerical quality judgment.

---

## 19. Pre-Outcome Versus Post-Outcome Data

This distinction must remain strict.

### Pre-outcome characteristics

Facts available from the change itself before downstream processes complete:

- diff statistics;
- structural graph changes;
- invariant impact;
- role impact;
- state changes;
- interface changes;
- dependency radius;
- historical characteristics;
- topology;
- novelty.

These may be used by predictive systems.

### Post-outcome evidence

Observed consequences after review, testing, deployment, remediation, or later use:

- findings;
- failures;
- regressions;
- remediation;
- reviewer disagreement;
- token usage;
- rework;
- later defects;
- release outcomes.

These are labels and validation evidence.

A predictor must not use downstream outcomes as inputs if it claims to predict those same outcomes beforehand.

---

## 20. Evidence Quality

Not every profile field will have the same epistemic status.

Candidate evidence classes include:

```text
DIRECT
Deterministically derived from repository state.

STRUCTURALLY_DERIVED
Derived from call graphs, dependency graphs, invariant mappings,
state models, or other explicit structures.

CLASSIFIED
Assigned by a deterministic or model-based classifier.

INFERRED
Estimated from incomplete or indirect evidence.

SELF_REPORTED
Reported by an agent or reviewer.
```

These distinctions should remain visible.

A model-classified semantic effect should not silently become equivalent to a deterministic line count.

---

## 21. Derived-Analysis Provenance

Every derived statistic should retain enough provenance to be recomputed.

At minimum:

- source revision;
- parent revision;
- diff identity;
- analysis-tool version;
- structural-graph revision;
- invariant-catalog revision;
- classifier version where applicable;
- model/version where model classification is used;
- timestamp;
- source artifact identities;
- limitations.

The authoritative evidence should remain the underlying repository and structural data.

Derived profiles should be replaceable.

---

# Downstream Uses

## 22. Review Analysis

The change profile can support questions such as:

- Which changes need deeper review?
- Which reviewers perform best on which change shapes?
- Which structural characteristics predict unique findings?
- Which semantic dimensions predict reviewer disagreement?
- Which changes should receive architectural review?
- Which changes are sufficiently narrow for lighter review?

The profile should support review decisions without being owned by the review subsystem.

---

## 23. Reviewer-Derived Measurements

Where multiple reviewers inspect a change, their outputs create a second analytical object.

For each change, retain:

- reviewer identity/configuration;
- reviewer model;
- reviewer environment;
- review token usage;
- findings produced;
- finding categories;
- severity;
- confidence;
- confirmed findings;
- rejected findings;
- duplicate findings;
- unique findings;
- review iterations;
- remediation requests;
- outcome.

Then derive comparative statistics such as:

- union of findings;
- intersection of findings;
- pairwise agreement;
- pairwise disagreement;
- unique contribution by reviewer;
- overlap rate;
- confirmed precision;
- reviewer-specific detection rates where later evidence permits.

These measurements remain separate from the static change profile.

---

## 24. Review-Surface Diffusion

Multiple reviewers may expose a property of a change that structural analysis alone misses.

Consider:

```text
Reviewer A: 3 findings
Reviewer B: 1 finding
Reviewer C: 4 findings

Union:        6
Intersection: 1
```

versus:

```text
Reviewer A: 2 findings
Reviewer B: 2 findings
Reviewer C: 2 findings

Union:        2
Intersection: 2
```

The first change appears to expose a diffuse review surface.

The second exposes a concentrated, consistently recognized surface.

Possible measurements include:

- reviewer-finding entropy;
- finding overlap;
- unique-finding ratio;
- disagreement rate;
- finding-category dispersion;
- remediation dispersion.

Whether these are useful should be determined empirically.

---

## 25. Test Selection

Change characterization may support targeted test selection.

Potential uses include:

- identifying directly affected tests;
- identifying transitive test impact;
- detecting persistence-sensitive changes;
- detecting concurrency-sensitive changes;
- detecting schema compatibility risks;
- selecting expensive integration tests only when relevant;
- identifying areas with historically weak coverage.

The profile should describe the change.

A separate testing system should decide how those characteristics affect test strategy.

---

## 26. Architectural Review

The profile may support architectural-review triggers such as:

- authority boundaries changed;
- invariants changed;
- new subsystem introduced;
- persistence ownership changed;
- cross-layer dependencies introduced;
- dependency radius unusually large;
- multiple architectural components affected;
- novel execution model introduced.

Architectural review should consume the characterization rather than duplicate code analysis independently.

---

## 27. Planning and Work Decomposition

A planner may use change characteristics to determine:

- whether work should be split;
- whether multiple slices are justified;
- which components should be handled independently;
- where checkpoints should occur;
- which specialists may be needed;
- whether a proposal needs revision before implementation.

Again, the profile provides facts.

Planning remains a separate judgment process.

---

## 28. Ownership and Routing

Change profiles may support:

- code-owner selection;
- reviewer routing;
- specialist escalation;
- security-review routing;
- persistence-review routing;
- UI-review routing;
- architectural-review routing.

The same change may route differently depending on the consumer.

---

## 29. Security Analysis

Security-sensitive characteristics may include:

- trust-boundary changes;
- authentication changes;
- authorization changes;
- input-validation changes;
- secrets handling;
- external-input paths;
- execution privileges;
- persistence of sensitive data;
- dependency changes;
- network exposure.

A security system can consume these fields without requiring the general change profiler to make the security decision itself.

---

## 30. Release and Risk Analysis

A future release system may correlate change profiles with historical outcomes to identify patterns associated with:

- regressions;
- rollback;
- hotfixes;
- later defects;
- deployment failures;
- migration problems;
- compatibility failures.

Any resulting risk model should remain downstream from the characterization layer.

---

## 31. Documentation Impact

The profile may help determine whether a change affects:

- public APIs;
- CLI behavior;
- configuration;
- schemas;
- user-visible behavior;
- operator behavior;
- architecture;
- deployment;
- migrations;
- security assumptions.

A documentation system can then decide which artifacts may require updates.

---

## 32. Historical Comparison

Structured profiles make it possible to ask:

- Have changes become structurally broader over time?
- Are architectural-boundary changes becoming more common?
- Is state-related work producing more regressions?
- Which subsystems accumulate the most semantically broad changes?
- Are review findings shifting toward particular change classes?
- How does the repository's change topology evolve?

This is much richer than historical LOC statistics.

---

## 33. Change Anomaly Detection

Given enough historical profiles, future systems may identify changes unlike prior repository work.

For example:

```text
This change is physically small but combines:
- authority modification
- persistence ownership
- new asynchronous path
- cross-component dependency
```

The useful output need not be:

```text
risk = 91
```

It may simply be:

> This change lies outside the common historical profile of this subsystem.

That can itself be actionable.

---

# Agent and Cost Research

## 34. Agent Evaluation

Agent evaluation is one consumer of the general change profile.

Without a representation of the work itself, agent-performance measurements are badly confounded.

Suppose:

```text
Run A: 40,000 tokens
Run B: 160,000 tokens
```

That difference says little unless the corresponding changes can be compared.

Run B may have:

- crossed more architectural boundaries;
- altered persistent state;
- modified authorization behavior;
- affected more invariants;
- introduced more control-flow paths;
- or simply involved a more difficult change.

Alternatively, the changes may be comparable and Run B's environment may have caused unnecessary exploration, repetition, or rework.

The general research model therefore becomes:

```text
ENVIRONMENT PROFILE
What conditions surrounded the agent?

CHANGE PROFILE
What transformation was being performed?

BEHAVIOR / OUTCOME PROFILE
What did the agent do, and what happened?
```

---

## 35. Environmental Research Integration

The change profile is necessary for serious environmental analysis.

Without it, a correlation such as:

```text
high procedural density
→ high token usage
```

may simply reflect procedure-heavy runs receiving harder changes.

Likewise:

```text
judgment-preserving environment
→ lower defect rate
```

could be misleading if those runs involved easier work.

The research model should therefore treat change characteristics as explanatory variables alongside environmental characteristics:

```text
E = environment vector
C = change vector

Behavior = f(E, C, ...)
Outcome  = f(E, C, Behavior, ...)
Tokens   = f(E, C, ...)
```

No functional form is assumed.

The purpose is to preserve enough structure that these relationships can be discovered empirically.

---

## 36. Token-Usage Estimation

With reliable runtime telemetry, future runs can preserve actual token usage together with change characteristics.

A research record may eventually resemble:

```text
CHANGE PROFILE
  physical statistics
  structural statistics
  semantic statistics
  state impact
  topology
  historical characteristics

ENVIRONMENT PROFILE
  role
  skill composition
  model
  reasoning effort
  context size
  visible capabilities
  procedural structure
  evidence environment

EXECUTION PROFILE
  inference calls
  tool calls
  subagents
  compactions
  iterations

OUTCOME
  correctness
  findings
  remediation
  later failures

RESOURCE USE
  input tokens
  output tokens
  total tokens
```

This permits questions such as:

- Which change properties best predict token usage?
- Does semantic breadth predict cost better than lines changed?
- Do state changes produce disproportionately expensive work?
- Does dependency radius predict review cost?
- Does reviewer disagreement correlate with token consumption?
- Do some environments use more tokens but reduce downstream rework?
- Can resource requirements be estimated before execution with useful uncertainty bounds?

---

## 37. Lifecycle Cost

Single-invocation resource use may be misleading.

A more useful long-term concept may be:

```text
implementation
+ review
+ review retries
+ remediation
+ validation
+ later correction
+ reopened work
= lifecycle resource cost
```

A workflow that minimizes first-pass tokens may increase total cost if it produces more defects, more rework, or repeated review cycles.

The change profile provides the stable problem description against which lifecycle costs can be compared.

---

## 38. Reviewer Selection

The same profile may support adaptive reviewer selection.

Historical evidence may eventually show patterns such as:

```text
state-heavy changes
→ reviewer configuration A performs better

API-boundary changes
→ configuration B finds more confirmed defects

large but semantically narrow changes
→ lighter review sufficient

small authority/invariant changes
→ stronger review justified
```

These relationships should be learned from outcomes rather than hard-coded into the change profile.

---

## 39. Benchmark Stratification

A benchmark consisting of heterogeneous changes can produce misleading aggregate results.

Change profiles can support stratification by dimensions such as:

- physical size;
- semantic breadth;
- state impact;
- architecture impact;
- topology;
- novelty;
- dependency radius;
- behavioral surface.

This makes comparisons between agents, reviewers, and workflows more meaningful.

---

# Research Discipline

## 40. Avoiding Circular Metrics

A metric must not be defined by the outcome it is later used to explain.

For example:

```text
review_difficulty =
    number_of_findings
  + review_tokens
  + remediation_cycles
```

cannot then meaningfully explain why review consumed many tokens.

The system should preserve distinctions between:

- characteristics of the change;
- characteristics of the environment;
- observable agent behavior;
- reviewer behavior;
- outcomes;
- resource consumption.

Derived variables must retain lineage to their constituents.

---

## 41. Composite Scores

Composite scores are not prohibited.

They are deferred.

A future system may discover that a particular weighted combination of:

```text
semantic breadth
dependency radius
state impact
historical defect rate
```

strongly predicts a specific outcome.

At that point, a derived predictor may be useful.

But it should be named according to what it predicts:

```text
expected_review_cost
predicted_regression_risk
expected_token_use
```

rather than being promoted into a universal notion of:

```text
change_complexity
```

unless evidence supports that broader interpretation.

---

## 42. Initial Implementation Strategy

The first implementation should favor extraction over interpretation.

### Phase 1 — Deterministic Baseline

Capture measurements that are cheap and objective:

- file counts;
- line counts;
- hunks;
- functions/classes touched;
- module/package distribution;
- test changes;
- interface signatures;
- dependency edges where available.

### Phase 2 — Existing Structural Models

Add measurements already represented elsewhere:

- invariants affected;
- authority relationships affected;
- roles affected;
- capabilities affected;
- lifecycle transitions affected;
- state ownership changes;
- evidence relationships affected.

### Phase 3 — Semantic Classification

Only after raw evidence exists, experiment with model-assisted classification of:

- responsibility breadth;
- behavioral-change categories;
- novelty;
- coherence;
- architectural impact.

Classifier outputs should remain derived evidence with provenance.

### Phase 4 — Downstream Correlation

Join the profiles with downstream evidence:

```text
change profiles
review outcomes
testing outcomes
environment profiles
token usage
lifecycle outcomes
```

and discover which relationships appear meaningful.

### Phase 5 — Controlled Validation

Where correlations look important, test them through controlled experiments, matched comparisons, or prospective validation.

---

## 43. Example Change Profile

A future record might contain:

```text
change_id:
  repository: ...
  parent_revision: ...
  revision: ...

physical:
  files_changed: 4
  lines_added: 90
  lines_deleted: 30
  hunks: 8
  functions_touched: 6

structural:
  modules_touched: 2
  dependency_edges_changed: 7
  public_interfaces_changed: 1

state:
  persisted_fields_changed: 2
  state_transitions_changed: 1
  persistence_boundaries_changed: 1

semantic:
  invariants_affected: 2
  roles_affected: 1
  authority_boundaries_affected: 1

testing:
  tests_added: 3
  tests_changed: 2

topology:
  connected_change_components: 1
  cross_module_edges: 3

history:
  affected_area_prior_defects: 4
  affected_area_change_frequency: ...

provenance:
  analyzer_version: ...
  graph_revision: ...
  invariant_catalog_revision: ...
```

No universal `complexity_score` is required.

---

## 44. Questions the Framework Should Eventually Support

The profile should make it possible for downstream systems to investigate questions such as:

- What exactly changed?
- How structurally broad was the change?
- How semantically broad was it?
- Which architectural relationships were affected?
- Which invariants were affected?
- What state or persistence behavior changed?
- What is the dependency radius?
- How topologically dispersed is the change?
- Is this change unusual for this subsystem?
- Which tests are relevant?
- Does this change warrant architectural review?
- Which reviewer configuration is most appropriate?
- Which change properties predict reviewer disagreement?
- Which properties predict later defects?
- Which properties predict rework?
- Which properties predict token consumption?
- Which environmental variables remain predictive after change characteristics are controlled?
- Which changes historically require greater lifecycle cost?
- Which characteristics should trigger specialist review?
- Which properties are useful for release or regression analysis?

The framework should support questions that have not yet been imagined as long as sufficient underlying evidence is retained.

---

## 45. Immediate Data-Preservation Requirement

The immediate objective should not be to answer all of these questions.

It should be to make sure the evidence exists when they become useful.

For every significant change, retain enough information to bind:

```text
exact repository subject
        ↓
change-profile source evidence
        ↓
work/proposal lineage
        ↓
implementation
        ↓
testing
        ↓
review
        ↓
remediation
        ↓
accepted revision
        ↓
later defect/correction evidence
        ↓
resource and behavioral evidence
```

Raw evidence should be preserved wherever practical.

Derived statistics can be recomputed.

Discarded evidence cannot.

---

## 46. Foundational Role

The code change profile should be treated as a reusable foundational data product rather than as an artifact belonging to any single workflow stage.

Its responsibility is:

> **Describe the change faithfully enough that later systems can reason about its structure, semantics, scope, relationships, history, and consequences without each subsystem inventing its own incompatible approximation of what changed.**

Review should consume it.

Planning should consume it.

Architectural analysis should consume it.

Testing should consume it.

Metrics should consume it.

Agent-environment research should consume it.

Future systems should be able to consume it without requiring the characterization layer to anticipate their exact questions.

---

## 47. Core Principle

The framework can be summarized as:

```text
DO NOT ASK FIRST:
"How complex is this change?"

ASK:
"What measurable properties does this change have?"
```

Then let evidence determine which of those properties matter for:

```text
review
testing
risk
cost
architecture
agents
release
history
or anything else built later
```

The purpose of code change characterization is not to manufacture a numerical representation of difficulty.

It is to create a durable, reusable description of change from which **many different kinds of useful reasoning can later be derived.**
