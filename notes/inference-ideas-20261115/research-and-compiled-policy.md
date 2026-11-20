# Research and compiled inference policy

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 221–301, 566–597, 678–725, and 747–761. These are proposed architectural
connections, not measured benefits or accepted integration contracts. See the
[collection index](README.md).

## Idea

Connect research, Plan IR, organizational compilation, inference policy, and
execution evidence. A compiled execution contract could establish eligible
models, required capabilities, permitted billing routes, spending ceilings,
operator constraints, and applicable qualified policies. Routing would operate
within those admitted options rather than decide which work is authorized.

Research could establish that a configuration performs adequately for a work
class, that effort adaptation reduces cost, that a failure-specific fallback
improves completion, or that a model benefits from a different context strategy.
The source's example of 60% lower API expenditure is hypothetical, not a result.

Qualified findings could inform a revisioned policy. The host's observations
would then support checking whether subsequent eligible executions still show
the expected behavior. Previous policy revisions would remain available for
comparison or rollback if later evidence contradicts a benefit.

## Connections identified in the notes

| Area | Proposed benefit or constraint |
| --- | --- |
| Research experiments | Compare routing, model, effort, cost, latency, and reliability under recorded configurations |
| Regression testing | Re-run jobs with pinned inference policies while treating provider outcomes as observations |
| Independent review | Establish the actual provider and model; prevent hidden substitution from invalidating a required independence claim |
| Accounting and recovery | Collect observations at dispatch and preserve interrupted attempts across routing changes |
| Multi-user hosting | Separate credential ownership and spending authority from worker execution |
| Distributed experiments | Carry qualified findings and recorded assignments into eligible execution environments |

The shared gateway is proposed as an integration point for these capabilities,
not as their universal semantic owner. Credential custody, authorization,
accounting, settlement, research qualification, and workflow decisions retain
their distinct responsibilities.

## Reproducibility and qualification

The deterministic-routing proposal would record policy revision, input
observations and timestamps, relevant pricing, authorization, unknowns, and any
experimental seed or assignment. This could reproduce why a decision was made;
it cannot promise identical provider responses, future quota, or cache hits.

The notes leave research qualification and policy applicability as important
boundaries. A finding about one workload or configuration is not automatically
a general model-selection rule. A policy cannot authorize spending, substitute
a model outside the execution contract, or silently weaken required evidence.

Related: [optimization and routing](inference-optimization-and-routing.md),
[host gateway](host-inference-gateway.md), and
[usage and attempt evidence](usage-and-attempt-evidence.md).
