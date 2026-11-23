# Architectural escalation after repeated failure

This is the owner's supervisor policy for outer engineering work, shared by
manual orchestration and configured slice supervision. Repeated failed repairs
can expose a wrong plan, ownership boundary or acceptance model. Treating every
failure as another implementation defect can preserve that mistake indefinitely.
The supervisor owns escalation; builders and reviewers supply evidence and
surface triggers, while the authorized design owner decides the resulting plan.

## Signals and required response

| Signal in one subsystem | Supervisor response |
| --- | --- |
| First independent implementation rejection with serious defects | Record an architectural-risk warning; a bounded repair may continue. |
| Second substantive rejection after a repair in that subsystem | Require an architectural/design review before another ordinary repair cycle. A retained independent reviewer can supply this second verdict. |
| Confirmed serious regression caused by a repair, or recurring violation of the same invariant | Escalate to design review immediately, even before the rejection threshold. |

Serious defects include acceptance-blocking correctness, authority, provenance
or evidence failures (normally P1 or blocking P2). Count substantive verdicts
on distinct submitted implementation revisions, not each finding or repeated
delivery of the same report. Preserve the review's actual independence and
continuity; repeated verdicts from one isolated reviewer do not create multiple
independent reviewers. A recurring invariant violation still triggers immediate
escalation even when it appears in an inherited defect or outside a new verdict.

Flex capacity errors, transport interruptions, quota failures and reviewer
budget or context exhaustion are infrastructure interruptions, not implementation
rejections. Preserve any substantive findings already established before an
interruption; an incomplete review establishes neither acceptance nor absence
of defects.
A review that establishes a serious evidence-design defect remains substantive
even if its verdict is `blocked_evidence`; classify the cause, not just the label.

## Preserve the evidence across repairs

Use the existing assignment, review history or continuation to retain the
subsystem and invariant, exact rejected subjects and reports, attempted repairs,
inherited versus introduced origin, and current warning or review requirement.
Inherited defects still contribute when they repeatedly expose the same
architectural boundary. Relabeling a defect inherited, changing a reviewer or
builder, splitting a packet, renaming the subsystem or refreshing context does
not reset that evidence.

Once review is required, hold ordinary repair dispatch in the affected subsystem.
Diagnosis, evidence preservation, result collection and design review may
continue within existing authority; unrelated authorized work can continue.
Replacing the builder or calling a repair diagnosis does not discharge the hold.

## Review and release of the hold

Commission a fresh, separate **Astra xhigh architectural/design review** of the actual
plan and boundary. Give it the objective and owning contracts, exact failure and
repair history, inherited counterexamples, affected invariants and current
candidate. Ask whether bounded implementation repair is still credible, a local
design correction is needed, or the architecture/plan must be reopened. This
threshold requires reconsideration; it does not presume redesign is correct.

Keep configured provider, independence, capability, budget and approval
requirements. If the required review cannot run within them, record the blocked
review and the decision needed; do not silently substitute a reviewer or resume
ordinary repair. In a configured campaign, delegate subject inspection to the
authorized specialist through the configured adapter; the supervisor collects
the judgment rather than doing the domain review itself.

Before another repair, record the design judgment, the authorized owner's
disposition, the exact retained or revised plan and the bounded next scope.
Bind that decision to the history it considered so later evidence can be judged
against the reviewed plan without recounting old verdicts as new rejections.
Preserve old findings and explain how the decision addresses them; a completed
review alone does not clear findings, accept implementation or grant new scope.
A reviewed plan can reopen repair, but its history remains evidence and a new
serious regression or recurring invariant violation escalates again.

Changing an acceptance or qualification condition is a contract change that
needs its owner's authority (core contracts §1.1). So for each change a
disposition makes, record: the condition as changed; the parent span it changes,
quoted with path, lines and digest from the source that governs the condition
(`none` for a new condition); the owner of the condition's meaning; and that
owner's decision or `pending`. A change is anything the quoted span would not by
itself require or allow, including your reading of ambiguous text. A realization
choice you present as yours to revise is not a change, but do not present it to
others as a requirement. Take the owner from the governing source, not from the
disposition's author, a reviewer, or a plan that defers to that source. If the
source splits ownership or leaves it unclear, name the other party and record
`pending`: settling that in your own favour is the decision you lack. On
2026-11-21 a supervisor adopted a reviewer's stronger cleanup requirement under
a general delegation, and the requirement's owner was never asked. Send each
`pending` row to its owner when you record it. Until the owner decides, the row
is neither required repair nor resolved; other rows proceed. A decision is the
owner's own message, or its board post with session id, that decides the change
or expressly delegates it. Do not call a requirement you restate or paraphrase
unchanged unless you quote its parent span beside it: a change described as
unchanged hides it from the owner who must decide it.
