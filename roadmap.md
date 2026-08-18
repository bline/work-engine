# Work Engine Optimization Roadmap

## Purpose

Evolve Work Engine from a model-heavy repository reconnaissance workflow into an evidence-driven engineering engine that minimizes durable model context without weakening architectural correctness, semantic verification, or independent review.

Efficiency means all of the following:

- lower model input and output token consumption;
- less irrelevant context retained by the persistent builder;
- correct architectural placement, not merely compiling code;
- an executable downstream semantic proof for each slice;
- independent falsification of important architectural assumptions when consequence or uncertainty warrants it;
- deterministic validation wherever model judgment is unnecessary;
- reproducible evidence and durable metrics;
- no hidden weakening of acceptance criteria in exchange for lower cost.

The central rule is:

> Information crosses a model boundary only when its expected lifetime justifies crossing that boundary.

The long-term direction is to make Codebase-Memory the primary repository-intelligence substrate while retaining model reasoning only for ambiguity, architectural judgment, implementation, and adversarial semantic review.

---

## Current architecture

The existing workflow has three useful information-lifetime boundaries and should preserve them unless measurement disproves their value:

1. **Supervisor** — tiny, durable campaign state and continuation decisions.
2. **Persistent slice builder** — durable understanding, placement judgment, implementation, and final acceptance for one slice.
3. **Disposable reconnaissance/review processes** — high-volume evidence gathering, falsification, test diagnosis, and adversarial review that should not pollute builder context.

The high-assurance placement flow is intentional for ambiguous, cross-boundary, or consequential work:

1. scout multiple plausible placement boundaries;
2. Codex selects a provisional placement;
3. encode that selection as a placement certificate;
4. use a fresh process to attempt to falsify the certificate;
5. implement only after placement is confirmed;
6. require a vertical semantic proof that reaches the intended consumer/consequence.

For an obvious, local, reversible boundary with a known producer, consumer, and focused proof, a direct targeted-evidence route is sufficient. Escalate to the high-assurance flow when evidence reveals competing ownership, hidden consumers, lifecycle conflict, broad consequence, or medium/high placement risk. Do not collapse important placement judgments into one model recommendation: independence between proposal, owner judgment, and falsification remains part of the correctness model when those conditions apply.

## Implementation status after the placement/efficiency slice

The current Work Engine already implements much of the pre-graph optimization foundation:

- the builder selects a direct or falsified-placement route from observed placement risk;
- placement alternatives are a distinct shallow reconnaissance step on the falsified-placement route;
- Codex owns the provisional placement decision and placement certificate;
- targeted evidence confirms, conflicts with, or leaves that certificate unresolved, with a fresh falsifier when independence is warranted;
- placement and targeted-recon outputs have explicit cardinality limits;
- ordinary validation runs through a deterministic `run_gate.py` manifest rather than Claude;
- Claude is reserved for bounded reconnaissance, useful failure diagnosis, and fresh adversarial semantic review when configured or risk-justified;
- `audit_receipt` and compact 300–800-token `handoff_receipt` are separate;
- schema-version-3 audit receipts persist placement certificate/verdict/risk and rejected alternatives;
- the builder, evidence skill, supervisor, receipt schema, and gate runner have explicit contract ownership boundaries.

Accordingly, Phases 1–4 below are retained as architectural requirements and regression criteria, not as greenfield implementation work. Phase 0 has useful historical baseline data but should continue accumulating comparable post-optimization measurements. The next new implementation work begins at Phase 5.

---

# Phase 0 — Establish the optimization baseline

**Status: active measurement.** Historical filesystem/Claude metrics exist; continue collecting comparable post-optimization slices so graph experiments have a stable control.

Before changing the retrieval substrate, make the existing workflow measurable enough that later experiments answer one question at a time.

## 0.1 Record a stable baseline

Capture several representative slices using the current Claude filesystem-based reconnaissance path. Include at least:

- one obvious/local placement;
- one cross-module placement;
- one state-ownership or persistence change;
- one slice with a meaningful runtime consequence;
- one slice that causes supplemental reconnaissance or placement reconsideration if available.

