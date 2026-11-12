# Research Knowledge Lifecycle and Decision Workflow — v0.2

Date: 2026-11-11. Status: **corrected design proposal for the Rust implementation**.
This is a planning candidate, not an accepted implementation contract. The user
authorized correcting the design and beginning implementation planning in Codex
session `01a1ccc5-012b-7111-9ec6-21fc5648f678`. That authorizes this planning work;
it does not accept the proposed semantics or authorize implementation.

## 1. Basis and authority

This develops the owner's full v0.1 draft supplied in that session and the
[saved v0.1 principles](research-knowledge-lifecycle-and-decision-workflow-v0.1-20261111.md).
The saved file recorded only thirteen principles because the full draft was
unavailable to its author. Keep it and the
[phase handoff](research-to-decision-workflow-contract-handoff-20261111.md)
as dated inputs. This proposal supplies the fuller design and addresses the
six advisory findings from the subsequent review; it adds no owner approvals.

[DESIGN.md](../DESIGN.md) is binding doctrine. [PHILOSOPHY.md](../PHILOSOPHY.md)
provides non-normative rationale. [Rust direction](../docs/rust-development.md)
governs implementation language and shared-code boundaries. Existing approved
semantics remain in force unless their owner changes them explicitly.

The established direction separates persistent observations, revisable
assessments, decision-specific applicability, applicability authorization and
execution authority. **D-6 is approved: an invalid research register produces
no A4 evidence card, including a partial or degraded card.**

The prior A4 approval record also preserves U1–U3 and B4: cross-decision
metadata-only disclosure, the approved pending/declined/other-revision notices,
omission of sibling-decision disclosure in A4 v1, and the historical-membership
safeguard. Their evidence and the remaining reservations are reconciled in the
[implementation plan](research-lifecycle-rust-implementation-plan-20261111.md).

## 2. Objective and first result

Work Engine should accumulate research about performing work, assess it with
the required independence, use it where justified, and learn from qualified
execution outcomes. It should distinguish what was recorded, what an assessment
supports, what applies to this decision, and what the actor may do.

The first result is one real governed decision with a reconstructible basis:
qualified research and material objections inform a reviewable recommendation;
separately established authority permits bounded execution; verification and
receipts become candidate research observations. An exploratory comparison
with an appropriate baseline measures the integrated workflow without claiming
that one successful decision establishes a general improvement.

## 3. Semantic owners and permitted changes

These are responsibilities, not a requirement for one service or crate per row.
Existing owners retain their meaning; the implementation plan binds concrete
components and identifies missing operations.

| Responsibility | Owns | Boundary |
| --- | --- | --- |
| Observation/artifact producer and custody owner | Attributed reports, source bytes or durable references, environment and custody evidence | Source authenticity alone does not prove that the reported empirical event happened |
| Research/register owner and authorized publishers | Research identities, revisions, links, register rules and admitted records | Publication or structural validation does not establish scientific adequacy |
| Assessment publisher under its assessment contract | An attributed, revision-bound assessment, evidence, limitations and independence evidence where required | A model may assess within a grant; it cannot appoint itself the qualification authority |
| Qualification/acceptance owner named by the research contract | Whether required qualification conditions have been established for a named use | Acceptance is scoped; it is not a permanent truth flag |
| Decision/applicability owner | Decision identity and revision, applicability disposition and its authorization scope | Association, relevance and authorization are distinct |
| A4 projection capability | Deterministic selection and faithful presentation under the approved contract | Read-only; no new qualification, applicability judgment, grant or decision routing |
| A5 supervisor | Interpretation, alternatives, uncertainty and a recommendation within its role | A recommendation does not create an execution grant |
| Execution authority, admission and execution owners | Grants, action-time admission, bounded effects and their receipts | Each retains its own scope; research standing supplies no missing permission |
| Verification and feedback publishers | Attributed acceptance evidence and candidate observations from outcomes | Successful execution does not automatically accept a broad research conclusion |

