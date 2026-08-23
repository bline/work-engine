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

## Current strategic route

The 2026-08-20 strategic reconciliation found that the command-and-control
direction remains sound, while distribution and live recovery lag behind the
repository-local terminal lifecycle. Fourteen accepted slices now establish
multi-slice completion, compact handoff consumption, recoverable validation,
truthful stopped and failed terminals, and route reopening. A fifteenth slice
stopped truthfully when its configured independent review became unavailable;
its local edits are not accepted evidence.

The review-remediation campaign has since established the exact authorized Git
delta, authoritative completion-commit receipts, a durable nonterminal
completion-offer lifecycle, open proposal provenance, review-evidence
applicability, and route-open persistent-state requirements. Those accepted
outcomes are now joined by two accepted active-slice recovery slices. A fresh
process recovers the same planning or review obligation, handled consequences,
actor binding, authoritative references, and retirement without stale
resurrection. These outcomes supersede the corresponding gaps in the 2026-08-20
and 2026-08-21 reconciliations without erasing their dated evidence.

The proposal-workflow direction has also advanced beyond its dated handoff.
The repository now has mechanically validated durable proposal packets,
adaptive proposal formation, authority-controlled decision recording, a
provisional revision-bound specialized-review procedure, approved candidate
meanings for review artifacts and adaptive panel coordination, and a successful
bounded claim-lineage dogfood. Independent-review episodes can preserve their
own remediation state and history through an authority-bound profile, including
bounded MCP access. These are substantial foundations, but they do not yet form
one controlled idea-to-proposal lifecycle. Raw idea intake is now a formed
proposal whose seven bootstrap-review findings are closed at revision
`89a3a38`, but its authority-owned disposition and implementation remain
pending. Review artifacts and panel coordination remain approved candidate
meanings rather than implemented capabilities, and no end-to-end exercise has
yet carried a source-bound intake through formation, revision, review, and an
authority-owned disposition.

The newly recorded
[`agent-instruction-structure-and-placement-review`](ideas/agent-instruction-structure-and-placement-review.md)
idea exposes a more immediate structural-integrity dependency. Agent-facing
text affects every future skill, role, and workflow, while current review roles
do not specialize in whether a normative instruction preserves a necessary
distinction or is placed at the correct authority, scope, and loading layer.
The user has prioritized intake of this capability before further expansion of
agent-facing instruction surfaces. That priority does not accept the idea or
make it a universal review gate.

The current priority order is:

1. reconcile accepted campaign work and keep stopped, experimental, and
   user-owned changes distinguishable;
2. treat agent-instruction structure and placement review as a critical
   structural-integrity interruption: carry the committed idea through intake,
   formation, specialized review, and bounded dogfood before expanding skills,
   roles, or workflow instruction surfaces;
3. resume and complete the controlled proposal workflow from low-friction raw
   idea intake through claim-addressable assessment, proposal formation,
   revision-bound adaptive review, and an explicit authority-owned disposition;
   the reviewed raw-intake proposal remains paused at its authority-decision
   boundary rather than rejected or implementation-authorized;
4. preserve the implemented packet, decision, claim-lineage, active-slice,
   transition-history, and reviewer-state boundaries while the completed
   workflow exercises them as real consumers;
5. establish an atomic, versioned install boundary with a neutral-directory
   smoke test before syncing the repository skill into Codex;
6. audit contract ownership and imperative runtime instructions;
7. complete standalone documentation, CI, versioning, and release surfaces;
8. promote evidence-backed evaluation, portfolio selection, organizational
   planning, and closed-loop learning only after the proposal workflow can
   hand them durable, authority-bounded proposal state.

This order is current strategic guidance, not an invariant procedure. A
strategic planning break should confirm or revise it when durable execution
evidence materially changes its assumptions, dependencies, or expected value.

---

# Workstream 0 — Establish the independent repository

**Status: active.**

## Invariant outcome

Work Engine has its own repository identity, history, paths, automation,
documentation entry point, and release boundary. No runtime or documentation
contract requires the source tree to be nested beneath `site2json/work-engine`.

## Current evidence

Work Engine now has its own repository, remote, package identity, license,
doctrine, skills, schemas, scripts, tests, campaigns, configuration, reviews,
and metrics. The owned roadmap campaign uses standalone-root-relative roadmap
and metrics paths. The repository-local suites and campaign preflight pass, but
the installed skill is an older incompatible copy and some comparative paths
and historical identifiers still assume the former Site2JSON layout.

