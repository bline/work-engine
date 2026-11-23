# Obligation-provenance safeguard: plan

**Status:** Draft plan, documentation only. Not approved, not reviewed, and not an implementation authorization.
**Date:** 2026-11-23
**Decision owner:** operator
**Requested next step:** independent plan review, then an operator decision on implementation as separate bounded slices.
**Research relationship:** independent of the [perspective and intent pilot](../design/ideas/pending/pilots/perspective-intent-pilot.md). P7 there may later evaluate this safeguard; this plan does not depend on P7.

## 1. Problem

A review finding, qualification amendment or decision request can present a **newly binding obligation** as if an authoritative source already required it. Downstream work then pursues the stronger obligation as mandatory.

**Motivating incident (Case F, U4-A, 2026-11-21).** The full chronology and SHA-256 pins are in [research note Case F](../design/ideas/pending/perspective-intent-research-note.md#case-f--cleanup-attribution-in-u4-a-a-planning-sentence-amplified-by-review) and [proposal §3](../design/ideas/pending/pilots/perspective-intent-pilot.md#3-seed-cases-and-the-actual-level-of-evidence). In short:

1. RL contract `043` said "The Host owns the child, same-session framed handshake, and exact stop", which bundles action, outcome and responsibility.
2. `PLAN.md` H4 required only independent post-exit confirmation that the container ID was absent.
3. An Astra architecture review read 043 as requiring "affirmative Host-attributed normal removal". It cited evidence-and-claims §3.9 as justification.
4. The coordinator adopted that reading in `QUALIFICATION-AMENDMENT.md`, under operator authority it cited only generally.
5. The decision request `OPERATOR-DECISION.md` described the stronger requirement as one of "the H1–H5 requirements [that] are unchanged".
6. About 6.5 hours of wall-clock time went to proving exclusive cleanup causality before the operator restored the outcome requirement and made Docker the removal owner.

**What failed.** Not the doctrine: core contracts §1.1 and evidence-and-claims §3.9 already forbid structural constraints without an independent authority. They failed because "required by an authority" could point at prose that supported either reading. Nothing required the citation to resolve to the **exact property** being made binding.

**Route.** The incident ran under manual orchestration as Markdown packets: an architecture review, a supervisor amendment and a decision request. It did not pass through `implementation-review/v1`, `OrganizationalRequirements` or review preparation. A safeguard on typed flows alone would not have caught it.

## 2. Objective

Prevent any review finding, amendment, revised plan or decision request from presenting an obligation as **binding and inherited** unless its provenance establishes that exact property. Cover both routes:

- **Typed route:** Rust review results and campaign remediation.
- **Manual route:** manual-orchestration architecture reviews, supervisor dispositions and amendments, and decision requests.

Neither route counts as protected until its actual enforcement point consumes the check's result.

## 3. Non-goals

- Interpreting arbitrary natural-language plans. Unstructured plans remain supporting evidence; ambiguity is reported, not resolved.
- Suppressing findings. A reviewer may still raise any concern, including a proposed new obligation.
- Changing ordinary defect findings. A panic, race or security defect needs no contractual citation.
- Reducing owner authority. An owner may adopt a stronger obligation; the safeguard then records and validates it.
- Replacing the review-result parser as an authority gate. `implementation_review_v1` keeps validating result shape.
- A production gate for the research programme's detection methods.

## 4. Semantics

**Obligation assertion:** a statement that a property is *required for acceptance*. Only assertions in the closed v0 property vocabulary are in scope:

| Property | Example |
|---|---|
| `initiator` | which component must issue the action |
| `outcome_owner` | which component or mechanism must bring about the outcome |
| `verifier` / observer class | who must establish the outcome, and with what independence |
| `independence` | required independence sources |
| `evidence_custody` | who must hold or produce the evidence |
| `path_fact` (realization constraint, by dimension) | how, or through which route, the outcome must be produced |

Statements outside the vocabulary are ordinary findings and pass through unchanged. Extending the vocabulary is a reviewed change.

Each assertion has a **status** and a **citation**:

- `inherited`, citing `{source id, revision, digest, property, span}`;
- `proposed`, a new obligation offered to the owner;
- `owner_adopted`, citing an owner decision record that names the property.

**Parent requirement fields** carry a status of `binding`, `selected_realization`, `unspecified` or `ambiguous`. `ambiguous` is set when the source text supports more than one reading; it is never resolved silently.

**The rule:**

> An assertion may be `inherited` only if its citation resolves to a binding parent requirement covering that exact property. Making binding a property the parent leaves `unspecified`, `ambiguous` or absent is strengthening, and needs `owner_adopted` provenance naming that property. Selecting a realization that satisfies the parent is not strengthening and stays model judgment (core contracts §4).

**Dispositions:**

| Disposition | Meaning | Consequence |
|---|---|---|
| `supported` | Inherited from a covering binding requirement, or adopted by a named owner decision | May be treated as binding |
| `proposal` | Marked `proposed` | Routed to the owner; cannot by itself force remediation |
| `requires_owner_decision` | Claimed inherited or binding, but the citation is missing, general, or does not cover the property | Same as `proposal`, plus a recorded provenance defect |
| `ambiguous_parent` | The cited parent field is `ambiguous` | Cannot claim inheritance; owner resolution needed |
| `contradictory` | Conflicts with a binding parent requirement | Owner decision required; the parent remains binding |

A general authority citation (for example "existing operator authority" or "§3.9") is never property-specific provenance. §3.9 requires an independent authority and cannot be its own source.

## 5. Components

### C1: pure provenance check (Rust)

A small pure library plus CLI. No model, no network, no I/O beyond explicit input files.

- **Inputs:**
  - an accepted-requirements set: typed `OrganizationalRequirements` conditions where available, otherwise a small structured requirements document with ids, properties, statuses, owner, revision and digest;
  - an assertion set;
  - optional owner decision records.
- **Output:** one disposition per assertion, with reasons and the spans and digests used.
- **CLI exit codes**, following the manual-orchestration convention (`handoff_validator.py`): `0` all assertions `supported` or `proposal`, `21` any provenance defect, `32` not established (missing or unreadable input; never treated as a pass).
- **Placement:** open question (§9). Candidates are a new small crate, or a module upstream of `work-engine-organization`. Not inside `OrganizationalRequirements` v0, whose single obligation kind (`candidate_review`) should not grow into a general interpreter.

### C2: typed route

- **Finding provenance.** Two options to compare in plan review:
  - **(a)** an `implementation-review/v2` schema whose findings may carry an optional `obligation` block (property, status, citation);
  - **(b)** a separate provenance sidecar bound to the result digest and finding ids, leaving v1 unchanged.

  `implementation_review_v1` uses a closed field set (`exact_fields`), so (a) is a versioned schema change, not a minor addition.
- **Enforcement point.** The policy is: a finding whose only binding basis is `proposal` or `requires_owner_decision` cannot by itself make remediation mandatory. A defect violating a `supported` requirement still blocks, and ordinary defect findings are unaffected. The candidate enforcement point is campaign remediation selection (`slice-campaign` `RemediationSelection`, which already requires verified acceptance sources), together with the episode reducer. Confirm the exact consuming boundary in plan review before implementation.

### C3: manual route

- **Adoption boundary.** The architectural-escalation policy (`skills/manual-orchestration/references/architectural-escalation.md`) requires, before another repair, a record of "the design judgment, the authorized owner's disposition, the exact retained or revised plan and the bounded next scope". In U4-A that record was `QUALIFICATION-AMENDMENT.md`. This is where an amplified obligation became binding, together with decision requests that restate requirements.
- **Mechanism.** Any such disposition, amendment or decision request that asserts an in-scope obligation carries a structured provenance block (fenced JSON) listing its assertions and citations. The supervisor runs the C1 CLI and records the receipt (exit code and output digest) in the document. Requirements described as "unchanged" are checked against the parent like any other inherited assertion.
- **Consuming boundary.** A CLI that detects amplification is not enforcement if the supervisor can adopt without running it. Options, to decide in plan review:
  - **(M1)** skill rule plus a required receipt in the disposition. This is auditable but not preventive.
  - **(M2)** repair dispatch packets must carry the passing receipt, and the worker reference instructs builders to refuse repair against a revised plan without one. This adds an instruction-level consumer at dispatch.
  - **(M3)** a mechanical check in dispatch tooling, if a suitable hook exists. Not yet investigated.

  The manual route counts as protected only at the level the chosen option actually achieves, and the plan's acceptance must say which.
- **Instruction changes** to architectural escalation, supervisor packets and reviewer references are agent-instruction changes. They need review under [instruction design](../docs/contracts/instruction-design.md) and reach agents only through skill release.

## 6. Fixtures and acceptance

**Decisive regression (F1, U4-A).** Reconstruct the chain from the pinned packet files (043, `PLAN.md` H4, the re-escalation finding, `QUALIFICATION-AMENDMENT.md`, `OPERATOR-DECISION.md`), copied into the test fixtures at their pinned digests. Expected:

- the H4 verified-absence requirement is `supported` and remains binding;
- the Host-attribution assertion is `requires_owner_decision` (043's outcome owner is `ambiguous`; no owner decision names the property);
- it cannot become mandatory remediation on the typed route, or pass the adoption boundary on the manual route without an owner decision naming the property;
- the decision request's "unchanged" claim for attribution is flagged as an unsupported inheritance.

A safeguard that does not catch F1 does not pass acceptance.

**Further fixtures:**

| Fixture | Expected |
|---|---|
| F2 owner explicitly requires Host-attributed removal | `supported` (owner_adopted) |
| F3 ordinary realization: Host chosen to issue the stop; Docker auto-removal chosen as mechanism | not flagged |
| F4 ambiguous parent: 043 wording typed as `outcome_owner: ambiguous` | `ambiguous_parent`; no inheritance |
| F5 plain correctness finding (panic on empty input) | passes untouched; no provenance required |
| F6 citation to a genuine accepted contract, wrong property | `requires_owner_decision` (citation does not cover property) |
| F7 genuine defect violating a `supported` requirement | still allowed to block |
| F8 general citation only ("existing operator authority", "§3.9") | `requires_owner_decision` |

**Route acceptance.** For each route, test **enforcement, not only validation**:

- typed route: remediation selection refuses a proposal-only blocker and accepts F7;
- manual route: the chosen consuming boundary rejects an adoption without a passing receipt, at the protection level the plan claims.

## 7. Proposed slices (each separately approved and reviewed)

1. **S1:** C1 library and CLI, property vocabulary v0, fixtures F1–F8 at the check level.
2. **S2:** typed route, choosing v2 or a sidecar, plus the remediation enforcement point, with route acceptance tests.
3. **S3:** manual route: provenance block, receipt, consuming boundary, instruction changes with instruction review and skill release, and the manual route acceptance test.

S1 has no dependency on S2 or S3. S2 and S3 can proceed independently once S1 is accepted.

## 8. Risks

- **Friction.** Requiring provenance too broadly would burden ordinary reviews. Mitigation: the closed property vocabulary, with ordinary defects exempt.
- **Ritual compliance.** Supervisors may fill citation blocks perfunctorily. Mitigation: substantive property matching (F6, F8), and receipts that bind digests.
- **False confidence on the manual route.** Mitigation: state the protection level actually achieved (§5 C3).
- **Vocabulary gaps.** Amplifications outside the v0 properties pass unchecked. Mitigation: record observed gaps; P7 can measure them.

## 9. Open questions for plan review

1. v2 schema versus provenance sidecar for typed findings.
2. C1 placement: new crate or module, and its relationship to `work-engine-organization`.
3. Whether `OrganizationalRequirements` conditions are actually the parent representation in current `slice-campaign` flows, or a separate requirements document is needed there too.
4. The exact typed consuming boundary (`RemediationSelection`, reducer, or both).
5. Manual consuming boundary: M1, M2 or M3, and the protection level each achieves.
6. Owner decision record format (board message, file with digest, or both).
7. Whether the v0 property vocabulary is sufficient for Cases B and F.

## 10. Review request

An independent plan review (Opus at xhigh effort through the `claude-sdk/` namespace, which bills the Claude subscription) should check:

- whether the semantics in §4 match core contracts §1.1, §4 and evidence-and-claims §3.9;
- whether the fixtures, F1 especially, would actually catch the incident on its real route;
- whether the proposed enforcement points exist as described;
- whether the plan avoids burdening ordinary defect findings.