Multiple assessment publishers may disagree. Their records remain available
under the consumption scope. A4 does not resolve disagreement by taking the
last timestamp. The qualification contract names any required resolver and
what unresolved competition means. One actor can fill several roles only
where the owning independence and authority contracts permit it.

## 4. Durable objects and revision binding

The design requires distinguishable identities and relationships, rather than
these exact field names or a universal storage framework:

- An observation identifies the attributed event/report, conditions, methods,
  artifacts, measurements and limitations. A receipt may be an observation's
  source; an unavailable measurement remains unavailable.
- A claim revision identifies a proposition and its conditions. Changed meaning
  creates a distinguishable revision rather than silently changing the subject
  of earlier assessments or authorizations.
- An assessment revision binds its claim subject, evidence revisions, assessor,
  basis, limitations, as-of context and any required qualification evidence.
  Corrections and competing assessments preserve the older record.
- An applicability record binds a finding/claim subject to an exact decision
  revision, a stated relation and an attributed disposition. Authorization has
  an independently established source and scope.
- A recommendation binds the decision, considered alternatives, decisive
  evidence, material uncertainty and proposed action. It need not preserve a
  private reasoning transcript.
- The decision basis binds the exact delivered card or equivalent projection,
  input and policy revisions, applicability authority, recommendation and the
  subsequent grant/receipt references. Existing receipts should carry these
  references where their owner permits, avoiding a parallel rationale owner.
- A reconsideration record names an affected basis, the triggering correction
  or change, the responsible decision owner and the resulting disposition or
  unresolved obligation.

**Proposed authorization binding:** the contract must identify the research
subject and conditions covered by an applicability authorization, in addition
to the decision revision. Reusing a stable finding ID cannot silently authorize
changed claim meaning. The owner defines whether a new assessment of the same
claim remains within that scope and which changes require reconsideration.
An assessment update does not inherently revoke a grant or require a new human
approval; neither may a consumer infer an extension of the existing scope.

Rust types and content identities can make subjects distinguishable. They
cannot establish that the named actor possessed authority. Public persisted
references and DTOs remain data; internal permits require the owning runtime
authorization checks.

## 5. Independent uncertainty facets

An assessment may express qualitative, evidence-backed concerns with explicit
unknowns. Numerical confidence, universal decay rates and uncalibrated
thresholds are outside the first milestone.

The dimensions include provenance, method reliability, reproducibility,
completeness, consistency, temporal relevance, context transfer and inferential
strength. Each reported judgment identifies its basis, limitations and what
was checked. A dimension not assessed is distinguishable from an assessment
whose result is uncertain.

Keep evidence/assessment standing, current-context relevance, applicability
relation, applicability authorization, anchor establishment and delivery
disposition independent. For example, an authorized relation may explicitly be
`not_established`; authorization does not turn it into positive support. A
historically well-recorded finding may simultaneously be contested and aging.

Refutation and supersession remain different relationships. A replacement
does not establish that its predecessor was false. Subsequent records may
change either conclusion without erasing earlier assessments.

## 6. Time, availability and historical meaning

Historical trust asks whether the recorded observation remains credible.
Current relevance asks whether it supports prediction or action under the
present conditions. Releases, configuration changes, contrary results or
pricing changes may matter more than elapsed time. No universal age penalty
is proposed.

The semantic distinctions needed include observation/effective time,
assessment time, durable recording or availability, decision consumption time,
and the scope of any reverification. Their exact encoding remains proposed.
A day-resolution timestamp or an asserted date cannot establish an ordering
that its evidence does not support; report that ordering as unknown.

Three historical products are different:

1. The actual contemporaneous basis: what this decision-maker received and
   relied on, with the authorization state and limitations then represented.
2. A later reading of research associated with an earlier decision revision.
3. A current reassessment of the earlier decision using later knowledge.

A later authorization or correction may appear in the second or third product.
It cannot be retroactively inserted into the first. D-8 must settle historical
delivery details without collapsing these products. An associated historical
finding is not proof that it was available or authorized at execution time.

