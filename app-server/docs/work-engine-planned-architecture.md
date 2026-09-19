# Work Engine: Planned Architecture

## Status

**Rebuilt 2026-09-16 from the 20 canonical architecture views** under [`app-server/docs/architecture/`](architecture/) — 13 truth dimensions, 5 cross-cutting mechanisms, 2 shared substrates. This document supersedes the prior version of this capstone, which synthesized directly from the original reconciliation-queue source documents. That synthesis work is not discarded — it is exactly what produced the 20 views, through an extended architecture-decomposition effort that pressure-tested every candidate dimension, mechanism, and substrate against explicit falsifiers before recognizing it, and ran a full residue audit before this rebuild was authorized. This document's own job now is narrower and more honest: **tell a reader where truth lives and how the pieces compose — not restate everything the views already say.**

**The governing writing rule this document holds itself to, without exception:** every substantive architectural statement here must either (a) derive from a canonical view, cited at the point of use, (b) identify itself as domain detail — a real, concrete instantiation the 20 views were never meant to catalog exhaustively, or (c) appear in §11's explicit deferred-residue ledger. Nothing may be asserted here that isn't traceable to one of those three. If a future addition can't clear that bar, it belongs in a view or in the ledger, not in prose here.

**Authority:** Exploratory synthesis only. This document does not accept any idea, authorize implementation, amend a migration roadmap, or select a permanent schema. It does not resolve anything the views themselves leave open. §10 states, per item, what already has production evidence versus what remains a document — derived from each view's own four-axis status, never from a flat restated label.

---

## 1. Purpose and Governing Principles

> Work Engine is a governed execution substrate for model-mediated semantic work. It externalizes mechanically knowable state, preserves the authority and consequences of semantic judgment, and reconstructs the smallest sufficient world for each bounded decision.

Five principles recur across every one of the 20 views, independently arrived at rather than imposed from above — each is cited here against its now-formal home, not restated as free-floating doctrine:

```text
Contracts constrain what must remain true. Models choose how to make it true.
    -- role-and-contract-structure.md (contract semantics vs. representation);
       material-decision-selection.md (materiality test vs. implementation
       discretion)

Define the space, not the solution.
    -- mechanisms/candidate-resolution-and-admission.md's own reusable shape:
       reduce and validate a candidate space, never supply its domain meaning

Nothing derived becomes authoritative merely by being derived.
    -- authority-and-ownership.md §12, generalized into a standing invariant
       with (as of this rebuild) five independently-arrived-at concrete
       instances: claim refresh, runtime-realization invalidation, plan
       failure, context unfitness, resource-lease supersession, and intent
       projection — see §7

Nothing owned is reconstructed instead of read.
    -- substrates/context-observer.md and substrates/evidence-anchor.md:
       both substrates supply normalized facts to independent consumers
       without ever acquiring the consumers' own semantic authority

Mechanically knowable work belongs in deterministic machinery; inference is
reserved for unresolved semantic consequence.
    -- substrates/evidence-anchor.md's five-state comparator
       (matches/differs/unknown/unsupported/failed); every dimension's own
       "candidate resolution reduces to 0/1/N" pattern
```

