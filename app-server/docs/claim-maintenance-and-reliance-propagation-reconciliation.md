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
- [`claim-maintenance-and-reliance-propagation`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md) — candidate 4 of 4, the one containing the missing `nominate_impact`/refresh-episode operation.

## 1. `production-claim-evidence-interface`: built without a paper trail

`family.md`'s own "State" and "Current evidence and uncertainty" sections
still say "no candidate is prioritized or authorized for implementation," and
`production-claim-evidence-interface/packet.json` still shows
`lifecycle_state: "formed"`, `implementation_authorized: false`, with no
`decision.json`. Both are stale.

The proposal's own text is exact about what it set out to build: "Create the
smallest production claim-evidence capability that real Work Engine roles can
use to publish, discover, resolve, and rely on exact evidence-backed
statements" (`proposal.md:22-24`), via "a canonical owner, closed versioned
records, domain authority profiles, transport-neutral operations, real
consumers, and truthful discovery and applicability results"
(`proposal.md:44-46`).

That is, verified this session, exactly what `app-server/src/services/claim-evidence`
already is: a real, ~2400-line implementation with six operations
(`create_claim`, `publish_revision`, `publish_lineage`, `record_reliance`,
`retire_reliance`, `retract_revision`), a domain-profile mechanism
(`review-scope-coordination-reconciliation.md` and
`proposal-research-maturity-and-freshness-reconciliation.md` both confirmed
real, distinct `revision-bound-review-finding-v1` and `proposal-research-v1`
profiles), and production consumers (native review's PPCE Slice 1 path).

The proposal's own evidence cutoff (`cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`,
2026-08-24) predates the App Server implementation's own first commit
(`f890057`, "feat: activate context lifecycle and add claims core,"
2026-08-26) by two days. This is the same pattern the reconciliation queue
found once before, in item 10
(`agent-instruction-structure-and-placement-review-reconciliation.md`): real
work happened after formation and nobody went back to close the loop with a
decision record.

**This is not this document's decision to make unilaterally.** Recording
"already built, approved in effect" as a `decision.json` requires the same
explicit-user-statement evidence every other decision.json in this repository
requires (see `agent-instruction-integrity/agent-instruction-structure-placement-review/decision.json`'s
own `authority.evidence` field as the precedent shape). This document places
the finding; the decision section below asks for the ruling.

## 2. `claim-maintenance-and-reliance-propagation`: what's confirmed still accurate

Checked directly against the three most relevant later documents this session
produced or reconciled:

- **`app-server/src/services/claim-evidence/contract.mjs`** — confirms the
  premise the proposal and the strategic plan both depend on: no
  `nominate_impact` or refresh-episode operation exists yet. Nothing in the
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

None of that list is the `nominate_impact` operation's own request/response
contract (inputs, authority checks, persistence shape) at the same level of
concreteness as the six existing `contract.mjs` operations — that contract
does not exist in this proposal, in `claim-evidence-service.md`, or in
`evidence-anchor-observation-and-impact-nomination.md`. All three
consistently and explicitly decline to specify it (`evidence-anchor...md §9`
names it as future work belonging to whichever document forms it next). It
remains a genuinely unowned design task, not something this reconciliation
can retire.

`acceptance.md`'s evidence bar (end-to-end vertical across both initial
domain profiles, structural/projection adversarial matrix, semantic-lifecycle
adversarial matrix, failure recovery on both sides of every durable
publication boundary) is real and unmet — this reconciliation does not touch
or shrink it. Meaning-approval (§4 below) is explicitly distinct from, and
does not substitute for, that acceptance bar — the same two-tier distinction
this session's own reconciliation-queue acceptance event already established
and used.

## 4. The residue, precisely

Two distinct, separately-authorizable things, not one:

1. **Decision-readiness for `production-claim-evidence-interface`**: its
   proposal meaning is already realized in production; the paper trail is
   the only thing missing.
2. **Decision-readiness for `claim-maintenance-and-reliance-propagation`**:
   its proposal meaning is unchanged and unconflicted by everything reconciled
   or designed since formation; nothing new was found that should change its
   text. What remains before implementation is authorized is (a) the
   `nominate_impact`/refresh-episode operation's own contract design — not
   attempted here, not owned by any existing document — and (b)
   `acceptance.md`'s full adversarial evidence matrix, unaffected by this
   pass.

Approving proposal *meaning* for both, at the same `approve_proposal_meaning`
disposition level already used for item 10
(`agent-instruction-structure-placement-review/decision.json`), would let
future work proceed straight to the operation's contract design (the
genuinely bounded next unit of work after this one) without re-litigating
whether the proposal's own shape is still right. It would not itself
authorize implementation, and does not shortcut `acceptance.md`'s evidence
bar.

## Acceptance

Not yet decided. This section is intentionally left for an explicit ruling,
not written unilaterally — matching this repository's own
`decision.json` authority-evidence convention and the standing rule
established during the sequel reconciliation queue's own acceptance event.
