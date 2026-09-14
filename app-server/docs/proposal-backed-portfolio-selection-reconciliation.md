# Reconciliation: Proposal-Backed Portfolio and Roadmap Selection

## Status

Reconciliation of `ideas/proposal-backed-portfolio-selection.md` against
current app-server implementation and current prospective architecture.
Wave 2, item 7 of the sequel reconciliation queue — closes Wave 2.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code, `app-server/ideas/pending/`, and existing
prospective-architecture documents, applying the pattern established by
items 1–6.

## Idea summary (unchanged)

`ideas/proposal-backed-portfolio-selection.md` asks that the roadmap become
an index/prioritization projection over proposal packet identity,
relationships, decisions, and current strategic consequences, rather than a
place where proposal meaning is copied and rewritten. Its "Required
consequence" list names: accepted proposal identity/revision; dependency and
enablement relationships; blocking or conflicting proposals; unlock value;
evaluated value/risk evidence; current strategic assumptions;
implementation/activation readiness; and explicit human priority decisions.
Its "Authority" section scopes the portfolio/roadmap owner to
priority/sequencing/deferral/rejection/campaign-selection decisions, stating
that proposal relationships and evaluator findings "inform" but do not
"mechanically schedule" work. Its "Does not own" section excludes proposal
formation, proposal evaluation, research freshness, execution envelopes,
campaign supervision, and closed-loop calibration.

## What a prior pass already established, re-verified rather than re-derived

`root-ideas-reconciliation.md:259-264` already checked
`authority-backed-architecture-directions-as-workflow-inputs.md` §7 and
found no portfolio/roadmap ownership there, and separately identified
`post-migration-strategic-plan.md` as "exactly the hand-authored-roadmap-
prose pattern this idea argues against — a useful critique of current
practice, not a disqualifying overlap." Re-checked directly here, with one
correction to the first pass of this reconciliation: most of
`post-migration-strategic-plan.md` is hand-authored prose (priorities,
staged phases, named risks) with no reference to proposal packet identity
and no typed relationships or priority field, confirming that half of the
prior finding. But the document also contains a real, structured
"Strategic planning handoff" block (`post-migration-strategic-plan.md:261-314`)
matching `strategic-planning-handoff.mjs`'s own schema exactly:
`evidence_cutoff` (with `roadmap_revision`, `repository_revision`,
`campaign_terminals`), `continuity`, `verdict`, `assumptions.confirmed/
changed/invalidated`, `route_changes.{priorities,dependencies,
newly_important,deferred}`, `recommended_campaign`, `open_uncertainties`,
`authority_required`, and `revisit_when`. Strategic assumptions are **not**
purely hand-authored prose — real structured precedent for exactly this
content already exists. What that precedent does *not* do is index over
proposal packet identity or bind itself to exact proposal/evaluation/
readiness revisions for a portfolio-wide decision — it is one point-in-time
campaign-scoped handoff snapshot, not a portfolio object. This narrows the
gap considerably; see the residue below.

## What is already implemented (retires several "Current evidence" and "Required consequence" claims)

### Typed proposal relationships (confirms "Current evidence," narrows "dependency and enablement relationships")

A direct search of every `proposals/*/*/packet.json` and `relationships.md`
found five real relationship types already in use: `depends_on`, `enables`,
`informs`, `related_to`, `split_from`. This confirms the idea's own
"Current evidence" claim that "typed proposal relationships" already exist —
not an aspiration.

It also narrows one "Required consequence" clause precisely: "blocking or
conflicting proposals" has **no existing relationship type**. `depends_on`
and `enables` express positive dependency; none of the five existing types
expresses mutual exclusion or conflict. This is a small, concrete, real gap
inside an otherwise-real mechanism — not evidence the whole relationship
system is missing.

This gap should not be treated as one relation type, though. Two different
things are bundled under "blocking or conflicting":

```text
conflicts_with
    the proposals are semantically incompatible as formulated
    (e.g. two proposals both claim ownership of the same boundary
    differently) — a durable fact about the proposals themselves,
    independent of any portfolio's current capacity or timing

mutually_exclusive_under_basis
    both cannot be selected or scheduled together under the current
    portfolio's capacity, policy, authority, or timing assumptions —
    a consequence of one portfolio decision's basis, not a fact about
    the proposals
```

Only the first is a candidate addition to the proposal relationship graph
alongside `depends_on`/`enables`/`informs`/`related_to`/`split_from`. The
second must not be encoded as permanent proposal-relationship truth merely
because the current portfolio happens to lack the capacity or authority to
pursue both — that would silently convert temporary scarcity into
durable semantic conflict between proposals that may be perfectly
compatible under a different basis.