Record per slice:

- placement candidate count;
- placement risk;
- placement conflicts and reconsiderations;
- targeted recon calls;
- supplemental recon calls;
- provider input/cache/output/thinking tokens when available;
- provider cost when available;
- files, ranges, and approximate lines retrieved;
- recon output tokens;
- context passed into the builder;
- builder model tokens when available;
- local repository exploration performed by the builder outside supplied evidence;
- vertical semantic proof result;
- focused-test result;
- full-suite result;
- adversarial-review findings by severity;
- post-review repair count;
- late semantic rejection or boundary invalidation;
- wall-clock duration.

## 0.2 Add derived efficiency metrics

Calculate at least:

- provider output tokens per retrieved source line;
- recon output tokens per accepted slice;
- builder-context tokens introduced by reconnaissance;
- supplemental calls per slice;
- total model tokens per accepted semantic change;
- rework tokens after adversarial review;
- placement failure rate;
- late semantic rejection rate.

Add an **information amplification ratio** when the necessary instrumentation exists:

`recon model output tokens / evidence tokens actually retained into builder context`

This exposes verbose interpretation that does not survive the handoff.

## Exit criterion

A repeatable baseline exists and later provider experiments can be compared without changing the success definition.

---

# Phase 1 — Remove model work that is deterministic

**Status: implemented; preserve as a regression requirement.** The builder now runs an ordered deterministic manifest through `scripts/run_gate.py`, with model use reserved for bounded diagnosis and semantic review.

This phase should happen before comparing retrieval providers so model-selection experiments are not contaminated by avoidable process overhead.

## 1.1 Add a deterministic gate runner

Move command execution out of Claude.

The gate runner should execute and return a compact structured result for:

1. vertical semantic test;
2. changed-file/boundary checks that can be expressed mechanically;
3. `git diff --check`;
4. required prechecks;
5. check-only freshness commands;
6. focused tests;
7. full suite.

The gate result should contain only:

- command identity;
- exit status;
- duration;
- pass/fail;
- small failure excerpts;
- failing test names/files where available.

Never send successful raw command output to a model.

A model is invoked only when:

- a failure requires diagnosis;
- an architectural proof cannot be established mechanically; or
- adversarial semantic review is required.

Default validation order:

`vertical proof → focused deterministic checks → deterministic full suite → semantic adversarial review → fixes → affected deterministic checks → final full suite`

Allow configuration to move semantic review before the first full suite when the suite is unusually slow. This is a latency optimization, not a token/correctness waiver.

## 1.2 Separate failure diagnosis from test supervision

When deterministic validation fails, supply a fresh diagnostic model only:

- the failing command;
- minimal failure output;
- task-owned changed files;
- relevant placement certificate;
- graph/source evidence required to diagnose the failing path.

Do not ask the model to rerun successful checks or supervise the full gate.

## Exit criterion

No LLM is required merely to execute a known validation command or report a successful result.

---

# Phase 2 — Shrink model-boundary contracts

**Status: implemented in the current Claude evidence adapter; benchmark and tighten further rather than redesigning it.** The builder selects a direct or falsified-placement route. On the latter, placement and targeted reconnaissance have separate bounded schemas and hard cardinality limits, while Codex owns the final architectural plan.

## 2.1 Slim placement reconnaissance

When placement alternatives are warranted, the first round remains exploratory but must not become an architecture report.

Return only information needed for Codex to choose responsibly:

- up to 3 plausible placement candidates;
- smallest discriminating facts;
- evidence for and against each candidate;
- unresolved preconditions;
- minimal canonical/ownership ranges if raw source is necessary;
- placement risk;
- available call statistics.

Do not return:

- a complete implementation plan;
- exact changed-file manifests;
- broad call graphs;
- acceptance-test design;
- detailed invariants beyond what distinguishes ownership;
- a singular recommendation that obscures uncertainty.

## 2.2 Make the placement certificate the primary durable artifact

Codex owns and records:

> When `<trigger>` occurs, `<producer>` writes `<state>` owned by `<boundary>`. `<consumer>` reads it and produces `<semantic outcome>`. `<downstream proof>` proves the consequence. This is not satisfied by `<plausible but insufficient substitute>`.

