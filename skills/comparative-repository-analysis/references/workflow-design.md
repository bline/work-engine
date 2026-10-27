# Comparative Repository Analysis Workflow — Design Document

## Purpose

Build a reusable workflow for analyzing external software-engineering orchestration systems one repository at a time, compressing each repository into a durable evidence-backed description, and then comparing those descriptions against one another and against Work Engine.

This workflow is not primarily a feature checklist generator.

Its purpose is to answer two higher-value questions:

1. **What useful mechanisms have other systems already solved that we should adopt, borrow, or learn from?**
2. **What have we discovered in Work Engine that is genuinely differentiated and should not be lost while adopting commodity infrastructure?**

The workflow should therefore function as both:

- a **feature harvester** from external projects; and
- an **introspection system** for Work Engine itself.

The final output must support architectural decisions, not merely summarize repositories.

---

# 1. Core goals

## 1.1 Specific goal

Analyze a set of orchestration/workflow repositories and produce a trustworthy comparison that identifies:

- overlapping capabilities;
- differentiated capabilities;
- stronger implementations elsewhere;
- weaker or risky implementations;
- mechanisms worth adopting wholesale;
- mechanisms worth borrowing selectively;
- mechanisms to avoid because they reproduce failure modes already observed in Work Engine;
- gaps in Work Engine that external systems expose;
- Work Engine ideas that appear uncommon and may represent meaningful differentiation.

The comparison should be detailed enough to guide future architecture and roadmap decisions.

## 1.2 Abstract goal

Create a general method for comparing complex software systems without requiring one model to hold all source repositories in context simultaneously.

The workflow should transform:

```text
large, heterogeneous repositories
        ↓
evidence-backed semantic compression
        ↓
comparable architectural descriptions
        ↓
targeted synthesis and decision support
```

The workflow should minimize total reasoning/context cost while preserving enough provenance to reopen any important claim.

---

# 2. Design philosophy

The workflow should embody the same lessons Work Engine has learned.

## 2.1 Rules belong in machinery; judgment belongs in the model

Mechanical concerns should be deterministic where possible:

- repository discovery;
- file/path handling;
- artifact persistence;
- schema validation;
- metrics harvesting;
- process lifecycle;
- retries;
- exact provenance references.

Semantic questions should remain model judgments:

- which mechanisms matter;
- whether two systems are meaningfully equivalent;
- whether a design is rigid or adaptive;
- whether a mechanism is genuinely differentiated;
- whether a feature should be adopted, borrowed, or avoided;
- whether more evidence would change the decision.

## 2.2 Constrain outputs, not reasoning routes

The workflow should specify what a trustworthy repository analysis must establish, but should not prescribe a fixed search sequence.

A repo-analysis agent may use:

- codebase graph/index tools;
- exact source reads;
- documentation;
- tests;
- configuration;
- schemas;
- call tracing;
- targeted searches;

according to what the repository and question require.

The output must be evidence-backed, but the route to that evidence should remain adaptive.

## 2.3 Fix affordances before adding instructions

If agents repeatedly take a bad route, first ask whether the intended route was harder to use.

Do not solve infrastructure friction by adding procedural warnings when the actual problem is:

- missing configuration;
- ambiguous paths;
- bad CLI ergonomics;
- missing examples;
- inaccessible artifacts;
- unclear ownership.

## 2.4 Preserve evidence, discard exploration

Raw repository exploration is low-durability context.

The durable result should be:

- claims;
- mechanisms;
- architectural interpretations;
- provenance;
- uncertainties;
- unresolved questions.

Once those are recorded, the exploratory context should be disposable.

---

# 3. Intended workflow shape

The workflow separates three primary responsibilities: mechanical coordination,
repository compression and claim reconciliation, and fresh cross-repository
synthesis.

```text
mechanical comparison coordinator
        ↓
frozen comparison-run contract
        ↓
checkout + Codebase Memory index
        ↓
bounded dimension-specific analysis passes
        ↓
durable per-pass evidence artifacts
        ↓
claim reconciliation + compressed repository profile
        ↓
analysis contexts discarded
        ↓
next repository
        ↓
...
        ↓
fresh cross-repository synthesis
```

