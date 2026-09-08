## Addendum: Execution-Profile Scoring and Strategy Selection

### Status

Design hypothesis subordinate to the Structural Plan IR pilot.

This addendum does not establish a permanent task taxonomy, scoring formula, model-routing policy, Plan IR resolution mapping, or executor admission rule. Its purpose is to identify the minimum additional structure needed to test whether different implementation tasks benefit from different combinations of planning resolution, executor capability, and verification strategy.

### Motivation

If Plan IR can be projected at different levels of explicitness for different executor capability profiles, then no single planning and implementation strategy should be assumed optimal for all engineering work.

A highly capable builder may execute successfully from a compact projection that preserves substantial local judgment. A lower-capability builder may require a more explicit projection of dependencies, invariants, judgment boundaries, acceptance relationships, and execution order.

However, increasing plan resolution also consumes upstream inference. Beyond some point, a high-capability planner may perform enough implementation reasoning that using a cheaper builder no longer reduces total accepted-work cost.

The relevant question is therefore not:

> Which model should implement this type of task?

It is:

> What remaining judgment surface does this implementation expose, what structural resolution is required to constrain it adequately, and which demonstrated execution strategy can realize it at the lowest total accepted-work cost?

### Execution profile

A compiled plan should be capable of producing a compact execution profile describing properties that are expected to affect executor difficulty.

Initial candidate dimensions include:

- **semantic novelty** — whether the work follows an established semantic pattern or introduces materially new behavior or architecture;
- **repository breadth** — whether implementation is localized or spans multiple components and ownership boundaries;
- **decision closure** — how much route-variant semantic choice remains unresolved or delegated downstream;
- **implementation discretion** — how much legitimate local judgment remains after planning;
- **dependency complexity** — the breadth and depth of ordering, coordination, and cross-component dependencies;
- **oracle strength** — how objectively implementation correctness can be demonstrated through tests or other observable evidence;
- **reversibility** — the cost and consequence of an incorrect implementation or rollback;
- **repository discovery burden** — how much environmental understanding the executor must acquire beyond the compiled plan;
- **integration consequence** — whether the work affects shared interfaces, migrations, concurrency, persistence, recovery, or other cross-boundary semantics.

These dimensions are hypotheses, not a closed ontology.

The pilot may show that some are redundant, poorly observable, or weak predictors, and may reveal stronger dimensions not listed here.

### Scores describe work, not models

Execution-profile values should describe properties of the implementation contract and its bound repository state.

They should not encode permanent assumptions such as:

```text
Luna = low capability
Sol = high capability
```

Model capability profiles should instead be established from observed execution outcomes and may change across model versions, harnesses, tools, context projections, and task classes.

The same execution profile may therefore map to different strategies as empirical capability evidence changes.

### Strategy selection

A strategy is a combination of at least:

```text
planning / compilation effort
+
Plan IR projection resolution
+
executor capability profile
+
execution autonomy
+
verification / review strength
```

Candidate strategies might include:

```text
compact structural plan
→ high-capability builder

moderately elaborated structural plan
→ lower-capability builder

high-resolution constrained projection
→ lower-capability builder
→ strong deterministic verification

exploratory or high-ambiguity plan
→ high-capability builder
→ broader implementation judgment
```

These are examples rather than predefined routing classes.

### Scoring should initially inform judgment, not replace it

The first scoring mechanism should expose dimensions rather than collapse them immediately into one scalar number.

For example:

```yaml
execution_profile:
  semantic_novelty: low
  repository_breadth: medium
  decision_closure: high
  implementation_discretion: low
  dependency_complexity: medium
  oracle_strength: high
  reversibility: high
  discovery_burden: low
```

A single aggregate score may eventually prove useful, but premature aggregation risks hiding materially different task shapes that happen to produce the same numeric result.

Historical evidence may show, for example, that weak oracle strength is much more consequential than repository breadth for a lower-capability executor, or that semantic novelty interacts strongly with implementation discretion.

The initial system should therefore preserve the component dimensions and allow strategy selection to remain an evidence-backed judgment.

### Economic objective

Strategy selection should optimize total accepted-work consequence rather than builder token price.

For a candidate strategy:

```text
total cost =
    planning inference
  + Plan IR formation
  + projection / elaboration
  + builder inference
  + supervision
  + verification and review
  + retries
  + repair
  + latency
  + failure consequence
```

A higher-resolution projection is useful only when its marginal planning and elaboration cost is outweighed by reduced downstream inference, repair, supervision, or failure risk at equivalent accepted quality.

This creates an expected resolution curve:

```text
insufficient resolution
    → executor reconstructs too much
    → error / repair / supervision rises

sufficient resolution
    → consequential structure is explicit
    → executor retains appropriate local judgment
    → total cost reaches a useful region

excessive resolution
    → planner begins performing implementation work
    → upstream inference dominates
    → cheaper execution loses economic value
```

The desired resolution is therefore not maximal detail.

It is the minimum structural resolution that makes the selected executor reliably capable of the remaining work.

### Relationship to Plan IR

The execution profile does not become another semantic owner.

Plan IR continues to represent the implementation contract.

The execution profile is a derived characterization of properties relevant to execution difficulty and strategy selection.

Likewise, target-specific projections may elaborate Plan IR meaning but must not change the underlying contract merely to make an executor appear admissible.

Conceptually:

```text
canonical Plan IR
      |
      +----> execution-profile derivation
      |
      +----> candidate projection P1
      |
      +----> candidate projection P2
      |
      +----> candidate projection P3
```

Historical execution evidence can then associate combinations of execution profile, projection resolution, and executor capability with observed outcomes.

### Pilot use

The first Plan IR pilot should record the candidate execution-profile dimensions but should not use them as an automatic routing gate.

For each experimental slice, record:

- the execution profile before implementation;
- Plan IR resolution supplied;
- executor and capability profile;
- planner and projection token cost;
- executor reasoning and output cost;
- supervisor and reviewer cost;
- repair turns;
- plan deviations;
- unauthorized or unnecessary judgment;
- first-pass acceptance;
- semantic and mechanical defects;
- wall time; and
- final accepted-work cost.

After enough observations exist, evaluate:

1. which execution-profile dimensions correlate with required plan resolution;
2. which dimensions predict executor success or failure;
3. whether interactions among dimensions matter more than individual scores;
4. whether useful task clusters emerge naturally;
5. whether a scalar difficulty score preserves enough information to be useful;
6. where increased Plan IR resolution stops producing economic leverage; and
7. which strategies dominate for identifiable regions of the execution-profile space.

### Initial success condition

The hypothesis becomes useful if historical evidence supports at least two materially different execution strategies whose preferred regions can be distinguished using observable properties of the compiled work.

For example, evidence might eventually support:

```text
bounded, strongly observable, decision-closed work
    → elaborated Plan IR
    → lower-cost executor

semantically novel, weakly observable, judgment-heavy work
    → compact Plan IR
    → high-capability executor
```

The exact categories, boundaries, and executor assignments must emerge from measured evidence rather than being encoded in advance.

### Governing principle

> **Characterize the remaining judgment surface, select sufficient plan resolution, and route to the least costly strategy demonstrated capable of realizing the accepted consequence.**

