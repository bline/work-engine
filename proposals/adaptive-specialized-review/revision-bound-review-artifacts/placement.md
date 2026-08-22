# Placement: Revision-Bound Review Artifacts

## Current candidate

A Git-backed review-artifact profile adjacent to the proposal-packet capability
is the plausible first adapter. It would preserve review episodes, outcomes,
findings, conflicts, synthesis references, and applicability as proposal-state
references without expanding `packet.json` into a review record or moving
review meaning into runtime state.

The profile would consume shared statement, baseline, provenance, sensitivity,
authority-scope, and lineage semantics from the separately formed
claim-centered-evidence-lineage candidate. Shared semantic ownership and the
location of the first proposal-local adapter remain different decisions.

Placement remains uncertain. Implementation review may reuse the
review-specific profile, but no implementation consumer has established whether
its remediation, acceptance, and context-lifetime requirements are genuinely
shared.

## Plausible owners

### Proposal-review planning layer

This is the narrowest first consumer and a plausible location for the initial
Git adapter. It risks making later implementation-review reuse an afterthought
if episode, outcome, finding, and synthesis semantics are actually shared.

### Shared cross-phase review capability

This could preserve one review profile across proposal and implementation
subjects while consuming the phase-neutral evidence-lineage foundation. It is
premature until an implementation-review consumer supplies discriminating
requirements rather than superficial similarity.

### Shared evidence-lineage capability

Rejected as the assumed owner of the whole review profile. It may own common
statement semantics, but reviewer episodes, omission/failure outcomes,
conflicts, and synthesis are review-domain meaning.

### Proposal packet itself

Packet references may expose current review state, but absorbing review history
would mix proposal lifecycle ownership with reviewer provenance, findings, and
applicability.

### Persistent runtime state

Rejected as an assumed primary owner. Runtime state may preserve an active
review obligation when recovery requires it, but does not own durable review
meaning or history.

## Reopening conditions

Reconsider placement when a proposal revision exercises the profile, an
implementation-review consumer supplies discriminating lifecycle requirements,
selection and synthesis need different owners, or observed concurrency requires
semantics unavailable from Git-backed state.

