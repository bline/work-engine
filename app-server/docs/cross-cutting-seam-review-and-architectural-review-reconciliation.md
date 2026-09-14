# Reconciliation: Cross-Cutting Seam Review and Architectural Diagnostic Review

## Status

Joint reconciliation of `ideas/cross-cutting-seam-review.md` and
`ideas/architectural-review.md` against current app-server implementation
and current prospective architecture. Wave 3, items 8–9 of the sequel
reconciliation queue, treated as one unit per
`root-ideas-reconciliation.md:170-227`'s `COUPLED_RECONCILIATION` finding,
which explicitly named the risk of reconciling them separately as "exactly
the failure mode `incremental-architecture-intake-and-seam-reconciliation.md`
exists to catch."

Per instruction: neither idea was improved or modernized. Every clause was
checked against implemented code, `app-server/ideas/pending/`, and existing
prospective-architecture documents.

## Idea summaries (unchanged)

`ideas/cross-cutting-seam-review.md` asks whether independently reasonable
components remain coherent *at their seams* — whether two otherwise-valid
parts "correspond truthfully and proportionately" (its own compact
statement), across seam types including principle↔implementation,
docs↔implementation, state-ownership↔UI-representation,
authority↔exposed-control, provenance↔displayed-certainty,
capability-contract↔provider-realization, and proposal-expectation↔
implementation-consequence. It explicitly distinguishes itself from
architectural review (model-correctness judgment) and from UI review
(human-interface representation quality specifically), and states it should
be consequence-selected, not a mandatory gate.

`ideas/architectural-review.md` asks for a dedicated diagnostic capability
whose explicit subject is whether the system model, ownership,
decomposition, or placement is wrong. Its output is evidence for proposal
formation, not a repair. It defines an authority boundary: it may diagnose,
challenge ownership/placement, recommend proposal formation or reopening,
and emit an attributed pause/stop recommendation with severity, confidence,
expected consequence of continuing, and limitations — but it does not
author the architecture, accept a proposal, authorize implementation, or
grant itself blocking authority. Its own "Relationship to other ideas"
section already names proposal research maturity (item 5) as the sufficiency
gate for relying on its diagnosis, cross-cutting seam review as a distinct,
non-substitutable capability, and organizational execution envelopes (item
3) as a future consumer it does not own.

## What a prior pass already established, re-verified rather than re-derived

`root-ideas-reconciliation.md:170-227` already read both documents in full
and found they already anticipate and answer the seam-review/architectural-
review boundary question consistently with each other (quoting both
documents directly), concluding the relationship is `SUPPLIES` not
`CONFLICTS`: "a seam-review mismatch is a plausible *input* to architectural
review, not a competing diagnosis of the same question." It also surfaced,
independently of either idea's own authors, that seam review's mechanism —
"which boundary is under review; which independent contracts/principles
meet there; evidence from each side; mismatch or disproportion" — closely
resembles `evidence-anchor-observation-and-impact-nomination.md`'s
anchor/observer/comparator/`may_affect` boundary, generalized from
code-citation anchors to a broader anchor-kind taxonomy, and asked whether
that generalization actually holds. This document resolves that question
directly rather than re-asserting the resemblance.

## Question 1: is seam review's mechanism an instance of the anchor/observer boundary, or genuinely distinct?

**Partially an instance, partially not — and the split falls exactly where
`evidence-anchor-observation-and-impact-nomination.md`'s own design already
draws a boundary.**

Re-read in full, `evidence-anchor-observation-and-impact-nomination.md`'s
comparator is deliberately, explicitly restricted to purely mechanical
comparison: "the comparator does not decide whether a change matters — the
claim author already decided that when it declared the relationship as a
dependency. The comparator only decides whether the declared relationship
still holds," returning exactly one of `matches`/`differs`/`unknown`/
`unsupported`/`failed`. Its own §9 ("What this document does not decide")
states it does not let the comparator "judge whether a mechanically observed
difference is semantically important."

