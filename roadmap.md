# Work Engine Roadmap

## Purpose

Work Engine is becoming an independent repository.

It began as a Site2JSON engineering subsystem, but its durable subject is now
broader: how capable models perform objective-driven work inside explicit
contracts while retaining authority over non-invariant decisions.

This roadmap carries that system from its current nested implementation to a
coherent standalone product. It is governed by:

- [`DESIGN.md`](DESIGN.md), the normative product doctrine;
- [`PHILOSOPHY.md`](PHILOSOPHY.md), the non-normative explanation of why that
  doctrine exists; and
- the four equal design principles inherited from Site2JSON and adopted here:
  Truth, Maintainability, Explainability, and Aesthetics.

The transition is complete only when Work Engine can be understood, installed,
run, tested, evolved, and released without knowledge of the Site2JSON repository
that incubated it.

---

## How to read this roadmap

The roadmap describes outcomes, ownership, dependencies, and completion
evidence. It does not prescribe one universal implementation route.

Some workstreams have causal order. Repository identity must exist before a
standalone release can be published; a receipt schema must exist before receipts
can be validated against it. Outside such dependencies, the model or maintainer
may combine, reorder, or revise work when evidence shows a better route.

Each workstream distinguishes:

- **Invariant outcome** — what must remain true;
- **Current evidence** — what is already present or known;
- **Remaining work** — unresolved product work, not a mandatory action ritual;
- **Completion evidence** — what demonstrates the outcome without overstating
  what was proved.

Status values are:

- **preserve** — implemented foundation that remains a regression requirement;
- **active** — current transition or alignment work;
- **planned** — required for the independent-repository completion state;
- **research** — potentially valuable, but not required for completion;
- **removed** — no longer Work Engine scope.

---

## Product boundary after extraction

Work Engine owns:

- objective, authority, approval, and acceptance contracts;
- supervisor, builder, evidence, review, and gate boundaries;
- model-facing capability affordances;
- route revision and stale-decision retirement;
- context and information-lifetime contracts;
- deterministic enforcement of mechanically decidable invariants;
- truthful receipts, provenance, telemetry ingress, and continuation state;
- reusable engineering campaign configuration;
- provider and capability adapters whose semantics are not tied to one product;
- reference campaigns, fixtures, and tests that demonstrate Work Engine itself;
- documentation, terminology, release, and developer experience for the
  standalone system.

Work Engine does not own:

- Site2JSON extension architecture;
- Site2JSON sidebar, service-worker, content-script, or extraction behavior;
- migration of Site2JSON modules from UMD or classic scripts to ESM;
- Site2JSON-specific UI, selector, recovery, or Chrome target policy;
- Site2JSON product priorities or its application roadmap.

Site2JSON may consume Work Engine and provide a project adapter. That adapter
must not become Work Engine's generic product contract.

---

## Existing roadmap disposition

The previous roadmap mixed Work Engine product development with work on the
repository that hosted it. This transition makes the boundary explicit.

| Previous roadmap area | Disposition |
| --- | --- |
| Optimization baseline and workflow metrics | Retain, but rebuild around comparable accepted outcomes and authoritative telemetry |
| Deterministic gate runner | Preserve as a tested Work Engine primitive |
| Model-boundary contract compression | Retain the objective; remove fixed route and cardinality rules that became policy |
| Audit receipt versus compact handoff | Retain and complete |
| Canonical contract ownership | Retain and simplify |
| Repository-evidence provider abstraction | Retain as capability machinery, not mandatory routing policy |
| Site2JSON vertical ESM migration | Remove from this roadmap; return it to Site2JSON |
| Remaining Site2JSON extension ESM migration | Remove from this roadmap; return it to Site2JSON |
| Codebase Memory experiments A/B and hybrid workflow | Consolidate into one evidence-capability workstream and optional benchmarks |
| Model-choice benchmark | Move to research; it is not a product-completion gate |
| Fixed adaptive-escalation ladder | Replace with evidence-based escalation consequences and truthful provenance |
| Codebase Memory as mandatory default substrate | Retain only as a configurable current default, never an invariant route |
| Semantic architectural memory overlay | Move to research until a durable consumer and invalidation contract exist |
| Evidence invalidation and proof-aware verification | Promote into the core route-revision contract |
| Counterexample-oriented review | Retain as an optional independent-review capability selected by consequence |

Historical reviews and metrics remain evidence about prior versions. They do
not silently define the current product and must be labeled or archived when
their claims become stale.

---

# Workstream 0 — Establish the independent repository

**Status: active.**

## Invariant outcome