## Remaining work

- Record the chosen repository identity, ownership, visibility, license, and
  extraction provenance in the standalone documentation.
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
- Define an atomic, versioned install/sync boundary for supervisor, builder,
  scripts, references, and runtime dependencies. Verify it from a neutral
  directory rather than relying on repository-local dependency resolution.
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

**Status: active; open route provenance is implemented.**

## Invariant outcome

Runtime instructions and deterministic validators preserve true contracts while
leaving non-invariant route choices to model judgment. Every binding command has
a causal parent and an identifiable failure mode.

## Current evidence

The doctrine clearly separates invariant structure, consequences, capability
affordances, and procedures. Runtime skills partially preserve that distinction,
but several still encode current preferred routes as mandatory sequences. The
receipt validator now accepts any nonempty workflow-route identity; `direct`
and `falsified-placement` remain named defaults rather than a closed product
enum. Tests exercise additional identities and preserve route revisions with
their retired decisions and retained evidence. The committed exploratory
agent-instruction structure and placement review now supplies a compact
two-question diagnostic and distinguishes normative-text placement from system
or component placement, but it has not yet passed intake, formation, review, or
dogfood.

## Remaining work

- Form and review the agent-instruction structure and placement capability, then
  dogfood it on materially agent-facing skill and workflow text before treating
  it as a reusable review specialist.
- Inventory binding runtime commands and classify each as invariant,
  consequence, capability affordance, current observation, or default route.
- Retain only commands whose violation breaks authority, ownership, security,
  mutation boundaries, interface validity, provenance, or an explicit
  acceptance contract.
- Convert fixed capability ladders into affordance descriptions unless order is
  causally required.
- Remove fixed evidence-count, retry-count, reviewer-reset, and escalation
  thresholds from product doctrine unless an owning external contract supplies
  them.
- Keep read-only independent review, user-work preservation, configuration
  identity, and required receipt provenance as hard boundaries.
- Reconcile the older broad doctrine-compliance-review outcome with the more
  precise instruction-structure candidate rather than creating duplicate review
  owners or attempting to mechanically lint semantic judgment.

## Completion evidence

- Routes outside the historical two-route taxonomy remain representable and
  accepted without weakening their provenance.
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
separation, fallback events, configuration and campaign-source provenance,
authoritative receipt assembly, append locking, terminal finalization, and
read-only inter-slice resume are present. Reference campaigns prove accepted,
stopped, failed, repaired, and reopened routes. Completion interaction now has
a supervisor-owned durable nonterminal offer lifecycle outside terminal metrics
and the ordinary worktree; the completion adapter retains ownership of real
branch mutation and authoritative created or refused evidence. Proposal
provenance is structured and open to truthful producers while remaining bound
to its checkpoint, tree, and task patch. Active slice attempts now have a
reachable fresh-process lifecycle for pending planning and review obligations:
slice-supervisor owns their semantic state and durable-state owns only opaque
compare-and-swap publication. Some historical records remain on old schemas,
authoritative builder telemetry is incomplete, and partial-artifact recovery
does not yet exist.

The Git-ref durable-state adapter now retains predecessor-linked immutable
history with exact-revision reads, deterministic bounded temporal listing, and
reachability across Git garbage collection. Active-slice recovery can
reconstruct a selected retained revision without replaying effects. A separate
independent-review-state profile preserves one authority-bound adversarial
review and remediation episode with compare-and-swap transitions, generation
fencing, attributed findings, pending actions, retirement, and bounded MCP
access. These implementations establish shared durability mechanics and one
real role-owned profile; they do not make durable-state or MCP the semantic
owner of every future role.

The persistent-state proposals retain durable review obligations as a possible
consumer rather than selecting the first implementation boundary in advance. A
review artifact owns its immutable subject, material assumptions, established
consequence, limitations, and lineage. Later consequential use must establish
applicability or record refresh, supersession, or composition. Overlap and
reservation machinery remains a useful conditional coordination capability,
and later workspace mutation does not retroactively change what was reviewed.
See [`ideas/persistent-agent-state-and-runtime-introspection.md`](ideas/persistent-agent-state-and-runtime-introspection.md)
and [`ideas/durable-review-queue-and-scope-coordination.md`](ideas/durable-review-queue-and-scope-coordination.md).