## 3.1 Mechanical comparison coordinator

The coordinator should remain thin and mechanical.

It owns:

- the set of repositories to analyze;
- analysis status;
- review artifact locations;
- the frozen comparison-run contract;
- run-level metrics;
- unresolved cross-repository questions; and
- process and artifact lifecycle.

It should not absorb raw repository exploration.

It should not inspect all repositories directly unless a targeted comparison question requires it.

It should not also own semantic cross-repository synthesis. A fresh synthesis
context should consume the reconciled repository profiles after the coordinator
has established that their contracts and schema versions are comparable.

## 3.2 Comparison-run contract

Adaptive analysis depth must not make the subjects incomparable. Before the
first repository pass, freeze a run contract containing:

- the question or decision the comparison is intended to inform;
- repository inclusion, exclusion, and stopping criteria;
- the repository corpus and any known selection or survivorship bias;
- ontology and artifact-schema versions;
- semantic definitions for the required core dimensions;
- evidence, confidence, absence, and runtime-correspondence semantics;
- the minimum evidence floor shared by every repository;
- allowed adaptive or repository-specific dimensions;
- the Work Engine snapshot and the method used to profile it;
- calibration and quality checks; and
- rules for schema migration, selective re-analysis, and invalidation.

The contract is a comparability boundary, not a fixed reasoning route. Agents
may investigate different questions at different depths, but identical artifact
states must retain identical meanings throughout one comparison run.

If the ontology changes materially, the workflow must either:

1. migrate prior artifacts without changing their established meaning;
2. selectively rerun the affected dimensions; or
3. begin a new comparison run.

It must not silently compare profiles produced under incompatible semantics.

## 3.3 Shared repository evidence substrate

Before semantic analysis begins, each checked-out repository should be indexed with Codebase Memory and its index health/coverage checked.

The index is shared infrastructure for all later analysis passes. It should provide structural and semantic repository evidence without requiring every agent to rediscover the same source relationships.

The intended separation is:

```text
Codebase Memory
  → durable repository structure and retrieval capability

analysis agents
  → temporary semantic interpretation

review artifacts
  → durable claims, provenance, uncertainty, and conclusions
```

Pre-indexing should remain a mechanical setup step. It does not decide which architectural claims matter and should not force a fixed query sequence.

A repository may still require targeted exact-source reads when:

- behavior depends on literal implementation details;
- wording in model-facing instructions matters;
- graph/index evidence is ambiguous;
- an important negative or exhaustive claim needs stronger coverage.

### Orientation before expensive passes

The shared index also supports a cheap orientation step before deeper analysis.

Useful orientation evidence includes:

- repository size and languages;
- index coverage and parse health;
- locations of skills, prompts, agent definitions, policies, schemas, and workflow files;
- likely orchestration/state/persistence/review/metrics subsystems;
- test structure;
- major architectural boundaries.

The coordinator may use this evidence to decide which semantic passes are worth running and at what depth.

### Establish the actual system boundary

A repository is a useful context boundary, but it is not always the whole
system. Orientation should record which conclusions depend on:

- hosted services or closed components;
- external model or provider behavior;
- plugins, dependencies, or generated artifacts;
- release packaging and deployment topology;
- configuration, feature flags, and default settings;
- operating system or runtime assumptions; and
- evidence outside the checked-out repository.

Record the commit, release or tag when available, branch, analysis timestamp,
relevant configuration, and unavailable external evidence. Do not credit the
repository for behavior that was only inferred from an external dependency.

## 3.4 Repo-compression agent

Each repository gets a fresh agent context.

The compression agent owns:

- understanding that repository;
- discovering relevant mechanisms;
- collecting evidence;
- interpreting architecture;
- writing a bounded review artifact;
- identifying unknowns and limitations.

It does **not** need to know the full Work Engine implementation or history.

It receives only the evaluation ontology needed to recognize relevant mechanisms.

This reduces duplicated Work Engine context across every repository analysis.

The ontology must include an open-discovery affordance rather than acting as a
closed checklist. Every repository pass should ask:

> What important mechanism, trade-off, or optimization in this system is not
> adequately represented by the current ontology?

This protects against treating Work Engine's current vocabulary as the full
space of possible designs. Emergent dimensions should be preserved without
silently changing the frozen core contract for repositories already analyzed.