Work Engine has its own repository identity, history, paths, automation,
documentation entry point, and release boundary. No runtime or documentation
contract requires the source tree to be nested beneath `site2json/work-engine`.

## Current evidence

The subtree already contains its doctrine, skills, schemas, scripts, tests,
campaigns, configuration, reviews, and metrics. Several commands and campaign
paths still assume invocation from the Site2JSON repository root. As the first
transition cleanup, the invalid Site2JSON roadmap campaign has been removed and
the owned Work Engine campaign now uses standalone-root-relative roadmap and
metrics paths.

## Remaining work

- Choose the standalone repository name, ownership, visibility, license, and
  release destination.
- Preserve meaningful history during extraction and record the source revision
  from which the standalone history began.
- Make repository-relative paths canonical. Commands should work from the new
  repository root without a `work-engine/` prefix.
- Keep a Site2JSON consumer example in the future only if it is explicitly
  labeled as an external integration fixture.
- Decide which historical metrics and reviews move with Work Engine, which
  remain with Site2JSON, and which are archived snapshots. Preserve provenance
  rather than rewriting their original paths or configuration.
- Add a root `README.md`, license, contribution guidance, security policy as
  appropriate, changelog/release convention, and supported-runtime statement.
- Add repository-owned CI for all supported languages and executable
  documentation checks.
- Ensure generic Chrome Vision behavior remains product-neutral; keep
  Site2JSON-specific aliases and recovery composition in the Site2JSON adapter.

## Completion evidence

- A fresh clone can install dependencies and run every supported test and
  preflight from the standalone root.
- No active Work Engine command or config contains an accidental dependency on
  the old parent path.
- Historical artifacts retain truthful source provenance.
- Site2JSON can reference Work Engine as an external dependency or checked-out
  integration without Work Engine importing Site2JSON product policy.

---

# Workstream 1 — Align implementation with the governing design

**Status: active.**

## Invariant outcome

Runtime instructions and deterministic validators preserve true contracts while
leaving non-invariant route choices to model judgment. Every binding command has
a causal parent and an identifiable failure mode.

## Current evidence

The doctrine clearly separates invariant structure, consequences, capability
affordances, and procedures. Runtime skills partially preserve that distinction,
but several encode current preferred routes as mandatory sequences. The receipt
validator accepts only `direct` and `falsified-placement` route labels, turning
a useful default taxonomy into a closed product world.

## Remaining work

- Inventory binding runtime commands and classify each as invariant,
  consequence, capability affordance, current observation, or default route.
- Retain only commands whose violation breaks authority, ownership, security,
  mutation boundaries, interface validity, provenance, or an explicit
  acceptance contract.
- Replace the closed workflow-route enum with open route provenance. Named
  defaults may remain available for comparison, but novel valid routes must be
  representable without misclassification.
- Convert fixed capability ladders into affordance descriptions unless order is
  causally required.
- Remove fixed evidence-count, retry-count, reviewer-reset, and escalation
  thresholds from product doctrine unless an owning external contract supplies
  them.
- Keep read-only independent review, user-work preservation, configuration
  identity, and required receipt provenance as hard boundaries.
- Add a small doctrine-compliance review to feature design without attempting
  to mechanically lint semantic judgment.

## Completion evidence

- A valid route outside the historical two-route taxonomy can be configured,
  executed, recorded, and accepted.
- Each imperative in a runtime skill traces to an owning product property or is
  visibly presented as a revisable default.
- Validators reject invariant violations but do not enforce a preferred
  reasoning style or tool order.
- Tests cover route revision, preserved evidence, retired stale decisions, and
  open route identity.

---

# Workstream 2 — Consolidate contract ownership and runtime projections

**Status: planned; foundations exist.**

## Invariant outcome

Each durable semantic contract has one canonical owner. Runtime instructions
are compact, role-scoped projections of that owner rather than parallel policy
documents.

## Current evidence

Configuration, builder, handoff, receipt, provider, telemetry, comparison, and
gate contracts exist. Their core distinctions are useful, but placement,
validation, context, and route doctrine are repeated across several skills and
references.

## Remaining work

- Publish a contract ownership map covering configuration, campaign state,
  builder output, handoff, audit receipt, telemetry ingress, repository evidence,
  independent review, comparison artifacts, and visual evidence.
- Keep wire shapes and mechanically decidable semantics in schemas and
  deterministic validators.
- Keep model-facing skills limited to the role's authority, consequences,
  available capabilities, and required output boundary.
- Remove repeated general doctrine from adapters; link to the owner without
  forcing every runtime role to ingest the full design history.