Seam review's own compact statement asks whether two parts "correspond
**truthfully and proportionately**" — not whether a declared, mechanically
checkable relation still holds, but whether a correspondence is still an
honest, well-proportioned representation. For several of seam review's own
example seams (docs↔implementation, capability-contract↔provider-
realization), this can genuinely be mechanical: does the documented function
signature still match the code, does the declared capability contract still
match what the provider realizes. For others (state-ownership↔UI-
representation, authority↔exposed-control, provenance↔displayed-certainty),
"truthfully and proportionately" is closer to an interpretive judgment —
does this UI element still honestly communicate what the underlying state
means, not merely whether a byte-identical field is displayed.

```text
mechanical correspondence checking
    "does the declared relationship still hold" (matches/differs)
    -> evidence-anchor's existing anchor/observer/comparator pattern,
       generalized by anchor kind (already extensible; not a new mechanism)

semantic correspondence judgment
    "does this correspondence still hold truthfully and proportionately"
    -> a domain-owner judgment, structurally the same slot evidence-anchor's
       own pipeline already reserves ("domain owner judges consequence"),
       not something the comparator itself may do
```

This means seam review does not need to become, and should not become, a
new standalone mechanism for the mechanical half — that is already owned,
and the correct action is extending the anchor-kind taxonomy (see residue
below), not building a parallel observer/comparator. But seam review is also
not reducible entirely to a re-skin of evidence-anchor: the semantic
"truthfully and proportionately" judgment for non-mechanically-decidable
seams is a real, distinct consequence that evidence-anchor's own design
explicitly refuses to make and explicitly leaves to a domain owner.

## Question 2: the explicit authority/invocation boundary between seam review and architectural review

Confirmed directly rather than re-quoted: the two ideas ask genuinely
different questions with different authority shapes.

```text
seam review
    question: do two independently-valid things still correspond
              (truthfully, proportionately)?
    scope: one boundary, two known sides
    output: a finding plus an attributed routing/disposition judgment
            (may flag architectural diagnosis as warranted), confidence,
            limitations
    authority: none beyond producing the finding; consequence-selected,
               not a mandatory gate

architectural review
    question: is the system MODEL, ownership, decomposition, or placement
              itself wrong?
    scope: the architecture as a whole, or a subsystem's design
    output: a diagnostic finding, competing explanations, an optional
            attributed pause/stop recommendation (severity, confidence,
            expected consequence of continuing)
    authority: may diagnose and recommend; does not author repair, accept
               a proposal, authorize implementation, or grant itself
               blocking authority
```

The relation is `SUPPLIES`: a seam review finding whose routing/disposition
judgment flags architectural diagnosis as warranted is exactly the trigger
condition for invoking architectural review, per both ideas' own text ("A
seam finding may trigger architectural review" / "Cross-cutting seam review
judges coherence across boundaries... it is not a substitute for
architectural diagnosis"). What that routing/disposition judgment actually
looks like — a fixed taxonomy, tags, or something else — is not decided by
either idea and is left open in the residue below, rather than settled here
as a scope enum.

## What is already implemented or already correctly deferred

### `evidence-anchor-observation-and-impact-nomination.md` (retires the mechanical half of seam review — narrowly)

Retires: **deterministic observation and comparison of an already-declared
seam dependency**, for any seam expressible as a mechanically-decidable
anchor/observer/comparator relationship, to the `EvidenceAnchorObserver`
pattern. This is narrower than "the `may_affect` nomination is seam review's
mismatch output" — that phrasing collapsed two distinct steps the source
document itself keeps separate:

```text
declared dependency
        |
        v
fresh observation
        |
        v
comparison = differs        <- the mechanical mismatch itself

comparison = differs
        +
known dependent claim/fact
        |
        v
may_affect nomination       <- a later step, against a KNOWN dependent
                                claim/fact, not the mismatch itself
```