The certificate is the compact semantic path that survives into implementation, validation, handoff, and later architectural memory.

Record applicable fields:

- selected candidate or directly supported boundary;
- rejected alternatives when alternatives were plausible;
- evidence for rejection;
- unresolved preconditions;
- confidence;
- later confirmation, conflict, or supersession.

## 2.3 Brutally slim targeted falsification/recon

On the falsified-placement route, the second fresh process assumes the selected certificate may be wrong. On the direct route, targeted evidence must still expose any competing owner, hidden consumer, or lifecycle conflict that would require escalation.

Its normal response contract should be bounded approximately as follows:

- one `placement_verdict`;
- failed certificate clause when applicable;
- up to 8 verified facts;
- up to 12 exact source ranges only when source is required;
- up to 5 relevant wiring/path facts;
- exact commands only when the deterministic runner cannot already derive them;
- up to 3 blockers/unknowns;
- concise confidence/risk.

Do **not** ask this process to derive the final implementation plan, full invariant catalog, changed-file list, acceptance test suite, vocabulary catalog, or broad deferred scope. The builder owns those conclusions.

## 2.4 Put cardinality limits in schemas

Prefer structural limits over prose such as “be concise.” Limits should be explicit, validated, and recorded when exceeded.

## Exit criterion

Recon packets contain evidence and uncertainty, while durable architectural reasoning remains in the builder.

---

# Phase 3 — Split audit persistence from builder handoff

**Status: implemented.** The builder returns separate `audit_receipt` and `handoff_receipt` views; the handoff explicitly excludes provider/model/gate accounting and targets 300–800 tokens.

## 3.1 Keep the full audit receipt

The durable audit record may continue to contain:

- run and slice identity;
- model/provider configuration;
- token/cost statistics;
- validation bookkeeping;
- timestamps;
- commands/results;
- provenance;
- acceptance status;
- detailed metrics.

This record exists for auditability and optimization analysis, not as next-slice context.

## 3.2 Introduce a compact handoff receipt

Target roughly 300–800 tokens under normal conditions.

The next builder receives only durable consequences such as:

- accepted placement certificate;
- architectural decisions introduced or superseded;
- new/changed invariants;
- durable state/ownership consequences;
- public/runtime contracts changed;
- proof/test relationships worth preserving;
- unresolved concerns;
- explicitly deferred work.

Provider accounting, test transcripts, per-command bookkeeping, and temporary evidence do not cross this boundary.

## Exit criterion

A future slice can recover the durable architectural result without reading a full audit receipt or prior conversation.

---

# Phase 4 — Establish canonical contract ownership

**Status: substantially implemented.** Current skills establish ownership across the evidence adapter, builder/handoff, supervisor/audit schema, and deterministic gate. Future provider work should extend these contracts without restating them.

Avoid independently restating complete contracts across installable skills.

Use ownership rather than a single physically shared file:

- reconnaissance skill owns provider input/output schemas;
- builder owns placement certificate and handoff contracts;
- supervisor owns lifecycle, limits, and acceptance orchestration;
- receipt schema owns durable audit persistence;
- deterministic gate runner owns executable validation result schema.

Other skills reference the owning contract and restate only the minimum semantics required for safe standalone installation.

Add compatibility/version fields where an independently installed component crosses a contract boundary.

## Exit criterion

Each semantic contract has one authoritative owner and drift can be detected without destroying skill portability.

---

# Phase 5 — Add a reconnaissance-provider abstraction

**Status: implemented.** Static provider identity is
configured independently from the concrete evidence-skill adapter. The builder
resolves their consistency deterministically. `claude-filesystem` and the
graph-first `claude-codebase-memory` path are available. The latter retains
filesystem access during migration and can remove it once the migrated boundary
no longer requires that fallback. Other graph-backed providers and `auto` remain
explicitly unavailable until later phases implement them.

Do not hard-code “Claude” or “Codebase-Memory” as the architecture.

Introduce a provider abstraction behind the evidence/recon contract.

Possible configured modes:

