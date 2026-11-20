# Inference optimization and routing

Status: idea extraction, 2026-11-15. Source: [scratch notes](../scratch.md),
lines 488–597 and 678–761, with the cache refinement at 874–918. The fully
deterministic router is a response's proposal, not an explicit operator
decision. See the [collection index](README.md) for attribution and provenance.

## Idea

Treat efficiency and quality as a separate concern from funding availability.
An inference optimization manager would consider task suitability, qualified
models, reasoning effort, expected quality, cost, latency, reliability, context
strategy, and cache affinity. It could return multiple ranked eligible
candidates for resource admission rather than selecting a single route that
may be unfunded.

Examples in the notes include a less expensive configuration for routine
development, a stronger configuration for a consequential review, and effort
changes within a continuing model session. These are optimization possibilities,
not established performance findings. The optimizer cannot grant spending
authority or relax a required review capability or independence condition.

## Proposed routing modes

| Mode | Behavior proposed in the source |
| --- | --- |
| Operator-controlled | Preserve the operator's selected model, provider, and billing route |
| Policy-driven | Choose among authorized routes using defined constraints and priorities |
| Research-informed | Apply qualified findings about cost, cache behavior, quality, and reliability |

The source proposes implementing all three as deterministic decisions over
pinned inputs. Those inputs include execution authorization, preferences,
policy revision, provider capabilities, budgets, quota observations, cache
affinity, prior attempt state, and qualified research.

Adaptive behavior can still be reproducible if its input observations and
policy are recorded. Randomized experiments could record seeds and assignments.
External outcomes remain uncertain: a provider may reject a request, a quota
observation may become stale, or a cache may miss.

For example, selecting a subscription route under one observation snapshot and
then observing quota exhaustion yields a new decision over changed inputs. It
does not retroactively make the original observation current or erase the first
attempt's accounting.

## Boundary left open

The conversation favors deterministic routing strongly, then distinguishes
mechanical resource management from improving quality and efficiency. The
exact boundary between a pinned policy and contextual optimization judgment
remains a design question. Preserving the proposal does not settle that
boundary or override the repository's [determinism and model judgment
contract](../../docs/contracts/system-design.md#9-determinism-and-model-judgment).

The proposed systems can evolve independently: quota handling can improve
without redesigning the research system, and newly qualified optimization
policies can be evaluated without replacing credential custody.

Related: [resource management](inference-resource-management.md),
[cache affinity](cache-affinity-and-effort.md), and
[research and compiled policy](research-and-compiled-policy.md).