- Separate historical compatibility readers from current artifact producers.
  Backward compatibility must not authorize new runs to emit obsolete formats.
- Give every schema and receipt version an explicit migration and retirement
  policy.

## Completion evidence

- Every shared field and state vocabulary has one documented owner.
- Changing an owned contract has an identifiable consumer/test impact path.
- New runs cannot emit legacy receipt versions merely because historical
  readers still accept them.
- Runtime prompt footprint decreases without losing authority, provenance, or
  failure consequences.

---

# Workstream 3 — Complete truthful state, provenance, and persistence

**Status: active; substantial implementation exists.**

## Invariant outcome

Durable state describes what actually happened. Observed, inferred, decided,
unavailable, conflicting, and unresolved states remain distinguishable.
Continuation preserves useful consequences without treating a transcript as
the product record.

## Current evidence

Versioned receipts, compact handoffs, route revisions, provider/evidence-mode
separation, fallback events, configuration provenance, append locking, and
telemetry-ingress validation are present. Some historical records remain on old
schemas, authoritative builder telemetry is incomplete, and comparative
analysis currently permits `confirmed_absent` despite recorded coverage
limitations.

## Remaining work

- Make material coverage limitations incompatible with definitive absence.
  Downgrade the claim or close the gap with relevant direct evidence.
- Complete deterministic receipt assembly from validated semantic builder
  output and authoritative host/provider telemetry.
- Enforce current schema versions for new durable writes while preserving
  explicit historical read/migration support.
- Bind effective configuration to its resolved source rather than relying on
  model transcription.
- Preserve `null` for unavailable measurements and zero only for observed zero.
- Define resumption semantics for interrupted campaigns, including stable run
  identity, accepted slice state, pending authority decisions, and compatible
  handoff context.
- Preserve route invalidations compactly: failed premise, stale dependent
  decisions, still-valid evidence, replacement route, and reason.
- Exercise append locking, crash boundaries, duplicate prevention, and partial
  receipt recovery under realistic concurrent or interrupted conditions.

## Completion evidence

- False definitive-absence artifacts fail validation.
- A new terminal receipt can be assembled without guessed measurements or
  model-copied authoritative telemetry.
- An interrupted campaign resumes from durable state without replaying accepted
  work or losing unresolved decisions.
- Exactly one durable terminal audit record exists for each terminal slice.

---

# Workstream 4 — Stabilize the capability and adapter architecture

**Status: planned; provider and evidence foundations exist.**

## Invariant outcome

Capabilities expose meaningful, composable state transitions with clear
affordances and limitations. Provider identity, capability choice, evidence
mode, fallback, and independent review remain separate concepts.

## Current evidence

The repository includes repository-search, Codebase Memory guidance, Claude
reconnaissance/review, deterministic gates, comparison tooling, and Chrome
Vision. Configuration version 2 separates repository retrieval from independent
review. Some skills still couple provider products to preferred workflows.

## Remaining work

- Define the minimal adapter interface for repository evidence, independent
  review, deterministic validation, rendered-state evidence, and builder
  execution.
- Describe what each capability can establish, its authority, its cost signals,
  its limitations, and the provenance it returns.
- Keep provider-specific metrics namespaced while mapping only genuinely
  corresponding measurements into shared semantic fields.
- Allow capabilities to coexist when authority and safety permit; do not force a
  provider change merely to use another evidence mode.
- Make unavailable, failed, timed-out, and unsupported capabilities explicit
  state rather than cognitive recovery instructions.
- Provide bounded extension points for additional builders and evidence
  providers without adding redundant controls that reach indistinguishable
  states.
- Make Chrome Vision an optional generic capability with a stable packet
  contract. Product selectors, aliases, and restart policy remain adapter-owned.

## Completion evidence

- At least two repository-evidence implementations satisfy the same semantic
  adapter contract, or the single supported implementation is truthfully
  documented without pretending portability has been proved.
- Retrieval and independent review can use the same or different products
  without conflating their roles.
- Capability selection is recorded separately from actual fallback.
- Adding an adapter does not require rewriting supervisor doctrine.

---

# Workstream 5 — Prove the campaign lifecycle

**Status: planned; single-slice paths have the most evidence.**

## Invariant outcome

The supervisor owns campaign state, authority, limits, durable receipts, and
continuation decisions without absorbing or recreating the builder's domain
reasoning. Builder context survives only while it improves the bounded work.

## Current evidence

The supervisor and builder contracts describe planning, acceptance,
implementation, gating, handoff, continuation, limits, stops, and intervention.
Recorded campaigns have exercised first slices; the repository does not yet
carry convincing evidence for a complete multi-slice lifecycle.

