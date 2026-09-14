# Root Ideas Reconciliation Against App Server

## Status

Evidence-backed reconciliation, not a proposal. This document does not
accept, promote, or draft any idea; it records disposition and evidence
only.

## Scope and method

`ideas/README.md` already reconciled a larger set of root brainstorm
documents into a "clean active-idea projection" — but that reconciliation
(pre-`2026-08-22`, per `ideas/history/2026-08-22-pre-reconciliation`) used
root `ARCHITECTURE.md` and pre-migration implementation as its evidence of
"what exists now." `ARCHITECTURE.md` has zero mentions of `app-server`
(confirmed earlier this session) and does not describe the substrate where
active development now happens. `app-server/docs/post-migration-strategic-plan.md`
already flagged this explicitly: the root idea index's "historical
implementation claims need refreshing against App Server." This document is
that refresh, for exactly the population `ideas/README.md` calls its "Active
clean idea set" — 16 files.

**Explicitly out of scope:** the originals `ideas/reconciliation-map.json`
already marked `Promoted` into `proposals/` (e.g. `adaptive-specialized-review-panels.md`,
`code-change-characterization-framework.md`). Those are proposal-owned now,
not ideas; whether their *proposals* still hold against app-server is a
separate, larger question this document does not attempt.

Each of the 16 was read in full and checked against specific, named
app-server evidence (not just title-matched) by four parallel passes,
grouped roughly by likely comparandum. Every disposition below cites its
evidence; three of the most load-bearing citations were independently
re-verified directly against source before this document was written.

Evidence cutoff: repository state as of this session, `2026-09-14`.

## Summary

| Idea | Disposition | Target / successor |
| --- | --- | --- |
| `environment-adapter-and-host-provided-runtime-services.md` | Superseded in shape | Many typed kernel/service primitives (`service-plane-inventory.md`), not one host abstraction; runtime-execution slice only → ports doc |
| `runtime-adapter.md` | Superseded | `provider-turn-harness-runtime-and-operator-projection.md` §3, with role-binding-registry gap partially realized in `operator-switchboard.mjs` |
| `control-plane-and-client-protocol.md` | Still relevant | Bring forward; update to cite `role-scheduler` and `operator-switchboard.mjs` as current partial evidence |
| `evidence-backed-proposal-evaluation.md` | Still relevant | Bring forward largely as-is |
| `proposal-backed-portfolio-selection.md` | Still relevant | Bring forward; also a critique of current planning practice |
| `proposal-research-maturity-and-freshness.md` | Still relevant | Bring forward; re-point dependency to `claim-evidence-service.md` |
| `closed-loop-engineering-learning.md` | Partially superseded | Narrow to proposal-outcome calibration only; model-capability half owned by `evidence-calibrated-plan-resolution...md` / `revisioned-research...md` |
| `role-decision-trace.md` | Partially superseded | `claim-evidence-service.md` subsumes the claim-lineage part; decision/judgment lineage (not a claim) remains open with no pre-decided owner |
| `cross-cutting-seam-review.md` | `COUPLED_RECONCILIATION` before bring-forward | Correspondence-checking half overlaps `evidence-anchor-observation-and-impact-nomination.md`'s anchor/comparator boundary, generalized beyond code; must be reconciled against `architectural-review.md` first |
| `architectural-review.md` | `COUPLED_RECONCILIATION` before bring-forward | Distinct from seam review (model-correctness judgment vs. correspondence check) but the boundary must be stated explicitly before either is drafted independently |
| `review-scope-coordination.md` | Still relevant | Bring forward; explicitly acknowledged open gap in real code |
| `organizational-execution-envelopes.md` | Still relevant, seam clarified on review | Bring forward; distinct from `RoleRealization` (multi-role org topology vs. single-role runtime binding), both consuming `role_contract` from the same already-implemented owner — stated explicitly, not just asserted against hierarchical orchestration |
| `work-engine-studio.md` | Still relevant | Bring forward; genuinely unaddressed |
| `ui-review-capability.md` | Still relevant | Bring forward; correctly gated behind its evidence dependency |
| `ui-experience-evidence.md` | Different disposition | Already promoted to `proposals/ui-experience-evidence/`; the open question is evaluating that proposal, not the idea |
| `agent-instruction-structure-and-placement-review.md` | Still relevant | Bring forward; complementary to, not addressed by, the skill compiler |