Work Engine should be profiled through the same observation and evidence
contract as external repositories. Where practical, its first descriptive pass
should occur without asking the analyst to prove a KEEP, ADOPT, BORROW, or AVOID
conclusion.

### Value-based stopping

The compression agent should stop exploring when the remaining unresolved questions are unlikely to change:

- the architectural description;
- the classification of important mechanisms;
- whether the repository is worth deeper investigation; or
- a later KEEP / ADOPT / BORROW / AVOID / INVESTIGATE decision.

Context pressure, repository size, elapsed time, and tool cost are evidence for this judgment, not fixed stopping thresholds.

The agent should preserve unresolved questions explicitly rather than continue into low-value architectural rabbit holes merely for completeness.

### Sparse compression

The review artifact is an ontology for material findings, not a form that must be filled densely.

Prefer a small number of strong claims with strong provenance over exhaustive
prose. Mark low-value dimensions as `not_sought` or `not_applicable` when that
is their real state rather than manufacturing detail. Required core dimensions
must not disappear through omission.

Where useful, bound cardinality rather than prose mechanically, for example:

- a short architectural summary;
- only material capability mechanisms;
- a small number of strongest evidence references per claim;
- only decision-relevant unknowns.

Exact limits may evolve with evidence from real runs; they should not become arbitrary semantic constraints.

---

# 4. Why repositories are analyzed one at a time

Each repository is a natural context boundary.

Analyzing them sequentially has several advantages:

- raw exploration never accumulates across repositories;
- early runs can reveal proposed schema improvements for the next comparison
  run or trigger an explicit migration under the current run contract;
- weak or irrelevant repositories can be abandoned cheaply;
- high-value repositories can receive deeper follow-up;
- cost can vary according to decision relevance;
- context compaction in one repository cannot corrupt the entire comparison.

The workflow should become narrower as evidence accumulates.

It should not become more exhaustive by default.

---

# 5. Information we care about

The analysis should distinguish **features** from **architectural mechanisms**.

A feature name such as “review,” “quality gate,” or “persistent state” is insufficient by itself.

We care about how the system implements and reasons about that feature.

## 5.1 Control and judgment architecture

Questions include:

- Is workflow behavior controlled by fixed state transitions, model judgment, or a hybrid?
- Does the model choose routes or merely execute predefined phases?
- Can a premise be falsified without terminating the entire task?
- Can valid evidence survive a route revision?
- Can the system revise scope or architecture when evidence demands it?
- Are acceptance conditions distinct from routes?
- Does the system preserve meaningful model autonomy?

## 5.2 Quality and semantic verification

Questions include:

- What does “verified” mean?
- Are tests merely executed, or is semantic consequence considered?
- Does verification reach the intended consumer?
- Can a locally coherent substitute incorrectly satisfy the workflow?
- Is there an equivalent to Work Engine's “insufficient substitute” concept?
- Is falsification explicitly encouraged?
- Are confidence and evidence proportional to consequence?

## 5.3 Review architecture

Questions include:

- Are reviewers independent from builders?
- Are reviewers fresh or persistent?
- What happens after findings?
- Does remediation preserve reviewer context?
- Is review mandatory or risk-proportional?
- Can review challenge architecture/placement, or only inspect implementation?

## 5.4 State and persistence

Questions include:

- Where does durable task state live?
- What survives model/context loss?
- Are artifacts persisted outside model memory?
- Are receipts/ledgers append-only?
- Can interrupted tasks resume safely?
- Who owns persistence?
- Is durable state model-authored or runtime-derived?

## 5.5 Context architecture

Questions include:

- What remains in persistent agent context?
- What is disposable?
- Is context rotated or compacted?
- Are large exploratory tasks isolated?
- How are summaries/handoffs created?
- Does the system track context pressure?
- Does context-management machinery preserve exact artifacts?

## 5.6 Repository evidence and retrieval

Questions include:

- Does the system use structural indexes or graph retrieval?
- How are source ranges selected?
- Are absence claims exhaustive or provisional?
- Is provenance preserved?
- Are semantic discovery and exact verification distinguished?
- Does retrieval pollute the durable builder context?