## Remaining work

- Run representative campaigns that accept more than one slice and consume a
  compact handoff in a later builder.
- Demonstrate route reopening when evidence invalidates an accepted boundary.
- Demonstrate a recoverable validation failure, a truthful stop, a required
  human-authority decision, and continuation after an accepted slice.
- Verify limits and stop conditions from observed state, including the
  distinction between exhausted capacity, unavailable capability, product
  failure, and incomplete evidence.
- Verify that supervisor context remains compact and does not accumulate raw
  source, logs, transcripts, or model-specific bookkeeping.
- Decide lifecycle boundaries from retained information value rather than a
  universal reset ritual.

## Completion evidence

- A multi-slice reference campaign reaches objective completion with valid
  terminal receipts and a consumed handoff.
- A separate campaign reaches a truthful non-success terminal state without
  representing it as completion.
- Resume, replan, repair, limit, approval, and notification behavior have
  executable coverage proportional to their consequence.
- Campaign completion is supported by evidence beyond a builder's unsupported
  assertion that no work remains.

---

# Workstream 6 — Make validation consequence-driven

**Status: planned; deterministic gate runner is preserve.**

## Invariant outcome

Mechanically determined checks run deterministically. Semantic breadth and
independence are selected from consequence, uncertainty, reversibility,
fan-out, and explicit acceptance requirements. Passing a check never proves
more than that check establishes.

## Current evidence

The deterministic gate runner accepts array commands, runs without shell
interpolation, bounds failure excerpts, fails fast, and reports executed totals.
Engineering profiles distinguish proportional and full validation, but some
runtime instructions still turn risk signals into mandatory stage rules.

## Remaining work

- Preserve the gate runner as the owner of command execution and compact
  observed results.
- Make validation requirements describe observable consequences rather than
  ceremonial stages where possible.
- Keep explicit full-gate configuration binding when broad consistency is
  itself part of the user's acceptance contract.
- Allow proportional validation to justify selected and omitted evidence
  without reporting omission as success.
- Treat independent review as a capability used when it materially reduces
  consequential correlated error or is explicitly required.
- Preserve useful reviewer context through bounded remediation unless renewed
  independence or changed premises require a fresh perspective.
- Add rendered-state and accessibility evidence when the changed consequence is
  visual or interactive; source and unit tests alone do not establish pixels or
  interaction.

## Completion evidence

- The same semantic requirement can be established through different valid
  evidence routes when no contract fixes the route.
- Deterministic validators reject malformed contracts and failed checks without
  judging open semantic choices mechanically.
- Review freshness corresponds to an independence claim rather than a universal
  phase transition.
- Receipts state validation breadth, limitations, and unresolved concerns
  truthfully.

---

# Workstream 7 — Build useful observability and evaluation

**Status: planned; historical measurements are incomplete.**

## Invariant outcome

Metrics inform future judgment and design without redefining success. Comparisons
use compatible accepted outcomes and distinguish missing measurements from
zero.

## Current evidence

Receipt contracts contain builder, provider-role, evidence-mode, route,
validation, review, timing, token, cost, and outcome fields. Historical records
are sparse, span schema versions, contain no second slices, and lack several
authoritative builder measurements.

## Remaining work

- Complete authoritative telemetry ingress and receipt assembly before treating
  cost comparisons as reliable.
- Define comparison cohorts by objective class, consequence, route, provider,
  model, validation breadth, and accepted outcome.
- Measure total cost to an accepted or truthful terminal decision, including
  repair, late rejection, user attention, context occupancy, and maintenance
  burden where observable.
- Keep historical incompatible records available for provenance but out of
  unsupported aggregate comparisons.
- Add calibration checks that reopen evidence and compare recorded claims with
  the artifacts they cite.
- Evaluate repository evidence, context isolation, reviewer persistence, model
  choice, and escalation as separable variables.
- Publish limitations with every benchmark; a sample or passing run is not a
  universal performance claim.

## Completion evidence

- New accepted runs carry authoritative available measurements and explicit
  unavailability for the rest.
- Comparable cohorts can answer at least one route or capability question
  without mixing incompatible schemas or outcomes.
- Evaluation includes quality and repair signals, not cost alone.
- No product acceptance rule is silently derived from an observational metric.

---

# Workstream 8 — Make the repository explain and sustain itself

**Status: planned.**

## Invariant outcome

The standalone repository is coherent to use and maintain. Documentation,
terminology, architecture, commands, errors, and presentation reveal the real
system and invite continued stewardship.

## Current evidence

