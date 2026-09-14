# Reconciliation: Agent-Instruction Structure and Placement Review

## Status

Reconciliation of `ideas/agent-instruction-structure-and-placement-review.md`
against current implementation and prospective architecture. Wave 3, item 10
of the sequel reconciliation queue — closes Wave 3.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code, `app-server/ideas/pending/`, and existing
prospective-architecture documents. **This item is structurally different
from items 1–9**: the idea is not exploratory. It has already been formed
into a proposal, decided, implementation-authorized, built, dogfooded, and
accepted — at both the root substrate and in app-server. The root idea
file's own Status line ("Exploratory specialized-review capability") is
simply stale relative to what already exists; almost everything in it
retires to real, running, tested code rather than to a design document or a
coupled pattern.

## Idea summary (unchanged)

`ideas/agent-instruction-structure-and-placement-review.md` asks for a
specialist review capability whose subject is normative agent-facing text
itself: a two-question diagnostic (does this instruction protect something
that must remain true for the outcome to stay valid/safe/authorized/
observable; would it still need to hold under a different route), a
placement-context binding (semantic owner, consumer, authority/scope,
loading behavior, protected distinction, competing/duplicate governance), a
required-consequence finding shape (instruction/placement,
protected-distinction-or-its-absence, route necessity, owner authority,
consequence of retaining current wording/placement, confidence/evidence/
limitations, recommended outcome), membership in an "open adaptive-review
registry," explicit relationships to architectural review, seam review,
adaptive panel coordination, skill authoring, and doctrine/authority review,
an authority boundary (diagnoses and recommends; does not author repair,
accept, authorize, or self-certify), and a named first dogfood subject.

## What already exists: a formed, decided, authorized, built, and dogfooded capability

Checked directly, not inferred from the idea's own text:

- **Formed and decided**: `proposals/agent-instruction-integrity/agent-instruction-structure-placement-review/`
  is a real proposal with `packet.json` (`lifecycle_state: "decided"`),
  `decision.json` (`disposition: "approve_proposal_meaning"`, explicit user
  approval evidence), `placement.md`, `relationships.md`, and an
  `implementation-plan.md`.
- **Implementation-authorized**: `implementation-authorization.md` records a
  real, explicit user grant ("Please proceed sir, you have the helm.") for
  one bounded `slice-supervisor` slice to build the capability.
- **Built at the root substrate**: `skills/agent-instruction-review/SKILL.md`
  and `skills/agent-instruction-review/references/finding-contract.md` exist
  and match the idea's diagnostic closely — the implemented skill's
  two-question test is extended to a third question discovered during
  dogfood: "does its governed agent also receive enough of the causal reason
  and concrete failure mode to understand why the protected distinction is
  necessary" (`SKILL.md`).
- **Built in app-server**: `app-server/src/services/agent-instruction-review/`
  (`contract.mjs`, `service.mjs`) is exported from `app-server/src/index.mjs`
  and covered by `app-server/tests/agent-instruction-review-specialist.test.mjs`.
  Its `validateFindingDetail` schema directly encodes the idea's required-
  consequence fields: `protectedDistinction`, `exactRouteNecessary`,
  `placement.{semanticOwner, consumer, audience, scope, precedence,
  loadingReach}`, `causalExposure`, `authoritySource`, `consequence`,
  `confidence`, `limitations`, and `advisoryOutcome` (`retain`/`restate`/
  `split`/`move`/`demote`/`remove` — an exact match to the idea's own
  recommended-outcome vocabulary). Its `bindAgentInstructionReviewResult`
  produces an `authority` object with every flag (`mutationAuthorized`,
  `architectureChoiceAuthorized`, `proposalAcceptanceAuthorized`,
  `reviewerSelectionAuthorized`, `implementationAcceptanceAuthorized`,
  `selfCertificationAuthorized`, `humanAuthorityConferred`,
  `independenceClaimed`) fixed `false` — a direct, mechanically-enforced
  encoding of the idea's entire "Authority and independence" section,
  including its own explicitly-flagged self-referential concern
  (`selfCertificationAuthorized: false`).
- **Delivered read-only**: `renderAgentInstructionReviewDelivery` requires
  `threadOptions.sandbox === "read-only"` and zero declared effects for both
  the reviewer role and the specialist skill — mechanically enforcing "does
  not mutate" beyond what the idea's prose alone could guarantee.
- **Dogfooded and accepted**: `metrics/agent-instruction-review.jsonl`
  records a real, accepted slice (`"status":"accepted"`,
  `"outcome":"Implemented and dogfooded the agent-instruction-review skill;
  deterministic gates passed and one high plus one medium accepted
  same-model review finding were verified resolved"`,
  `"vertical_semantic_test_passed":true`, `"placement_verdict":"confirmed"`).
  The same record includes `"rejected_placement_alternatives"` explicitly
  considering and rejecting "Architecture, doctrine-authority, or seam
  reviewer" ("Neighboring but distinct review subjects"), "Skill authoring"
  ("Cannot certify its own instruction choices"), "Adaptive coordinator or
  bootstrap procedure" ("Coordinates rather than owns findings"), and
  "Mechanical linter" ("Route causality requires semantic judgment") — this
  is the idea's own "Relationship to neighboring capabilities" section,
  empirically tested during dogfood rather than merely asserted in prose.

## What is already implemented (retires nearly every clause)

Every major section of the idea retires directly to the evidence above:

- **Core diagnostic** → `SKILL.md`'s two-(now three-)question test, dogfooded.
- **Placement context** → `projectAgentInstructionClosure`'s instruction
  closure (subject, manifest, role, fragments with `precedence`/`inclusion`/
  `condition`, omissions, limitations) — a materially richer, mechanically
  validated realization of the idea's own placement-binding list.