## Remaining work

- Preserve `null` for unavailable measurements and zero only for observed zero.
- Extend durable live state only when a named consumer requires semantics beyond
  the accepted planning/review obligation lifecycle. Preserve stable logical
  identity separately from provider runtime identity and keep semantic ownership
  out of the opaque durable-state primitive.
- Generalize retryable capability or gate unavailability beyond the implemented
  active-slice and completion-offer lifecycles only from an identified consumer,
  without redefining final stopped and failed terminals.
- Define pending-authority, mid-slice interruption, partial-artifact recovery,
  and deterministic reconciliation semantics.
- Exercise the implemented independent-review episode profile through the
  revision-bound proposal-review consumer while keeping final review artifacts,
  proposal decisions, and claim revisions with their stronger owners.
- Make consequential use of existing review evidence establish applicability
  to the candidate state or record refresh, supersession, or composition.
- Expose overlap analysis, protected scopes, and reservations as coordination
  capabilities, requiring them only where an identified concurrency or
  authority constraint would otherwise permit an invalid transition.
- Prevent confirmed conflicts from reusing stale evidence while allowing later
  non-conflicting work to continue without weakening the original review truth.
- Exercise crash boundaries, duplicate-consequence prevention, and partial
  recovery under realistic concurrent or interrupted conditions. Append
  locking remains one possible mechanism where the selected owner requires it.

## Completion evidence

- False definitive-absence artifacts fail validation.
- A new terminal receipt can be assembled without guessed measurements or
  model-copied authoritative telemetry.
- An interrupted campaign resumes from durable state without replaying accepted
  work or losing unresolved decisions.
- Exactly one durable terminal audit record exists for each terminal slice.

---

# Workstream 4 — Complete the controlled idea-to-proposal workflow

**Status: active; packet, formation, decision, evidence-lineage, and reviewer-state foundations exist.**

## Invariant outcome

Work Engine can turn low-ceremony speculative idea sources into durable,
claim-addressable assessments and then into zero, one, or several independently
decidable proposal candidates. Each surviving proposal can be revised,
challenged by a consequence-selected specialist review panel, and presented to
the named authority for an explicit disposition without allowing intake,
formation, review, persistence, or mechanical validation to acquire that
authority.

Raw capture, intake assessment, proposal meaning, review evidence, claim
history, and authority decisions remain distinct durable owners. Physical
organization may change as the workflow is exercised, but a move or cleanup
operation must not erase source identity, provenance, assessment history, or
resolvable proposal origins.

This workstream ends at a durable authority-owned proposal disposition and a
truthful handoff to downstream planning. Evidence-backed portfolio comparison,
roadmap activation, organizational compilation, campaign authorization, and
implementation remain separate consumers.

## Current evidence

- `proposal-packets` owns stable proposal identity, closed manifest and decision
  contracts, repository discovery, typed relationships, and deterministic
  validation. Nine current proposal packets plus three fixtures validate
  mechanically.
- `proposal-former` supplies the initial adaptive formation contract and a
  transcript-free one-idea/one-proposal consumer proof. Split, merge,
  duplication, reopening, and human-scope-change scenarios remain to be
  exercised as one coherent capability.
- Authority-controlled proposal decisions are implemented and have recorded
  real user dispositions without changing roadmap priority or authorizing
  implementation.
- The bootstrap proposal-review procedure has exercised fresh specialist entry,
  retained remediation context, multi-perspective synthesis, and truthful
  authority separation on several real proposal revisions.
- Proposal meanings for revision-bound review artifacts and adaptive review-panel
  coordination are approved with provisional placement, but neither capability
  is implemented as a durable product boundary.
- The bounded claim-lineage dogfood passed stable-identity, non-authoritative
  `may_affect`, changed and retained-unchanged refresh, and exact-revision
  reliance proofs. Its placement and completeness remain experimental.
- Independent-review episode state, retained transition history, and bounded MCP
  access now preserve reviewer remediation context without making live role
  state the owner of final review artifacts or decisions.
- `work-engine.raw-idea-intake` now defines the formed source-bound intake
  consequence, claim-level assessment, proposal-formation handoff, manual-map
  migration, and cleanup-authority boundary. Its adaptive bootstrap review is
  closed with no remaining finding; acceptance, final placement, and
  implementation remain separately pending.