## 5.7 Process and mechanical substrate

Questions include:

- retry policy;
- process lifecycle;
- subprocess supervision;
- path resolution;
- worktree isolation;
- locking;
- atomic persistence;
- schema validation;
- config ownership;
- CLI/tool ergonomics;
- preflight checks;
- failure classification.

This is the area most likely to contain commodity infrastructure worth adopting.

## 5.8 Metrics and observability

Questions include:

- What does the runtime measure?
- Are token/context metrics derived or self-reported?
- Are latency, failures, retries, and provider cost captured?
- Can runs be compared over time?
- Can the system detect behavioral drift?
- Can it tell whether “proportional” behavior has become ritualized?

## 5.9 Flexibility versus proceduralization

This is a central comparison dimension.

Look for:

- amount of always-loaded normative prose;
- number of mandatory workflow transitions;
- fixed evidence/review sequences;
- rigid decision tables;
- opportunities for model-selected routes;
- infrastructure rules implemented deterministically in code;
- whether heterogeneous tasks produce meaningfully different workflows.

A system may describe itself as flexible while behaving identically on every task.

## 5.10 Executable interface integrity

Look for:

- path-anchored invocation examples;
- tested examples;
- runtime config availability;
- clear CLI usage;
- useful `--help`;
- deterministic preflight;
- no silent-success failure modes;
- no mismatch between documentation and actual executable interfaces.

## 5.11 Mechanism relationships and system effects

Dimensions must not become isolated feature columns. For each important
mechanism, ask:

- What does it depend on?
- What does it enable?
- What does it constrain or make more expensive?
- Which mechanism can substitute for it?
- Which state or lifecycle does it share with other mechanisms?
- Does its value disappear when removed from the surrounding architecture?
- What beneficial or harmful behavior emerges only from the combination?

The repository profile should preserve these relationships so synthesis can
compare architectural systems rather than detached mechanisms.

## 5.12 Operating and adoption context

These dimensions are conditional rather than mandatory deep-analysis passes,
but they become required when they could change an adoption decision:

- security, privacy, permissions, and trust boundaries;
- licensing and redistribution constraints;
- external-service, provider, and platform dependence;
- extensibility and interoperability;
- migration and compatibility requirements;
- maintenance burden and operational complexity;
- project activity, governance, and ecosystem health;
- lock-in, reversibility, and exit cost;
- supported task classes and intended users; and
- default configuration versus optional or experimental capability.

The purpose is not to rank projects by popularity or accumulate metadata. It is
to avoid calling a mechanism adoptable when its surrounding constraints make it
unavailable, unsafe, unsustainable, or mismatched to Work Engine.

## 5.13 Capability correspondence

The workflow should distinguish progressively stronger claims:

```text
documented
  ↓
present in code or configuration
  ↓
reachable through an executable path
  ↓
enabled by default in the observed configuration
  ↓
demonstrated by tests
  ↓
observed at runtime
  ↓
supported by production or longitudinal evidence
```

These are evidence states, not a universal maturity ladder. A capability may be
intentionally optional, and a test may prove a narrower claim than runtime
observation. Record the strongest state actually supported, the configuration
under which it applies, and any counterevidence or correspondence gap.

---

# 6. Work Engine concepts we especially want to test for differentiation

The comparison should explicitly look for equivalents to:

## 6.1 Placement certificate

A semantic ownership proof connecting:

```text
trigger
→ producer
→ owned state
→ consumer
→ consequence
→ proof
→ insufficient substitute
```

## 6.2 Fresh falsification

A separate perspective explicitly asked to assume the selected boundary may be wrong and attempt to disprove it.

Fresh falsification is an evidence capability, not a mandatory per-repository stage.

First-pass compression should establish likely mechanisms and provenance. Fresh adversarial falsification should be reserved for claims whose correctness could materially change a comparison or adoption decision, such as:

- a mechanism that appears capable of replacing a Work Engine subsystem;
- a claimed differentiator that may actually exist elsewhere;
- a high-consequence KEEP / ADOPT / BORROW / AVOID classification;
- a disputed architectural interpretation.

This preserves the value of independence without doubling the cost of routine compression.

## 6.3 Vertical semantic proof

