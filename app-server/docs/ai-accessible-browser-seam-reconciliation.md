# AI-Accessible Browser: Seam Reconciliation

## Status

Seam reconciliation, following `incremental-architecture-intake-and-seam-reconciliation.md`'s
method and vocabulary. Reconciles
[`AI_ACCESSIBLE_BROWSER_DESIGN.md`](../ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md)
(dated 2026-08-26, written without reference to any of the three documents
below) against:

- [`proposals/ui-experience-evidence/ui-experience-evidence-interface/proposal.md`](../../proposals/ui-experience-evidence/ui-experience-evidence-interface/proposal.md)
  — a formed, unevaluated proposal, surfaced this session by
  `root-ideas-reconciliation.md`'s finding that its origin idea
  (`ideas/ui-experience-evidence.md`) had already been promoted but never
  checked against app-server.
- [`claim-evidence-service.md`](claim-evidence-service.md) — authorized App
  Server design target for claim lifecycle and impact/refresh propagation.
- [`evidence-anchor-observation-and-impact-nomination.md`](../ideas/pending/evidence-anchor-observation-and-impact-nomination.md)
  — this session's own dependency-observation boundary design.

**Authority:** Exploratory only. This document does not accept any
constituent design, authorize implementation, or amend the migration
roadmap.

## Summary

Four layers, not one document competing with three others:

```text
Chrome
    ↓
AI-Accessible Browser
    browser reality / capture / normalization / indexes / projections
    ↓
UI Experience Evidence Interface
    product-view identity / experience topology / UI-specific evidence
    semantics
    ↓
claim-evidence
    claim identity/revisions / may_affect publication / refresh judgment /
    exact reliance
```

**Headline finding:** `AI_ACCESSIBLE_BROWSER_DESIGN.md` survives this
reconciliation strongly — its browser-capture/normalization/indexing/
projection substrate (§7–§23) is real, detailed, unclaimed territory that
none of the other three documents own or want to own. The substantive
narrowing is concentrated in two places (§24, §30), where it independently
invented infrastructure that now has an owner or an owner-in-waiting; a
first pass at applying that narrowing left residual claim-ownership language
scattered through the perception loop, a stated goal, a governing invariant,
the system architecture diagram, one tool description, and one open
question — all pre-reconciliation phrasing that assumed local claim
ownership, not additional substantive findings. Sweeping those was necessary
for internal consistency, not a sign the reconciliation itself was
incomplete. This is the same compression pattern every reconciliation this
session has found: not a giant missing architecture, an old idea whose
surviving core needs correct placement — and, this time, whose vocabulary
needed a consistency pass to match.

## Seam records

```yaml
seam:
  id: browser-substrate.unclaimed-territory
  left: {concern: "AI_ACCESSIBLE_BROWSER_DESIGN.md §7-23 (capture, normalization, identity, coordinate spaces, spatial index, projection, pixel evidence, probes)", owner: ai-accessible-browser}
  right: {concern: "ui-experience-evidence-interface/proposal.md 'Authority and ownership' — explicit disclaimer of 'Chrome transport and lifecycle'", owner: ui-experience-evidence-interface}
  relation: CORRESPONDS
  status: ALIGNED
  evidence: >
    The proposal's own ownership section explicitly refuses "Chrome
    transport and lifecycle" as something it owns. That is exactly what
    the browser design's substantial architecture provides. No document
    in this family claims this territory besides the browser design. No
    conflict.
```

```yaml
seam:
  id: claims-integration.ownership-conflict
  left: {concern: "AI_ACCESSIBLE_BROWSER_DESIGN.md §30 'Claims integration' — a local claims system with dependency-bound invalidation and cached judgments", owner: ai-accessible-browser}
  right: {concern: "claim-evidence-service.md's claim lifecycle, and ui-experience-evidence-interface/proposal.md's explicit refusal to 'establish a second claims database'", owner: claim-evidence-service + ui-experience-evidence-interface}
  relation: OWNERSHIP
  status: RESOLVED (in favor of right)
  evidence: >
    The proposal states directly: "The candidate consumes the
    claim-centered evidence-lineage semantics as a hypothesis under test.
    It does not establish a second claims database." It further specifies
    the exact same `may_affect` boundary claim-evidence-service.md and
    evidence-anchor-observation-and-impact-nomination.md already use: "An
    implementation completion or other immutable source event may nominate
    `may_affect` relationships using observed change evidence. It cannot
    declare a visual claim false, attribute semantic causality, or reopen a
    downstream decision." §30's independently-invented claims system is the
    same concept under a different name, invented before this lineage
    existed to consume. Resolution: §30 is retired as a local claims
    system; the browser design retains only browser-owned observations and
    deterministically derived facts, which flow upward through the UI
    evidence façade to claim-evidence exactly as the summary diagram shows.
```

