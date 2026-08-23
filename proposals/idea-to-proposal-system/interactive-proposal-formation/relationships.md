# Relationships: Interactive Proposal Formation

## Depends on: durable proposal packets

Type: semantic dependency, not mandatory delivery order.

The capability needs a defined target contract to persist authoritative state.
The contract and capability may be implemented together. When formation hands a
candidate to semantic review, formation owns invoking publication of the
complete packet at an immutable Git revision; packet machinery owns artifact
shape and mechanical validation, not semantic readiness or review authority.

## Hands off to: evidence-backed proposal evaluation

Type: consumer relationship.

Formation makes a candidate intelligible and identifies evidence needs.
Evaluation establishes support, estimates, and confidence. Formation may gather
targeted evidence when it resolves meaning or placement, but it does not absorb
the full evaluation role.

## Consumed by: strategic planner

Type: consumer relationship.

The planner uses formed/evaluated packet projections to reason about priority,
dependencies, and roadmap changes. It may request reopening but does not own the
formation transcript.

## Supplies objectives to: campaign supervisor

Type: authority-gated downstream relationship.

Only an accepted or otherwise authorized proposal can become a campaign
objective. Formation itself does not perform that transition.