```yaml
builder:
  context:
    reconnaissance:
      provider: claude-filesystem
```

```yaml
builder:
  context:
    reconnaissance:
      provider: claude-codebase-memory
```

```yaml
builder:
  context:
    reconnaissance:
      provider: codex-codebase-memory
```

Later:

```yaml
builder:
  context:
    reconnaissance:
      provider: auto
```

The builder must receive the same semantic contract regardless of provider.

Provider-specific telemetry stays namespaced in the audit receipt.

## Exit criterion

Changing the repository-evidence mechanism requires configuration, not changes to builder semantics or supervisor lifecycle.

---

# Phase 6 — Prove an ESM migration through one vertical runtime slice

**Status: implementation complete; final gate pending.** The canonical authored
modules are `property-profile.mjs`, `correction-promotion.mjs`, and
`sidebar.mjs`. The sidebar imports correction promotion directly, which imports
property profile directly. A generated `property-profile.js` compatibility
artifact temporarily serves the unmigrated classic semantic-toolbox consumer;
its build script owns both generation and freshness checking, so it is not a
parallel authored implementation. Focused scoring, compatibility, Semantic
Library, and sidebar tests pass. Real-extension interaction, the broader suite,
freshness gates, adversarial review, and post-change import-aware graph evidence
remain unverified until the final gate runs.

The extension currently uses UMD wrappers and `globalThis` APIs primarily as a
scaffolding inheritance, not as a desired architectural constraint. Before
using call-graph quality to judge Codebase-Memory, migrate one complete runtime
path to native ES modules and measure both runtime behavior and graph evidence.

Use the correction-promotion path as the first bounded slice:

`property profile → correction promotion → sidebar consumer → browser interaction`

## Required behavior

1. expose property-profile behavior through explicit named ESM exports;
2. import those exports directly in correction promotion;
3. expose correction-promotion operations through explicit named exports;
4. import those operations directly in the sidebar entry module;
5. load the sidebar entry with module semantics rather than depending on
   first-party script-tag ordering for the migrated path;
6. update Node tests to consume the ESM boundary without duplicating production
   implementations or maintaining parallel CommonJS logic;
7. keep any temporary compatibility bridge small, one-way, visibly marked, and
   owned by a later removal slice.

Do not count a producer-only conversion as completion. The slice is vertical
only when the browser consumer no longer reaches the migrated behavior through
`globalThis`.

## Runtime and graph proof

Prove all of the following independently:

- the real extension sidebar loads and the correction-promotion interaction
  still reaches its persisted semantic-library consequence;
- focused tests cover named imports, revision conflicts, and successful saves;
- the broader extension suite remains green;
- Codebase-Memory resolves the selected cross-file calls as `CALLS` edges using
  import-aware evidence rather than weak same-name matching;
- unresolved dynamic service calls remain represented as unresolved/dynamic
  boundaries rather than being claimed as statically proven calls.

Graph improvement is evidence about static correspondence, not proof of browser
behavior. Browser execution and tests remain separate acceptance evidence.

## Exit criterion

One complete production path uses native ESM from producer through sidebar
consumer, has no first-party global API dependency within that path, passes
runtime and test verification, and produces precise import-backed graph edges.

---

# Phase 7 — Migrate the remaining first-party extension graph to ESM

Continue from the proven vertical slice in dependency order. Treat each module
cluster as a separately reviewable ownership decision rather than converting
the entire extension in one flag day.

## 7.1 Inventory real runtime boundaries

Record, from source and manifest evidence:

- browser entry points and their execution environments;
- first-party classic-script ordering dependencies;
- `globalThis.Site2Json*` producers and consumers;
- CommonJS-only test consumers;
- service-worker, content-script, injected-page, and extension-page boundaries;
- vendor scripts that should remain external/classic rather than being rewritten.

Keep observed runtime wiring distinct from inferred migration order.

## 7.2 Migrate by coherent dependency cluster

For each cluster:

1. select the smallest complete producer-to-consumer path;
2. replace first-party global lookup with named or namespace imports;
3. preserve runtime ownership and initialization timing explicitly;
4. update tests and entry-point declarations in the same slice;
5. visually and behaviorally inspect affected extension surfaces;
6. reindex and verify representative import/call edges;
7. remove the compatibility bridge as soon as its final consumer migrates.

Do not create a permanent ESM implementation plus CommonJS implementation pair.
Tests should import production behavior through the same authored module
boundary, even when the test runner needs dynamic `import()` or a scoped module
configuration.

## 7.3 Preserve truthful mixed-mode boundaries

During migration, document which clusters are ESM, which remain classic, and
why. A classic vendor asset, page-injected script, or platform-constrained entry
point is not technical debt merely because its execution model differs. A
first-party compatibility global with no remaining consumer is debt and should
fail a freshness check or boundary test.

## 7.4 Completion checks

- first-party extension modules no longer use UMD solely for dual browser/Node
  loading;
- migrated dependencies are visible as explicit imports;
- entry-point module settings match the actual Chrome execution environment;
- accessibility, keyboard behavior, light/dark themes, narrow/wide panels, and
  maximized layouts remain coherent where a migrated cluster affects UI;
- focused tests, the full suite, generated-asset checks, and real-extension
  smoke tests pass;
- documentation and developer vocabulary describe the resulting module model;
- Codebase-Memory edge checks are recorded as static-analysis evidence with
  confidence/strategy, not overstated as complete runtime proof.

## Exit criterion

The remaining first-party extension runtime uses explicit ESM boundaries except
where a documented platform boundary requires another execution model; obsolete
globals and compatibility bridges are removed, and graph-first experiments no
longer begin from a known UMD/global alias-resolution handicap.

---

# Phase 8 — Codebase-Memory experiment A: change retrieval, not model

This is the cleanest first graph experiment.

Keep constant:

- Claude model;
- placement contract;
- targeted recon contract;
- Codex builder model/effort;
- deterministic validation;
- adversarial review requirements;
- representative slice mix.

Change only:

`Claude + Grep/Glob/Read` → `Claude + Codebase-Memory graph-first retrieval`

Use the installed Codebase-Memory Claude integration and scout agent where appropriate.

## Required behavior

Claude should:

1. use Codebase-Memory as the primary repository navigation and structural evidence source;
2. avoid reconstructing broad repository architecture in prose;
3. query only until the requested placement/falsification questions are resolved;
4. read raw source only when graph evidence is insufficient or exact behavior must be verified;
5. return only evidence that deserves to cross into builder context.

Source remains ground truth. The graph is an evidence/navigation substrate, not an unquestionable authority.

## Instrument hidden context

Measure separately when possible:

- automatic Codebase-Memory context injected at session/subagent start;
- graph-query result tokens;
- raw source-read tokens;
- final recon output tokens;
- builder-bound packet tokens.

Do not call an approach efficient merely because injected or cached context is cheaper to bill. Context-window occupancy and reasoning pollution also matter.

## Acceptance comparison

Compare against Phase 0 baseline on:

- placement conflicts;
- late placement invalidation;
- supplemental recon frequency;
- adversarial-review findings;
- post-review repairs;
- builder-context growth;
- total model tokens;
- Claude output tokens;
- wall-clock duration;
- source ranges read;
- builder local exploration.

## Decision rule

Adopt graph-first Claude recon only if token/context savings do not materially worsen placement quality, semantic-review findings, or late rework.

---

# Phase 9 — Codebase-Memory experiment B: preserve recon as an airlock

Test the hypothesis that disposable graph exploration protects persistent builder context.

Compare:

### A. Disposable recon agent + graph

`slice → disposable Codebase-Memory scout/recon → bounded evidence packet → persistent builder`

### B. Persistent builder + graph directly

`slice → persistent builder explores Codebase-Memory → implementation`

Use equivalent slices and validation.

## Measure especially

- builder-context growth;
- number of exploratory graph results entering builder context;
- abandoned query paths;
- builder compaction frequency;
- total slice tokens;
- implementation/rework quality;
- late semantic review findings.

## Working hypothesis

Exploration remains disposable; known-question retrieval may happen directly in the builder.

The intended rule is:

