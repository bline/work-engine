# Obligation-provenance safeguard: plan

**Status:** C0 implemented and released (operator-approved 2026-11-23). Texts A–C applied in commit `e76f3adc`, deployed as skill release `e76f3adcc88cda99f76951ca440acd70e44e4dc2` (verification `identical`), announced on the board as #6817. The parked components (§7) remain unapproved.
**Date:** 2026-11-23
**Decision owner:** operator
**Revision 3.1:** adopts the corrected wording from the [C0 instruction review](../reviews/obligation-provenance-c0-instruction-review-20261123/review.md) (`accept_with_corrections`, conditional on the owner rule and change test, both adopted here), states C0's actual reach, and reruns the U4-A walk-through against the corrected text. **Revision 3:** narrowed to C0, an instruction-level change on the manual route, by operator decision (option A, 2026-11-23) after the [second independent review](../reviews/obligation-provenance-plan-opus-20261123/review-r2.md) (`revise_and_resubmit` on revision 2). The Rust comparator and typed route (revision 2's C1–C3) are parked; §7 records their unmet preconditions. Earlier revisions and both reviews are kept in [the review record](../reviews/obligation-provenance-plan-opus-20261123/README.md).

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

Make every change to an acceptance or qualification condition **visible as a change**, with its **owner taken from the governing source** and that owner's decision or `pending`, before work treats the change as required or as resolved.

**Reach.** Text A goes into `architectural-escalation.md`, which is shared by manual orchestration and configured slice supervision, and is read before every repair dispatch (`supervisor-dispatch.md`; `slice-supervisor/SKILL.md`). So A binds both routes and every disposition that file governs, not only post-design-review dispositions. In configured campaigns, `pending` rows are delivered through the configured adapters. Text C applies to decision requests when reporting or stopping. Text B reaches reviewers that load manual orchestration; a reviewer launched under another skill or prompt does not receive it.

**Protection level:** instruction-level. Nothing mechanical prevents a supervisor from denying that a change goes beyond its parent span; the record makes such a denial visible and auditable afterwards.

## 3. Non-goals

- Any Rust component (parked, §7).
- Suppressing findings or concerns, including proposed new obligations.
- Changing ordinary defect findings.
- Reducing owner authority, or demoting obligations an owner has accepted.
- Blocking unrelated work, diagnosis, design review, or realization choices the supervisor makes and labels as its own.

## 4. C0 rules

1. **What counts as a change.** A change is anything the quoted governing span would not by itself require or allow, including the author's reading of ambiguous text. Qualification rules are included. A wholly new condition has parent span `none`; the column is never an escape.
2. **Change record.** For each change a disposition makes, record: the condition as changed; the parent span (path, lines, digest) from the governing source; the owner of the condition's meaning; and that owner's decision or `pending`.
3. **Owner from the governing source.** Not the disposition's author, a reviewer, or a plan that defers to the governing source. If the source splits ownership or leaves it unclear, name the other party and record `pending`. Settling that in your own favour is the ownership decision core §11 reserves.
4. **Realization choices.** A realization choice presented as the author's to revise is not a change. It must not be presented to others as a requirement.
5. **Deliver and hold.** Each `pending` row goes to its owner when recorded. Until the owner decides, the row is neither required repair nor resolved, in either direction. Other rows proceed.
6. **Decisions.** A decision is the owner's own message, or its board post with session id, that decides the change or expressly delegates it. A general delegation that never addresses the change is not a decision. This leaves the operator free to delegate explicitly.
7. **"Unchanged".** A requirement restated or paraphrased as unchanged must have its parent span quoted beside it.
8. **Decision requests.** A decision request that restates requirements shows each changed or reinterpreted condition as a change, with span, owner and decision or `pending`. "Changed" is judged against what the owner last decided, not against the supervisor's or a reviewer's disposition. Assent to a request does not adopt a change the request labelled unchanged.
9. **Reviewers.**
   - Quote the governing source's exact span for any requirement presented as required by it.
   - Label readings of ambiguous text, or anything beyond the span, as proposed changes for the condition's owner.
   - Limit realization-specific requirements to the consequence they name.
   - Do not discourage the owner question unless citing that owner's decision on it.

## 5. Proposed instruction text

Exact wording, as corrected by the instruction review. Each rule keeps its reason next to it (instruction design §3.1).

**A. `skills/manual-orchestration/references/architectural-escalation.md`.** A new, self-contained paragraph after the end of "Review and release of the hold" ("…serious regression or recurring invariant violation escalates again."), so it does not disturb the antecedent of "Bind that decision…":

> Changing an acceptance or qualification condition is a contract change that needs its owner's authority (core contracts §1.1). So for each change a disposition makes, record: the condition as changed; the parent span it changes, quoted with path, lines and digest from the source that governs the condition (`none` for a new condition); the owner of the condition's meaning; and that owner's decision or `pending`. A change is anything the quoted span would not by itself require or allow, including your reading of ambiguous text. A realization choice you present as yours to revise is not a change, but do not present it to others as a requirement. Take the owner from the governing source, not from the disposition's author, a reviewer, or a plan that defers to that source. If the source splits ownership or leaves it unclear, name the other party and record `pending`: settling that in your own favour is the decision you lack. On 2026-11-21 a supervisor adopted a reviewer's stronger cleanup requirement under a general delegation, and the requirement's owner was never asked. Send each `pending` row to its owner when you record it. Until the owner decides, the row is neither required repair nor resolved; other rows proceed. A decision is the owner's own message, or its board post with session id, that decides the change or expressly delegates it. Do not call a requirement you restate or paraphrase unchanged unless you quote its parent span beside it: a change described as unchanged hides it from the owner who must decide it.

