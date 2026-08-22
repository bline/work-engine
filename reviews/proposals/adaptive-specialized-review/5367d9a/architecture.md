# Architecture and Placement Review

Role: read-only architectural reviewer. Consequence: diagnostic and advisory;
no acceptance, architecture mutation, scope reopening, or implementation
authority.

Subject: the exact revision and content set in [subject.md](subject.md).

## Supported conclusions

- One review-artifact contract is a defensible semantic candidate. Revision
  identity, provenance, applicability, lineage, limitations, and
  non-authoritative consequences form a coherent durable contract without
  absorbing proposal lifecycle or authority.
- Artifact durability and adaptive coordination are independently decidable.
  Persistence cannot supply the judgment needed for panel selection or
  synthesis.
- The dependency direction is sound as an output-semantic dependency. Ephemeral
  coordination is possible, but the proposed durable consequence depends on
  revision-bound artifact semantics. This does not impose construction or
  acceptance order.
- Coordination should remain separate from artifact representation. Specialist
  roles own diagnoses, coordination owns selection and synthesis judgment, the
  artifact contract owns representation, and named authorities own decisions.
- Selection and synthesis plausibly need different inputs and lifetimes, but
  current evidence does not justify separate product owners.
- Shared implementation-review reuse is credible for the artifact core but
  premature for coordination behavior, authority projections, or readiness
  synthesis.

## Findings

1. **Medium, observed — final placement lacks consumer evidence.** No proposal
   revision cycle or implementation-review consumer has supplied discriminating
   lifecycle requirements. Proposal-local and shared cross-phase ownership must
   remain unsettled.
2. **Low, inferred — semantic ownership and initial adapter placement need a
   sharper distinction.** A proposal-local Git adapter adjacent to packets could
   be the first physical surface without deciding the permanent semantic owner.
3. **Low, observed — synthesis has two ownership senses.** The artifact can
   represent a synthesis consequence while coordination owns the judgment that
   produces it. Implementation must not let persistence acquire judgment.

## Conflicts and uncertainty

- `causal: true` could be mistaken for mandatory delivery order; the family
  prose correctly gives it the narrower semantic meaning.
- Prospective implementation reuse must not become architectural evidence merely
  because it was durably recorded.
- The review did not resolve which fields must be mechanically closed, whether
  a revision cycle supports the proposed lineage vocabulary, whether
  implementation review shares more than the artifact core, or whether
  concurrency eventually requires runtime state.

## Limitations and applicability

No schema, validator, proposal revision cycle, implementation-review consumer,
or concurrency behavior was available. The external challenge mainly sharpened
the semantic-owner versus adapter-placement distinction; it did not establish
that selection and synthesis need separate owners. Re-review if the bound
content changes or named consumer evidence arrives.

Readiness consequence: `material_uncertainty_unresolved`.
