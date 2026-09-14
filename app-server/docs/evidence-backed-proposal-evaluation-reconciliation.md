# Reconciliation: Evidence-Backed Proposal Evaluation

## Status

Reconciliation of `ideas/evidence-backed-proposal-evaluation.md` against
current app-server implementation and current prospective architecture.
Wave 2, item 6 of the sequel reconciliation queue.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code, `app-server/ideas/pending/`, and existing
prospective-architecture documents. This item inherits a sharpened question
from item 5's own findings: what does evaluation genuinely own, versus what
is evidence supplied to item 5's decision-specific readiness and item 7's
portfolio selection? Applied here rather than assumed.

## Idea summary (unchanged)

`ideas/evidence-backed-proposal-evaluation.md` asks for a reusable capability
that evaluates a formed proposal using attributed evidence about value,
implementation surface, risk, reversibility, maintenance consequences,
validation burden, alternatives, and uncertainty — producing decision
support, not a decision. It proposes a typed model (facets, claims, evidence
items, measures, readiness judgments — the last explicitly named as "issued
by their authorized owner," not evaluation itself) and a "conditional
comparison contract" mechanism for dominance/Pareto findings across
proposals, bounded by an explicit, authorized contract naming included
measures, directionality, treatment of missing/contested/stale evidence, an
evidence cutoff, and the authority that selected the comparison surface. Its
"Does not own" section already excludes acceptance, implementation
authorization, roadmap mutation, research maturity, organizational choice,
and closed-loop calibration.

## What a prior pass already established, re-verified rather than re-derived

`root-ideas-reconciliation.md:249-258` already checked
`authority-backed-architecture-directions-as-workflow-inputs.md` §7
("Proposal-formation behavior") in full and found it owns conformance
mapping (is a proposal consistent with an accepted direction), not
value/risk evaluation — no overlap. Re-checked directly here with a targeted
search of §7's text for "value," "risk," "complexity," and "dominance": zero
occurrences, confirming the prior finding rather than overturning it. The
same prior pass found `product-development`'s `proposal-delivery.mjs` and
`intake-delivery.mjs` are mechanical packet validation only — re-checked
directly here with the same targeted search: zero occurrences of
risk/complexity/reversibility/dominance/facet/measure anywhere in
`proposal-delivery.mjs`, confirming the finding still holds.

## The idea's own un-compressed history, checked directly

`ideas/history/2026-08-22-pre-reconciliation/evidence-backed-proposal-evaluation.md`
is an earlier, less-condensed draft of this exact idea (not itself a live
idea to reconcile). Comparing it to the current file shows the current
version is already a refinement, not a lossy compression missing content
that item 5's history check would suggest checking for — most differences
are wording tightening. Two things worth surfacing:

- it explicitly cross-references the same sibling document item 5 already
  grounded its findings in: "Research maturity, decision-specific readiness,
  Git evidence baselines, authority, and staleness propagation are developed
  in [Research Maturity, Evidence Snapshots, and Staleness]" — the same
  document behind item 5's reconciliation, confirming items 5 and 6 were
  originally written as one family and later split, consistent with their
  current tight coupling;
- it once bundled a "Possible research roles" section (repository/ownership
  analyst, design-alignment reviewer, metrics analyst, risk/reversibility
  analyst, proposal synthesizer) and an embedded `proposal-backed-roadmap.md`
  stub — both dropped from the current, refined idea. The roles section
  would be organizational-execution-envelopes' (item 3) territory if ever
  adopted, not evaluation's; the roadmap stub became today's
  `proposal-backed-portfolio-selection.md` (item 7). Dropping both from the
  current idea is correct scoping, not a loss to recover.

Disposition: this document is evidence, not a live idea; it confirms the
current idea's scope is already well-drawn rather than revealing a gap in
its own editing (unlike item 5, where the history check materially
strengthened a retirement).

## What is already implemented and designed

### `claim-evidence-service.md` / `app-server/src/services/claim-evidence` (retires "claims" and "evidence items")

The idea's own typed-model definitions — "claims: evidence-backed semantic
conclusions within a facet" and "evidence items: observations supporting or
challenging a claim" — are exactly claim-evidence's existing claim and
evidence-reference primitives, domain-scoped to proposal evaluation.
`authorized-vertical.mjs`'s `proposal-research-v1` profile (already found in
item 5's reconciliation) is the concrete, implemented home for these:
subject/evidence-baseline/decision-scope/content-set mechanics already
exist. A targeted search of `claim-evidence-service.md` and every file in
`app-server/src/services/claim-evidence/` for "facet," "measure,"
"dominance," "pareto," or "comparison contract" found zero occurrences —
confirming the substrate below the claim/evidence level exists, but nothing
above it toward typed multi-dimensional comparison does.

Disposition: retire "claims" and "evidence items" from the idea's typed
model as already owned by `claim-evidence`'s existing primitives — not a new
schema evaluation must define, a domain-scoped application of one that
already exists.

### `ideas/proposal-backed-portfolio-selection.md` (item 7, not yet reconciled — confirms `SUPPLIES` direction, not competing ownership)

Read directly to check whether portfolio selection (item 7) already claims
the comparison-contract/dominance-finding mechanism this idea proposes,
since items 5, 6, and 7 were originally one family (per the history check
above) and item 5 already found one such overlap requiring a `SUPPLIES`
relationship rather than a merge. It does not: item 7's own "Required
consequence" list states portfolio judgment reasons *over* "evaluated
value/risk evidence" and "implementation/activation readiness" — consuming
already-computed findings — and its "Does not own" section explicitly
excludes "proposal evaluation." Its own "Authority" section scopes portfolio
to priority/sequencing/deferral/rejection/campaign-selection decisions, a
different kind of authority than evidence comparison.

One minor terminology overlap is worth flagging for whoever reconciles item
7 next, in the same spirit as item 5's R3/item-6 flag: both this idea's
"Evaluation dimensions" and item 7's "Required consequence" list name
"unlock value." This idea would compute unlock value as a proposal-level
claim/measure; item 7 separately owns "dependency and enablement
relationships" and "blocking or conflicting proposals" across many
proposals. The likely resolution is that evaluation supplies a per-proposal
unlock-value claim that portfolio then contextualizes against its own
cross-proposal dependency graph — not a competing definition — but this is
not resolved here, since item 7 has not yet had its own standalone
reconciliation.

Disposition: confirm, not retire — `SUPPLIES` relationship (evaluation's
dominance/comparison findings and per-proposal claims feed portfolio
selection; portfolio does not itself perform evidence comparison).

## What the idea's own boundary already gets right

The idea's own "Typed evaluation model" section already names "readiness
judgments" as "decision-specific conclusions issued by their authorized
owner" — not evaluation's own output — directly consistent with item 5's
finding that evaluation is a likely evidence *supplier* to a separately
owned readiness judgment, not the judgment's owner. This idea does not fall
into the trap item 5's review specifically warned against ("item 6 should
not start absorbing 'ready/not ready' merely because it produces value/risk/
complexity evidence") — it was already written to avoid that, and this
reconciliation confirms the avoidance holds rather than needing correction.

The "Does not own" section (acceptance, implementation authorization,
roadmap mutation, research maturity, organizational choice, closed-loop
calibration) is retired as accurate: research maturity → item 5; roadmap
mutation → item 7; organizational choice → item 3; the remainder (accept/
reject, implementation authorization) have no competing claimant found
anywhere in this queue.

## The smallest remaining semantic consequence still lacking an owner

Applying the item-5 lesson — decompose a compound section before declaring
it a unified residue — the idea's "Typed evaluation model" does not survive
as one undifferentiated gap. "Claims" and "evidence items" are retired
above. What remains is narrower and, unlike facets, does not depend on an
unresolved ontology question the idea itself has not settled.

**Facets** ("durable domains of investigation") are explicitly flagged by
the idea's own text as unresolved: "This is not yet an accepted universal
registry. Formation must decide whether a candidate field is a facet,
claim, measure, epistemic metadata, or relationship before schema
adoption." Per the same caution applied to item 5's R0–R5 ladder, this
reconciliation does not assert facets as settled new canonical state
lacking only an owner — the idea itself has not decided whether "facet" is
a real primitive or a convenience grouping over claims. This is noted as an
open modeling question the idea itself owns, not placed as a residue
requiring a different owner.

**Measures were mistyped in the first pass of this reconciliation** as an
*evidence* sub-schema ("typed evidence items with declared units, direction,
scope, and cutoff"). The idea's own examples — expected impact, complexity,
risk, reversibility, maintenance benefit, expected cost — are estimates
*derived from* evidence, each carrying its own evidence basis, limitations,
provenance, and confidence, not evidence itself. Retyped correctly:

```text
evidence (claim-evidence: identity, references, provenance,
          revision/freshness mechanics)
    |
    v
evaluation claim / estimate (evaluation domain: what "expected cost" or
    "regression risk" means, the typed value/scale, directionality, scope,
    confidence/uncertainty, evidence cutoff)
```

This still identifies a real gap — `claim-evidence`'s evidence references are
generic (digest, status, freshness) with no directional, unit-bearing
estimate structure above them, and nothing found anywhere defines one — but
the gap is **typed evaluation measure/estimate semantics** (what evaluation
concludes from evidence, in a form comparable across proposals), not an
evidence sub-schema. This keeps the ownership split intact: `claim-evidence`
owns the evidence beneath an estimate; evaluation owns the estimate itself.

**The conditional comparison contract** (dominance/Pareto findings bounded
by an explicit, authorized contract naming included/excluded measures,
directionality, treatment of missing/contested/stale evidence, an evidence
cutoff, and the selecting authority) is the clearest, most load-bearing gap:
a targeted search across `app-server/src`, `app-server/docs`, and
`app-server/ideas/pending` found zero occurrences of "dominance," "pareto,"
or "comparison contract." This is evaluation's own genuine, non-overlapping
territory — distinct from `claim-evidence` (which has no comparison
mechanism at all, per its own non-goals: "does not... turn retrieval rank or
graph reachability into applicability"), distinct from item 5's readiness
(single-proposal sufficiency for a decision, not cross-proposal comparison),
and distinct from item 7's portfolio selection (consumes comparison findings
for scheduling authority; does not perform the comparison).

Ownership of the contract itself should not be read as "evaluation decides
what counts as better" — the idea already guards against this by binding the
contract to the authority that selected the comparison surface, and that
split should stay explicit rather than implicit:

```text
evaluation capability owns:
    comparison-contract schema and validation
    measure comparability rules
    dominance/Pareto derivation mechanics
    truthful incomparable/insufficient outcomes

decision or portfolio owner owns:
    which measures matter for this decision
    which proposal set is being compared
    the authorized comparison surface
    any weighting or tradeoff preference
```

With that split, dominance derivation becomes a deterministic consequence of
two separately-owned inputs, not a new authority:

```text
authorized comparison contract (decision/portfolio owner)
        +
typed evaluation estimates (evaluation)
        |
        v
deterministic comparison where defined
        |
        v
A dominates B / A and B incomparable / comparison blocked by
    stale, missing, or contested measure
```

No portfolio or acceptance authority is created by that result — it remains
a conditional finding under a named contract and cutoff, exactly as the idea
itself already states.

```text
claim-evidence-service.md
    owns: claim identity, evidence, lineage, freshness
        |
        v  missing (typed evaluation measure/estimate semantics)
        v  missing (comparison-contract schema/validation +
                     dominance/Pareto derivation mechanics)
        |
        v
item 5 readiness           item 7 portfolio selection
    consumes: evaluation        consumes: evaluation's dominance
    findings as one input       findings + readiness's sufficiency
    to a decision-specific      judgments, for priority/sequencing/
    sufficiency judgment        deferral authority

facets: unresolved modeling question the idea itself has not settled —
    not placed as an owned-elsewhere gap or a settled new primitive
```

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Claims, evidence items | Retired — already owned by `claim-evidence`'s existing primitives, domain-scoped via the implemented `proposal-research-v1` vertical profile. |
| Facets | **Not retired; not placed as residue.** The idea's own text already flags this as an unresolved modeling question (primitive vs. convenience grouping) — left open, not assigned a missing owner. |
| Measures | **Not retired; retyped.** Not an evidence sub-schema (first-pass error) — evaluation *estimates* (impact, complexity, risk, etc.) derived from claim-evidence's evidence, each with its own scale, directionality, scope, and confidence. Part of the residue as "typed evaluation measure/estimate semantics." |
| Conditional comparison contract (dominance/Pareto) | **Not retired. Primary residue**, with ownership split made explicit: evaluation owns the contract schema/validation and dominance-derivation mechanics; the decision/portfolio owner owns which measures matter, which proposals are compared, and the authorized comparison surface. Zero occurrences anywhere in app-server. |
| Readiness judgments (named but explicitly not owned by this idea) | Confirmed correct — matches item 5's independent finding that evaluation supplies readiness, does not own it. |
| Does not own | Retired — each exclusion already has a confirmed or independently-queued owner. |
| Relationship to `proposal-backed-portfolio-selection.md` (item 7) | Confirmed `SUPPLIES`, not competing ownership; one minor "unlock value" terminology overlap flagged for item 7's own reconciliation, not resolved here. |
| Relationship to `authority-backed-architecture-directions-as-workflow-inputs.md` §7 | Re-confirmed no overlap (conformance mapping, not value/risk evaluation). |
| Current evidence (proposal packets, formation, decisions) | Retired as accurate — `product-development`'s delivery services re-confirmed as mechanical-only, exactly the layer the idea already says is insufficient. |

## Recommended status change to the idea file

Update `ideas/evidence-backed-proposal-evaluation.md`'s Status section to
note that this reconciliation exists; that "claims" and "evidence items" are
retired to `claim-evidence-service.md`'s already-implemented substrate; that
"facets" remain an open modeling question the idea itself has not settled,
not a placed gap; and that the primary remaining open scope is **typed,
comparison-ready evaluation conclusions plus the bounded comparison-contract
machinery that can derive non-authoritative dominance/incomparability
findings from them** — evaluation owns the estimate semantics and the
comparison mechanics; the decision or portfolio owner owns which measures,
proposals, and comparison surface apply. This is evaluation's own genuine
territory, distinct from and supplying both item 5's readiness judgments and
item 7's portfolio-selection authority.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. Implementation
of the stated residue — typed evaluation estimates and the comparison-
contract/dominance mechanism, with evaluation owning the contract mechanics
and the decision/portfolio owner picking what is compared — is authorized
to proceed.