> **Exploration → disposable context. Known-question retrieval → builder context.**

Do not adopt the rule by assumption; validate it empirically.

---

# Phase 10 — Hybrid graph workflow

If Experiment B supports the airlock model, make it the default for ambiguous, cross-boundary, or consequential placement. Preserve direct targeted retrieval for obvious local boundaries.

## 10.1 Primary placement scout

Use a disposable Codebase-Memory-enabled scout to find up to three plausible boundaries and the facts that discriminate them.

Codex remains the placement owner and selects the provisional candidate.

## 10.2 Fresh placement falsifier

Use a new process, graph-first, that receives:

- objective and work source;
- placement candidates;
- provisional placement certificate;
- rejected alternatives and reasons;
- relevant repository instructions.

It assumes the certificate may be wrong and attempts to falsify only the selected boundary.

Freshness remains mandatory on this falsification route because epistemic independence is the reason for selecting it.

## 10.3 Builder microscope

Allow the persistent builder to query Codebase-Memory directly only for narrow, already-known questions exposed during implementation, for example:

- exact consumer path for a known state;
- callers of a known symbol;
- tests covering a known persistence path;
- ownership neighborhood of a known module;
- impact of changing a known symbol.

Broad exploratory neighborhood walking belongs in disposable recon.

## 10.4 Supplemental retrieval policy

For a missing fact during implementation:

1. attempt deterministic/local graph retrieval when the question is narrow;
2. if the result is sufficient, return only the minimal evidence to the builder;
3. use recon-session continuation only when the missing fact heavily depends on the existing packet and doing so is explicit;
4. use a fresh process when independence matters;
5. escalate to a stronger model only when evidence remains ambiguous.

## Exit criterion

Raw repository reading is no longer the default navigation strategy for either placement or targeted recon.

---

# Phase 11 — Benchmark model choice independently of retrieval choice

Only after the graph-first workflow is stable, compare model choices.

Candidate placements/recon backends may include:

- current Claude model + Codebase-Memory;
- cheaper Claude model + Codebase-Memory;
- Codex subagent + Codebase-Memory;
- other configurable models that satisfy the evidence contract.

Do not optimize only for provider price.

Compare:

- correct placement rate;
- disagreement with builder placement;
- falsifier conflict rate;
- late semantic rejection;
- supplemental retrieval;
- adversarial-review findings;
- post-review repairs;
- total tokens and cost;
- wall-clock duration.

A cheaper placement model is accepted only if the overall accepted-slice cost and semantic quality improve.

## Exit criterion

Model selection becomes evidence-based and configurable rather than embedded in workflow semantics.

---

# Phase 12 — Adaptive escalation

After provider/model baselines exist, introduce an `auto` mode.

Resolve each uncertainty with the cheapest mechanism that can establish it reliably:

`deterministic graph query → inexpensive graph-enabled model → strong model → independent reviewer → human judgment`

Possible escalation triggers:

- graph returns no path for a required placement-certificate clause;
- multiple viable owners survive discrimination;
- evidence conflicts across graph/source/canonical documentation;
- placement confidence or candidate margin falls below a configured threshold;
- ownership requires semantic interpretation rather than structural discovery;
- falsifier identifies a failed certificate clause;
- existing accepted architectural decision conflicts with current repository structure.

Do not allow a cheap model to silently resolve ambiguity by inventing certainty.

## Exit criterion

Strong-model tokens are spent primarily on real ambiguity and adversarial reasoning, not routine repository navigation.

---

# Phase 13 — Move repository context to Codebase-Memory as the default substrate

This is the intended migration if prior experiments succeed.

## Default architecture

```text
campaign supervisor
        │
        ▼
persistent slice builder
        │
        ├── placement decision / certificate ownership
        │
        ├── implementation
        │
        └── narrow graph queries for known questions

slice intent
        │
        ▼
disposable placement scout
        │
        ▼
Codebase-Memory
        │
        ▼
bounded candidate/evidence packet

provisional certificate
        │
        ▼
fresh graph-enabled falsifier
        │
        ▼
confirm / conflict / unresolved

implementation
        │
        ▼
deterministic gate runner
        │
        ▼
fresh independent semantic reviewer
        │
        ▼
findings / counterexamples
```