Validation must establish that the intended downstream consumer observes the intended consequence, not merely that a local producer behaves correctly.

## 6.4 Insufficient substitute

The workflow identifies in advance what would look like success while failing the actual objective.

This is a protection against reward-hacking/local-optimum completion.

## 6.5 Route revision with evidence preservation

A falsified premise invalidates only dependent decisions.

Valid evidence remains usable.

The system can replan without treating correction as failure.

## 6.6 Proportional evidence and validation

Evidence effort, review depth, and validation breadth scale with:

- consequence;
- uncertainty;
- blast radius;
- reversibility;
- detectability;
- durability;
- architecture reach.

## 6.7 Seam-oriented reasoning

Potentially important boundaries include:

- principles ↔ implementation;
- architecture ↔ code placement;
- UI ↔ underlying process;
- tests ↔ claimed behavior;
- docs ↔ implementation;
- evidence ↔ confidence;
- user action ↔ actual ownership;
- runtime state ↔ rendered state.

We want to know whether other systems reason explicitly about these seams.

---

# 7. Durable analysis artifacts and two-level compression

The workflow should use two levels of semantic compression:

```text
raw repository
  ↓
dimension-specific evidence artifacts
  ↓
claim reconciliation + relationship-aware repository profile
  ↓
cross-project comparison
```

Dimension-specific artifacts preserve the strongest findings from each semantic pass without requiring later synthesis to absorb the raw exploration context.

A repo-level reconciliation step then resolves duplicate claims, preserves
conflicts, connects cross-dimensional mechanisms, and produces one compact
repository profile. It must not flatten disagreement into false consensus.

This separation allows:

- independent views of mechanism and semantic strategy;
- correspondence analysis without rereading the entire repository;
- targeted supplementation of one dimension;
- migrating or selectively rerunning affected dimensions when the comparison
  ontology evolves;
- discarding raw exploration safely.

## 7.1 Repo review artifact

Each repository should ultimately produce a durable evidence-backed profile.

The artifact is the primary compression boundary.

It should contain enough information to support later synthesis without reopening the repository for routine questions.

It must also contain enough provenance to reopen the repository when a claim becomes decision-critical.

## 7.2 Suggested conceptual structure

```yaml
project:
  name:
  repository:
  commit:
  release_or_tag:
  branch:
  observed_at:
  languages:
  size:

comparison_contract:
  run_id:
  ontology_version:
  artifact_schema_version:
  required_core_dimensions:

system_boundary:
  included_components:
  external_dependencies:
  unavailable_evidence:
  relevant_configuration:
  intended_tasks_and_users:

architecture:
  summary:
  primary_control_model:
  major_components:

capabilities:
  - id:
    name:
    mechanism:
    enforcement: code | prompt | config | mixed
    correspondence:
      documented:
      present:
      reachable:
      default_enabled:
      test_demonstrated:
      runtime_observed:
      production_evidenced:
    applicability:
    confidence:
    evidence_ids: []
    counterevidence_ids: []
    relationships:
      depends_on: []
      enables: []
      constrains: []
      conflicts_with: []
      substitutes_for: []
      shares_state_with: []

judgment_architecture:
  route_selection:
  premise_revision:
  evidence_preservation:
  falsification:
  proportional_validation:
  semantic_proof:
  insufficient_substitute_protection:

context_model:
  persistent_context:
  disposable_context:
  compaction_or_rotation:
  retrieval_strategy:

review_model:
  independence:
  lifecycle:
  remediation:

mechanical_substrate:
  persistence:
  retries:
  process_lifecycle:
  worktrees:
  metrics:
  path_resolution:
  artifact_lifecycle:
  config_and_cli:

failure_behavior:
  provider_failure:
  validation_failure:
  conflicting_evidence:
  context_exhaustion:
  interruption:

operating_and_adoption_context:
  security_and_trust:
  licensing:
  platform_and_provider_dependencies:
  compatibility_and_migration:
  maintenance_and_governance:
  lock_in_and_reversibility:

claims:
  - id:
    subject_id:
    observation:
    interpretation:
    comparison_implication:
    state: supported | disputed | unresolved | superseded
    evidence_ids: []
    counterevidence_ids: []
    confidence:

evidence:
  - id:
    mode: source | indexed_structure | documentation | test | runtime | external
    path_or_locator:
    symbol_or_section:
    range:
    commit:
    configuration:
    supports:
    limitations:

unknowns:
  - question:
    why_it_matters:
    current_evidence:
    cost_to_resolve:

provenance:
  files_examined:
  evidence_queries:
  limitations:
  provider_and_coverage_state:
```