**A sixth principle, different in kind from the five above, governs how any of them — or a future mechanism or substrate — gets recognized in the first place.** It was applied throughout this document from the 2026-09-16 rebuild onward without ever being stated as its own rule; it is now a canonical page in its own right rather than capstone prose, so it is cited here, not restated: [`architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md). In short — a candidate pattern is sorted by kind through an explicit classifier, then promoted only after clearing that kind's own eligibility test and showing typed evidence of cross-domain need; an instance count alone is bookkeeping, never the criterion. Every mechanism and substrate in §5–§6 was pressure-tested against that page directly (§13 there for the five mechanisms, §14 for the two substrates); the result, including four real documentation gaps the tests exposed, is recorded there rather than summarized twice here.

---

## 2. How to Read This Architecture

Three architectural kinds, never conflated:

```text
DIMENSION  — owns a class of architectural truth. Changes what Work Engine
             believes or is committed to about its domain.
MECHANISM  — preserves a reusable invariant or transition discipline across
             independently owned truth domains. Cannot supply domain meaning.
SUBSTRATE  — supplies normalized observations/facts to multiple independent
             consumers. Gains no semantic authority from being consumed.
```

Two further coordinate axes survive as genuinely orthogonal to the three kinds above — verified directly, not assumed compatible:

```text
IMPLEMENTATION PLANE — KERNEL / SERVICE / WORKFLOW: which deployment tier
    enforces a given truth. Thin by nature (workspace-coordination's own
    generic lease/fencing shape, now mechanisms/resource-lease-and-
    fencing.md, is the only confirmed KERNEL-shaped primitive found).

DOMAIN — software engineering, browser/UI, research, review-profile
    specializations, Studio, and future domains: orthogonal instantiation,
    not a fourth architectural kind. See §9.
```

A prior, coarser "four functional systems" axis (Realization / Planning-Compilation / Orchestration / Evidence-History) is retired here — it was found to be a redundant, coarser regrouping of the same territory the 13 dimensions already divide precisely, not an independent question.

**Status is four independent axes, never one flat label**, per [`status-grammar.md`](architecture/status-grammar.md):

```text
design:          accepted | proposed | exploratory | superseded
reconciliation:  reconciled | partial | unreconciled | not_applicable
authorization:   exploration_only | design_work_authorized |
                 implementation_authorized | unrecorded
implementation:  none | planned | partial | implemented
```

`unrecorded` authorization is not silence treated as a ceiling — it means no citable "build this" decision was found, distinct from `exploration_only`'s confirmed ceiling. Implementation having occurred is never sufficient by itself to justify `implementation_authorized` — this is the single most-repeated correction across the whole decomposition effort. §10 uses these four axes directly, per item, never a restated flat enum.

**Residue disposition**, used in §11 for everything the 20 views don't own: `HOMED` (a view owns it, cited), `DOMAIN_DETAIL` (a real instantiation, no top-level owner warranted), `DEFERRED` (real, confirmed residue, correctly left open), `ABSORBED` (looked separate, fully reduced into an existing owner), and content genuinely out of this scheme's scope (meta-process commentary, §12).

---

## 3. End-to-End Architectural Cross-Section

The lowering chain from a proposal to an accepted, published implementation, every stage naming its own owner:

```text
proposal formation, evaluation
    -> proposal-evaluation.md: typed estimates (value/risk/complexity/
       reversibility) + comparison-contract/dominance mechanics.
       ACCEPTED, implementation_authorized, none built.
    |
    v
decision-specific readiness
    -> decision-specific-readiness.md: is the evidence sufficient for
       THIS decision, decision-relative not proposal-absolute.
       ACCEPTED, implementation_authorized, none built.
    |
    v
portfolio selection
    -> portfolio-selection.md: PortfolioDecision -- priority, sequencing,
       exclusion, never silently mutating a proposal's own lifecycle.
       ACCEPTED, implementation_authorized, none built.
    |
    v
material decision selection
    -> material-decision-selection.md: the materiality test (route-
       invariance), authority classification (reserved/delegated), the
       decision-set schema. Final selection reuses Candidate Resolution
       and Admission (3rd confirmed instance).
       PROPOSED, exploration_only, none built.
    |
    v
implementation-contract compilation
    -> implementation-contract-compilation.md: the implementation basis,
       compiler (3 honest outcomes), contract schema, plan-conformance
       gate ("establishes plan readiness only"). Owns stage 1
       (contract characterization) of the routing.vs.admission ruling.
       PROPOSED, exploration_only, none built.
    |
    v
executor-class routing / acceptance
    -> the supervisor / routing-policy authority named in the
       routing.vs.admission ruling (2026-09-15, closed) -- stage 2 of
       that same pipeline. Named, but not architecturally homed by any
       of the 20 views. A nomination, advisory until accepted for the
       slice.
    |
    v
runtime resolution / admission
    -> runtime-realization.md: stage 3. Given the accepted class plus
       current capabilities and policy, which exact realization is
       admitted now. Reuses Candidate Resolution and Admission (2nd
       confirmed instance) and Revision/CAS (realization lineage).
       PROPOSED, exploration_only, partial (real precursors only).
    |
    v
execution
    -> runtime-realization.md's own safe-execution-boundary and fenced-
       active-binding content (mechanisms/resource-lease-and-fencing.md,
       2nd confirmed instance: at most one realization generation may
       hold authoritative active-binding for a role instance).
    |
    v
review
    -> review.md: seam review, architectural review, review-scope
       validity, and (confirmed 2026-09-16) post-execution implementation
       conformance -- all one judgment shape, different grain/trigger.
       ACCEPTED, design_work_authorized (mixed per residue item), partial
       (3 real instances: review-finding-bridge.mjs, agent-instruction-
       review, native-review-host.mjs).
    |
    v
acceptance / publication
    -> authority-and-ownership.md (accept/reject/stop consequence,
       capability.checkpoint_lifecycle/accept enforcing non-self-
       authorization in real code) + evidence-and-claims.md
       (production-path-v1 materializes the acceptance fact) +
       mechanisms/revision-cas-and-publication.md (completion-
       publication.mjs's real prepared -> sealed -> published lifecycle,
       7th confirmed instance -- composing three dimensions' outputs,
       owned by none of them alone).
    |
    v
revisioned history / evidence / future impact
    -> evidence-and-claims.md's own may_affect/refresh pipeline, fed by
       substrates/evidence-anchor.md's nomination candidates.
```

**`production-path-v1` is the flagship worked example of this entire architecture's central discipline**, confirmed as real, implemented composition, not merely planned: `production-path-contract.mjs` mechanically throws `"production-path claim is self-authorized"` if the accepting owner is `reviewer`/`builder`/`adapter`/`terminalizer` — enforcing in running code, not just in prose, that **reviewer produces judgment ≠ accepting authority ≠ claim materializer ≠ publication mechanism.**

---

## 4. The 13 Truth Dimensions

Each owns a class of architectural truth no other dimension, mechanism, or substrate supplies.

| Dimension | Owns | Status (design/auth/impl) |
|---|---|---|
| [`semantic-planning-hierarchy.md`](architecture/semantic-planning-hierarchy.md) | The chain by which an objective becomes an accepted orchestration topology, then accepted branch plans, then bounded execution (Preplanner → Orchestrator → Branch Planners → Supervisor → Builders). | proposed / unrecorded / partial |
| [`authority-and-ownership.md`](architecture/authority-and-ownership.md) | Who may observe, recommend, nominate, decide, admit, execute; authority projection and delegation; the generalized invalidation-never-mints-authority invariant (§12). | proposed / unrecorded / partial |
| [`organizational-compilation.md`](architecture/organizational-compilation.md) | Turning accepted semantic work into a lawful execution organization (vantages, roles, bounded authority) without changing what the work means. Recursive, bounded by accepted semantic structure. | proposed / unrecorded / none (§7 override: accepted / design_work_authorized) |
| [`role-and-contract-structure.md`](architecture/role-and-contract-structure.md) | The canonical relation vocabulary a role contract is built from (Agent Environment Graph); contract semantics independent of representation. | accepted / implementation_authorized / partial |
| [`runtime-realization.md`](architecture/runtime-realization.md) | Capability observation, concrete runtime composition, the immutable `RoleRealization` artifact, its invalidation and rematerialization, fenced active-binding. | proposed / exploration_only / partial |
| [`context-lifecycle.md`](architecture/context-lifecycle.md) | When a retained reasoning context should be replaced and how continuation meaning survives replacement — strictly separate from organizational topology. | proposed / unrecorded / partial |
| [`evidence-and-claims.md`](architecture/evidence-and-claims.md) | Claim identity, evidence, provenance, revision/lineage mechanics; materialization is always a subordinate copy, never the source. Three real domain profiles (`proposal-research-v1`, `revision-bound-review-finding-v1`, `production-path-v1`). | accepted / implementation_authorized / implemented |
| [`review.md`](architecture/review.md) | Semantic judgment production for seam review, architectural review, review-scope validity, and post-execution implementation conformance — one shape, four grains. The thinnest-scoped dimension: borrows its mechanical half, materialization, coordination, and acceptance authority from elsewhere. | accepted / design_work_authorized / partial |
| [`proposal-evaluation.md`](architecture/proposal-evaluation.md) | Typed evaluation estimates + comparison-contract/dominance derivation mechanics. | accepted / implementation_authorized / none |
| [`decision-specific-readiness.md`](architecture/decision-specific-readiness.md) | Decision-specific readiness contracts and attributed assessments — decision-relative, not proposal-absolute. | accepted / implementation_authorized / none |
| [`portfolio-selection.md`](architecture/portfolio-selection.md) | The `PortfolioDecision` record: basis, cross-proposal analysis, priority/sequencing/exclusion — never mutating a proposal's own lifecycle. | accepted / implementation_authorized / none |
| [`material-decision-selection.md`](architecture/material-decision-selection.md) | The materiality test, decision-authority classification (reserved/delegated), the decision-set schema for an accepted proposal's unresolved route choices. | proposed / exploration_only / none |
| [`implementation-contract-compilation.md`](architecture/implementation-contract-compilation.md) | The implementation basis, compiler, contract schema, plan-conformance gate — contract characterization only, no slice-level or runtime authority. | proposed / exploration_only / none |

---

## 5. The 5 Cross-Cutting Mechanisms

Each preserves a reusable invariant across independently owned truth domains — never supplying the domain meaning itself. All five pass [`architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md)'s own mechanism eligibility test (§5 there); §13 there records each mechanism's actual typed evidence and two documentation gaps the pressure test exposed — not reclassifications.