## Default evidence policy

- Codebase-Memory is primary for repository topology, callers/callees, symbol relationships, impact, and structural navigation.
- Raw source reads are targeted verification, not default exploration.
- Disposable recon absorbs broad graph exploration.
- Builder direct graph use is narrow and purpose-known.
- Claude or another strong model remains available for placement falsification and semantic adversarial review.
- Provider choice remains configurable so the filesystem path can be retained as fallback and benchmark control.

## Migration safety

Do not delete the raw filesystem recon path immediately. Keep it as:

- fallback when Codebase-Memory indexing is unavailable/stale;
- control provider for regression experiments;
- escalation path when graph coverage is demonstrably insufficient.

Record whether a slice used graph-only, graph-plus-source, or raw-filesystem recon.

## Exit criterion

The routine workflow no longer asks a model to rediscover repository structure by reading broad source ranges when equivalent structural evidence already exists in Codebase-Memory.

---

# Phase 14 — Semantic architectural memory overlay

Do this only after the Codebase-Memory substrate has proven useful. Do not rebuild low-level code graph functionality already supplied by Codebase-Memory.

The Work Engine-owned layer should capture **verified semantic knowledge**, not duplicate AST/call/import indexing.

Begin with the highest-value durable objects:

- placement certificates;
- accepted architectural decisions;
- rejected/insufficient substitutes;
- invariants;
- state ownership;
- durable producer/consumer consequences;
- proof/test relationships;
- superseded decisions.

Possible semantic node types:

- `Concept`
- `Responsibility`
- `State`
- `Boundary`
- `Invariant`
- `Decision`
- `Proof`
- `Test`

Possible semantic edge types:

- `OWNS`
- `PRODUCES`
- `CONSUMES`
- `AFFECTS`
- `VERIFIED_BY`
- `MUST_NOT_DEPEND_ON`
- `REJECTED_SUBSTITUTE`
- `SUPERSEDES`
- `IMPLEMENTED_BY`

Every semantic edge must retain provenance and epistemic status. Distinguish:

- deterministic fact;
- accepted architectural decision;
- verified inference;
- active hypothesis;
- stale/superseded assertion.

Models may propose semantic assertions but should not silently promote hypotheses into authoritative architectural facts.

## First experiment

Before building automatic semantic graph reasoning, persist accepted placement certificates and handoff decisions in a queryable form and test whether later slices require less recon.

## Exit criterion

The engine preserves architectural knowledge that Codebase-Memory cannot derive from source structure alone.

---

# Phase 15 — Evidence invalidation and proof-aware verification

This is a later optimization, not a prerequisite for the Codebase-Memory migration.

Model accepted slice claims explicitly, for example:

- state belongs to boundary X;
- runtime layer does not depend on editor-only identity;
- stale binding cannot influence scoring;
- persistence preserves semantic identity;
- downstream consumer observes the new state.

Attach proof evidence such as:

- placement certificate;
- deterministic graph fact;
- static dependency rule;
- vertical semantic test;
- focused regression test;
- full-suite result;
- adversarial reviewer finding/clearance.

Track dependencies/digests so a repository change can invalidate only affected claims and proofs.

The long-term target is selective re-verification:

> Re-establish stale proof obligations rather than reconstructing all previously accepted knowledge.

This phase should be justified by measured repeated verification/recon cost before implementation.

---

# Phase 16 — Counterexample-oriented adversarial review

Keep the independent semantic reviewer, but compress its output around falsification.

Prefer findings that identify one of:

- concrete counterexample;
- violated invariant;
- failed placement-certificate clause;
- contradictory source/graph evidence;
- missing proof obligation;
- unreachable claimed consequence;
- unsafe/stale persistence or identity behavior.

Avoid broad review essays when a small falsifier set is sufficient.

Codex remains responsible for deciding whether findings are valid and in scope, implementing repairs, and accepting the final slice only after deterministic checks and blocking semantic findings are clear.

---

# Measurement and decision policy