**Headline finding:** none of the 16 turned out to be simply dead. Every one
is either superseded-with-a-named-successor, still open, or already
progressed past idea stage into an unevaluated proposal. That validates the
original reconciliation's *problem identification* — it filtered out the
right things to keep as unresolved concerns. It does not, on its own,
validate that each idea's *original shape* is still the right one: several
of the superseded/narrowed entries below survived only because their core
concern migrated into a richer successor architecture than the one the idea
originally sketched, not because the idea's own proposed mechanism was
correct.

## Superseded

### `environment-adapter-and-host-provided-runtime-services.md`

Its central claim — host mechanics must not own semantic meaning — is
achieved in app-server, but not through the single universal adapter it
proposes. `service-plane-inventory.md` shows durable storage,
authority-adjacent coordination, and CAS-protected state solved per-domain
instead: `workspace-coordination` (fencing/CAS, resource types spanning
`port`/`database`/`review-budget`, not just Git), `claim-evidence`
(ledger/CAS), `review-episode` (CAS + writer-generation) — this session's
own service-plane reconciliation confirmed many typed kernel/service
primitives is the actual direction, not one host abstraction layer. Only the
"runtime execution" slice of this idea's scope maps onto
`provider-turn-harness-runtime-and-operator-projection.md`'s
`ProviderTurnPort`/`HarnessRuntimePort`. Bringing this forward as-is would
misdescribe the current direction.

### `runtime-adapter.md`

`provider-turn-harness-runtime-and-operator-projection.md` §3
(`HarnessRuntimePort`) states the same boundary more completely and more
currently. Its companion's "role binding registry" concept is concretely,
partially realized already: `app-server/src/operator-switchboard.mjs`'s
`bindingView()` records exactly `logicalRoleInstanceId`, `provider`,
`threadId`, `protocolVersion`, `environmentFingerprint`, `bindingRevision`,
`boundAt` (verified directly against source). Per the ports doc's own §9,
the full provider-neutral contract isn't built — the production builder path
is still statically Codex-bound — but the territory has a clear, more
precise owner now.

### `role-decision-trace.md` (partially superseded; distinct concern remains, disposition corrected on review)

An earlier pass at this document treated this idea as "claim-evidence, but
broader," with its remaining scope suggested as a probable amendment to
`claim-evidence-service.md`. That understates the idea and pre-decides its
eventual owner too early.

`claim-evidence-service.md`'s "Canonical claim history" genuinely subsumes
part of this idea: stable identity, immutable revisions, typed lineage
relations, exact-revision reliance records — as authorized,
partially-implemented design, not merely an idea.

But the idea's actual subject is **decision lineage**, not claim lineage:
"decision class," "assumptions and limitations," "relations to earlier
decisions," with a relation vocabulary of `PREMISE_FOR`/`SUPERSEDES`/
`WEAKENS`/`CONTRADICTS`/`AFFECTS`/`REOPENED_BY`/`CHANGED_BECAUSE_OF`
(confirmed against the idea's full text). A claim is an evidence-backed
statement about reality; a decision is a selection or delegation among
materially different routes. `proposal-decision-gated-implementation-compilation.md`
already treats these as separate first-class concepts — its "material
decision set" and "sealed decision-set revision" (confirmed present in that
document) are decisions, not claims, and are never expressed as
`claim-evidence` entries. A decision-lineage record — "I chose route A over
B because of assumption X, and that assumption was later weakened by
evidence Y" — does not naturally fit `claim-evidence`'s claim/reliance model
at all.

Corrected disposition:

```text
claim-evidence subsumes:
    evidence-backed propositions
    revision lineage
    exact-revision reliance

remaining distinct concern:
    durable semantic judgment / decision ancestry
    that is not naturally a claim
```

Where that remaining concern eventually lives is genuinely open — it may
converge with `claim-evidence-service.md` after all, or with the
independently-discovered "revisioned state + admitted transition + CAS +
authoritative successor" kernel-primitive pattern this session found four
times (`claim-evidence`'s ledger, `review-episode`, `slice-campaign`
campaign state, decision-gated compilation's own decision set —
`architecture-direction-synthesis.md` §2.3), since `PREMISE_FOR`/
`SUPERSEDES`/`CONTRADICTS` is exactly the shape of typed, revisioned,
successor-tracked state that pattern already describes. This document does
not decide between them. If brought forward, the idea should say only that
it is a distinct concern from claims, not name a specific target owner.

### `closed-loop-engineering-learning.md` (narrowed, not fully superseded)