| Mechanism | Reusable shape | Confirmed instances |
|---|---|---|
| [`mechanisms/revision-cas-and-publication.md`](architecture/mechanisms/revision-cas-and-publication.md) | Identity as content digest, predecessor chains, computed heads, CAS-published succession. | 7: Evidence/Claims and Context Lifecycle (implemented), Organizational Compilation (accepted design), Semantic Planning (named/proposed), Runtime Realization (`RoleRealization`'s own lineage proposed; partially implemented, 2026-09-16, via its executable-generation substrate — not an 8th instance), Material Decision Selection (proposed), post-execution implementation acceptance (implemented, composing three dimensions' outputs, owned by none alone). |
| [`mechanisms/candidate-resolution-and-admission.md`](architecture/mechanisms/candidate-resolution-and-admission.md) | `AVAILABLE ∩ AUTHORIZED ∩ SATISFIES(REQUIRED) → {0 gap / 1 determined / N residual judgment} → selected → admitted`. | 3 confirmed (Organizational Compilation, Runtime Realization, Material Decision Selection), Portfolio Selection likely a 4th. |
| [`mechanisms/transition-fencing-and-leases.md`](architecture/mechanisms/transition-fencing-and-leases.md) | Preparation-vs-publication: a transition that took real preparation time is still valid by activation time. Two named fence classes (decision-episode, topology-transition) plus a third real instance protecting neither, never collapsed into one lock. | Context Lifecycle (implemented); Runtime Realization's executable-generation substrate (implemented, 2026-09-16 — protects which host-process generation may realize a role's `harness_runtime`, fitting neither named class); Organizational Compilation topology-transition (named, not implemented). |
| [`mechanisms/resource-lease-and-fencing.md`](architecture/mechanisms/resource-lease-and-fencing.md) | Standing mutual-exclusion-with-fenced-handoff: authority to mutate a protected resource is bound to a current lease generation; a stale holder cannot exercise it. Structurally distinct from Transition Fencing (standing relationship, not a one-shot preparation interval). | `workspace-coordination` core (implemented, 7 real resource types) — the definition; fenced active-binding and review-scope protection are new consumers, each accepted for implementation, none built. |
| [`mechanisms/authority-preserving-intent-projection.md`](architecture/mechanisms/authority-preserving-intent-projection.md) | Operator/human intent → bounded candidate under the current authority envelope → domain-owned admission → effect → lifecycle feedback (proposed/pending/admitted/refused/completed/stale). May constrain and encode authority; may never enlarge it. | Studio's command/edit projection (accepted design, not built), Runtime Realization's operator policy overlay (real, partial, independently arrived at). |