The exact schema can evolve.

The important requirement is consistent semantics across repositories.

The exact serialization can be normalized differently, but claim IDs,
mechanism IDs, evidence IDs, typed states, schema versions, and relationship
edges should survive semantic compression.

## 7.3 Separate descriptive profiles from decisions

Repository profiles should describe what was observed and inferred. They should
not embed a final Work Engine adoption verdict as if it were a property of the
external repository.

A separate decision artifact should reference profile claims and record:

```yaml
decision:
  classification: KEEP | ADOPT | BORROW | AVOID | INVESTIGATE
  target_work_engine_boundary:
  expected_value:
  architectural_fit:
  evidence_strength:
  integration_cost:
  migration_cost:
  maintenance_burden:
  security_privacy_and_license_constraints:
  reversibility_and_lock_in:
  alternatives_considered:
  assumptions:
  invalidation_triggers:
  unresolved_questions:
```

This separates four questions that should not be collapsed:

1. Does the mechanism exist?
2. How does it behave in its native system?
3. Is it better for the relevant purpose?
4. Is adopting it into Work Engine worthwhile and feasible?

---

# 8. Separate observation from interpretation

Every important conclusion should distinguish:

```text
observed mechanism
        ↓
architectural interpretation
        ↓
comparison implication
```

Each stage should have its own claim identity and evidence references where
needed. Supporting evidence, counterevidence, alternative interpretations, and
unresolved conflicts must remain distinguishable. A later interpretation may
be superseded without invalidating the original observation.

Example:

```text
Observed:
scheduler.go maps REVIEW_FAILED → IMPLEMENTING.

Interpretation:
The system supports bounded remediation rather than terminal failure.

Comparison implication:
This resembles Work Engine recovery behavior, but the route is encoded
as a fixed state-machine transition rather than selected by model judgment.
```

This preserves truth and makes later disagreement tractable.

Before a repository profile is finalized, reconciliation should check for:

- duplicate mechanisms described under different dimension vocabularies;
- mutually inconsistent claims from separate passes;
- claims whose evidence establishes only documentation or code presence while
  the interpretation assumes reachability or runtime behavior;
- relationship edges that cross dimension boundaries; and
- claims invalidated by a system-boundary or configuration mismatch.

Unresolved conflict is a legitimate repository-profile state.

---

# 9. Provenance requirements

Compression must not become irreversible summarization.

Important claims should retain:

- file path;
- symbol or section;
- line/range when available;
- reason the evidence supports the claim;
- commit identity.

Later synthesis should be able to say:

> “This looks important. Reopen the exact evidence.”

without repeating broad repository exploration.

---

# 10. Negative claims, uncertainty, and evidence stopping

Do not require exhaustive proof that a capability is absent during first-pass compression.

Prefer:

```yaml
falsification:
  observed_equivalent: false
  confidence: low
  exhaustive_search: false
```

rather than:

> “This repository definitely has no falsification mechanism.”

Exhaustive negative searches are expensive.

Pay for them only when the absence would materially change an adoption decision.

Unknowns are legitimate outputs.

Use closed, non-overlapping states rather than omission or free-form nulls:

```text
not_applicable
not_sought
not_observed
provisionally_absent
confirmed_absent
unknown
conflicting_evidence
```

`not_observed` means the analysis looked but did not establish the mechanism;
it does not mean the mechanism is absent. `not_sought` means the dimension was
outside the selected depth. `confirmed_absent` requires a bounded exhaustive
search, recorded scope, and no unresolved coverage limitations. If a limitation
remains, use a weaker state or close the gap with relevant direct evidence
before claiming `confirmed_absent`. Omission must not carry any of these meanings
implicitly.

The same principle applies to positive exploration: gather more evidence only while it has a credible chance of changing the durable repository description, the value of deeper analysis, or a consequential comparison decision.