No optimization phase is accepted solely because it lowers tokens.

For each experiment, classify the outcome across three dimensions:

## Token/context efficiency

- provider input/output tokens;
- hidden/injected context where measurable;
- builder-context growth;
- graph-result tokens retained;
- raw source lines/ranges read;
- supplemental calls;
- compaction frequency.

## Correctness

- placement conflict rate;
- placement reconsideration rate;
- vertical proof failures;
- late semantic rejection;
- regressions;
- post-review repair count;
- unresolved architecture stops.

## Verification quality

- independent falsification preserved;
- deterministic gate completeness;
- adversarial-review findings;
- missing-proof findings;
- ability to reproduce evidence;
- source-grounded confirmation when graph evidence is insufficient.

A change that lowers tokens while increasing late semantic failures is not an optimization.

---

# Proposed near-term slice order

The placement/efficiency slice and reconnaissance-provider abstraction are
already implemented. The next campaign should therefore proceed approximately
in this order:

1. **Prove one complete ESM runtime slice** through property profile, correction
   promotion, the sidebar consumer, browser behavior, tests, and import-backed
   graph evidence.
2. **Migrate the remaining first-party extension clusters to ESM in dependency
   order**, removing each compatibility global when its final consumer moves
   while preserving documented platform/vendor boundaries.
3. **Run the Claude + Codebase-Memory graph-first experiment** against the
   existing Claude filesystem baseline while keeping model, schemas, builder,
   and validation constant.
4. **Compare disposable graph recon with builder-direct graph exploration** to
   measure whether the recon airlock protects durable builder context.
5. **Adopt the hybrid airlock + builder-microscope policy** only if the
   measurements support it.
6. **Benchmark cheaper graph-enabled placement/recon models** independently of
   the retrieval change.
7. **Add adaptive escalation** so strong-model reasoning is spent on unresolved
   structural/semantic ambiguity rather than navigation.
8. **Make Codebase-Memory the default repository-intelligence substrate** if
   correctness and verification acceptance metrics hold, retaining filesystem
   recon as fallback/control.
9. **Experiment with a Work Engine semantic overlay** beginning with accepted
   placement certificates, architectural decisions, rejected substitutes, and
   proof relationships.
10. **Add proof invalidation/selective reverification** only if measured repeated
    reconstruction or verification cost justifies it.
11. **Compress adversarial review around counterexamples and violated proof
    obligations** while preserving an independent reviewer.

The already-implemented deterministic gate, bounded recon schemas, audit/handoff split, and contract ownership remain prerequisites and should be protected by tests while these experiments proceed.

---

# Non-goals

Do not:

- replace independent review with compilation or tests alone;
- let Codebase-Memory become an unquestioned source of truth;
- build a new AST/call/import graph before proving existing Codebase-Memory capabilities insufficient;
- introduce agent swarms merely to reduce individual context sizes;
- persist raw recon prose as architectural memory;
- optimize provider cost while ignoring builder-context pollution;
- let an LLM directly execute routine validation merely for orchestration;
- allow a provider-specific response shape to leak into the builder contract;
- silently weaken placement falsification when changing retrieval providers;
- treat cached tokens as free when they still occupy reasoning context;
- preserve a recon session merely for convenience when independence is semantically valuable.

---

# Target end state

The desired Work Engine does not primarily move source code between models.

It moves **small, verified evidence and architectural claims** across explicit lifetime boundaries:

- Codebase-Memory supplies deterministic repository structure and navigation;
- disposable scouts absorb exploratory graph traversal;
- Codex owns architectural placement and implementation;
- the placement certificate carries the intended semantic path;
- deterministic runners prove mechanical correctness;
- fresh reviewers attempt to falsify semantic correctness when consequence, uncertainty, or configuration requires independence;
- compact handoffs preserve only durable architectural consequences;
- strong models are escalated for ambiguity and adversarial reasoning rather than routine retrieval;
- later semantic memory preserves what the project has learned and why it is still believed.

The ultimate optimization target is not the fewest tokens per call. It is the fewest tokens required to produce an accepted change whose placement, downstream consequence, and verification remain trustworthy.