---

## 6. The 2 Shared Substrates

Each supplies normalized observations to multiple independent consumers without acquiring their semantic authority. Both pass [`architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md)'s own substrate eligibility test (§6 there), and Context Observer also establishes that page's canonization warrant (§7.1) cleanly. `substrates/evidence-anchor.md` does not currently establish warrant: its own "Multiple Independent Consumers" claim turns out, on inspection, to be one dimension's own domain-profile parameterizations (`architecture`/`plan`/`research` claim, all owned by `evidence-and-claims.md`) rather than independent cross-domain convergence, and its one genuine cross-domain candidate is a passing mention, not an accepted design (§14 there, in full). This substrate's `design: proposed` status and its presence in this table do not by themselves establish any taxonomy-admission history one way or the other — `design: proposed` proves only the absence of a design-acceptance event, and this table's own listing proves only that a rendering exists (per §7.2 there, this capstone renders, it does not publish or admit); neither fact is evidence for or against a past taxonomy-admission event, which nothing currently records or could verify (capstone §11 item 6). What §7.1 does establish, checked directly rather than inferred from either of those facts, is that this substrate's own warrant is not currently reconstructable — its "Multiple Independent Consumers" claim turns out to be domain-profile parameterizations of one dimension, not independent convergence. That gap is what §7.3 there records as a **corpus-conformance conflict.** This rendering of the entry stands exactly as listed below, neither removed nor grandfathered, and `substrates/evidence-anchor.md`'s own `reconciliation: reconciled` status is unaffected (`status-grammar.md` §4.2: a later document creates new reconciliation work, it does not retroactively falsify an earlier, scope-relative marker). **This capstone is not the authority that resolves it, and this table is a rendering, not the publication act itself** — per this document's own Authority line above, it does not resolve anything the views themselves leave open, and per the recognition page's own §7.2, authoritative publication runs through `mechanisms/revision-cas-and-publication.md`'s own discipline in principle, never through an edit to this file. Resolution requires an authority-bearing admission act (§7.2 there), sourced from whatever eventually answers the open residue named in `authority-backed-architecture-directions-as-workflow-inputs.md` §18 item 1 ("who owns architecture decisions, and how is that authority represented and verified") — not from this capstone and not from the recognition page itself.

| Substrate | Owns | Consumers |
|---|---|---|
| [`substrates/context-observer.md`](architecture/substrates/context-observer.md) | The `ContextObservation` schema and normalization contract for reasoning-environment facts — never external reality itself, never a derived metric. | Context Lifecycle and Organizational Compilation, as independent, equally-ranked peers. |
| [`substrates/evidence-anchor.md`](architecture/substrates/evidence-anchor.md) | One observer per anchor kind, exact-revision-bound observation, a five-state mechanical comparator, `may_affect` nomination candidates ("beautifully weak" — never a judgment about whether a difference matters). | Architecture, plan, and research claims (named siblings in the source's own "impact nomination substrate" diagram); the coordinate/service-state map as a candidate fourth. |

---

## 7. Authority and Invariant Relationships

**No authority minting.** `authority-and-ownership.md` §12's generalized invariant — "failure of an authorized candidate does not authorize a previously unauthorized alternative" — now has independently-arrived-at concrete instances across five different mechanisms and dimensions: claim refresh (Evidence/Claims), runtime-realization invalidation, resource-lease supersession (`mechanisms/resource-lease-and-fencing.md`), intent projection (`mechanisms/authority-preserving-intent-projection.md`), and Candidate Resolution and Admission's own rerun-against-unchanged-ceiling behavior.

**Source truth vs. derived state.** Every materialization pattern in this architecture is a subordinate copy, never the source: `evidence-and-claims.md`'s own claims materialize planning, organizational, review, and production-path facts without ever becoming their authority; `substrates/context-observer.md` and `substrates/evidence-anchor.md` supply facts, never derived consumer-specific metrics.

**Projection vs. ownership.** `mechanisms/authority-preserving-intent-projection.md`'s own governing line: "The UI does not decide what is authorized. It projects the existing authority model." A projected candidate exposes or encodes authority already granted; collecting or displaying an intent never grants standing beyond it.

**Judgment vs. admission.** `review.md` produces judgment; `authority-and-ownership.md` owns acceptance and disposition. `strategic-planning-handoff.mjs` and `capability.checkpoint_lifecycle/accept` are both real, concrete instances of this separation, the latter mechanically enforcing that the accepting owner may never be the producer or reviewer.

**Invalidation behavior.** Rematerialization after invalidation (`runtime-realization.md` §8) and recursive organizational compilation after local evidence confirms a plan (`organizational-compilation.md` §12) are both, structurally, an ordinary rerun of Candidate Resolution and Admission against unchanged authority — never an occasion to widen what's permitted.

---

## 8. Runtime, Organizational, and Lifecycle Composition

`organizational-compilation.md` and `context-lifecycle.md` are independent, equally-ranked consumers of the same shared `substrates/context-observer.md` and the same shared `mechanisms/transition-fencing-and-leases.md` — neither senior to the other, neither owning the mechanism. A stable boundary test: a role boundary usually implies a vantage boundary; a vantage boundary usually implies a context boundary; a context boundary does not imply a role boundary — so most context-lifecycle work never needs to touch organizational compilation at all.

`runtime-realization.md` composes with `mechanisms/resource-lease-and-fencing.md` for fenced active-binding (which realization generation currently holds authoritative right to execute as a role — the dimension's own decision; the mechanism's own fencing mechanics make it race-safe and host-enforced) and with `mechanisms/authority-preserving-intent-projection.md` for its own operator-policy-overlay (a real, partial instance, independently arrived at with zero cross-reference to the mechanism's other consumer, Studio).

The executor-class-routing seam remains the one deliberately-unresolved gap in this otherwise-complete chain: stage 2 (executor-class routing/acceptance) is named — "the supervisor / the routing-policy authority" in the `routing.vs.admission` ruling's own Stage 6 — but not architecturally homed by any of the 20 views. Named consistently wherever it appears (`runtime-realization.md` §5, `role-and-contract-structure.md` §6, `implementation-contract-compilation.md` §6), never silently resolved.

---

## 9. Domain-Specific Architecture

The DOMAIN axis (§2) composes onto the 13-dimension grid without requiring a new architectural kind. Real, grounded domain instantiations found this session, each `DOMAIN_DETAIL` per §11's own disposition scheme:

- **Software engineering** — code evidence via `codebase-memory-mcp` (confirmed permanently read-only), `CodeEvidenceAdapter`, Candidate Trajectory family (real-code evidence, not yet architecturally homed beyond `substrates/evidence-anchor.md`'s own general observer pattern).
- **Browser/UI** — AI-Accessible Browser, `ui-experience-evidence-interface` (formed, undecided), `UIReviewProfile` (a domain-specific instantiation of `review.md`'s own judgment shape, explicitly self-excluded by that page as "not any one domain's own concern list" — confirmed `DOMAIN_DETAIL`, not a competing dimension).
- **Research** — historical execution coordinates, capability learning, `incremental-terminal-accounting-projection.md` — real, execution-observation infrastructure, uncited by any of the 13 dimensions by design.
- **Operator/Studio** — `control-plane-causal-observability-ui.md` (an already-accepted, uncited-elsewhere design), `mechanisms/authority-preserving-intent-projection.md`'s own Studio instance (§5).

None of these require their own dimension — the 13 dimensions were never meant to be an exhaustive implementation ledger, and inventing an architectural home for every real code artifact would turn the views into a directory index rather than a truth map.

---

## 10. Implementation-State Atlas

Per-item four-axis status, derived directly from each item's own canonical view — never a restated flat label. This replaces the prior capstone's single-enum table entirely; no lossy translation was attempted between the old vocabulary and this one, per explicit instruction earlier in this decomposition effort.

| Item | Owning view | design | reconciliation | authorization | implementation |
|---|---|---|---|---|---|
| Claim/evidence core substrate | `evidence-and-claims.md` | accepted | reconciled | implementation_authorized | implemented |
| `production-path-v1` | `evidence-and-claims.md` §2 | accepted | reconciled | implementation_authorized | implemented |
| Review judgment (3 instances) | `review.md` §3 | accepted | reconciled | design_work_authorized | implemented (3 instances), partial (general form) |
| Agent Environment Graph / role-compiler | `role-and-contract-structure.md` | accepted | reconciled | implementation_authorized | implemented (read side) |
| `workspace-coordination` (resource lease/fencing core) | `mechanisms/resource-lease-and-fencing.md` | accepted | reconciled | **unrecorded** | implemented |
| Execution-envelope compiler | `organizational-compilation.md` §7 | accepted | reconciled | design_work_authorized | none |
| Fenced active-binding | `runtime-realization.md` §12 | accepted | reconciled | implementation_authorized | none |
| Review-scope protection | `review.md` §7 item 5 | accepted | reconciled | implementation_authorized | none |
| Seam-evidence adapter extensions | `substrates/evidence-anchor.md` | accepted | reconciled | implementation_authorized | none |
| Proposal evaluation / readiness / portfolio (front-end chain) | respective views | accepted | reconciled | implementation_authorized | none |
| Material decision selection / implementation-contract compilation | respective views | proposed | reconciled | exploration_only | none |
| Studio command/edit projection | `mechanisms/authority-preserving-intent-projection.md` | accepted | reconciled | design_work_authorized | none |
| `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` | `runtime-realization.md` | proposed | reconciled | **exploration_only** | partial |

**One correction applied here that the prior capstone's own flat enum could not express and had gotten wrong**: `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` was previously labeled `AUTHORIZED_DIRECTION`, overstating its own source's explicit `exploration_only` ceiling ("does not authorize implementation"). `runtime-realization.md`'s own retrofit corrected this; this atlas now inherits the corrected value directly rather than the old, mistaken one.

**`implementation: partial` sharpened, not changed, 2026-09-16**: the executable-generation lifecycle (real, tested code) implements `runtime-realization.md` §9 (safe execution boundaries) and §10 (revision/CAS lineage) at the executable substrate layer beneath `RoleRealization` — the same real code is now a second confirmed instance of `mechanisms/transition-fencing-and-leases.md` and strengthens `mechanisms/revision-cas-and-publication.md`'s own Runtime Realization row without adding an 8th instance to its count of seven. §8's own `RoleRealization` rematerialization through Candidate Resolution and Admission remains unbuilt; this does not change the dimension's own three-value status above, only what evidence supports the `partial` value.

For every item not listed here, consult the owning view's own "Source and Status" section directly — this atlas is not exhaustive, and never claims to be a substitute for the views themselves.

---

## 11. Deferred Architecture

Real, confirmed residue the 20 views do not yet own — correctly left open, not smoothed over. Each was pressure-tested against an explicit null hypothesis before being placed here; none is a brainstorm. Items 1, 3, and 4 are held open specifically by the cross-domain-evidence route named in [`architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md) §7 — no second independent domain has converged on them yet; that is a claim about current evidence, not about whether the pattern is real. Items 5 and 6 deliberately use that page's own owner-decision escape valve instead of waiting on convergence that may never come.

