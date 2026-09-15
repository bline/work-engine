# Reconciliation: Claim Maintenance and Reliance Propagation

## Scope and method

This is the first of the two bounded next-work packets Sol recommended
deriving from the 2026-09-14 `post-migration-strategic-plan.md` revision
(priority: complete the claim-evidence impact/refresh vertical). It follows
the same method as the twelve-item sequel reconciliation queue: check a
pre-existing document against current implementation and current prospective
architecture, identify the smallest semantic consequence still lacking an
owner, and place it — do not design it here.

The subject is not a root idea file this time. It is an existing, formed
proposal family under `proposals/evidence-lineage/` that predates this
session and was never reconciled against the App Server build that landed
after it was formed:

- [`production-claim-evidence-interface`](../../proposals/evidence-lineage/production-claim-evidence-interface/proposal.md) — candidate 3 of 4.
- [`claim-maintenance-and-reliance-propagation`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md) — candidate 4 of 4, the one whose still-unformed production operation-contract surface admits durable impact nominations and refresh episodes.

## 1. `production-claim-evidence-interface`: implementation correspondence confirmed, historical authorization not reconstructed

**Corrected 2026-09-15** (Sol's review): a first pass at this section
concluded "built without a paper trail" and treated "already built, approved
in effect" as roughly what a `decision.json` would record, with "the paper
trail is the only thing missing." That collapses three distinct facts that
must stay separate:

```text
implementation exists
≠ proposal meaning was accepted
≠ implementation was authorized / accepted
```

`family.md`'s own "State" and "Current evidence and uncertainty" sections
still say "no candidate is prioritized or authorized for implementation," and
`production-claim-evidence-interface/packet.json` still shows
`lifecycle_state: "formed"`, `implementation_authorized: false`, with no
`decision.json`. Both are stale relative to what actually exists.

The proposal's own text is exact about what it set out to build: "Create the
smallest production claim-evidence capability that real Work Engine roles can
use to publish, discover, resolve, and rely on exact evidence-backed
statements" (`proposal.md:22-24`), via "a canonical owner, closed versioned
records, domain authority profiles, transport-neutral operations, real
consumers, and truthful discovery and applicability results"
(`proposal.md:44-46`). That corresponds strongly, at a general level, to what
`app-server/src/services/claim-evidence` already is: a real, ~2400-line
implementation with six operations (`create_claim`, `publish_revision`,
`publish_lineage`, `record_reliance`, `retire_reliance`, `retract_revision`),
a domain-profile mechanism (`review-scope-coordination-reconciliation.md` and
`proposal-research-maturity-and-freshness-reconciliation.md` both confirmed
real, distinct `revision-bound-review-finding-v1` and `proposal-research-v1`
profiles), and production consumers (native review's PPCE Slice 1 path).

**What this reconciliation did not do:** audit that correspondence against
the proposal's own specific acceptance bar. `proposal.md`'s "Evidence and
acceptance needs" (`:230-246`) names concrete, checkable items — a closed
schema and validator for the shared core and both initial profiles;
deterministic identity, reference, lineage, branch/conflict, migration, and
projection-completeness checks; authority-bound idempotent publication and
exact-revision reliance; a Codex role discovering and relying on a material
claim without receiving its identity in advance; an external read-only
consumer via the MCP projection; one real proposal-research claim and one
real revision-bound review finding published and consumed without
domain-ownership collapse; and named failure cases (unauthorized publication,
conflicting predecessors, dangling evidence, partial projections, unavailable
evidence, misleading newest-revision selection). None of these were checked
item-by-item here. General correspondence is not the same evidence as that
list being satisfied.

The proposal's own evidence cutoff (`cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`,
2026-08-24) predates the App Server implementation's own first commit
(`f890057`, "feat: activate context lifecycle and add claims core,"
2026-08-26) by two days — real work happened after formation and nobody went
back to close the loop with any decision record, the same pattern found once
before in reconciliation-queue item 10
(`agent-instruction-structure-and-placement-review-reconciliation.md`). But
item 10 had an actual dogfood metrics record and an explicit "Please proceed
sir, you have the helm" authorization on file; this candidate has neither —
only implementation correspondence, confirmed at a general level.

**Finding, precisely:** implementation correspondence is confirmed;
lifecycle and authority metadata are stale; historical authorization or
acceptance is not reconstructed by this pass and would require the
item-by-item audit above, not assumed from general correspondence. This is
not this document's decision to make unilaterally in any case — any
`decision.json` requires the same explicit-user-statement evidence every
other decision.json in this repository requires (see
`agent-instruction-integrity/agent-instruction-structure-placement-review/decision.json`'s
own `authority.evidence` field as the precedent shape). The decision section
below asks only for what this pass actually supports a ruling on.

## 2. `claim-maintenance-and-reliance-propagation`: what's confirmed still accurate

Checked directly against the three most relevant later documents this session
produced or reconciled:

- **`app-server/src/services/claim-evidence/contract.mjs`** — confirms the
  premise the proposal and the strategic plan both depend on: no
  `nominate_impact` or refresh-episode operations exist yet, in any number.
  Nothing in the
  proposal contradicts the six operations that do exist; it depends on them
  ("It consumes that proposal's stable identities, immutable revisions,
  authority profiles, exact-revision reliance, publication mechanics, and
  projection provenance. It does not redefine them," `proposal.md:96-98`) and
  is additive, not competing.
- **`app-server/docs/claim-evidence-service.md`**'s "Impact, refresh, and
  reliance propagation" section (`:457-481`) states the same pipeline shape
  at a higher level of abstraction (`repository or contract event → may_affect
  nomination → optional domain-owned refresh episode → outcome → optional new
  claim revision → exact reverse reliance projection → downstream-owner
  applicability and reopening judgment`) that the proposal's own "Semantic
  chain" section (`proposal.md:106-129`) already states in more detail, using
  compatible vocabulary (`may_affect`, refresh outcomes named identically:
  `retained_unchanged` / `changed` / `inapplicable` / `insufficient` /
  `contested` / `deferred` / `superseded`, per `evidence-anchor-observation-and-impact-nomination.md`
  §7's own retirement note). No conflict found.
- **`app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md`**
  §6 (`:325-382`) independently arrived at the same ownership conclusion this
  proposal already states — a production anchor/dependency registry needs "the
  same kind of durable, revision-bound owner `claim-evidence` already gives
  its other typed relationships" — and explicitly declines to design the
  `nominate_impact` contract change itself, naming it as
  `claim-evidence-service.md`'s (and, transitively, this proposal's) own
  authorized work to specify (§9, `:442-445`). Nothing in that document's
  later, independent arrival at the same conclusion requires revising this
  proposal.

No drift found. The proposal's semantic model, invariants, and ownership
boundary (`proposal.md:153-210`) remain accurate against everything built or
designed since it was formed.

## 3. What's genuinely still open — unchanged by this pass

The proposal is explicit about its own remaining gap, and this reconciliation
does not narrow it further (per the standing constraint: place, don't
design):

> "The remaining uncertainty is implementation-contract correctness rather
> than basic graph feasibility. Formation has not yet selected the exact
> realization of: structural namespace ownership and owner-aware edge
> identity; a content-bound index-generation token...; atomic inclusion of
> evaluated projections...; truthful unresolved endpoint records;
> preservation and rebuild of compiled projection specifications...; or the
> boundary between retrieved impact candidates, derived cache edges, and
> canonical `may_affect` nominations." (`proposal.md:50-61`)

None of that list is a production operation-contract surface (request/
response shape, authority checks, persistence shape) for durable impact
nomination and refresh-episode admission, at the same level of concreteness
as the six existing `contract.mjs` operations — that surface does not exist
in this proposal, in `claim-evidence-service.md`, or in
`evidence-anchor-observation-and-impact-nomination.md`.

**Corrected 2026-09-15, twice** (Sol's review, two rounds). First round: a
first pass called this "a genuinely unowned design task." That repeats the
exact distinction this session's own routing-seam correction just fixed
elsewhere: a missing design is not the same as a missing owner. The semantic
owner is not in question — `evidence-anchor-observation-and-impact-nomination.md`
§9 (`:442-445`) explicitly names it: "It does not propose the
`claim-evidence` contract change (`nominate_impact` or equivalent)... That is
`claim-evidence-service.md`'s own authorized work to specify." And this
proposal's own text already claims the operational consequence as its
subject — impact nominations, refresh episodes, reliance, obligations,
delivery, and recovery (`proposal.md:67-90`). Ownership is settled; what is
unformed is a concrete production operation-contract surface by which that
already-located owner admits durable impact nominations and refresh
episodes — request shape, authority check, idempotency identity, publication
semantics, persistence transition, and result/failure vocabulary.

Second round: the phrase "the `nominate_impact`/refresh-episode **operation**"
(singular) presumed a conclusion the proposal's own `semantic-model.md` does
not support. Impact nomination and refresh episodes are stated there as
distinct objects with their own separate stable identities and lifecycles —
a nomination "identifies one exact source event and revision," "targets one
exact claim revision," and "begins with a visible disposition independent of
any refresh episode" (`semantic-model.md:7-17`), while "a refresh episode has
stable identity, exact subject revision, triggers, domain profile, authorized
owner, evidence cutoff, current writer generation, and immutable lifecycle
transitions" (`:35-40`) — a separate, richer object, not the same record
under two names. Whether the eventual production surface is one operation,
two, or a small family sharing a transition substrate is exactly the
question the contract-design work still needs to answer; naming it "the
operation" in scheduling language would have quietly pre-decided that. All
three documents consistently and explicitly decline to specify this surface
(`evidence-anchor...md §9` names it as future work belonging to whichever
document forms it next), so this reconciliation correctly does not attempt
to design it here — but it should be recorded as an unformed
**operation-contract surface** beneath a settled owner, left open to
discover its own operation count, not as an ownerless design task or a
presumed single operation.

`acceptance.md`'s evidence bar (end-to-end vertical across both initial
domain profiles, structural/projection adversarial matrix, semantic-lifecycle
adversarial matrix, failure recovery on both sides of every durable
publication boundary) is real and unmet — this reconciliation does not touch
or shrink it. Meaning-approval (§4 below) is explicitly distinct from, and
does not substitute for, that acceptance bar — the same two-tier distinction
this session's own reconciliation-queue acceptance event already established
and used.

## 4. The residue, precisely

**Corrected 2026-09-15** (Sol's review) — restated as two differently-shaped
outcomes, not a symmetric pair:

```text
production-claim-evidence-interface
    → implementation correspondence CONFIRMED (general level)
    → lifecycle / authority record STALE
    → item-by-item acceptance audit NOT performed here
    → user decision required before claiming acceptance

claim-maintenance-and-reliance-propagation
    → proposal meaning UNDRIFTED
    → semantic ownership SETTLED (claim-evidence / claim-maintenance)
    → production operation-contract surface for impact nomination and
      refresh-episode admission UNFORMED (operation count undecided)
    → implementation NOT YET AUTHORIZED
    → original acceptance.md evidence bar RETAINED, untouched by this pass
```

For `production-claim-evidence-interface`, four distinct actions exist and
should not collapse into one:

```text
A. approve current proposal meaning
B. acknowledge that corresponding implementation already exists
C. decide whether that existing implementation is accepted as satisfying
   the proposal
D. repair the stale lifecycle metadata
```

This reconciliation supports (A) and (B) directly. It does not support (C) —
that requires the item-by-item audit against `proposal.md:230-246` this pass
did not perform — and (D) is administrative, contingent on how (A)-(C) are
resolved. The decision section below asks only for (A)/(B), and names the
audit that would be needed before (C) could responsibly be decided.

For `claim-maintenance-and-reliance-propagation`, this reconciliation found
essentially no remaining *ownership* seam — the semantic architecture is
settled and undrifted. What remains is a *design* gap beneath that settled
owner: a production operation-contract surface for durable impact nomination
and refresh-episode lifecycle admission. Deliberately left open: whether that
surface is one operation, two, or a small family sharing a transition
substrate — `semantic-model.md` gives nomination and refresh episodes
separate stable identities and lifecycles, so the design work should discover
the operation count, not have it presumed by scheduling language. The bounded
next unit of work this reconciliation surfaces is exactly that: form the
production operation-contract surface for durable impact nomination and
refresh-episode lifecycle admission, without revisiting the already-settled
semantic architecture. Approving proposal *meaning* now, at the same
`approve_proposal_meaning` disposition level already used for item 10
(`agent-instruction-structure-placement-review/decision.json`), would let
that contract-design work proceed without re-litigating whether the
proposal's own shape is still right. It would not itself authorize
implementation, and does not touch `acceptance.md`'s evidence bar.

## Acceptance

**Decided 2026-09-15**, after Sol's review corrected two findings above
(§1's collapse of implementation-existence with historical authorization,
and §3's "unowned" mischaracterization of the operation-contract gap). The
user ruled, via explicit statement on each of the two differently-shaped
outcomes §4 restates:

- **`claim-maintenance-and-reliance-propagation`**: proposal meaning
  approved. Recorded in
  [`decision.json`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/decision.json)
  (`approve_proposal_meaning`, `implementation_authorized: false`). The next
  bounded unit of work this reconciliation surfaces: form the production
  operation-contract surface for durable impact nomination and
  refresh-episode lifecycle admission — deliberately left open whether that
  surface turns out to be one operation, two, or a small family — without
  revisiting the already-settled semantic architecture. `acceptance.md`'s
  full adversarial evidence matrix remains retained and unmet. **2026-09-15
  addendum:** that design is now drafted, per Sol's decomposition test
  applied to the actual semantic chain rather than presumed — see
  [`operation-contract-surface.md`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/operation-contract-surface.md)
  (three operations: `nominate_impact`, `open_refresh_episode`,
  `publish_refresh_judgment`, reusing the existing `publish_revision`/
  `publish_lineage` operations for any successor revision). Not yet
  reviewed; `implementation_authorized` remains `false`.
- **`production-claim-evidence-interface`**: proposal meaning approved and
  implementation correspondence acknowledged (items A and B of §4's
  four-way split). Recorded in
  [`decision.json`](../../proposals/evidence-lineage/production-claim-evidence-interface/decision.json)
  (`approve_proposal_meaning`, `implementation_authorized: false`). Item C —
  whether the existing implementation is accepted as satisfying the
  proposal — remains explicitly undecided, and would require a separate,
  item-by-item audit against `proposal.md`'s own "Evidence and acceptance
  needs" (`:230-246`) before it could responsibly be decided. Item D
  (repairing the stale lifecycle metadata) is addressed by this decision
  record and the corresponding `packet.json` lifecycle_state update to
  `decided`.

Neither decision authorizes implementation of anything. Both are meaning-level
acceptances only, per this repository's own two-tier authority convention.