`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`'s
"Continuous learning and authority" section and
`revisioned-research-and-execution-architecture.md` §13–14 ("Production Work
as Research Corpus" / "Production and Research Form a Closed Loop") already
own the *model/harness/strategy capability* calibration loop this idea
describes — the same "execution observations → revisioned capability
evidence → derived capability profile → routing or policy candidate" shape,
the same "does not grant mutation rights/spending/acceptance authority"
boundary. That slice is superseded. The *broader* proposal-outcome
calibration this idea also describes — was the proposal's own prediction
right (expected value/complexity/reach/burden vs. actual), not just which
model handled it well — is not covered by either document and remains open.
**Disposition: do not bring forward as-is; narrow to the proposal-expectation-
vs-outcome slice specifically**, cross-referencing both superseding documents
for the part that's no longer this idea's to own.

## Coupled reconciliation required before bring-forward

An earlier pass at this document carried `cross-cutting-seam-review.md` and
`architectural-review.md` forward independently, each citing this session's
own reconciliation work (`service-plane-reconciliation.md`,
`architecture-direction-seam-map.md`) as evidence the underlying activity is
real and undersupplied. That evidence is accurate, but treating the two
ideas independently is exactly the failure mode
`incremental-architecture-intake-and-seam-reconciliation.md` exists to catch:
two related pending concerns reconciled separately instead of against each
other first.

Read in full, both idea documents already anticipate the question and answer
it consistently with each other:

> "Architectural review asks whether the system model or ownership is wrong.
> Seam review asks whether two otherwise valid parts correspond truthfully
> and proportionately. A seam finding may trigger architectural review, but
> the capabilities are not identical." — `cross-cutting-seam-review.md`

> "Cross-cutting seam review judges coherence across boundaries after or
> around concrete changes; it is not a substitute for architectural
> diagnosis." — `architectural-review.md`, "Relationship to other ideas"

That is a real distinction, not a cosmetic one: seam review checks whether
two independently-valid things still *correspond* (a mechanical-leaning
question — do the docs still match the code, does the UI still represent the
state truthfully); architectural review judges whether the *model itself* is
wrong (a semantic judgment, with an explicit escalation/pause-recommendation
authority seam review never claims). The seam between them is `SUPPLIES`, not
`CONFLICTS`: a seam-review mismatch is a plausible *input* to architectural
review, not a competing diagnosis of the same question.

**A further seam this reconciliation surfaces, not found by either idea's
own authors:** seam review's actual mechanism — "which boundary is under
review; which independent contracts/principles meet there; evidence from
each side; mismatch or disproportion" — is close to a direct restatement of
`evidence-anchor-observation-and-impact-nomination.md`'s anchor/observer/
comparator/`may_affect` boundary, generalized from code-citation anchors to
a broader anchor-kind taxonomy (principle↔implementation, docs↔implementation,
UI↔state, authority↔exposed control, capability-contract↔provider-realization).
If that generalization holds, "cross-cutting seam review" may not need to be
a new standalone review *capability* at all — it could be the same
observer/comparator family applied to a wider set of anchor kinds, with
`architectural-review.md` remaining the genuinely distinct semantic-judgment
layer that interprets a `may_affect` nomination as an architecture defect (or
doesn't).

**Disposition: `COUPLED_RECONCILIATION` before bring-forward, for both.**
Do not draft two new `app-server/ideas/pending/` documents from these
independently. The next step is one document (or one amendment to
`evidence-anchor-observation-and-impact-nomination.md`) that states: (1)
whether seam-review's correspondence-checking mechanism is actually an
instance of the anchor/observer boundary or a genuinely distinct mechanism,
and (2) the explicit authority and invocation boundary between whatever seam
review becomes and `architectural-review.md`'s diagnostic/escalation
capability. This document does not resolve that — it names the coupling so
it isn't lost.

## Still relevant — bring forward

For each, "bring forward" means draft a new `app-server/ideas/pending/`
document derived from the original, not that the original file should move
as-is — every one below needs at least a "Current evidence" refresh, since
each cites pre-app-server artifacts. None of this drafting is done here.

- **`control-plane-and-client-protocol.md`** — not superseded by the ports
  family (different territory: role activation/scheduling/routing, not
  provider/harness/UI mechanism). Root `ARCHITECTURE.md`'s own "Control
  plane and human interface direction" section is essentially a compressed
  restatement of this same idea, both naming `role-scheduler` as the sole
  implemented precursor. `operator-switchboard.mjs` (331 lines) partially
  realizes only the "client protocol" half — a bounded command/binding
  surface, no activation leases, no versioned runtime-selection policy
  overlay — matching the ports doc's own §9 admission almost verbatim. Its
  "Runtime binding" section is now stale next to the richer
  `RoleRealization`/capability-inventory model in
  `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` and
  needs updating accordingly.
- **`evidence-backed-proposal-evaluation.md`** — checked
  `authority-backed-architecture-directions-as-workflow-inputs.md` §7
  ("Proposal-formation behavior") in full: it owns *conformance mapping*
  (is a proposal consistent with an accepted direction), not value/risk
  evaluation — no overlap. `product-development`'s implemented delivery
  services (`proposal-delivery.mjs`, `intake-delivery.mjs`) are mechanical
  packet validation — exactly the layer this idea itself says is
  insufficient. Nothing in app-server does typed multi-dimensional proposal
  evaluation. Needs minimal updates: re-point "proposal packets" language at
  the now-implemented `product-development` services as current evidence.
- **`proposal-backed-portfolio-selection.md`** — same §7 check: no
  portfolio/roadmap ownership there either. No app-server document treats
  the roadmap as a projection over proposal identity. Notable: `post-migration-strategic-plan.md`
  (the actual current planning artifact) is exactly the hand-authored-roadmap-
  prose pattern this idea argues against — a useful critique of current
  practice, not a disqualifying overlap.
- **`proposal-research-maturity-and-freshness.md`** — its own Status line
  says it should consume a claim-centered evidence-lineage candidate
  promoted to `proposals/evidence-lineage/`; that candidate's App Server
  placement is confirmed to be `claim-evidence-service.md` (its own Status
  section: "carries forward the evidence-lineage semantics... but changes
  their runtime placement"). `revisioned-research-and-execution-architecture.md`
  §27 and `evidence-calibrated-plan-resolution...md`'s evidence-sources
  section are adjacent (claim-relative adequacy, evidence-class distinctions)
  but neither defines proposal readiness stages or claim-freshness semantics
  — genuinely unaddressed. Needs its "current evidence" section re-pointed
  at `claim-evidence-service.md`.
- **`review-scope-coordination.md`** — `claim-evidence/review-finding-bridge.mjs:96`
  states outright, in a live template string: `"applicability and reliance
  remain consumer decisions"` (verified directly against source) — claim-
  evidence deliberately does not resolve whether a review result still
  applies. The only overlap-detection logic found anywhere is a narrow,
  single-purpose path check in `slice-campaign/completion-publication.mjs:129`,
  not a general coordinator. This is a confirmed, self-acknowledged open gap,
  not an inferred one. Its "Current evidence" section should now cite
  `claim-evidence-service.md`'s reliance/impact model and
  `evidence-anchor-observation-and-impact-nomination.md` as foundations that
  didn't exist when the idea was written.
- **`organizational-execution-envelopes.md`** — an earlier pass reconciled
  this only against `hierarchical-planning-and-multi-supervisor-orchestration.md`
  and called it complementary (work *decomposition* vs. role/capability
  *assembly* — that distinction still holds; hierarchical planning's own
  "Does not own" list never mentions role profiles or capability
  composition). That check was correct but incomplete — it missed a closer
  seam. This idea's own "Execution envelope" section (multi-role: instantiated
  roles, ownership and delegation, information-flow boundaries, collaboration
  and mediation) sits directly next to
  `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` §5's
  `RoleRealization` (confirmed: binds `role_contract`, `policy_overlay_revision`,
  `provider_turn`, `harness_runtime`, `tools`, `context_transition` —
  single-role realization mechanics). The seam, checked directly: both
  documents take `role_contract` as a *given input* from an already-implemented
  owner (this idea's own "Current evidence" names the Agent Environment
  Graph; app-server's current equivalent is the skill-compiler/role-compiler
  machinery per `structural-core-ownership.md`, which already owns "role
  identity, label, objective, context lifetime"). Given that, the layering is
  clean rather than overlapping:

  ```text
  Agent Environment Graph / skill-compiler (already implemented)
      owns: role_contract identity/shape
          |
          +--> organizational-execution-envelopes.md
          |       composes MULTIPLE role_contracts + profiles + system
          |       environment into one execution envelope (org topology
          |       for one problem)
          |
          +--> RoleRealization (pre-indexed-capability-resolution)
                  materializes ONE role_contract's concrete provider/
                  harness/tool binding
  ```

  This is `ExecutionEnvelope` (semantic definition of the org/roles a
  problem requires) feeding `RoleRealization` (concrete admitted
  implementation of one role within it) — a `SUPPLIES` relation, not
  competing ownership of "the immutable description of a role's runnable
  environment." Still relevant and worth bringing forward, but the original
  should state this seam explicitly rather than only its relationship to
  hierarchical orchestration. No envelope-compiler implementation evidence
  found anywhere in `app-server/src`.
- **`work-engine-studio.md`** — `operator-switchboard.mjs` is a narrow CLI
  attachment/binding-view parser, nowhere near Studio's five proposed views
  (contract/design, organization, runtime, control, forensics-replay). No
  Studio UI, replay surface, or diagnostics engine found anywhere in
  app-server. Genuinely unaddressed.
- **`ui-review-capability.md`** — no app-server UI-review role, panel, or
  four-lens judgment mechanism found. Explicitly depends on
  `ui-experience-evidence.md`'s evidence boundary and has not itself been
  promoted to a proposal. Correctly still gated behind its dependency.
- **`agent-instruction-structure-and-placement-review.md`** —
  `role-compiler-proposal.md` (read in full) is a mechanical transformation
  system: decompose `SKILL.md` into structured sources, generate projections,
  byte-for-byte round-trip. It performs no normative judgment about whether
  an instruction is correctly placed or belongs at a different governing
  layer — exactly and only what this idea's two-question diagnostic does.
  Complementary: the compiler changes *how* skill sources are structured;
  this idea judges *where* normative text belongs regardless of structure.
  Its dogfood-subject citations predate app-server's skill-compiler work and
  need updating, but the core diagnostic is untouched by anything built this
  session.

## Different disposition

### `ui-experience-evidence.md`

Its own Status line states it is "the authorized idea source for the formed
candidate in `proposals/ui-experience-evidence/family.md`" — confirmed on
disk: the family exists, with one formed candidate
(`ui-experience-evidence-interface/proposal.md`, confirmed present), state
explicitly "not reviewed, evaluated, accepted, prioritized, or authorized
for implementation." No app-server implementation evidence exists for it
either. This idea's job is already done — it produced a proposal. The
actionable next step is evaluating that existing proposal against
app-server, not writing a new `app-server/ideas/pending/` document from the
idea itself. This is a preview of the larger, explicitly out-of-scope
question this document set aside at the top: whether `Promoted` originals'
resulting proposals still hold against app-server. That question is real and
this is one concrete instance of it, but it is not resolved here.

## What this document does not decide

- It does not draft any of the "bring forward" ideas as new
  `app-server/ideas/pending/` documents. That is explicitly separate,
  follow-on work.
- It does not evaluate `proposals/ui-experience-evidence/`'s formed
  candidate against app-server, despite surfacing that it should be.
- It does not check any other `Promoted`-disposition original's resulting
  proposal against app-server — `ui-experience-evidence.md` surfaced this
  as a real category of remaining work, not something this document
  completes.
- It does not authorize implementation, acceptance, or prioritization of
  anything named here.
- It does not amend `ideas/README.md` or `ideas/reconciliation-map.json`
  themselves.
- It does not resolve the `COUPLED_RECONCILIATION` between
  `cross-cutting-seam-review.md` and `architectural-review.md`, or decide
  whether seam review is an instance of `evidence-anchor-observation-and-impact-nomination.md`'s
  observer family — it names the coupling and the candidate connection, not
  their resolution.
- It does not decide where `role-decision-trace.md`'s remaining
  decision-lineage concern is eventually owned — `claim-evidence-service.md`
  and the four-instance revisioned-state kernel-primitive pattern
  (`architecture-direction-synthesis.md` §2.3) are both named as candidates,
  neither selected.

## Relationships

| Direction | Relationship |
| --- | --- |
| `ideas/README.md`, `ideas/reconciliation-map.json` | The reconciliation this document refreshes — scoped to the same 16-file "active clean idea set," using app-server rather than pre-migration `ARCHITECTURE.md` as current-implementation evidence. |
| `app-server/docs/post-migration-strategic-plan.md` | Named this exact refresh as needed work ("historical implementation claims need refreshing against App Server") before this document existed. |
| `app-server/docs/service-plane-inventory.md`, `app-server/docs/claim-evidence-service.md` | The primary implemented-reality evidence this reconciliation checked against. |
| `app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md` | Named as a candidate mechanism `cross-cutting-seam-review.md` may turn out to be an instance of, once the coupled reconciliation with `architectural-review.md` happens. |
| `app-server/docs/architecture-direction-synthesis.md` §2.3 | Its four-instance revisioned-state kernel-primitive finding is named as one candidate (not the selected) eventual owner for `role-decision-trace.md`'s remaining decision-lineage concern. |
| `app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md`, `pre-indexed-capability-resolution-and-frozen-runtime-realization.md`, `hierarchical-planning-and-multi-supervisor-orchestration.md`, `authority-backed-architecture-directions-as-workflow-inputs.md`, `revisioned-research-and-execution-architecture.md`, `evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`, `evidence-anchor-observation-and-impact-nomination.md` | The primary prospective-architecture evidence this reconciliation checked against. |