Evidence-anchor's own §5 is explicit that the comparator "only decides
whether the declared relationship still holds" and never judges semantic
importance — `may_affect` requires a known dependent claim/fact to nominate
against, which presupposes the dependency was already declared. Evidence-
anchor does not discover seams; something else still has to declare "these
two things are expected to correspond" before its deterministic machinery
can test that correspondence. What retires here is the observation/
comparison mechanics once a seam dependency is declared — not the discovery
of the seam itself, and not automatically the full `may_affect` pipeline.

### `claim-evidence-service.md` (couples architectural review's finding shape, following the item-4/5/6 domain-profile pattern)

Architectural review's own "Required consequence" list — the architectural
claim being challenged, observed symptoms/evidence, suspected defects,
affected contracts/invariants, competing explanations, confidence and
limitations, and reopening conditions — matches claim-evidence's existing
claim/finding shape closely: `review-finding-bridge.mjs`'s
`revisionPayload` already carries `assumptions`, `limitations`,
`confidence`, `evidence_references`, and `reopening_conditions` for one
domain profile (review findings). `claim-evidence-service.md` names "two
initial domain profiles" as "initial," not exhaustive.

This is the same shape of coupling found in items 4, 5, and 6: the
substrate an architectural-diagnostic-finding record would need already
exists and has already been demonstrated twice (review findings,
proposal-research). An "architectural finding" domain profile is buildable
on that substrate, not free-standing unowned territory — but it does not
exist yet, and this reconciliation does not assume `claim-evidence` intends
to host it merely because the fields match, per the same caution items 4 and
6 required after an initial overreach.

### `strategic-planning-handoff.mjs` (confirms "Blocking consequence remains separately owned" is achievable — a concrete campaign-level consumer, not the universal owner)