**B. `skills/manual-orchestration/references/worker.md`.** Append to the opening paragraph:

> As a reviewer, quote the governing source's exact span for any requirement you present as required by it. Label a reading of ambiguous text, or anything beyond what the span states, as a proposed change for that condition's owner, not as inherited. If a requirement holds only for the current realization, say so and limit it to the consequence you name, since other realizations may not need it. Do not discourage asking the owner whether a requirement applies unless you cite that owner's decision on it: a requirement its source does not plainly state binds only through its owner's decision.

**C. `skills/manual-orchestration/references/supervisor-acceptance.md`.** Append to the reporting paragraph of "Acceptance and result":

> When a decision request to the user or another owner restates requirements, show each changed or reinterpreted acceptance condition as a change, with its quoted parent span, owner, and decision or `pending`, as in architectural escalation. Judge "changed" against what the owner last decided, not against your own or a reviewer's disposition. An owner shown a change labelled unchanged cannot decide it, and assent to that request does not adopt it.

(In the target file, "architectural escalation" links to `architectural-escalation.md`.)

## 6. Acceptance

1. **Instruction review.** Done: [`accept_with_corrections`](../reviews/obligation-provenance-c0-instruction-review-20261123/review.md). §5 now carries its corrected A–C. If any later edit departs from that wording, the review is re-run on the edit.
2. **U4-A walk-through against the corrected text.** This is an inference, not a demonstration.
   - **08:59Z.** `043-u4a-held-contract.md:19` (`4bf5ee60…`) requires "exact container removal observed against the owned identity". It does not by itself require a positive witness distinguishing normal removal from "already absent", so the amendment's rule is a change (rule 1).
   - **Who owns it.** `PLAN.md:32` gives dev-vertical "the separate acceptance decision", but `PLAN.md` defers to 041 and 043, so it cannot settle ownership (rule 3). The reviewer's statement (`12127e46:69`: "Dev-vertical owns the local Host amendment…") cannot settle it either. 041:15 splits the Host controller (dev-vertical) from receipt semantics (RL), so ownership is split.
   - **The row:**

     | Condition as changed | Parent span | Owner | Decision |
     |---|---|---|---|
     | Clean cancel requires a positively attained normal-removal witness distinguishing normal removal from already absent | `043-u4a-held-contract.md:19`, `4bf5ee60…`, "exact container removal observed against the owned identity" | RL (041:15 receipt semantics; split with dev-vertical) | `pending` |

     That row is held and sent to RL. The amendment's other repairs proceed.
   - **09:43Z.** "Changed" is judged against 043 and `PLAN.md`, not the amendment, so the operator sees a change row instead of "The H1–H5 requirements are unchanged".
   - **Residual.** A supervisor could still deny that the witness goes beyond `043:19`. Nothing mechanical prevents that (§2).
3. **Counter-checks.**
   - An ordinary defect finding produces no row.
   - A supervisor-labelled realization choice (for example a supervisor deciding to add an extra inspection step) is not a change. If it is later presented to builders or the owner as a requirement, that presentation violates rule 4.
   - A self-owned addition still produces a row (owner: self, decision: this disposition). That is an audit trail, not a hold.
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

## 10. Response to the C0 instruction review

| Finding | Disposition |
|---|---|
| F1 (owner rule lets a sincere coordinator name itself) | Adopted: governing source ranked above deferring plans and reviewers; split or unclear ownership becomes `pending` (rule 3, text A) |
| F2 (trigger depends on author's classification) | Adopted: objective change test; qualification rules named; `none` parent span (rule 1, text A) |
| F3 (C's baseline undefined, no reason, linked row format) | Adopted: baseline is the owner's last decision; fields inline; reason added (text C) |
| F4 (general delegation admitted as decision) | Adopted (rule 6) |
| F5 ("unchanged" presupposes rows; too broad) | Adopted: targets restated or paraphrased requirements (rule 7) |
| F6 (hold one-directional) | Adopted: neither required nor resolved (rule 5) |
| F7 (exemption duplicative and escapable) | Adopted: duplicates removed; realization choice defined (rule 4) |
| F8 (insertion breaks antecedent) | Adopted: self-contained paragraph at the end of "Review and release of the hold" |
| F9 (B's trigger narrow; "Never" over-broad) | Adopted (text B) |
| F10 (reach) | Adopted: §2 states both routes and all dispositions the file governs |
| F11 (trailing approval sentence) | Adopted: dropped; replaced by text C's last sentence |
| F12 (weak examples) | Adopted: §6.2 row with lines and digest; new counter-check |
| F13 (B's reach limited to manual-orchestration reviewers) | Recorded as a known limit (§2) |