- [`ideas/agent-instruction-structure-and-placement-review.md`](ideas/agent-instruction-structure-and-placement-review.md)
  is the user-prioritized critical intake subject before further skill and
  workflow instruction expansion. It is still an exploratory idea, not an
  accepted reviewer contract.

## Remaining work

- Pause further advancement of the reviewed raw-intake proposal at its
  authority-decision boundary while the critical instruction-review idea is
  assessed; later resume from the immutable proposal and review artifacts
  without reconstructing this planning decision.
- Establish a structured intake owner that binds an exact raw-source revision,
  preserves stable idea identity, distinguishes extracted source claims from
  assessor inference, records evidence cutoffs and authority, and can return a
  truthful unresolved or no-proposal result.
- Reconcile intake claims against current architecture, implementation,
  proposals, reviews, and other assessed ideas without turning similarity or
  code matches into automatic semantic decisions.
- Preserve claim-level intake dispositions, uncertainty, reopening conditions,
  and the smallest useful handoff to proposal formation. Keep raw-source cleanup
  as a separately authorized repository consequence.
- Extend formation evidence across one-to-zero, one-to-one, split, merge,
  duplicate, reopened-placement, and human-scope-change outcomes without making
  those cases a mandatory procedural sequence.
- Implement a revision-bound review-artifact profile that preserves exact
  subject identity, episode context, stable findings, assumptions, limitations,
  conflicts, truthful empty or failure outcomes, applicability lineage,
  synthesis references, and non-authoritative decision readiness.
- Implement adaptive review-panel coordination that selects perspectives from
  material consequences, records relevant omissions and failures, retains
  specialist ownership, preserves disagreement, and never treats synthesis as
  a vote or authority decision.
- Connect the live independent-review episode profile to the review workflow as
  resumable operational state while publishing final findings and synthesis to
  their stronger durable artifact owner.
- Exercise the whole boundary on materially different raw ideas and proposal
  revisions before deciding permanent intake layout, shared claim placement,
  review storage, or richer control-plane integration.
- Reconcile the dated proposal-workflow planning handoff and roadmap evidence
  after each accepted vertical so completed prerequisites do not remain listed
  as future work.

## Completion evidence

- A raw idea remains cheap to capture and can later be assessed against one
  exact immutable source revision without silently changing subject when the
  file changes or moves.
- Representative duplicate, promoted, apparently implemented, split, genuinely
  novel, uncertain, and no-surviving-candidate cases retain truthful attributed
  dispositions; none is forced into a proposal or cleanup action.
- A fresh proposal former consumes one intake projection without rereading the
  entire idea collection or reconstructing the assessment conversation.
- Formation produces durable zero, one, split, merge, revision, and
  human-decision consequences while packet validation continues to establish
  only mechanically decidable properties.
- A changed proposal revision receives a consequence-selected specialist panel;
  exact findings, omitted or unavailable perspectives, retained remediation,
  conflicts, applicability, and synthesis references remain reconstructable
  without a review transcript.
- The named proposal authority can record an explicit disposition from the
  durable packet and review evidence without review, validation, or persistence
  acquiring acceptance, roadmap, or implementation authority.
- At least one end-to-end exercise reaches a decided proposal and another
  truthfully reaches no proposal or unresolved intake, with source provenance,
  claim lineage, resume state, and downstream handoff still intact.

---

# Workstream 5 — Stabilize the capability and adapter architecture

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

# Workstream 6 — Prove the campaign lifecycle

**Status: active; core multi-slice lifecycle evidence exists.**

## Invariant outcome

The supervisor owns campaign state, authority, limits, durable receipts, and
continuation decisions without absorbing or recreating the builder's domain
reasoning. Builder context survives only while it improves the bounded work.

## Current evidence

The supervisor and builder contracts describe planning, acceptance,
implementation, gating, handoff, continuation, limits, stops, and intervention.
Production-boundary reference tests now demonstrate multi-slice completion,
exact compact handoff consumption, recoverable validation failure, truthful
stopped and failed terminals, byte-preserving nonresumable resume, and route
reopening followed by later completion. Completion offers add a durable,
retryable human-interaction lifecycle without occupying the unique terminal
receipt or blocking checkpoint continuation.

## Remaining work

- Extend required human-authority and retryable-capability continuation evidence
  beyond the implemented completion-offer case without redefining final stopped
  receipts.
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