```yaml
seam:
  id: epistemic-status.richer-but-not-directly-portable
  left: {concern: "AI_ACCESSIBLE_BROWSER_DESIGN.md §24 epistemic status: observed / deterministically-derived / model-claimed / unknown / inaccessible / stale / contradictory / unsupported", owner: ai-accessible-browser}
  right: {concern: "evidence-anchor-observation-and-impact-nomination.md §5 comparator states: matches / differs / unknown / unsupported / failed", owner: evidence-anchor-observation}
  relation: TERMINOLOGY / REPRESENTATION
  status: UNRESOLVED (deliberately — see reasoning)
  evidence: >
    §24's eight states are not one axis. They mix at least four different
    questions: provenance/epistemic class (observed / deterministically-derived
    / model-claimed), availability/coverage (unknown / inaccessible /
    unsupported), temporal/correspondence state (stale), and
    relationship/semantic state (contradictory). Directly importing this
    enumeration into evidence-anchor-observation's comparator would conflate
    a mechanical outcome (a declared relation no longer holds — CALLS(A,B)
    now absent) with a semantic one (visible salary contradicts structured-
    data salary, which the browser design's own §21 admits "may require
    model judgment"). Resolution: do not port the enumeration. Feed the
    *need* for richer coverage/state decomposition back into
    evidence-anchor-observation-and-impact-nomination.md's own open
    "coverage, not just presence" question (already inherited from
    service-plane-and-kernel-domain-boundary.md §10, refinement 11) as
    corroborating evidence that a single flat vocabulary is insufficient —
    without deciding the final shape here.
```

```yaml
seam:
  id: effect-classification.independent-convergence
  left: {concern: "AI_ACCESSIBLE_BROWSER_DESIGN.md §19.4 effect classification: observational / reversible local / locally mutating / navigational / externally mutating / unknown effect", owner: ai-accessible-browser}
  right: {concern: "service-plane-and-kernel-domain-boundary.md §3 ServiceOperation effect class (observe/derive/mutate/orchestrate), and provider-turn-harness-runtime-and-operator-projection.md's authority/capability model", owner: service-plane + ports}
  relation: CORRESPONDS
  status: ALIGNED
  evidence: >
    Independently arrived-at, finer-grained instance of the same
    "interaction does not imply authority" principle (§6.9 of the browser
    doc, near-verbatim to the ports doc's "policy is not capability"
    invariant). Worth naming as another independent convergence on this
    session's recurring pattern, not worth merging — the browser doc's
    classification is legitimately more granular for browser-specific
    actions (hover vs. navigate vs. externally-mutating form submission)
    than the generic service-plane grammar needs to be.
```

An earlier pass at this document bundled the following three candidate
correspondences under one `SHARES_DECISION` seam. That was wrong: nothing
ties them together such that deciding one constrains the other two — each is
an independent possible correspondence between one browser mechanism and one
generic mechanism proposed elsewhere this session, decidable (or not) on its
own evidence. Split into three:

```yaml
seam:
  id: browser-dependency-invalidation.vs.evidence-anchor-observer
  left: {concern: "browser dependency invalidation (§18 sparse temporal history, digest-based)", owner: ai-accessible-browser}
  right: {concern: "EvidenceAnchorObserver family", owner: evidence-anchor-observation}
  relation: CORRESPONDS
  status: UNRESOLVED — recorded, not generalized
  evidence: >
    Resembles the same shape (compare a declared/recorded state against a
    freshly observed one) but nothing currently confirms identity rather
    than resemblance. Per this session's established discipline (evidence
    to watch, not a reason to design a generic primitive prospectively —
    service-plane-reconciliation.md), implementation evidence decides this,
    not this reconciliation.
```

```yaml
seam:
  id: browser-evidence-revisions.vs.revisioned-state-primitive
  left: {concern: "browser evidence revisions (validity intervals, Merkle-style propagation)", owner: ai-accessible-browser}
  right: {concern: "the four-instance revisioned-state kernel primitive", owner: "service-plane (architecture-direction-synthesis.md §2.3, unnamed primitive)"}
  relation: CORRESPONDS
  status: UNRESOLVED — recorded, not generalized
  evidence: >
    A fifth independent instance of "durable revisioned state + admitted
    transition + successor identity" is suggestive, not sufficient. Recorded
    as a candidate fifth data point for whoever eventually names that
    primitive to check against; not decided here.
```

```yaml
seam:
  id: browser-projection-planner.vs.evidence-calibrated-projection
  left: {concern: "browser projection planner (target x lens x resolution x dimensions x state x budget)", owner: ai-accessible-browser}
  right: {concern: "evidence-calibrated plan resolution's projection_profile", owner: evidence-calibrated-plan-resolution}
  relation: CORRESPONDS
  status: UNRESOLVED — recorded, not generalized
  evidence: >
    Both select a resolution/structure vector for a consumer given a
    question and a budget, but one projects browser evidence and the other
    projects implementation-plan structure for an executor — genuinely
    different consumers. Resemblance noted; no claim of identity.
```

