# Architecture and Placement Review

Role: read-only architecture and placement specialist. The initial pass used a
fresh role-scoped context; the `6460190` reassessment retained that context and
does not claim renewed independence.

Readiness consequence: `revise_before_review_closure`.

## Findings

1. **High — predecessor-linked history is over-prescribed.** Existing route
   openness supports durable reconstruction, replay prevention, crash safety,
   and reference integrity, but does not establish one mandatory history
   mechanism. Preserve those binding consequences while keeping predecessor
   chains, journals, and other valid realizations as candidates.

2. **High — workflow and agent-state ownership overlap.** The workflow is said
   to own semantic state while “agent state” separately owns recovery-critical
   consequences. Define the role-owned workflow projection and keep separately
   owned runtime/control-plane facts as references, or provide an exact
   field-level contract that prevents duplicate or circular ownership.

3. **High — exploratory control-plane placement is treated as settled.** The
   current bounded campaign controller is real, but the broader control-plane
   decomposition remains product direction. The proposal may form and test that
   placement hypothesis; it must not make it non-reopenable architecture.

4. **Medium — topology concerns are under-decomposed.** Parent/child source
   policy, authoritative owner, enforcement point, observation source, and
   query/projection owner are distinct questions. The same separation is needed
   for visibility and lifecycle queries.

5. **Medium — early slices depend on safeguards deferred to slice 4.** The
   supervisor/builder and reviewer recovery proofs already require minimum
   identity, visibility, reference-integrity, and replacement/fencing rules.
   Establish those prerequisites before or inside the first vertical.

6. **Medium — projection modes need a field-level rule.** Distinguish
   role-owned facts, references to stronger owners, and explicitly derived
   values. Reviewer findings and dispositions must not become one undifferenced
   owner.

7. **Low — one snapshot per role is too coarse.** Durable identity must account
   for role instance, workflow/attempt, and execution envelope where more than
   one active or retained execution is possible.

## Architecture correction consequence

The `6460190` correction closes the documentation-level implication that future
UI/control-plane projections are verified invariants and clarifies the present
planning capability contracts. It does not modify the proposal. It strengthens
findings 2–4, narrows finding 6, and closes none of the proposal findings.

The proposal's motivation and a role-owned recovery projection remain
architecturally plausible. The current revision is not ready for review closure.
