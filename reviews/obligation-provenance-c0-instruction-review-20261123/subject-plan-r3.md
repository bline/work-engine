# Obligation-provenance safeguard: plan

**Status:** Draft plan, revision 3, documentation only. Not approved and not an implementation authorization.
**Date:** 2026-11-23
**Decision owner:** operator
**Revision 3:** narrowed to C0, an instruction-level change on the manual route, by operator decision (option A, 2026-11-23) after the [second independent review](../reviews/obligation-provenance-plan-opus-20261123/review-r2.md) (`revise_and_resubmit` on revision 2). The Rust comparator and typed route (revision 2's C1–C3) are parked; §7 records their unmet preconditions. Earlier revisions and both reviews are kept in [the review record](../reviews/obligation-provenance-plan-opus-20261123/README.md).

**Research relationship:** independent of the [perspective and intent pilot](../design/ideas/pending/pilots/perspective-intent-pilot.md). P7 there may later evaluate C0.

## 1. Problem

A review finding, escalation disposition or decision request can present a **changed acceptance condition** as if an authoritative source already required it, or as unchanged. The condition's actual owner is then never asked, and downstream work pursues the changed condition as mandatory.

**Motivating incident (Case F, U4-A, 2026-11-21).** The chronology and SHA-256 pins are in [research note Case F](../design/ideas/pending/perspective-intent-research-note.md#case-f--cleanup-attribution-in-u4-a-a-planning-sentence-amplified-by-review) and [proposal §3](../design/ideas/pending/pilots/perspective-intent-pilot.md#3-seed-cases-and-the-actual-level-of-evidence). In short:

1. **The contract.** RL contract `043` assigns the Host "exact stop" and requires clean cancel to conclude after "exact container removal observed against the owned identity". For removal *attribution* it is ambiguous to unspecified. 041 records the ownership split: "RL owns task-host and lifecycle admission/receipt semantics; dev-vertical owns the actual Pi Host controller and rendered container boundary."
2. **The plan.** `PLAN.md` defers to 041 and 043. Its H3 requires the Host to "distinguish confirmed removal, removal failure and unknown cleanup", and its H4 requires independent post-exit absence. The selected realization had the Host run `docker rm`.
3. **The first review.** The re-escalation review (`12127e46…`) made a point supported by H3: while the Host performs removal, a removal that failed or never started is reported as failure (`held_failed + confirmed_absent`), not as a clean cancel. It also demanded "an affirmative result" that distinguishes the Host's removal from "already missing". On a shared Engine that means excluding concurrent removers, which is causal attribution and goes beyond H3.
4. **Adoption without the owner.** Within about a minute, the coordinator adopted a "positively attained normal-removal witness" and dispatched repair writers (08:59Z). It acted under a general overnight delegation, without asking RL, the owner of the clean-cancel receipt meaning.
5. **The realization-independent strengthening.** The resolved-object review (`e8304cda…`) made Host-positive removal an independent §3.9 claim, ruled out Engine auto-remove, and advised "Do not ask for permission to accept eventual absence instead". The decision request, and the coordinator's 09:43Z message to the operator, described the change as "The H1–H5 requirements are unchanged" inside a yes/no design question.
6. **The cost.** Causality work ran from about 08:58Z to 09:43Z. U4-A was then parked awaiting the operator until 15:07Z, and the operator's realization change (create-time `--rm`, Docker as removal owner) followed at about 15:26Z. An owner question raised at 08:59Z would also have waited for the operator. The realistic saving is about 45 minutes of engineering, a correctly framed question, and an honest record, not the full 6.5-hour window.

**What failed.** Not the doctrine; core contracts §1.1 and evidence-and-claims §3.9 already require an independent authority. The failure was in the record:

- a change to an acceptance condition never appeared as a change;
- its owner was taken from whoever wrote the disposition, not from the governing source;
- the owner question was actively discouraged;
- the operator was shown the change as "unchanged".

**Route.** Manual orchestration, as Markdown packets and free-text Pi `subagent` dispatch. There is no mechanical consumer on this route today.

## 2. Objective

On the manual route, make every change to an acceptance condition **visible as a change**, with its **owner taken from the governing source** and that owner's decision or `pending`, before work treats the change as binding.

**Protection level:** instruction-level. Nothing mechanical prevents a supervisor from ignoring the rule; the table makes the omission visible and auditable afterwards.

## 3. Non-goals

- Any Rust component (parked, §7).
- Suppressing findings or concerns, including proposed new obligations.
- Changing ordinary defect findings.
- Reducing owner authority, or demoting obligations an owner has accepted.
- Blocking unrelated work, diagnosis, design review, or realization choices the supervisor makes and labels as its own.

## 4. C0 rules

1. **Change table.** An escalation disposition that changes or reinterprets an acceptance condition lists each change in a table: the condition as changed, the quoted parent span it changes (path, lines, digest), the owner of that condition's meaning, and that owner's decision reference or `pending`.
2. **Owner from the governing source.** The owner is whoever the governing source names for the condition's meaning. In U4-A that was RL, for clean-cancel receipt semantics (041). The disposition's author is the owner only if the source names it.
3. **Deliver `pending` rows.** Each `pending` row goes to its owner when recorded.
4. **Per-row hold.** A `pending` row is not treated as an acceptance condition or required repair until its owner decides. Other rows, unrelated work, diagnosis and design review continue. So do realization choices the supervisor makes within its authority and labels as its own, rather than as owner requirements.
5. **"Unchanged" only with the span.** A requirement may be described as unchanged only when its row quotes the unchanged parent span.
6. **Decision requests.** A decision request to the operator or another owner that restates requirements presents every changed or reinterpreted condition as a change row, never as unchanged. An approval adopts only what the request presented as changes; generic assent adopts nothing presented as unchanged.
7. **Decision references are instruction-level.** A reference may be the owner's own message, or its board post with its session id. These are auditable, not authenticated. Authentication belongs to the parked comparator (§7).
8. **Reviewers.**
   - For a requirement called inherited, quote the exact parent span, and label interpretations of ambiguous text as interpretations.
   - For a requirement that holds only for the current realization, say so, and limit it to the consequence named.
   - Never advise against asking the owner whether a requirement applies.

## 5. Proposed instruction text

Exact wording for the instruction review. Each rule keeps its reason next to it, as instruction design (§3.1) requires.

**A. `skills/manual-orchestration/references/architectural-escalation.md`.** Insert after "Before another repair, record the design judgment, the authorized owner's disposition, the exact retained or revised plan and the bounded next scope.":

> When that decision changes or reinterprets an acceptance condition, list each change in a table: the condition as changed, the quoted parent span it changes (path, lines and digest), the owner of that condition's meaning, and that owner's decision reference or `pending`. Take the owner from the source that governs the condition, not from who wrote the disposition. On 2026-11-21 a supervisor adopted a reviewer's stronger cleanup requirement under a general delegation, and the requirement's owner was never asked. Deliver each `pending` row to its owner when you record it. Until the owner decides, do not treat a `pending` row as an acceptance condition or as required repair. Other rows, unrelated work, diagnosis, design review and realization choices you make and label as your own continue. Call a requirement unchanged only when its row quotes the unchanged parent span: a change described as unchanged hides it from the owner who must decide it. A decision reference may be the owner's own message, or its board post with session id.

**B. `skills/manual-orchestration/references/worker.md`.** Append to the opening paragraph:

> As a reviewer, quote the exact parent span for any requirement you call inherited, and label interpretations of ambiguous text as interpretations. If a requirement holds only for the current realization, say so and limit it to the consequence you name, since other realizations may not need it. Never advise against asking the owner whether a requirement applies: your reading becomes binding only through the owner's decision.

**C. `skills/manual-orchestration/references/supervisor-acceptance.md`.** Append to "Acceptance and result":

> When a decision request to the user or another owner restates requirements, present every changed or reinterpreted acceptance condition as a change row, as in architectural escalation, never as unchanged.

(In the target file, "architectural escalation" links to `architectural-escalation.md`.) An approval adopts only what the request presented as changes.

## 6. Acceptance

1. **Instruction review** of A–C under [instruction design](../docs/contracts/instruction-design.md), using the `agent-instruction-review` skill. It checks structural necessity, causal reason placement, semantic ownership and loading placement. It also checks that the rules do not prescribe a method where only an outcome is required.
2. **U4-A walk-through,** conditional on rules 2–6. At 08:59Z, the amendment's table would contain:

   | Condition as changed | Parent span | Owner | Decision |
   |---|---|---|---|
   | Clean cancel requires a positively attained normal-removal witness | 043 "Terminal cleanup" | RL (041 ownership split) | `pending` |

   That row's repair is held and delivered to RL. The amendment's other repairs (durable evidence and the fixture caller contract) proceed. A coordinator naming itself as owner violates rule 2, because 041 names RL. At 09:43Z the operator would have seen the change as a change row, not as "unchanged".
3. **Counter-checks:**
   - adopting create-time `--rm`, labelled as the supervisor's realization choice, is not held;
   - an ordinary defect finding produces no table row.
4. **Skill release.** The changed references reach agents only through [skill release](../skills/skill-release/SKILL.md), as a separately approved step.

## 7. Parked: Rust comparator and typed route

Revision 2's C1 (pure provenance comparator), C2 (manual receipt with builder re-run) and C3 (typed review route) are parked. The second review found they would return a provenance defect on every real document until owners change how they record decisions. They are revisited when the preconditions below hold, or when P7 or a typed-route incident supplies evidence.

| Precondition | Source |
|---|---|
| Owner-typed requirement sets exist and are published per workstream | review-r2 N4, §9 Q3 of revision 2 |
| Agent owners such as RL can authenticate decisions; board fields are caller-supplied | N4, revision 2 §9 Q1 |
| Operator decisions are recorded as change rows with digests. C0 rule 6 starts producing this record | N4 |
| `realization_conditional` constrained to the protected requirement's value space, with an independent challenge before it can pass; F1a corrected and F1a′ added | N1 |
| F1 expectations stated for each stand-in typing of 043, not by claimed blind typing | N2 |
| Coverage tested by superset for inherited obligations and membership for realizations, with value enumerations defined | N3 |
| A subordinate source cannot settle what its governing source leaves open; 041 added to F1 | N7 |
| A boundary between in-vocabulary defects and obligations; `acceptance_evidence` narrowed | N8 |
| Any first comparator slice is fixture-only, or gated on the above for one real workstream | N10 |

## 8. Risks

- **Ritual tables.** Supervisors may fill tables perfunctorily. Mitigation: owners come from the governing source (rule 2), and spans must be quoted (rules 1 and 5), which makes a wrong row visible.
- **Owner unavailability.** Holding a row can stall the work that depends on it. Mitigation: holds are per row, and everything else continues. The U4-A park shows the stall would have occurred anyway.
- **Instruction-level only.** Nothing mechanical enforces C0. Mitigation: stated plainly (§2); the parked comparator is the mechanical path.

## 9. Response to the second review

| Finding | Disposition in revision 3 |
|---|---|
| N5 (owner column fillable by its author; decision requests uncovered) | Rules 2, 3 and 6; text A and C |
| N6 (hold scope; RL release; cost claim) | Rules 4 and 7; §1 item 6 corrected |
| N11 (design-document inaccuracies) | Corrected in the research note and pilot in the same commit |
| N1–N4, N7, N8, N10 | Parked with the comparator, as preconditions in §7 |
| N9 (comparator mechanics) | Parked |
| H2, H3, L1, L2, M1, M6, M7 (closed in revision 2) | Retained where they apply to C0; the rest moves to the parked comparator |
