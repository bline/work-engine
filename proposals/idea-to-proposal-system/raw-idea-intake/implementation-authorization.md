# Bounded Implementation Authorization

## Authority and subject

This artifact records the user's separate implementation grant for proposal
`work-engine.raw-idea-intake`. The approved proposal decision is bound to
repository revision `0a71dfada0852b3d8445961ffeef9b15c27e0b43` and recorded in
[`decision.json`](decision.json).

On 2026-08-23, after approving the proposal meaning, the user instructed:

> Please use $slice-supervisor to implement the intake proposal. Perform as
> many slices as needed. Network is slow, increase timeout on all operations
> that require network, including Claude.

The user is the authority owner for this grant.

## Authorized consequence

The grant authorizes one `slice-supervisor` campaign to implement the approved
Raw Idea Intake and Claim Assessment proposal through as many coherent,
evidence-supported slices as are needed to complete its accepted scope.

The campaign may create the proposed intake capability and durable record
profile, validators, tests, documentation, representative dogfood artifacts,
and the minimum integration required to demonstrate the proposal's protected
semantic consequences. Each slice remains subject to the campaign's configured
planning, validation, review, workspace-integrity, and stop boundaries.

## Operational constraint

Network-dependent operations must use substantially increased timeouts. Claude
and other long-running network providers should receive at least a 20-minute
timeout when supported and should be polled non-blockingly. Slow progress alone
is not failure; actual timeout, disconnect, provider, and fallback outcomes must
remain truthful in active-slice state and terminal receipts.

This constraint changes runtime tolerance, not evidence quality, provider
identity, review independence, or validation requirements.

The user additionally instructed on 2026-08-23:

> can you announce each agent state transition?

The supervising agent must announce each observable builder, reviewer, provider
obligation, and slice lifecycle transition while this campaign is active. This
requires truthful transition visibility; it does not require exposing hidden
reasoning, raw provider transcripts, or noisy internal operations that do not
change state.

## Temporary reviewer amendment

On 2026-08-24, after Claude exhausted its weekly quota, the user instructed:

> We are going to authorize Sol Low as the replacement reviewer until Claude
> weekly reset.

For the duration of that provider outage, the campaign's review role is amended
from Claude independent review to a fresh-process Codex `gpt-5.6-sol` reviewer
at low reasoning effort. Its evidence class is
`accepted_same_model_review`; it must report `independence_claimed: false` and
must not be described as cross-provider, cross-model, or independent-reasoning
evidence. This amendment also authorizes a fresh Sol Low process to discharge
the still-pending targeted placement-falsification obligation without inheriting
the builder's context. Restoring Claude after its weekly reset requires a new
recorded provider amendment; it is not automatic.

## Boundaries not changed by this grant

This authorization does not:

- change proposal packet version 1's required
  `implementation_authorized: false` manifest value;
- settle permanent physical placement, shared claim-lineage ownership, or any
  recorded reopening condition without implementation evidence;
- authorize unrelated repository changes, automatic raw-idea cleanup, or
  mutation of pre-existing user work;
- permit intake, validation, review, or persistence machinery to acquire
  proposal, architecture, portfolio, or user-decision authority;
- weaken configured validation requirements because a network provider is slow
  or temporarily unavailable; the separately recorded temporary reviewer
  amendment changes evidence class and provider truthfully rather than claiming
  independent-review evidence; or
- authorize ordinary user-visible commits, tags, pushes, or publication. Those
  remain separate explicit decisions under their owning workflows.

The user may amend, pause, or revoke this grant. Evidence that invalidates the
accepted proposal boundary returns the campaign to planning or the applicable
authority owner rather than silently broadening the work.