**1. `Coordinate` / `service_state`** (`service-plane-and-kernel-domain-boundary.md`). `service_state` itself is substrate-shaped (purely referential, no consumer-specific derivation) but blocked on two things: a stated population/inclusion rule for what belongs in one given coordinate instance (no primary source answers this), and confirmed shared-consumer evidence beyond one named candidate. `coverage` is explicitly "not accepted or applied" per its own source. `frontier`'s own membership is an open numbered question in its own source document. Resolved by: a stated inclusion rule, plus either `coverage`'s acceptance or its replacement.

**2. The plane×system×DOMAIN grid's function axis.** §2 already resolved plane and DOMAIN as orthogonal survivors; the function axis (Realization/Planning-Compilation/Orchestration/Evidence-History) is retired as redundant with the 13-dimension taxonomy, not deferred — listed here only for completeness of the original finding, already acted on in §2.

**3. Semantic judgment relations** (`PREMISE_FOR`/`WEAKENS`/`CONTRADICTS`). Real residue: no domain currently authors these edges, and no adjudication owner exists for what a `CONTRADICTS` edge implies. Category (dimension/mechanism/substrate) is genuinely undetermined — leans structurally closer to a mechanism (reused across whatever eventually owns each judgment class) by analogy to Revision/CAS, but this is not established. Zero second consumer found among the 13 dimensions today. Blocked upstream on `role-decision-trace-reconciliation.md`'s own still-open question: a general revisioned-state primitive, or growing from `material-decision-selection.md`'s own sealed decision-set architecture. Resolved by: that upstream choice, then a real second consumer or an explicit single-domain scoping.