### Proposal decisions (confirms "Current evidence," narrows "explicit human priority decisions")

Every `decision.json` inspected across multiple proposals this queue has
read (`revision-bound-review-artifacts`, `adaptive-review-panel-coordination`,
`claim-centered-evidence-lineage`) shares one schema: `disposition`,
`lifecycle_state`, `placement_state`, `placement_claim`, `rationale`,
`reopening_conditions`, `authority`, `constraints`. This confirms "proposal
decisions" already exist as real, structured records — not prose.

It also narrows another "Required consequence" clause: none of these fields
represents *priority* or *sequencing*. A proposal decision records whether
its meaning was approved and how firmly it is placed; it does not record
where it stands relative to other proposals in an ordered or prioritized
set. "Explicit human priority decisions" therefore has real, adjacent
infrastructure (the decision-record schema and authority pattern) but no
priority-specific field or record type anywhere.

### Strategic planning exists, but is scoped to one campaign, not a portfolio (narrows "Current evidence")

The idea's "Current evidence" section names "a strategic planner" as
something Work Engine already has. This is real but narrower than the
idea's use of the term: `app-server/roles/strategic-planning-handoff.mjs`
and `app-server/src/services/slice-campaign/strategic-reconciliation.mjs`
implement a strategic-planning **handoff** for one active campaign, with
verdicts (`continue`/`revise`/`pause`/`reorder`/`split_campaign`/
`stop_campaign`) and campaign dispositions
(`continue_current`/`amend_current`/`start_new`/`none`). This decides
whether *one running campaign* should continue, not which proposals across
a *portfolio* should be prioritized, sequenced, deferred, or rejected. Same
pattern as item 3's `role_profile` and item 4's "sealed decision set" — a
real neighbor whose fields must not be assumed to already cover the idea's
broader scope merely because the name matches.

Disposition: retire "typed proposal relationships" and "proposal decisions"
as accurate "Current evidence." Narrow, not retire, "a strategic planner" —
real, but campaign-scoped, not portfolio-scoped. Narrow "blocking or
conflicting proposals" and "explicit human priority decisions" to the
specific missing pieces identified above, rather than treating either
clause as either fully retired or fully unowned.

## Confirmed `SUPPLIES` relationships from items 5 and 6

- Item 5's decision-specific readiness contract/assessment is the
  confirmed supplier of "implementation/activation readiness" — item 5's
  own reconciliation already states this idea is a named consumer, and
  this idea's own text independently names the same clause, agreeing from
  both directions.
- Item 6's typed evaluation estimates and comparison-contract/dominance
  findings are the confirmed supplier of "evaluated value/risk evidence."
  Item 6's own reconciliation flagged one overlap for resolution here:
  "unlock value" appears in both idea's required-consequence lists. Checked
  directly: item 7's own list separately names "dependency and enablement
  relationships" and "blocking or conflicting proposals" as *its own*
  required consequences, distinct from "unlock value." This confirms Sol's
  prediction in item 6's review — evaluation estimates a proposal-local
  quantity (how much value this proposal is expected to unlock), while
  portfolio selection owns the combination of that estimate with the real
  dependency graph. Neither idea needs correction; this is a clean
  `SUPPLIES` boundary, not a competing definition.

  That combination should not be described as a new canonical stored fact,
  though. Whether "A likely unlocks B and C" is mechanically derivable
  depends on semantics `depends_on`/`enables` do not yet establish:
  transitivity, degree of enablement, whether B is blocked *only* by A, or
  whether C remains strategically relevant. Bound inputs (an exact graph
  revision, exact evaluation revisions, exact readiness revisions, and the
  strategic basis in force) may make some closed cases mechanically
  decidable later, but the current evidence does not establish that this is
  a deterministic derivation today. It is better classified as **portfolio-
  relative unlock/enablement analysis, bound to exact graph and evaluation
  revisions** — an attributed judgment produced when a portfolio decision is
  made, not a new property automatically maintained on the proposal graph
  itself.

## What the idea's own boundary already gets right

"Does not own" (proposal formation, proposal evaluation, research
freshness, execution envelopes, campaign supervision, closed-loop
calibration) is retired as accurate: proposal evaluation → item 6; research
freshness → item 5; execution envelopes → item 3; the remainder have no
competing claimant found anywhere in this queue. The "Authority" section's
own distinction — relationships and evaluator findings "inform" but do not
"mechanically schedule" work — is exactly the sufficiency-judgment-is-not-
decision-authority boundary items 5 and 6 already established; this idea
states the same boundary independently, for its own decision type
(scheduling), rather than needing it imported.

