# Relationships: Durable Proposal Packets

## Enables: interactive proposal formation

Type: semantic dependency.

The formation capability needs an intelligible durable target whose ownership
and required consequences are defined. This does not require separate proposal
acceptance or deployment order; the two changes may be co-designed and shipped
together.

## Enables: evidence-backed proposal evaluation

Type: semantic dependency.

Comparable evaluation needs stable proposal identity, current scope, placement,
uncertainty, and evidence ownership. Evaluation methodology is not part of this
proposal.

## Informs: strategic planner and proposal-backed roadmap

Type: consumer relationship; non-causal for initial packet implementation.

The strategic planner can use packet decisions and relationships. A roadmap can
later become a thin selected view over proposal IDs. Neither consumer must be
changed merely to prove the initial packet contract.

## Referenced by: persistent runtime state

Type: identifier interoperability; non-causal.

Runtime events may carry a `proposal_ref`. The packet contract should define the
stable proposal ID, while runtime state owns the event field and its execution
lifecycle. A shared string/reference convention is sufficient initially.

## Related to: durable review queue

Type: adjacent infrastructure; non-causal.

Both require truthful identity and provenance, but proposal artifacts do not
need review-queue transactions or mutation reservations until observed
multi-writer behavior establishes that consequence.