The repository contains substantial doctrine and reference material, but lacks
a concise standalone entry point. Current, historical, speculative, and
Site2JSON-specific documents are not always visibly distinguished, and several
contracts repeat the same ideas.

## Remaining work

- Create a documentation map with explicit categories: normative doctrine,
  runtime contracts, operator guidance, reference, roadmap, research, history,
  and generated artifacts.
- Give every historical review, experimental note, and superseded proposal a
  visible status and relationship to current doctrine.
- Consolidate duplicate design drafts after preserving any unique evidence or
  reasoning that still matters.
- Use consistent names for supervisor state, builder state, evidence mode,
  provider role, route revision, validation result, receipt, and handoff.
- Provide minimal runnable examples that demonstrate normal, uncertain,
  fallback, intervention, and failure paths without presenting scripted demos
  as general workflow law.
- Make errors identify what happened, which contract failed, what evidence is
  available, and what authority or action is needed next.
- Establish formatting, linting, test, release, and contribution workflows that
  make internal maintenance pleasant and predictable.
- Review documentation and developer surfaces against Truth, Maintainability,
  Explainability, and Aesthetics together.

## Completion evidence

- A new contributor can identify the canonical design, run a reference campaign,
  locate each contract owner, and understand a failed receipt without reading
  historical reviews.
- No active documentation makes a known stale claim about current behavior.
- Links, examples, commands, schemas, and campaign preflights are checked in CI.
- Repository organization makes current product surfaces visibly distinct from
  research and history.

---

# Workstream 9 — Release the completed standalone foundation

**Status: planned.**

## Invariant outcome

Work Engine has a versioned standalone foundation whose claims match its
observed behavior. Completion means the core system is coherent and supported;
it does not mean every possible provider, optimization, or research idea has
been implemented.

## Release candidate requirements

- Independent repository extraction and root-relative operation are complete.
- Doctrine and runtime projections pass the alignment criteria in Workstream 1.
- Contract owners and current schema-production rules are explicit.
- Truthful absence, route revision, provenance, configuration identity, and
  receipt persistence are executable and tested.
- At least one multi-slice reference campaign completes and one representative
  campaign stops truthfully.
- Capability roles are distinct and adapters report truthful availability,
  use, failure, and fallback.
- Deterministic and semantic validation boundaries are demonstrated.
- Current telemetry supports limited, explicitly scoped evaluation.
- Standalone documentation, contribution, CI, versioning, and release surfaces
  are present and coherent.
- Site2JSON-specific product work is absent from the active Work Engine roadmap
  and generic runtime contracts.

## Completion review

The completion review evaluates all four principles equally:

### Truth

Do state, evidence, provenance, confidence, uncertainty, fallbacks, revisions,
and limitations correspond to what actually happened?

### Maintainability

Does each contract have one owner? Can providers, builders, and campaigns evolve
without duplicating doctrine or transferring unrelated maintenance burden?

### Explainability

Can a user or maintainer trace objective, authority, evidence, decision,
execution, validation, and outcome without reconstructing hidden machinery?

### Aesthetics

Do terminology, repository organization, documentation, code boundaries,
commands, and presentation form one coherent system worth caring for?

Work Engine reaches roadmap completion when the release candidate requirements
are observed and unresolved limitations are explicitly recorded. Optional
research remains open without making the completed foundation fictional.

---

# Research after the standalone foundation

These areas may improve Work Engine, but they are not prerequisites for the
first completed standalone release:

- controlled comparisons of repository-evidence providers;
- controlled model and reasoning-effort comparisons;
- semantic architectural memory with explicit invalidation and consumer
  contracts;
- richer proof-aware verification and claim dependency tracking;
- additional independent-review strategies;
- new domain-specific builders;
- distributed or remote campaign execution;
- richer UI, visualization, and human collaboration surfaces.

Research becomes product work only when it has an owner, a protected
consequence, a truthful provenance model, and evidence that it adds meaningful
reach rather than another equivalent control.

---

## Immediate transition slice

The first bounded slice should establish the standalone boundary without trying
to finish every internal alignment issue at once.

Its intended consequence is:

> A fresh checkout behaves as the Work Engine repository, with canonical root
> paths, an accurate entry document, valid owned campaigns, preserved historical
> provenance, and no active dependency on Site2JSON product structure.

Expected scope includes repository extraction decisions, root-relative path
normalization, campaign disposition, documentation entry points, CI bootstrap,
and a migration inventory for historical artifacts. Changes to receipt
semantics, route openness, and runtime doctrine remain separately reviewable
Workstreams 1–3 unless the extraction exposes a causal dependency that requires
them sooner.