## The idea's own dropped scope, checked rather than assumed correct

`ideas/history/2026-08-22-pre-reconciliation/evidence-backed-proposal-evaluation.md`
contains an embedded `proposal-backed-roadmap.md` stub (this idea's
direct ancestor) with a "Packet projection contract application" section:
role-scoped bounded projections from the canonical proposal packet feeding
supervisor/builder/reviewer/research roles, citing what are now
`organizational-execution-envelopes.md` (as "Context-Derived Organizational
Execution Envelopes") and a Studio design by name. This content does not
appear in the current, live idea file at all.

Checked rather than assumed dropped-correctly: item 3's reconciliation
already places "Role projection" (one role's authorized view of an
envelope) as part of its own residue, folded in because "it is only
meaningful once an envelope exists." A role-scoped *packet* projection is
the same shape of concept applied to proposal packets specifically. Its
omission from the current idea is consistent scoping, not a loss — it
belongs with item 3's envelope/projection residue if it is ever built, not
duplicated here.

## The smallest remaining semantic consequence still lacking an owner

Applying the items-5/6 discipline — decompose the compound "Required
consequence" list rather than treating it as one residue — most clauses
already have owners, near-owners, or real structured precedent (above).
What remains, precisely, after narrowing the strategic-assumptions,
conflict-relation, and unlock-value corrections above:

```text
already owned, narrowly-gapped, or real structured precedent exists
    accepted proposal identity/revision        -> proposal packets
    dependency/enablement relationships        -> depends_on/enables/informs/
                                                   related_to/split_from
    blocking (intrinsic incompatibility)       -> narrow gap: no conflicts_with
                                                   relationship type
    evaluated value/risk evidence              -> item 6 (SUPPLIES)
    implementation/activation readiness        -> item 5 (SUPPLIES)
    explicit human priority decisions          -> narrow gap: decision.json
                                                   has no priority/sequence field
    strategic assumptions (the content itself) -> real structured precedent
                                                   in post-migration-strategic-
                                                   plan.md's handoff schema

still genuinely missing
    a revision-bound portfolio decision basis that references the exact
        strategic assumptions, proposal/evaluation/readiness revisions, and
        authority under which one portfolio judgment was made -- not new
        strategic-assumption content, but a portfolio-scoped binding of
        content that already has a schema
    portfolio-relative mutual exclusion/blocking ("mutually_exclusive_under_basis")
        as a decision consequence of one portfolio's basis, kept separate from
        any durable conflicts_with proposal relationship
    portfolio-relative unlock/enablement analysis bound to exact graph and
        evaluation revisions -- an attributed judgment produced per portfolio
        decision, not an automatically-maintained property of the proposal graph
    a durable priority/sequencing/deferral/exclusion record type, referencing
        proposal packet identity, that keeps portfolio non-selection separate
        from the proposal's own lifecycle/acceptance state (see below)
```

None of these are owned by proposal packets, `claim-evidence`, item 5's
readiness contracts, item 6's evaluation/comparison machinery, or
`strategic-planning-handoff.mjs`'s single-campaign verdict. This is narrower
than the first pass of this reconciliation claimed — real structured
precedent for strategic-assumption *content* already exists — but the
*portfolio-scoped binding* of that content to an exact proposal set and
decision remains missing, alongside three other genuinely open pieces.

**The strongest surviving artifact is a `PortfolioDecision` record**, not a
new strategic-assumptions store, a new conflict taxonomy, or a new
proposal-graph property:

```text
PortfolioDecision
    portfolio/proposal-set identity + revision
    exact strategic basis (referencing the existing handoff schema)
    exact proposal revisions
    exact evaluation revisions (item 6)
    exact readiness revisions (item 5)
    applicable relationship-graph revision
    attributed cross-proposal analysis (unlock/enablement, mutual exclusion
        under this basis)
    decision owner / authority

    dispositions:
        prioritize
        sequence
        defer
        exclude_from_current_portfolio
        select_for_campaign
```

`exclude_from_current_portfolio` is deliberately not named `reject`. The
portfolio owner can exclude a proposal from the current portfolio decision
without that becoming semantic rejection of the proposal itself — item 2 and
item 4's own findings already established that proposal decisions
(`decision.json`'s `disposition`/`lifecycle_state`) are a separately owned
record. A `PortfolioDecision` marking a proposal `excluded`, `deferred`, or
`not_selected` must not silently mutate or shadow that proposal's own
lifecycle state unless the same authority separately and explicitly owns
proposal rejection:

```text
proposal's own lifecycle
    accepted / formed / whatever its decision.json already says
        (unaffected by portfolio non-selection)

PortfolioDecision's disposition for that proposal
    not_selected / deferred / excluded_from_current_portfolio
        (a portfolio-scoped fact, not a proposal-scoped one)
```

```text
proposal packets (identity, relationships: depends_on/enables/informs/
                   related_to/split_from)
claim-evidence + item 6 (evaluated value/risk, unlock-value estimate)
item 5 (readiness)
strategic-planning-handoff.mjs (single-campaign, structured assumption schema)
        |
        v  missing
PortfolioDecision: revision-bound basis + attributed cross-proposal analysis
    + priority/sequencing/exclusion disposition, referencing packet identity
    without mutating the proposal's own lifecycle
conflicts_with relationship type (intrinsic incompatibility only)
        |
        v
portfolio/roadmap owner's authority (already correctly scoped by the idea:
    informs, does not mechanically schedule)
```

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Accepted proposal identity/revision | Retired — proposal packets, already real. |
| Dependency and enablement relationships | Retired — `depends_on`/`enables`/`informs`/`related_to`/`split_from`, confirmed directly in `packet.json` files. |
| Blocking or conflicting proposals | **Split.** Intrinsic incompatibility (`conflicts_with`) is a narrow relationship-type gap, part of the residue. Portfolio-relative mutual exclusion (`mutually_exclusive_under_basis`) is a decision consequence of one portfolio's basis, not a proposal-relationship gap — must not become durable proposal-graph truth. |
| Unlock value | Retired as a boundary question — confirmed `SUPPLIES` from item 6 (proposal-local estimate). The cross-proposal combination is reclassified as **portfolio-relative unlock/enablement analysis**, an attributed judgment per portfolio decision, not an automatically-maintained proposal-graph property. |
| Evaluated value/risk evidence | Retired — confirmed `SUPPLIES` from item 6. |
| Current strategic assumptions | **Corrected, then narrowed.** Real structured precedent already exists (`post-migration-strategic-plan.md`'s "Strategic planning handoff" schema, matching `strategic-planning-handoff.mjs` exactly) — the first pass of this reconciliation incorrectly called this "only hand-authored prose." What remains is a portfolio-scoped *binding* of that content to an exact proposal set and decision, not new assumption content. Part of the residue, much narrower than first stated. |
| Implementation/activation readiness | Retired — confirmed `SUPPLIES` from item 5. |
| Explicit human priority decisions | **Not retired.** `decision.json`'s schema has no priority/sequence field, and any portfolio disposition must stay separate from the proposal's own lifecycle/acceptance state (`exclude_from_current_portfolio` ≠ proposal rejection). Part of the residue. |
| A strategic planner (Current evidence) | Narrowed, not retired — real but campaign-scoped (`strategic-planning-handoff.mjs`), not portfolio-scoped; its schema is real structured precedent the residue should bind to, not redefine. |
| Authority (inform, not mechanically schedule) | Retired as correct — independently consistent with items 5/6's sufficiency-judgment-is-not-authority boundary; extended here to require that portfolio non-selection never silently mutate proposal-level acceptance. |
| Does not own | Retired — each exclusion already has a confirmed owner from this queue. |
| Dropped "packet projection contract" scope (from history) | Retired as correctly dropped — belongs with item 3's role/envelope projection residue if ever built, not duplicated here. |

## Recommended status change to the idea file

Update `ideas/proposal-backed-portfolio-selection.md`'s Status section to
note that this reconciliation exists; that proposal identity, relationships,
and decisions are already real (narrowed to specific missing fields: an
intrinsic `conflicts_with` relationship type and a priority/sequence field);
that "a strategic planner" in its "Current evidence" section refers to a
campaign-scoped mechanism whose structured handoff schema
(`post-migration-strategic-plan.md`) is real precedent for strategic
assumptions, not evidence they are missing; and that the primary remaining
open scope is a **`PortfolioDecision` record** — a revision-bound basis
(exact strategic assumptions, proposal/evaluation/readiness revisions) plus
attributed cross-proposal analysis (portfolio-relative unlock/enablement,
portfolio-relative mutual exclusion) and a priority/sequencing/exclusion
disposition that references proposal packet identity without mutating the
proposal's own lifecycle. Confirmed `SUPPLIES` relationships to
`ideas/evidence-backed-proposal-evaluation.md` (item 6) and
`ideas/proposal-research-maturity-and-freshness.md` (item 5) should also be
noted.
