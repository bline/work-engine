# Relationships: Adaptive Review-Panel Coordination

Only relationships to another currently formed proposal have a resolvable
stable proposal ID and therefore appear as typed relationships in `packet.json`.
The consumer and prospective relationships below preserve semantic context but
do not invent packet targets for roles or capabilities that are not proposals.

## Depends on: revision-bound review artifacts

Type: causal semantic dependency.

Coordination needs durable subject identity, provenance, applicability, and
supersession semantics so findings and synthesis remain truthful after revision.
This dependency does not impose a mandatory delivery order.

## Consumed by: proposal decision authority

Type: non-authoritative consumer relationship.

The decision owner receives supported findings, conflicts, limitations,
unresolved consequences, and a readiness projection. Review does not accept,
reject, defer, or prioritize the proposal.

## Advises: proposal formation and evaluation

Type: information relationship.

Review findings may justify revising a candidate or may supply evidence useful
to later evaluation. The coordinator does not own either transition.

## Potentially reused by: implementation review

Type: prospective consumer relationship; non-causal for initial formation.

Implementation review may later use the same selection and synthesis substrate
with a narrower subject projection. This possibility preserves an architectural
question; it does not authorize shared infrastructure now.