## 7. A4 read-only knowledge projection

A4 consumes one validated, pinned register and evidence under the approved
scope. Its anchors are computed in process against the same snapshot identity,
as required by the Rust R-1a direction. Exact externally owned claim references
retain their owner, revision and digest; A4 cannot substitute a search result
or nearby claim for an unavailable exact reference.

Structural register validity is an admission prerequisite, not a judgment that
every record is established support. Invalid input produces diagnostics and
no card under D-6. A read failure that prevents validity from being established
also cannot furnish a valid-register input; preserve that distinct failure.

A4 copies recorded judgments and authority facts, applies approved mechanical
selection and derivation rules, and reports unresolved inputs. New semantic
evaluation or contradiction research is separately authorized work, outside the
projection operation. A4 must not become a second question router.

Useful delivery views include established authorized support, qualified context,
known objections, unresolved inputs, and refutation/supersession history. These
are views over the independent facets, not a single canonical knowledge-state
enum. Which records may supply each view is governed by approved selection and
disclosure rules; labeling content contextual does not authorize its delivery.

In particular, pending and declined records retain their approved notices and
delivery restrictions. Cross-decision disclosures remain metadata-only within
the caller's existing read scope. Historical unversioned current findings do
not become verified historical members. Omitted or truncated coverage is
visible; a bounded card never claims to contain all relevant research or all
possible contradictions.

**Q-R2-2 remains open:** for a valid register with an unqualified anchor,
including the rename cases, the owner must settle limited delivery versus
refusal of the affected card. Either policy preserves the gap and excludes the
unestablished relationship from confirmed support. A null class is not
evidence of `absent`. This proposal does not resolve that reservation.

## 8. Consumption and change between stages

A pin preserves a basis; it does not prove that its justification still applies
when consumed. The decision contract must state the relevant context and the
policy for consuming a basis after material changes.

The proposed first-milestone contract binds the card, recommendation and
execution request to exact subjects. At the owning admission boundary, check
the current decision revision, applicable grant and the change conditions that
the accepted consumption policy requires. That policy defines whether a known
correction, changed environment or changed research subject requires a revised
recommendation, additional evidence, explicit owner judgment or a truthful
unmet precondition. The implementation must make the selected boundary and its
evidence observable, including changes occurring during admission.

This is not a requirement to fetch the latest research before every action.
An owner may authorize a bounded use of an older basis. Material-change
eligibility and grant validity remain separate: concern does not silently
revoke authority, and an unexpired grant does not establish evidence adequacy.
Unrelated authorized work does not acquire a global halt from one unmet claim.

## 9. Workflow as a default operating shape

The useful lifecycle is capture → propose → assess → maintain → request a
decision → project → reason → authorize → execute/verify → feed back.
These are composable operations, not ten mandatory phases for every decision.
The model may reuse existing records, iterate, defer or change route within
its authority. Causal dependencies remain binding: a projection needs valid
inputs; execution needs applicable authority; a claimed independent result
needs its required independence evidence; feedback cannot publish an accepted
conclusion merely by relabeling an execution result.

A5 compares alternatives and material uncertainty. Depending on stakes,
reversibility, verification cost and existing authority, it may recommend an
action, propose an experiment, request evidence, defer or escalate. A cheap
reversible experiment and an irreversible production choice need not have the
same evidence requirements. The research standing itself grants neither action.
Already established authority can be used within its terms; a new approval is
needed only where the owning contract requires one.

## 10. Corrections, maintenance and feedback

Corrections append new assessments or events and nominate potentially affected
decisions through recorded dependencies. Preserve the earlier basis. The
reconsideration owner records whether the decision remains justified, requires
new work, is superseded or remains unresolved. Nomination and acknowledgment
alone do not establish that reconsideration occurred.

The first milestone needs this pathway for its selected real decision. It does
not require discovering every implicit dependency or an autonomous scheduler.
Report the known dependency scope and any unavailable coverage. Unresolved
reconsideration obligations need a durable owner rather than model context.