The workflow should distinguish:

```text
more evidence could change the decision
→ continue

remaining evidence is mostly confirmatory or encyclopedic
→ stop and record uncertainty
```

This is a semantic stopping condition, not a fixed file-count, token-count, or turn-count rule.

---

# 11. Incremental durability and compaction safety

The previous Work Engine run showed that exact artifacts must not exist only in model context near the end of a large session.

Therefore repo compression should preserve durable findings before context pressure becomes dangerous.

The workflow should distinguish:

- disposable exploration;
- durable semantic findings.

Potential implementation approaches can vary, but the design requirement is:

> **A context compaction or worker termination must not destroy already-established claims and provenance.**

The final artifact should not depend on reconstructing exact state from a compacted model memory.

Context pressure may inform whether the worker checkpoints or continues, but should not become a rigid phase rule.

---

# 12. Metrics for the comparison workflow

Operational metrics should be derived from runtime logs where possible.

Per repository:

```text
builder input
cached input
output
reasoning
peak context
turns
wall clock
tool calls
tool-output volume
compactions
repository evidence calls
files/ranges inspected
review calls if any
cost where available
```

These metrics serve two purposes:

1. control comparison cost;
2. evaluate the repo-compression workflow itself.

Do not rely on model self-report for final operational metrics.

Operational efficiency is not evidence that the analysis is accurate. The
workflow should also measure, through sampled calibration rather than mandatory
duplication of every pass:

- agreement and meaningful disagreement between independent profiles of the
  same repository or dimension;
- unsupported-claim and unresolved-contradiction rates;
- whether cited evidence can be reopened and still supports the claim;
- correspondence gaps discovered by runtime or test evidence;
- decision reversals after targeted deep investigation;
- important mechanisms found only by open discovery;
- information lost during repo-level compression; and
- whether adopted recommendations produce the expected result in Work Engine.

Calibration should include a small set of anchor repositories or dimensions
reanalyzed under the same frozen contract. The purpose is not to force model
agreement. It is to expose ambiguous semantics, ontology blind spots, and
compression loss before they scale across the corpus.

---

# 13. Cross-repository synthesis

After enough reconciled repository profiles exist, a fresh synthesis agent
compares the profiles rather than reading every repository from scratch.

The synthesis stage is also the main place to identify which compressed claims deserve fresh falsification or targeted deep dives. Routine repository claims should not be independently re-reviewed merely because a review capability exists.

The synthesis agent receives:

- reconciled repository profiles and their comparison-run contract;
- the Work Engine comparison doctrine;
- Work Engine's profile produced under the same descriptive contract;
- profile compatibility and migration status;
- targeted source access only when necessary.

It should reject or quarantine profiles whose schema or ontology semantics are
not compatible with the active comparison run. It should reopen source when a
claim is decision-critical, disputed, dependent on unavailable external state,
or materially stronger than its recorded correspondence evidence.

Its job is to identify:

## KEEP

Work Engine mechanisms that appear differentiated or superior.

## ADOPT

Commodity infrastructure another project appears to solve better and that can reasonably replace ours.

## BORROW

Specific mechanisms worth extracting without adopting the entire system.

## AVOID

Approaches that reproduce failure modes we have already observed.

## UNKNOWN

Questions whose evidence is insufficient and whose resolution may be worth a targeted deep dive.

These classifications belong to decision artifacts, not to the descriptive
repository profiles. A mechanism may support different classifications for
different Work Engine boundaries or under different assumptions.

---

# 14. Feature harvesting

Feature harvesting should not merely produce a list of interesting ideas.

For each harvested feature/mechanism, preserve:

```text
what it does
why it matters
how it is implemented
where the evidence is
what assumptions it depends on
what other mechanisms enable or constrain it
whether it is reachable, default-enabled, tested, or runtime-observed
what it might replace or augment in Work Engine
integration cost/risk
security, license, provider, maintenance, and migration constraints
reversibility and invalidation triggers
whether it constrains model judgment
```

The output should be directly usable as roadmap input.

---

# 15. Introspection into Work Engine

The workflow should deliberately reflect findings back onto Work Engine.

For each comparison dimension, ask:

- Do we actually implement this better?
- Is our implementation more complicated than necessary?
- Is our claimed differentiator really unique?
- Are we carrying commodity infrastructure someone else solved better?
- Did another project expose a failure mode we have not considered?
- Does our flexible architecture remain genuinely flexible?
- Have our invariants accumulated into soft procedure by volume?
- Are mechanical concerns leaking into model instructions?

The goal is not to prove Work Engine is superior.

The goal is to learn what Work Engine should become.

---

# 16. Final comparison output

The final deliverable should be decision-oriented.

It should contain:

## 16.1 Architectural landscape

How the projects differ in:

- orchestration philosophy;
- state/persistence;
- review;
- context handling;
- validation;
- process substrate;
- observability;
- mechanism relationships and trade-offs; and
- operating and adoption context where decision-relevant.

## 16.2 Capability comparison

Evidence-backed overlap and differentiation.

## 16.3 Work Engine introspection

What the comparison reveals about our own architecture.

## 16.4 Adoption opportunities

Specific external mechanisms worth:

- adopting;
- borrowing;
- benchmarking;
- studying further.

## 16.5 Differentiated Work Engine concepts

Mechanisms that appear uncommon and worth preserving/testing.

## 16.6 Risks

External designs likely to recreate known failures:

- procedural rigidity;
- context overload;
- self-reported telemetry;
- exact artifacts held only in model memory;
- mandatory review ritual;
- poor executable affordances;
- state-machine constraints around semantic judgment.

## 16.7 Targeted unknowns

Questions worth additional expensive investigation.

## 16.8 Roadmap implications

Concrete recommendations classified by:

```text
KEEP
ADOPT
BORROW
AVOID
INVESTIGATE
```

---

# 17. Success criteria

The workflow succeeds if:

- every profile identifies the frozen comparison-run contract under which it
  was produced;
- repositories are pre-indexed once and later passes reuse the same evidence substrate;
- no synthesis agent needs all repositories in raw context;
- mechanism and semantic-strategy claims remain distinguishable;
- mechanisms retain important dependency, constraint, substitution, and shared-state relationships;
- correspondence between model-facing strategy and implementation can be evaluated explicitly;
- documentation, code presence, reachability, default enablement, tests, and runtime evidence are not conflated;
- each important claim has usable provenance;
- comparison artifacts are consistent enough to synthesize;
- incompatible schema or ontology versions are migrated, selectively rerun, or quarantined rather than silently compared;
- absence, inapplicability, unsearched state, uncertainty, and conflicting evidence remain distinguishable;
- negative claims are not over-proven by default;
- high-cost reasoning is reserved for consequential uncertainty;
- context compaction cannot destroy durable analysis;
- operational cost is measurable;
- sampled calibration measures evidence fidelity, contradiction, compression loss, and decision reversal;
- open discovery can add emergent mechanisms without corrupting the frozen core comparison semantics;
- the final report reveals both external opportunities and weaknesses in Work Engine;
- useful external infrastructure can be separated from Work Engine's judgment architecture;
- descriptive repository profiles remain separate from Work Engine adoption decisions;
- external services, configuration, licensing, security, migration, and maintenance constraints are represented when they affect a decision;
- the comparison does not accidentally turn Work Engine back into a procedural system.

The ultimate success criterion is:

> **We can confidently decide what to keep, what to replace, what to borrow, and what to test—without paying to repeatedly understand the same repositories.**

A healthy workflow should also show evidence of judgment rather than ritual: different repositories should naturally produce different analysis depth, different unresolved questions, and different use of expensive falsification according to their decision value.

Before scaling to a large corpus, run a pilot across a small heterogeneous set:

1. freeze the first comparison-run contract;
2. profile Work Engine and two or three external repositories;
3. independently repeat at least one repository or consequential dimension;
4. reconcile claims and inspect relationship preservation;
5. reopen a sample of evidence references;
6. perform one decision-oriented synthesis; and
7. record which schema changes require migration or re-analysis.

The pilot succeeds when the artifacts are not merely complete, but comparable:
independent readers can tell what was observed, how strongly it corresponds to
real behavior, how mechanisms interact, what remains unresolved, and why a
later decision follows from the evidence.