Architectural review's own text is careful not to claim blocking authority
for itself, and explicitly defers "whether a finding actually pauses a
campaign" to "the owning campaign, planning, or human-authority contract."
This is not merely a sound design principle — it points at a real,
already-implemented mechanism. `app-server/roles/strategic-planning-handoff.mjs`
(found and read in items 5 and 7's reconciliations) already defines exactly
this authority: a verdict enum
(`continue`/`revise`/`pause`/`reorder`/`split_campaign`/`stop_campaign`)
issued by the strategic-planning handoff process for one campaign, with a
real, structured instance already dogfooded in
`post-migration-strategic-plan.md`'s "Strategic planning handoff" block
(confirmed in item 7's reconciliation).

Disposition: **boundary confirmed, not retired to one universal owner.**
`strategic-planning-handoff.mjs` is a concrete, real, implemented owner for
*campaign-level* pause/revise/reorder/stop consequences — this demonstrates
the separation architectural review's text asks for is genuinely achievable,
not merely aspirational. But it is one consumer among several an
architectural finding might route to, not the universal blocking owner
architectural review defers to in general:

```text
architectural finding occurs
    during an active campaign
        -> campaign strategic planning may own pause/revise/stop
           (strategic-planning-handoff.mjs — a real, concrete instance)
    during proposal formation
        -> the proposal workflow may own reconsideration
    against an accepted architecture direction
        -> architecture authority may own reopening or supersession
    outside any campaign
        -> human/product authority may own disposition
```

The idea's own invariant — architectural review does not acquire blocking
authority — is broader than `strategic-planning-handoff.mjs` alone. Treating
that one mechanism as fully retiring the section would accidentally imply
the campaign strategist is the architecture authority, which neither idea
claims.

## What the idea's own boundary already gets right

Both ideas' distinctions from each other and from neighboring capabilities
are retired as accurate: architectural review's own cross-references to item
3 (organizational execution envelopes, "a future consumer... does not own
this diagnostic function") and item 5 (proposal research maturity,
"determines whether enough evidence exists to rely on the diagnosis")
already match those items' own reconciliations independently. Seam review's
"Distinction from UI review" section is not re-verified in depth here —
`ui-review-capability.md` is queued for Wave 4 and blocked on evaluating the
`ui-experience-evidence` proposal — but no conflict is apparent from seam
review's own framing (seam review "may have no human-facing surface at
all," a strictly broader scope than UI review's human-interface-specific
question).

## The smallest remaining semantic consequence still lacking an owner

Decomposing rather than treating either idea as one residue, four concrete
pieces remain, none owned by evidence-anchor, `claim-evidence`, or
`strategic-planning-handoff.mjs`:

```text
1. Mechanical seam evidence realization
   Selected seam types need evidence-source adapters, and, only where
   existing anchor kinds cannot truthfully express the dependency,
   additional anchor kinds. This is deliberately not predeclared as four
   new kinds (UI-representation, authority-declaration, documentation-
   narrative, capability-contract/provider-realization): some may already
   decompose onto the existing four (a documentation narrative may be a
   TextAnchor; an authority declaration or capability contract may be a
   TextAnchor/ServiceStateAnchor/ImplementationRevisionAnchor). UI/browser
   evidence may genuinely need its own adapter, but
   `ai-accessible-browser-seam-reconciliation.md` already deliberately left
   open whether browser dependency observation is literally an
   `EvidenceAnchorObserver` instance. The exact taxonomy is
   implementation-evidence-driven, not decided here.

2. The semantic "truthfully and proportionately" correspondence judgment
   Who makes it, and how it is recorded, for seams that are not
   mechanically decidable (state-ownership <-> UI-representation,
   authority <-> exposed-control, provenance <-> displayed-certainty).
   Evidence-anchor's own pipeline reserves a "domain owner judges
   consequence" slot for exactly this, but no owner or record shape for
   that judgment, specific to seam correspondence, exists anywhere.

3. Attributed seam-finding disposition and consumer routing
   Not a fixed enum. The first pass of this reconciliation proposed
   promoting "local/architectural/documentation/UI/workflow" into a shared
   taxonomy and calling it the mechanical trigger for architectural
   review -- that classification is itself semantic (a finding can easily
   be UI-and-architectural, or documentation-and-local at once; the labels
   do not form one clean dimension) and should not be frozen. What is
   actually needed is narrower: a seam finding must be able to carry an
   attributed routing/disposition judgment indicating that architectural
   diagnosis is warranted, so the handoff after that judgment can be
   mechanical. Whether this becomes a multidimensional taxonomy, tags, or
   another representation remains open.

4. An architectural-finding domain profile, and its routing to an owning
   decision boundary
   Coupled to claim-evidence's existing domain-profile pattern (same
   shape as items 4/5/6's couplings), not built anywhere. Would carry
   architectural review's diagnostic shape (claim challenged, evidence,
   competing explanations, confidence, limitations, reopening conditions)
   and an optional pause/stop recommendation. What that recommendation
   connects to is not one universal verdict mechanism -- it is whichever
   owning decision boundary applies (campaign strategic planning is one
   concrete, real first consumer via `strategic-planning-handoff.mjs`;
   proposal reconsideration, architecture-direction reopening, or human
   authority are others named by the idea itself). That routing step is
   itself unbuilt for any of them.
```

```text
declared seam expectation
        |
        v
mechanical observation/comparison where possible
    EvidenceAnchorObserver family (existing anchor kinds, extended only
    where evidence shows an existing kind cannot express the dependency)
        |
        v
evidence: matches / differs / unknown / unsupported / failed
        |
        v  missing (semantic judgment's owner, for non-mechanical seams)
semantic seam judgment where needed
    "does this correspond truthfully/proportionately?"
        |
        v  missing (attributed routing/disposition concept itself)
attributed routing/disposition
    local/domain consequence, or architectural diagnosis warranted
        |
        v  missing (architectural-finding domain profile, coupled to
                     claim-evidence's pattern)
architectural finding
    evidence, competing explanations, severity/confidence, recommendation
        |
        v  missing (the routing step itself, for any owner)
applicable owning decision boundary
    campaign strategic planning (strategic-planning-handoff.mjs -- one
        real, concrete consumer) / proposal owner / architecture authority /
        human authority
```

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Seam review: mechanical observation/comparison of a declared dependency | Retired — `evidence-anchor-observation-and-impact-nomination.md`'s `EvidenceAnchorObserver` pattern. Narrower than "retires `may_affect`": `differs` is the mechanical mismatch; `may_affect` is a later nomination requiring a known dependent claim/fact, not the mismatch itself. |
| Seam review: semantic "truthfully and proportionately" judgment | **Not retired.** A domain-owner judgment evidence-anchor's own design reserves but does not own; part of the residue. |
| Seam review: scope/routing classification | **Not retired; re-scoped.** Not a fixed "local/architectural/documentation/UI/workflow" taxonomy (that classification is itself semantic and multidimensional) — narrowed to an attributed routing/disposition judgment that can flag "architectural diagnosis warranted." Representation left open. Part of the residue. |
| Seam review: distinction from architectural review | Retired as correct — independently re-confirmed, not merely re-quoted. |
| Seam review: distinction from UI review | Retired as consistent — not in conflict with the not-yet-reconciled `ui-review-capability.md`. |
| Seam review: invocation (consequence-selected, not a mandatory gate) | Retired as correct — no competing invocation model found. |
| Architectural review: diagnostic finding shape | **Coupled, not retired.** Buildable as a `claim-evidence` domain profile following the items-4/5/6 pattern; no such profile exists yet. Part of the residue. |
| Architectural review: "Blocking consequence remains separately owned" | **Boundary confirmed, not retired to one owner.** `strategic-planning-handoff.mjs` is a real, concrete campaign-level consumer, demonstrating the separation is achievable — not the universal owner of every architectural-finding consequence (proposal reconsideration, architecture-direction reopening, and human authority are other named owning boundaries). |
| Architectural review: relationship to item 5 (research maturity) | Retired — independently confirmed by item 5's own reconciliation. |
| Architectural review: relationship to item 3 (organizational execution envelopes) | Retired — independently confirmed by item 3's own reconciliation. |
| The routing step: architectural finding → applicable owning decision boundary | **Not retired.** No mechanism found anywhere that routes an architectural finding's pause/stop recommendation to whichever owning boundary applies (campaign planning, proposal workflow, architecture authority, or human authority). Part of the residue. |

## Recommended status change to both idea files

Update `ideas/cross-cutting-seam-review.md`'s Status section to note that
this joint reconciliation exists; that deterministic observation/comparison
of an already-declared seam dependency is retired to
`evidence-anchor-observation-and-impact-nomination.md`'s `EvidenceAnchorObserver`
pattern (narrower than retiring `may_affect` itself, which requires a known
dependent claim/fact); and that the remaining open scope is
implementation-evidence-driven seam-evidence adapters/anchor extensions, the
semantic correspondence judgment's owner, and an attributed routing/
disposition concept (not a fixed scope taxonomy) that can flag architectural
diagnosis as warranted.

Update `ideas/architectural-review.md`'s Status section to note that this
joint reconciliation exists; that its "Blocking consequence remains
separately owned" boundary is confirmed achievable — `strategic-planning-handoff.mjs`
is a real, concrete campaign-level consumer, not the universal owner of
every architectural-finding consequence — and that the remaining open scope
is an architectural-finding domain profile coupled to
`claim-evidence-service.md`'s existing pattern, plus a routing step from a
recommendation to whichever owning decision boundary actually applies
(campaign planning, proposal workflow, architecture authority, or human
authority).

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. Split
authorization: implementation of the mechanical seam-evidence adapter
extensions (implementation-evidence-driven, per the existing extensible
`EvidenceAnchorObserver` taxonomy) is authorized to proceed. The semantic
"truthfully and proportionately" correspondence-judgment owner, the
attributed routing/disposition concept, the architectural-finding domain
profile, and the routing step to an owning decision boundary are authorized
for **design work only, not implementation yet** — each requires an owner
decision this reconciliation deliberately left open.