```yaml
seam:
  id: first-prototype.unaffected-by-narrowing
  left: {concern: "AI_ACCESSIBLE_BROWSER_DESIGN.md §34.3 'Deliberately deferred' — explicitly excludes claims-system integration from the first prototype"}
  right: {concern: "this reconciliation's §30 narrowing"}
  relation: ALIGNED
  status: ALIGNED
  evidence: >
    The document's own prototype scope (§34.2: attach, capture, normalize,
    index, observe/inspect/probe/verify, SVG + pixel crops, diff, receipts)
    never required §30's claims system. The narrowing in this reconciliation
    removes only speculative, unimplemented, already-superseded design —
    the actually buildable path is untouched.
```

## What changes in `AI_ACCESSIBLE_BROWSER_DESIGN.md`

Applied directly to that document as part of this reconciliation (not a
separate future step, since the correction is specific and unambiguous per
the seam records above):

1. Add the four-layer ownership diagram and citations near the top (Abstract
   or a new short section), so a reader does not have to rediscover this
   placement independently.
2. §30 "Claims integration" — retitled and narrowed to explicitly disclaim
   local claims ownership, point to `claim-evidence-service.md` and
   `ui-experience-evidence-interface/proposal.md`'s `may_affect` boundary,
   and retain only what's still true: browser observations and
   deterministically-derived facts are the *evidence* a claim would cite,
   which remains this document's job to produce. Also fixes a self-contradiction
   the first narrowing pass left in place ("an earlier pass proposed a local
   claims system" followed immediately by "this document... never has").
3. §24 "Epistemic coverage" — **not** retained as a browser-internal
   vocabulary (a first pass at this section said it should be; that did not
   survive its own analysis). Marked unresolved even within the browser
   model: `model-claimed` cannot coherently be a `Fact` epistemic state
   under this document's own §7.1 definition ("an observed or
   deterministically derived proposition"), and `contradictory` still mixes
   a mechanical case with a semantic one. The enumeration is kept only as an
   original design observation pending decomposition into separate
   provenance/coverage/freshness/correspondence dimensions — not attempted
   here.
4. Sweep residual local-claim-ownership language left over from before §30's
   correction: the cybernetic perception loop (§2), Goal 9, the "Evidence
   precedes description" invariant (§6.2), the system architecture diagram
   (§9, which listed a "Claims system" as an owned pipeline stage rather
   than an external downstream consumer), `browser.inspect`'s tool
   description (§28.2, which accepted a `claim` reference as a browser
   primitive), and one open question (§38) that referred to "automatic
   claim continuity." None of these were caught by the first narrowing
   pass, which focused on §24 and §30 specifically and missed that the
   same assumption was stated in six other places.
5. Add a "Relationship to neighboring designs" section (matching this
   session's established convention) citing all three reconciled documents.

## What changes in `evidence-anchor-observation-and-impact-nomination.md`

Its existing open question about coverage vocabulary gains a concrete,
citable data point (§9's third correction, and open question 10) — not a
new vocabulary, a strengthened case that the current five-state comparator
is too flat, with a named example (`contradictory`'s mechanical/semantic
split) of exactly the danger to avoid when eventually designing the richer
version.

## What this document does not decide

- It does not decide the final coverage-state vocabulary for either
  `evidence-anchor-observation-and-impact-nomination.md` or the browser
  design — it records that richer distinctions are needed and names one
  concrete danger (`contradictory`'s dual nature).
- It does not resolve whether browser dependency invalidation, evidence
  revisions, or the projection planner are literal instances of
  `EvidenceAnchorObserver`, the revisioned-state kernel primitive, or
  evidence-calibrated projection profiles. Those remain open seams for
  implementation evidence to settle.
- It does not authorize implementation of `AI_ACCESSIBLE_BROWSER_DESIGN.md`,
  `ui-experience-evidence-interface/proposal.md`, or any part of either.
- It does not evaluate `ui-experience-evidence-interface/proposal.md` for
  acceptance — that remains a separate, larger question
  `root-ideas-reconciliation.md` already flagged as out of scope.

## Relationships

| Direction | Relationship |
| --- | --- |
| [`AI_ACCESSIBLE_BROWSER_DESIGN.md`](../ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md) | The document this reconciliation narrows; changes applied directly per the section above. |
| `ui-experience-evidence-interface/proposal.md` | Already independently states the same `may_affect` boundary and the same Chrome-transport disclaimer this reconciliation relies on — corroborating evidence, not invented here. |
| `claim-evidence-service.md`, `evidence-anchor-observation-and-impact-nomination.md` | The claim-lifecycle and dependency-observation owners `AI_ACCESSIBLE_BROWSER_DESIGN.md` §30 should have deferred to. |
| `root-ideas-reconciliation.md` | Surfaced `ui-experience-evidence-interface/proposal.md` as unevaluated-but-relevant; this document is a direct continuation of that finding. |
| `service-plane-reconciliation.md` | Source of the "record convergence as a seam, don't generalize prematurely" discipline applied to the browser-mechanisms seam above. |