# Workstream 7 — Make validation consequence-driven

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

# Workstream 8 — Build useful observability and evaluation

**Status: planned; historical measurements are incomplete.**

## Invariant outcome

Metrics inform future judgment and design without redefining success. Comparisons
use compatible accepted outcomes and distinguish missing measurements from
zero.

## Current evidence

Receipt contracts contain builder, provider-role, evidence-mode, route,
validation, review, timing, token, cost, and outcome fields. One fifteen-slice
campaign now provides sequential lifecycle evidence, with fourteen accepted
slices and one truthful quota stop. Records still span schema versions and only
the latest slices contain authoritative builder measurements, so aggregate
cost or performance comparisons remain premature.

The review-bench research tree has explicit artifact owners: protocol-versioned
corpora, sealed truth, results, scoring, reports, and provenance are retained
research evidence governed by its artifact contracts and deterministic
validator. A complete exported review packet, including its snapshot archive,
case, prompt, and export receipt, is generated evidence whose members remain
bound together. These artifacts preserve the snapshot and protocol under which
a descriptive claim was observed; neither a benchmark result nor a generated
packet is production acceptance evidence for the reviewed change. Disposable
local exports and interpreter caches are not part of that publication boundary.

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

# Workstream 9 — Make the repository explain and sustain itself

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

# Workstream 10 — Release the completed standalone foundation

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
- A controlled idea-to-proposal workflow preserves source-bound intake,
  claim-addressable assessment, adaptive formation, revision-bound review, and
  authority-owned disposition without turning the workflow into implementation
  authority.
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

## Verified review remediation campaign

The completion-commit and planning review has a dedicated remediation work
source at
[`planning/review-remediation/README.md`](planning/review-remediation/README.md)
and a runnable campaign configuration at
[`campaigns/review-remediation.yaml`](campaigns/review-remediation.yaml).

Accepted remediation slices now prove that a user-visible commit contains its
exact authorized delta, created completion receipts are authoritatively verified,
and pending completion interaction has a durable nonterminal lifecycle. They
also open proposal provenance to truthful evidence-bound producers, make later
use establish review applicability, and preserve persistent recovery
consequences without selecting event history, projection, hydration,
idempotency, storage, or coordination mechanisms in advance. Truthful stopped
planning attempts remain historical evidence rather than accepted repairs.

The dated strategic handoff at
[`planning/2026-08-20-persistent-runtime-state.yaml`](planning/2026-08-20-persistent-runtime-state.yaml)
is the canonical owner of its original evidence cutoff and later reconciliation.
The corresponding `skills/planning/` artifact is a compatibility reference to
that owner, not a second handoff. Protocol-bound review-bench artifacts and
complete exported packets remain research evidence under their recorded
snapshot and protocol provenance. The unowned root `files.zip` proposal export
is preserved as a disposable local file and excluded from this campaign's
publication boundary; its unique bytes do not make it a durable planning owner.

The remaining remediation responsibility is to keep this live roadmap and its
artifact ownership claims aligned with those observed outcomes. This campaign
does not prove executable recovery after context replacement or repeated writes,
and it does not complete the broader standalone-product roadmap.

---

## Accepted active-slice recovery foundation

The persistent-state campaign established a bounded authoritative owner and a
reachable consumer. Slice-supervisor owns active attempt identity, pending
planning/review obligations, handled consequences, waiting state, actor binding,
authoritative references, and retirement. The shared durable-state capability
owns only opaque integrity-checked compare-and-swap publication through its
current Git-ref adapter.

Its accepted consequence is:

> A fresh process can recover and resume the same active planning or review
> obligation without duplicating handled consequences, accepting stale writes,
> resurrecting retired state, or treating terminal receipts and model context as
> the live-state owner.

This foundation satisfies the recovery prerequisite recorded by the 2026-08-21
proposal-workflow direction. It does not make runtime state the owner of
proposal meaning. Durable proposal packets now provide the first bounded
planning-layer consumer of the prerequisite through their own Git-backed
semantic contract and repository validator. Proposal formation and
authority-controlled decision recording have since become additional consumers.
The shared Git-ref adapter now retains application-owned transition history,
and the independent-review profile supplies one bounded role-owned state
consumer with MCP access. General scheduling, review queues, mutation
reservations, installed-skill distribution, and broader role profiles remain
separate objectives until a concrete owner and consumer failure mode requires
them.