- **Required consequence** → `validateFindingDetail`'s schema, field-for-field,
  including the exact `retain`/`restate`/`split`/`move`/`demote`/`remove`
  outcome vocabulary.
- **Adaptive applicability / open registry membership** → confirmed
  consistent with `proposals/adaptive-specialized-review/adaptive-review-panel-coordination/proposal.md`'s
  own framing ("Potential specialist capabilities remain an open registry");
  no separate registration mechanism was needed or built, matching that
  proposal's own "open and model-interpreted" registry design.
- **Relationship to neighboring capabilities** → empirically exercised during
  dogfood (rejected-alternatives evidence above), not merely stated.
- **Authority and independence, including the self-referential concern** →
  mechanically enforced via the `authority` object's fixed-`false` flags and
  the read-only, zero-effect delivery boundary.
- **Candidate placement and first evidence** → superseded by the real
  dogfood: the named bounded first subject was exercised and accepted,
  producing `reviews/implementations/agent-instruction-review/2387a32/`
  (gate, independent-review-authority, remediation, and accepted-same-model
  review records).

## What the decision itself already leaves open (not a gap this reconciliation found)

`decision.json`'s own `reopening_conditions` already name the specific
uncertainties this decision preserved rather than resolved, and the
accepted slice's own completion record explicitly names one as still open:
"Preserve proposal/implementation lifecycle sharing as a non-blocking
empirical uncertainty." This maps to `decision.json` reopening condition 4:
"Proposal and implementation subjects require incompatible reviewer
contracts or evidence lifecycles." This is not a gap this reconciliation
discovered — it is the one uncertainty the decision's own authors already
flagged and deliberately left open, pending further dogfood evidence. The
idea's own "Open evidence needs" section names the same question
("Observe whether proposal and implementation subjects genuinely share one
reviewer capability before accepting shared placement").

The other four reopening conditions are standing falsification tests for
the accepted decision (e.g., "the two-question diagnostic systematically
demotes exact routes that are causally required for safety, authority,
validity, or observability"), not open architecture — they define when this
already-accepted capability should be reconsidered, which is a property of
a mature, decided artifact, not evidence of missing ownership.

## The smallest remaining semantic consequence still lacking an owner

**None, at the architecture level.** This is the first item in this queue
where the idea does not survive as unbuilt territory of any size — it has
already been built, tested, and accepted. What remains is narrower than any
prior item's residue:

```text
already fully owned and built
    the two/three-question diagnostic
    placement-context binding
    the finding schema and outcome vocabulary
    the authority/independence boundary (mechanically enforced)
    open-registry membership
    empirical relationship-boundary evidence vs. neighboring capabilities

satisfied for initial acceptance; standing by design, not open architecture
    the other four decision.json reopening conditions (diagnostic
    distinctiveness, causal-route preservation, minimal context,
    classification-vocabulary reuse) -- one dogfood run supplied enough
    evidence to accept, not permanent immunity from later reconsideration

not a gap -- an explicitly preserved dogfood question
    whether proposal-subject and implementation-subject review genuinely
    share one reviewer capability (decision.json reopening condition 4;
    the idea's own "Open evidence needs" already names this)

genuinely stale, not architectural
    the root idea file's own Status line still says "Exploratory,"
    undercounting what already exists
```

The only actionable output of this reconciliation is a documentation
correction: pointing the idea file at what has already been decided, built,
and dogfooded, and naming the one uncertainty its own decision record
already preserves — not placing a new architectural gap.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Core diagnostic | Retired — implemented in `SKILL.md`, dogfooded, extended to a third question by real evidence. |
| Placement context | Retired — `projectAgentInstructionClosure`'s instruction closure. |
| Required consequence | Retired — `validateFindingDetail`'s schema, field-for-field match including exact outcome vocabulary. |
| Adaptive applicability / open registry | Retired — consistent with `adaptive-review-panel-coordination`'s own open, model-interpreted registry design; no separate mechanism needed. |
| Relationship to neighboring capabilities | Retired — empirically exercised during dogfood (rejected-placement-alternatives evidence), not merely asserted. |
| Authority and independence (including self-referential concern) | Retired — mechanically enforced via fixed-`false` authority flags and a read-only, zero-effect delivery boundary. |
| Candidate placement and first evidence | Retired — superseded by the real, accepted dogfood record. |
| Open evidence needs: proposal-vs-implementation subject sharing | **Not retired, but not a newly-found gap** — the decision's own reopening conditions already preserve this exact uncertainty. |
| Open evidence needs: diagnostic distinctiveness, causal-route preservation, minimal context, classification-vocabulary reuse | **Satisfied for initial acceptance, not fully retired.** Each was exercised during the accepted dogfood slice, per the metrics record's validation-breadth and rejected-alternatives evidence — sufficient to close them as architecture-development blockers. Where the accepted decision names the same property as a reopening/falsification condition (`decision.json`), it remains a standing condition by design; one dogfood run establishes sufficient initial evidence, not permanent immunity from later reconsideration. |

## Recommended status change to the idea file

Update `ideas/agent-instruction-structure-and-placement-review.md`'s Status
section to state plainly that this idea has been formed, decided,
implementation-authorized, built (`skills/agent-instruction-review`,
`app-server/src/services/agent-instruction-review`), and dogfooded to
acceptance — pointing at `proposals/agent-instruction-integrity/agent-instruction-structure-placement-review/decision.json`
and `metrics/agent-instruction-review.jsonl` as the authoritative record.
Note that `decision.json`'s reopening condition 4 (whether proposal-subject
and implementation-subject review genuinely share one reviewer capability)
remains the one deliberately preserved open question, not a gap this
reconciliation found. The idea file's exploratory framing should be retired
in favor of pointing at the real artifacts.