**4. Governing-judgment state** (active/superseded/retired/promoted transitions). Real residue: the source explicitly treats "currently governs execution" as an open ownership question, not a mere consequence of admission — confirmed that a judgment can remain historically valid while no longer governing, with no existing owner (`authority-and-ownership.md` §6 and `mechanisms/candidate-resolution-and-admission.md` both end at a one-shot "admitted," with no post-admission governing-state vocabulary). Checked directly against material decisions, readiness, organizational compilation, and review — zero convergence found. Resolved by: a second independent domain organically developing the identical distinction, or an explicit decision to scope this to one domain only.

**5. Unowned judgment classes** — interpretations, assumptions, and placement judgments (as their own classes, distinct from material decisions and now-homed sufficiency judgments). No dimension owns any of the three. Resolved by: an owner decision for each, following the same pressure-test discipline used throughout this decomposition — not assumed to need one shared home.

**6. Architecture taxonomy membership** — the proposition "X is currently an admitted Dimension/Mechanism/Substrate." Found by [`architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md) §7.2 pressure-testing its own admission concept one step further, then re-audited once more against a distinction its own first pass had conflated: **decision authority, membership-state ownership, the publication mechanism, and projection are four different things, and disproving the first does not by itself disprove the second.**

```text
decision authority        who may decide to change membership
                           -- still open (item 1, above)

membership-state owner     who owns the durable record of current
                           membership -- the question this item tests

publication mechanism      how an owned change gets safely propagated
                           -- mechanisms/revision-cas-and-publication.md's
                           own discipline, in principle, not owning what
                           the state means

projection                 a rendering of whichever state currently
                           holds, for a reader
```

This capstone's own Authority line ("does not accept any idea... does not resolve anything the views themselves leave open") proves only that it lacks decision authority — it does not, by itself, prove it lacks membership-state ownership; a dimension can legitimately own durable state it did not decide (`evidence-and-claims.md` owns claim identity and revision without ever deciding whether the underlying proposition is true). The real test, run directly against this capstone's own declared responsibilities rather than inferred from its Authority line alone: **does this capstone already own the current membership/status inventory, receiving externally authorized updates, while decision authority stays elsewhere — or is it only a projection of a membership state whose owner is genuinely absent?**

Choosing between the two requires positive evidence, not merely the absence of decision authority, and this capstone's own text supplies it directly. Its own Status section states its job as "tell a reader where truth lives and how the pieces compose — not restate everything the views already say" — a self-description as a pointer to truth held elsewhere, not as the truth's holder. Its own governing writing rule is stronger still: "every substantive architectural statement here must either (a) derive from a canonical view, cited at the point of use, (b) identify itself as domain detail... or (c) appear in §11's explicit deferred-residue ledger. Nothing may be asserted here that isn't traceable to one of those three" — a membership-table row is exactly such a substantive statement, and by this capstone's own rule it cannot originate one; §5's and §6's own tables comply by citing each candidate's own page and `architectural-kind-recognition.md`'s own pressure test (§13, §14) as the source, never asserting membership on this document's own standing. Procedurally, this capstone is periodically **rebuilt** "from the 20 canonical architecture views," not incrementally maintained as a standing ledger receiving discrete authorized edits, and its own Non-goals state the taxonomy "may need revision under further reconciliation, exactly as this rebuild itself was produced by revising the taxonomy that preceded it" — change flows through reconciliation elsewhere, then a rebuild follows, rather than this document being the thing directly amended. That is option **C**: this capstone is explicitly, by its own declared discipline, only a projection of a membership state whose owner is genuinely absent — not option A (no positive evidence supports this capstone itself holding the register) and not option B (no other existing dimension, mechanism, or substrate covers it either, per the check already performed above). Item 6 survives, on stronger grounds than its first statement gave it.

**One consequence follows only from C, and is stated as conditional rather than absolute because of it:** editing this capstone's own tables cannot make an admission authoritative *because* this capstone holds no state of its own to authoritatively change — an edit here only updates a rendering, and the real change, if C is ever superseded by an owner decision that this capstone (or some other artifact) should instead **be** the membership register, would need to say so explicitly; direct capstone editing would then become exactly how an externally authorized update *is* implemented, not something it "can never" do. That reclassification is not performed here — it is the same owner decision item 6 already defers, not a conclusion this re-audit reaches on its own.

**Pressure-tested against one further null before treating this as a new truth class at all: can membership be deterministically reconstructed from already-owned authoritative state without semantic judgment?** Checked, not assumed — no. It is not reducible to the `status-grammar.md` axes already owned by each candidate's own page: `substrates/context-observer.md` is `design: proposed`, not `accepted`, yet is already listed in §6 above as a current substrate, so "design: accepted" cannot be the deterministic membership rule real practice actually follows. Nor is there any other already-owned field this could be computed from — the actual test applied, `architectural-kind-recognition.md`'s own eligibility and warrant criteria, requires reading and judging each candidate's own prose (does removing domain nouns leave a real invariant, does substantial residue survive refusal, does convergence genuinely hold) — a semantic-judgment act every time, not a stored fact to look up. Membership is therefore retained as genuine unowned truth, not reframed as a derived projection. This ledger entry does not name or imply a home for it — not a fourteenth dimension, not any existing one — since doing so would be exactly the owner decision this item defers. Resolved by: an explicit owner decision naming a home for this truth.

---

## 12. Architecture Maintenance Risk

Distinct from §11: not an unresolved architectural boundary, but a standing process risk this decomposition effort caught repeatedly, not once — independently evolving pages can drift into stale counts, one-directional cross-references, and silently-regressed rulings even when each page's own content is individually sound. Concrete evidence from this session alone: a closed ruling (`routing.vs.admission`) was found quietly restated as open in two views after a later synthesis pass rewrote them without checking the closure; three separate stale instance-counts were found in mechanism pages after new instances were added elsewhere; one status page understated its own authorization tier after a sibling reconciliation narrowed it.

There is no single fix. The 20-view structure and its four-axis status grammar are the current mitigation — every page owning itself, every cross-reference checked bidirectionally, every count independently re-derived rather than restated — not a one-time correction. Any future addition to `app-server/docs/architecture/` should be checked against the existing 20 views for overlap and against this document's own §4–§9 before being treated as independent.

---

## 13. Source and Provenance Map

| Layer | Role |
|---|---|
| [`app-server/docs/architecture/`](architecture/) (20 views) | The primary, canonical layer. Every architectural claim in this document traces here first. |
| [`status-grammar.md`](architecture/status-grammar.md) | Defines what the four status axes mean; never the semantic owner of any specific claim. |
| The original sequel reconciliation queue (`*-reconciliation.md` documents in `app-server/docs/`) | The deepest source layer each view's own "Source and Status" section cites directly. Retains its own full disposition tables and evidence; this document does not restate their reasoning. |
| `app-server/ideas/pending/` source proposals | Where a view's own content is still `proposed`/`exploratory`, its owning idea document is the authority on open questions, not this capstone. |
| The prior version of this capstone | Historical cross-section and provenance source only, per this rebuild's own governing rule (Status, above) — no longer an independent synthesis layer. |

---

## Non-goals

- This document is not itself a proposal and authorizes no implementation.
- It does not establish decision authority, priority, or sequencing beyond what each cited view or source document already states.
- It does not claim completeness beyond the 20 views' own declared scope — real domain systems, root-substrate code, and pending ideas outside that scope may exist and are not represented here except as named `DOMAIN_DETAIL`.
- It does not resolve anything listed in §11. Naming a residue item is not deciding it.
- It does not treat the dimension/mechanism/substrate taxonomy, the plane×DOMAIN axes, or the status grammar as permanently final — each may need revision under further reconciliation, exactly as this rebuild itself was produced by revising the taxonomy that preceded it.
- It does not restate any view's own reasoning where a citation suffices — see §13.