Verified outcomes become attributed candidate observations, including failures,
refusals, costs and unknown effects. Bind them to the execution and verification
receipts, exact conditions and acceptance scope. Retries/imports must retain
source identity so the same execution does not appear as independent replication.
The research contract decides qualification and any broader inference.

Later maintenance may nominate refreshes using decision impact, uncertainty,
reuse, transfer conditions, cost and required independent verification. Monitoring
or a due date supplies no experiment or spending authority. Distributed research
and sharing remain deferred and require their own custody, privacy, provenance
and local-applicability contracts.

## 11. Minimum Rust composition

Keep ownership narrow. The register owns research records, their validation and
pinned reading. A4 owns the read-only decision projection. Assessment and
decision-specific sufficiency remain attributed judgments of their respective
owners. A5 consumes the projection through a bounded adapter; the existing
execution/admission and receipt owners retain grants and effects.

Reuse existing claim, receipt and runtime contracts where their semantics and
qualification actually fit. Their existence alone does not establish readiness
for this composition. Rust private construction, distinct IDs and dependency
boundaries can prevent accidental mixing; durable publication and runtime
evidence establish cross-process truth.

No universal research manager, shared mutable status object, generic workflow
engine, new organization compiler or parallel authority system is required.
The first implementation should preserve one complete chain with the smallest
needed owned extensions. The companion plan identifies candidate placements,
qualification gates and what discovery has not established.

## 12. Acceptance scenarios for contract formation

Retain the original scenarios: current authorized evidence; aging evidence;
contradiction; decision revision change; unresolved anchor; invalid register;
correction after execution; different risk under identical uncertainty; and
candidate feedback from a qualified execution.

Add the cases exposed by this review:

- Decision revision unchanged, research claim meaning changed: the old
  authorization's scope is checked rather than inferred from the finding ID.
- Evidence or applicability authorization recorded after execution: historical
  association does not rewrite the contemporaneous basis.
- Correction between card issuance and admission, including an interleaving
  during admission: the selected consumption policy produces an observable
  outcome without conflating grant validity with evidence eligibility.
- Authorized, contested and aging at once: all facets survive projection.
- Pending/cross-decision material labeled contextual: disclosure restrictions
  still hold, and truncated opposition coverage is reported.
- Competing assessments and insufficient timestamp precision: no invented
  winner, chronology or confidence.
- Crash/retry during basis publication or feedback import: no mixed-revision
  basis, duplicate observation or implied settled effect.

The seventeen frozen A4 cards remain regression evidence for their earlier
contracts. Re-derive affected expectations after explicit owner decisions;
neither their current bytes nor this proposal silently settle D-8.

## 13. Comparison and measurement

Before interpreting the first comparison, bind the question, comparator,
acceptance criteria, evidence exposure, costs, failures and verification method.
Keep setup/research cost and per-decision consumption cost distinguishable.
Capture operator attention and unavailable measurements where material.

A replay after exposure to the research does not establish an unaffected
without-research baseline. Prefer a separately formed or otherwise justified
comparator whose limitations are recorded. One decision demonstrates integration
and supplies exploratory outcome evidence. Broader claims about improvement,
cost or transfer require a separately qualified study.

## 14. Decisions still required

The proposed subject binding, historical products, consumption policy,
qualification roles, concrete durability contracts and Rust interfaces need
owner acceptance before they govern implementation. The companion plan turns
these into bounded decision and engineering packets.

D-2, D-3 and D-8 remain open; their full subdecisions must be reconciled against
the source record. Q-R2-2 remains open. Q-B1 remains unresolved, with the saved
R-2 integration/distribution restrictions. R-2 repair `0d021609` has regression
evidence but no completed independent delta verdict; the coordinator's
acceptance-with-reservations remains separately attributed. See the
[qualification checkpoint](../records/research-register-r2-qualification-checkpoint-20261111.md).

This design neither waives those conditions nor mandates automatic research
spending, calibrated global scores, generalized scheduling, multi-installation
sharing or broad infrastructure rewrites.
