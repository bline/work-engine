# Bounded Implementation Authorization

## Authority and subject

This artifact records the user's separate execution grant for proposal
`work-engine.agent-instruction-structure-placement-review`. The approved
proposal decision is bound to public commit
`3ba4e72fa54ebc1958b39141fc858e0324380a5e`.

After the coordinator stated that the approved proposal would be implemented
through one bounded `slice-supervisor` slice, the user instructed:

> Please proceed sir, you have the helm.

The user is the authority owner for this grant.

## Authorized consequence

The grant authorizes one bounded implementation slice that may:

- create the proposed agent-instruction structure and placement review skill;
- add the minimum repository-local metadata, validation, and integration needed
  to make that capability usable and observable;
- exercise the capability on real agent-facing instruction surfaces identified
  by the accepted implementation plan; and
- repair valid in-scope findings established by an independently defined review
  capability.

The slice remains bound by the approved proposal, its review synthesis, its
reopening conditions, repository doctrine, and an evidence-based plan accepted
through the slice-supervisor workflow.

## Boundaries not changed by this grant

This authorization does not:

- change proposal packet version 1's required
  `implementation_authorized: false` value;
- settle permanent architecture or the proposal-versus-implementation
  lifecycle-sharing question preserved for dogfood;
- make the new reviewer a universal gate or give it proposal, architecture,
  doctrine, seam, panel-coordination, acceptance, or implementation authority;
- allow the candidate skill to certify its own implementation;
- authorize unrelated repository changes or mutation of the review-bench work
  already present in the worktree; or
- pre-authorize an ordinary user-visible completion commit, which remains a
  separate per-slice decision after acceptance.

The user may reopen or supersede this grant. Evidence that invalidates the
accepted boundary returns the slice to planning or to the relevant authority
owner rather than silently broadening the work.
