# Placement: Adaptive Review-Panel Coordination

## Current candidate

A proposal-review coordination capability in Work Engine's planning layer is
the narrowest immediate placement. It would consume formed packet revisions and
research evidence, select relevant review perspectives, and produce
revision-bound, finding-linked, non-authoritative synthesis for the proposal
decision owner.

Placement remains uncertain. Implementation review may reuse coordination
behavior, and evidence has not established whether selection and synthesis need
one owner, separate information lifetimes, or different durable-state owners.

## Plausible owners

### One proposal-review coordinator

This provides a cohesive first vertical for selecting specialists, collecting
truthful outcomes, and synthesizing decision support. It remains provisional
until dogfooding shows that synthesis can share the selection context without
losing required independence or useful lifecycle separation.

### Separate selection and synthesis capabilities

This may preserve different context and state lifetimes and reduce
contamination, but no observed use yet establishes two independently decidable
product changes. It remains a reopening condition rather than another proposal.

### Shared proposal-and-implementation review capability

This may avoid duplicated coordination while using one review-artifact profile.
It is premature until an implementation consumer supplies concrete shared
requirements and ownership consequences.

### Review-artifact or evidence-lineage capability

Rejected as implicit coordinator owners. Those candidates may represent
findings and shared evidence semantics, but persistence does not select
specialists, synthesize judgment, or decide applicability and readiness.

### Strategic planner or campaign supervisor

Rejected as implicit owners. They may consume review consequences or request
reconsideration within their authority, but neither acquires proposal-review
selection, synthesis, acceptance, or scope-reopening authority by consuming the
result.

## Reopening conditions

Reconsider placement when dogfooding distinguishes selection from synthesis
lifetimes, an implementation-review consumer establishes a shared boundary, the
open registry requires a separately governed capability owner, or durable
recovery requires state beyond revision-bound artifacts.

