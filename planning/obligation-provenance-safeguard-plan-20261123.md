# Obligation-provenance safeguard: plan

**Status:** Draft plan, revision 2, documentation only. Not approved and not an implementation authorization.
**Date:** 2026-11-23
**Decision owner:** operator
**Revision 2:** responds to the independent review in [`reviews/obligation-provenance-plan-opus-20261123/`](../reviews/obligation-provenance-plan-opus-20261123/review.md) (verdict `revise_and_resubmit` on revision 1, commit `e882e8cc`). The changes:

- §4 adds who may type a parent, authenticated owner origin, `realization_conditional`, and value-level comparison.
- §5 corrects the enforcement points.
- §6 splits F1 into F1a and F1b and adds variants where non-owners do the typing.
- §7 makes an instruction-level change the first manual slice and the typed route conditional.

The map from findings to changes is in §11.

**Research relationship:** independent of the [perspective and intent pilot](../design/ideas/pending/pilots/perspective-intent-pilot.md). P7 there may later evaluate this safeguard; this plan does not depend on P7.

## 1. Problem

A review finding, qualification amendment or decision request can present a **newly binding obligation** as if an authoritative source already required it. Downstream work then pursues the stronger obligation as mandatory.

**Motivating incident (Case F, U4-A, 2026-11-21).** The chronology and SHA-256 pins are in [research note Case F](../design/ideas/pending/perspective-intent-research-note.md#case-f--cleanup-attribution-in-u4-a-a-planning-sentence-amplified-by-review) and [proposal §3](../design/ideas/pending/pilots/perspective-intent-pilot.md#3-seed-cases-and-the-actual-level-of-evidence). In short:

1. **The contract.** RL contract `043` says "The Host owns the child, same-session framed handshake, and exact stop". It requires clean cancel to conclude only after "exact container removal observed against the owned identity". For *removal attribution*, this is ambiguous to unspecified.
2. **The plan.** `PLAN.md` defers to 041 and 043 ("the latter documents own the broader qualification contract"). Its H3 requires the Host to "distinguish confirmed removal, removal failure and unknown cleanup", and its H4 requires independent post-exit absence. The selected realization had the Host run `docker rm` itself.
3. **The first review (realization-conditional).** The re-escalation review (`architecture-reescalation-review/ASTRA-ARCHITECTURE.md`, `12127e46…`) held that "**for this realization**" Host-attributed normal removal is required. Its reason: another actor's removal could qualify a Host whose own removal failed. It also found a removal setup failure folded into confirmed absence. Both points were legitimate under H3 while the Host performed removal.
4. **Adoption without the owner.** The coordinator adopted a "positively attained normal-removal witness" in `QUALIFICATION-AMENDMENT.md` and dispatched repair writers at 08:59Z. It acted under the operator's overnight delegation ("try to continue; use escalation rules and architectural reviews to help make decisions"). It did not consult the operator or RL, the owner of the acceptance meaning.
5. **The realization-independent strengthening.** The resolved-object review (`resolved-object-review/ASTRA-ARCHITECTURE.md`, `e8304cda…`) made Host-positive removal an independent claim citing evidence-and-claims §3.9. It ruled out Engine auto-remove and advised "Do not ask for permission to accept eventual absence instead". The decision request `OPERATOR-DECISION.md` then described this as one of "the H1–H5 requirements [that] are unchanged".
6. **The correction.** About 6.5 hours of wall-clock time went to proving exclusive cleanup causality before the operator changed the realization: create-time `--rm`, Docker as removal owner, independent verification. That dissolved the first review's point rather than refuting it.

**What failed.** Not the doctrine: core contracts §1.1 and evidence-and-claims §3.9 already require an independent authority for structural constraints. Three things went wrong:

- a realization-conditional requirement was turned into a realization-independent one;
- authority was claimed through general citations and a general delegation;
- a non-owner adopted a change to an acceptance meaning, which was then labelled "unchanged".

Nothing required a property-specific citation from the **owner** of that meaning.

**Route.** The incident ran under manual orchestration as Markdown packets: an architecture review, a supervisor amendment, free-text Pi `subagent` dispatch, and a decision request. It did not pass through `implementation-review/v1`, slice-campaign remediation, `OrganizationalRequirements` or review preparation.

## 2. Objective

Prevent a review finding, amendment, revised plan or decision request from presenting an obligation as **binding** unless one of these holds:

- it is inherited from an **owner-typed** parent that covers the exact property and value;
- it is adopted by an **authenticated owner decision** naming the property;
- it is explicitly conditional on the selected realization and protects a supported requirement.

Each route counts as protected only at the level its actual consumer achieves (§5), and the plan states that level honestly.

## 3. Non-goals

- Interpreting natural-language plans. The checker never types prose (§4.1).
- Suppressing findings. Any concern, including a proposed new obligation, may still be raised.
- Changing ordinary defect findings. A panic, race or security defect needs no contractual citation.
- Reducing owner authority. Owners may adopt stronger obligations; the safeguard records and validates them.
- Demoting accepted obligations. Owner-accepted amendments stay binding until superseded (§4.6).
- Making the review-result parser an authority gate.
- A general semantic policy engine. Comparison is limited to closed enumerations (§4.4).

## 4. Semantics

### 4.1 Who may type a parent

Only the **owner of a parent source** may author or accept its typed fields: property, value and status. The typing is digest-pinned and must exist *before* the derived assertion it is checked against. The asserting party, its reviewer, an extractor or a fixture author may **propose** a typing but cannot create `supported` from it. If no owner-accepted typing exists, an `inherited` assertion of an in-vocabulary property resolves to `requires_owner_decision`.

The checker (C1) therefore never decides semantics. It enforces **who** decided them, checks that the decision covers the exact property and value, and checks that the bytes it cites are the bytes that were decided.

### 4.2 Requirements set

A digest-pinned requirements set records, per requirement:

- id, source path, revision and digest, and quoted span;
- property and value, with status `binding`, `selected_realization`, `unspecified` or `ambiguous`;
- **owning authority** per requirement and per property;
- **precedence** between sources. For example, 043 owns the clean-cancel meaning and `PLAN.md` makes it observable; where the two differ, the owning source governs.

Subject-level and review-level properties live in **separate namespaces** (`subject.*` and `review.*`). `OrganizationalRequirements` v0 types review conduct only (continuity, mutation, independence sources, evidence custody, observer class, realization dimensions of the *review*). `slice-campaign` does not depend on it. It is therefore not a parent for subject acceptance requirements, and its fields must never resolve a `subject.*` assertion.

### 4.3 Property vocabulary (v0, closed)

`subject.initiator`, `subject.outcome_owner`, `subject.verifier` (with observer class), `subject.independence`, `subject.evidence_custody`, `subject.path_fact` (by dimension), and `subject.acceptance_evidence`, the required evidence content of an acceptance predicate. The last was added because the U4-A amendment stated its requirement as evidence content: "a positively attained normal-removal witness".

Classification is **declared by the author** and may be **challenged** by an independent reviewer, or later by a P7 extractor. An undeclared in-vocabulary assertion found on review is a recorded provenance defect. Statements outside the vocabulary are ordinary findings. Extending the vocabulary is a reviewed change.

### 4.4 Assertions and comparison

Each assertion has a property, a value from a closed enumeration, and a status:

| Status | Citation | Meaning |
|---|---|---|
| `inherited` | `{requirement id, revision, digest, property, value, span}` | Claims the parent already requires it |
| `realization_conditional` | `{selected realization, supported requirement it protects, named consequence}` | Binding only while that realization is selected |
| `owner_adopted` | Authenticated owner decision record (§4.5) naming property and value | A new obligation the owner adopted |
| `proposed` | none required | Offered to the owner; not binding |

**Coverage** is checked per property **and per value**. Values come from closed enumerations and are compared by equality or explicit containment, as `RealizationConstraint.allowed` already does. A free-text value is never mechanically covered; it returns `requires_owner_decision`, or `not_established` if inputs are missing.

**The realization-conditional test.** Would a different realization that removes the named consequence satisfy the finding?

- **Yes:** the assertion is `realization_conditional` and may block the selected realization.
- **No:** it requires a property independent of the realization, and is strengthening.

Who answers this test is part of the declared classification (§4.3), open to challenge, not decided by C1. C1 checks that the cited supported requirement exists, that the realization named is the one recorded as selected, and that the assertion is withdrawn when the selection changes.

**The rule:**

> An assertion may be treated as binding only if it is (a) `inherited` from an owner-typed `binding` parent field covering the exact property and value, (b) `realization_conditional` on the currently selected realization and protecting a `supported` requirement, or (c) `owner_adopted` by an authenticated decision of the property's owner. Anything else that a document treats as binding is a provenance defect. Selecting a realization that satisfies the parent is not strengthening and stays model judgment (core contracts §4).

### 4.5 Owner decisions

- An `owner_adopted` record must come from the property's recorded owner, with **authenticated origin**.
  - For the operator, that means a session-transcript user entry: session id, entry id, text digest.
  - An explicit delegation that names the property also qualifies.
- These do **not** qualify:
  - board messages, whose `author` and `session_id` are caller-supplied strings;
  - supervisor dispositions;
  - general delegations such as "try to continue; use escalation rules…".
- How an agent owner such as RL authenticates its decisions is open (§9).
- A disposition or amendment that fails provenance does **not** enter the requirements set. This applies transitively: a later finding citing it gets no support from it.
- An "unchanged" claim is checked against the **named source revision**, not against the accumulated set.

### 4.6 Dispositions and consequences

| Disposition | Meaning | Consequence |
|---|---|---|
| `supported` | (a) or (c) above | Binding |
| `realization_conditional` | (b) above | Binding while the realization is selected; may block it |
| `proposal` | Marked `proposed` | Routed to the owner; never binding; never forces remediation |
| `requires_owner_decision` | Treated as binding without (a)–(c) | Provenance defect; routed to the owner; dependent work held (below) |
| `ambiguous_parent` | Cites a parent field the owner typed `ambiguous` | Cannot claim inheritance; owner resolution needed |
| `contradictory` | Conflicts with a binding parent, including **narrowing** it | Owner decision required; the parent stays binding |

**Accepted obligations are never demoted.**

- An amendment the owner accepted stays binding until the owner supersedes it. C1 may report a defect against it but does not weaken it.
- An amendment adopted by a **non-owner** is not treated as optional either. It is routed to the owner, and repair or acceptance that depends on it is **held** until the owner decides. Evidence-and-claims §3.9 forbids silent weakening as firmly as strengthening.

**This safeguard enforces.** Unlike the advisory P7 check in the research pilot, its dispositions are consumed by the enforcement points in §5, at the protection level each achieves.

## 5. Components

### C0: instruction-level change on the manual route (first)

The cheapest intervention on the incident's real route; it needs no Rust.

- **Architectural escalation** (`skills/manual-orchestration/references/architectural-escalation.md`). The disposition recorded before another repair must include a **per-change table**:

  | Changed or reinterpreted acceptance condition | Quoted parent span (path, lines, digest) | Owner of that condition | Owner decision reference, or `pending` |
  |---|---|---|---|

  Repair or acceptance that depends on a `pending` row stays held. Unrelated authorized work continues. "Unchanged" may be claimed only for conditions whose rows quote the unchanged parent span.
- **Reviewer reference.** For any requirement claimed as inherited, quote the exact parent span and label interpretations as interpretations. Mark whether a requirement is conditional on the current realization. **Never discourage the owner question**; contrast "Do not ask for permission to accept eventual absence instead".
- **Protection level:** instruction-level. Applied to U4-A, the table would have shown "owner: RL; decision: `pending`" at 08:59Z, holding the repair dispatch.
- These are agent-instruction changes. They need review under [instruction design](../docs/contracts/instruction-design.md) and reach agents only through skill release.

### C1: pure provenance comparator (Rust)

A small pure library plus CLI. No model, no network, no I/O beyond explicit input files. **It is a comparator over owner-typed inputs, not an interpreter.**

- **Inputs:**
  - the digest-pinned requirements set (§4.2);
  - an assertion set listing **every assertion the document treats as binding**;
  - owner decision records with authentication data;
  - the selected-realization record.
- **What it proves mechanically:**
  - citations resolve to a requirement id at the stated revision and digest, and the quoted span exists in those bytes;
  - property and value match by equality or containment in closed enumerations;
  - the cited parent field is `binding` and owner-typed, with the typing older than the assertion;
  - decision records name the property and value, come from the recorded owner, and authenticate;
  - general citations never resolve;
  - "unchanged" matches the named source revision;
  - records whose own receipt failed are excluded, transitively;
  - `realization_conditional` assertions name the recorded selected realization and a `supported` requirement.
- **What it does not do:** type prose, decide whether a statement is an obligation, or decide realization-conditionality. Those are declared, owner-typed or challenged (§4.1, §4.3, §4.4).
- **Exit codes,** following `handoff_validator.py`:
  - `0` only if every binding assertion is `supported` or `realization_conditional`; proposals are listed as not adopted;
  - `21` on any `requires_owner_decision`, `ambiguous_parent` or `contradictory`;
  - `32` not established (missing or unreadable input; never a pass).
- **Placement:** a new small crate. It does not depend on or extend `OrganizationalRequirements` (§4.2).

### C2: manual receipt (after C1)

The disposition, amendment or decision request carries the provenance block and the C1 receipt (exit code and output digest). Repair dispatch packets carry the receipt.

- **What this adds:** a receipt is meaningful only because §4.1 and §4.5 put its decisive inputs with the owner: the owner-typed requirements set and authenticated decisions. A dispatched builder can then **re-run C1** against those owner-originated inputs instead of trusting the supervisor's receipt.
- **Protection level:** instruction-level enforcement at dispatch. The builder is instructed to re-run C1 and refuse repair against a revised plan that fails.
- **Mechanical enforcement (M3)** would need a hook in the Pi `subagent` host or an extension. The U4-A route dispatched free-text tasks, and `role_host.py` takes a prompt string with no packet validation. Whether such a hook exists is unverified (§9). Until one exists, the manual route is **not mechanically protected**.

### C3: typed route (conditional, §7)

Only if P7 or a typed-route incident shows the need.

- **Consumers:** today, findings drive remediation through the builder evaluation's per-finding `valid` judgment (`slice-campaign` `initial_outcome.rs` `BuilderEvaluationDecision`). An outcome is reported when every finding is `!valid || verified_resolved`. The episode reducer derives the remediation phase from the verdict alone (`review-episode-core` `reducer.rs`). `RemediationSelection` checks startup inputs and does not consume findings, so it is **not** an enforcement point.
- **Enforcement:**
  - a finding whose only binding basis is `proposal` or `requires_owner_decision` cannot make the verdict `remediation_required`;
  - it cannot be counted as a valid finding requiring repair; instead it routes to the owner, reusing the existing `evidence_unestablished` / `route_unestablished_claim_to_owner` pattern (`reducer.rs`, `state.rs`);
  - findings that are `supported` or `realization_conditional`, and ordinary defects, are unaffected.
- **Schema:** the closed `implementation_review_v1` field set means this needs either a v2 schema or a sidecar. A sidecar cannot reach the pure verdict rule unless `record_result`'s inputs change, so it does not leave v1 untouched in effect.
- **Parent:** the typed route has no typed parent for subject requirements today. It needs the same owner-typed requirements set as C1.

## 6. Fixtures and acceptance

### F1: the U4-A regression (decisive)

**Setup.** Reconstruct the chain from the pinned packet files, copied into fixtures at their pinned digests: 043, `PLAN.md`, the re-escalation review, `QUALIFICATION-AMENDMENT.md`, the resolved-object review and `OPERATOR-DECISION.md`.

**Owner typing.** The fixture's owner-typing of 043 and `PLAN.md` stands in for RL's typing. It is authored **blind to the outcome**: the operator's turn and the auto-remove documents are withheld, as in P7. It is recorded as a fixture stand-in, not RL's actual typing. Attribution is judged against 043 and absence against `PLAN.md`, following §4.2 precedence.

**Expected, per boundary:**

| Document | Assertion as stated | Cited source | Expected |
|---|---|---|---|
| Re-escalation review (`12127e46`) | **F1a:** under the selected Host-`rm` realization, a removal setup failure must not be reported as confirmed absence; Host removal must be affirmative | `PLAN.md` H3, selected realization | `realization_conditional`; may block that realization |
| Re-escalation review | "Narrowing success to eventual absence … would require an owner decision", treating attribution as the baseline | 043:11,17–25 | `ambiguous_parent` (owner typed 043 removal attribution `ambiguous`/`unspecified`) |
| `QUALIFICATION-AMENDMENT.md` | Adopts a "positively attained normal-removal witness" as the amended rule | the review; "existing operator authority" | `requires_owner_decision` (non-owner adoption, general authority). Does not enter the set; dependent repair held |
| Resolved-object review (`e8304cda`) | **F1b:** Host-positive removal as an independent claim regardless of realization; auto-remove excluded | 043, `PLAN.md`, the amendment, §3.9 | `requires_owner_decision` (amendment excluded transitively; §3.9 general; fails the realization-conditional test) |
| `OPERATOR-DECISION.md` | "H1–H5 requirements are unchanged", including attribution | "H1–H5" | `requires_owner_decision` (unsupported "unchanged" against the named `PLAN.md` revision) |

**Variants. Neither may yield `supported`:**

- **F1-v1:** the supervisor types 043 itself as `outcome_owner: binding Host`. This is not owner typing, so the result is `requires_owner_decision`.
- **F1-v2:** the supervisor cites `QUALIFICATION-AMENDMENT.md` as its owner decision. This is not authenticated owner origin, so the result is `requires_owner_decision`.

**Requirement:** a safeguard that does not produce these results, including F1a staying able to block, does not pass acceptance.

### Further fixtures

| Fixture | Expected |
|---|---|
| F2: authenticated owner decision requires Host-attributed removal | `supported` (owner_adopted) |
| F3: declared realizations: Host issues the stop; `--rm`/`AutoRemove` before ready required while auto-remove is selected | `realization_conditional`; not flagged |
| F4: a citation to a parent field the owner typed `ambiguous` | `ambiguous_parent` |
| F5: plain correctness finding (panic on empty input) | outside vocabulary; passes untouched |
| F6: citation to a genuine accepted requirement, wrong property | `requires_owner_decision` |
| F7: genuine defect violating a `supported` requirement | still blocks |
| F8: general citation only ("existing operator authority", "§3.9", "H1–H5") | `requires_owner_decision` |
| F9: strengthening within one property (`outcome_owner: Host` → Host exclusively causal on a shared Engine) | `requires_owner_decision` (value not contained) |
| F10: an actor narrows a binding requirement | `contradictory`; the parent stays binding |
| F11: an undeclared in-vocabulary assertion found by a challenging reviewer | recorded provenance defect |
| F12: an owner-accepted amendment later reported as defective | stays binding; defect reported only |

### Route acceptance

Test **enforcement, not only validation**, at each route's actual consumer:

- **C0:** the instruction change is reviewed under instruction design. A U4-A walk-through shows the `pending` row holding repair.
- **C2:** a builder re-running C1 against owner-originated inputs refuses the F1 amendment.
- **C3 (if built):** the verdict rule and builder evaluation refuse a proposal-only blocker and accept F7.

## 7. Slices (each separately approved and reviewed)

1. **S0 (C0):** instruction-level escalation table and reviewer reference rules, with instruction review and skill release. No Rust.
2. **S1 (C1):** the comparator crate and CLI, vocabulary v0, fixtures F1–F12 at comparator level.
3. **S2 (C2):** manual receipt with builder re-run at dispatch. Depends on S1 and on an owner-typed requirements set being available for the workstream.
4. **S3 (C3):** typed route. **Conditional** on evidence from P7 or a typed-route incident, per the pre-release simplicity policy.

S0 is independent and comes first. S1 does not depend on S0.

## 8. Risks

- **Friction.** Mitigation: the closed vocabulary, ordinary defects exempt, and C0 needing only a table.
- **Ritual compliance.** Mitigation: owner typing (§4.1), authenticated origin (§4.5), value-level matching (F6, F9), and challengeable declarations (F11).
- **False confidence on the manual route.** Mitigation: protection levels stated per component (§5).
- **Owner availability.** Holding dependent repair on a `pending` row can stall work when the owner is unavailable. Mitigation: unrelated work continues, and the hold is visible and attributable rather than silent.
- **Vocabulary gaps.** Mitigation: record gaps; P7 can measure them.

## 9. Open questions for review

1. How agent owners such as RL authenticate decisions, given that board fields are caller-supplied.
2. Whether the Pi `subagent` host or an extension offers a hook for mechanical dispatch enforcement (M3).
3. Where owner-typed requirements sets live and how owners publish them (for example durable state or a digest-pinned file).
4. Whether `subject.acceptance_evidence` is sufficient to express Case B's terminal-settlement requirement.

## 10. Review request

Re-review revision 2 against the findings of the revision 1 review, with the same adversarial focus: semantic authority, false positives, false negatives, enforcement, authority preservation and scope.

## 11. Response to the revision 1 review

| Finding | Change |
|---|---|
| B1 (decisive judgment unowned; F1 passes by construction) | §4.1 owner-only typing; C1 is a comparator; F1 owner-typing blind and marked as stand-in; F1-v1 |
| B2 (`owner_adopted` without owner identity) | §4.5 authenticated owner origin; supervisor dispositions and general delegations excluded; failed adoptions excluded transitively; F1-v2 |
| H1 (no realization-conditional category) | §4.4 `realization_conditional` and its test; F1a/F1b split; F3 rewritten |
| H2 (wrong typed enforcement point) | §5 C3 names builder evaluation and verdict rule; `RemediationSelection` removed; sidecar limitation stated |
| H3 (`OrganizationalRequirements` not a subject parent) | §4.2 namespaces; C1 does not depend on it |
| H4 (no mechanical manual consumer) | §5 protection levels; C0 first; C2 builder re-run; M3 unverified |
| M1 (exit 0 included proposals) | §5 C1 exit codes |
| M2 (F1/F4 contradiction) | §6 per-boundary table consistent with §4.6 |
| M3 (parent precedence) | §4.2 precedence; attribution judged against 043 |
| M4 (property-level only) | §4.4 value-level containment; F9 |
| M5 (self-declared scope) | §4.3 challengeable declaration; `acceptance_evidence`; F11 |
| M6 (adopted amendments, narrowing) | §4.6 no demotion, hold instead; F10, F12 |
| M7 (cheaper intervention missed) | C0 / S0 first |
| L1 (merged reviews) | §1 items 3 and 5 attribute each review separately |
| L2 (S2 unjustified) | S3 conditional |
