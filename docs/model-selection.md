# Model selection for engineering work

Current guidance: 2026-10-20. Available choices for this recommendation:
`gpt-6-astra` and `gpt-6-sol`.

This is repository-wide decision guidance, not product doctrine or an automatic
router. Explicit user choices and owning campaign/provider contracts take
precedence. This document does not change existing configuration, authorize
agent launches, or relax review, budget, provider-use, or acceptance boundaries.

## Choose for the uncertainty, not just the job title

Prefer the model and reasoning effort with the lowest expected total cost of
reaching the required outcome: reasoning, latency, rework, review, operator
attention, and maintenance all count. Cheap tokens are not necessarily cheap
accepted work; a larger model is not automatically useful for every step.

The distinction follows [DESIGN.md](../DESIGN.md): contracts constrain the
result, while model selection remains judgment wherever the owning contract
leaves it open. These defaults can be revised when task evidence supports a
better authorized choice.

## Current defaults

On 2026-10-20, the user selected `gpt-6-sol` with `medium` reasoning for
builders, replacing the previous high-effort default. Apply that selection to
new builder assignments and move retained builders at a completed handoff.
Reviewer and architecture-role selections remain separate; this choice does
not change an accepted review contract or a live trial's provider configuration.

| Work | Model | Starting effort | Why this allocation |
| --- | --- | --- | --- |
| Consequential architecture, ownership contracts, and implementation planning with unresolved design questions | `gpt-6-astra` | `xhigh` | Spend reasoning on decisions whose mistakes propagate across the system. |
| Implementation, integration, tests, and ordinary remediation under a sufficiently clear contract | `gpt-6-sol` | `medium` | User-selected builder setting. In a configured Slice Builder campaign, keep one capable builder responsible for the complete slice and its compiler/test loop. In manual orchestration, remediation for a builder over the context trigger goes to a fresh builder with a packet ([skill](../skills/manual-orchestration/references/context-boundary.md)). |
| Bounded repository reconnaissance, routine documentation, or straightforward edits | `gpt-6-sol` | `medium` or `high` | Match effort to the remaining uncertainty and consequence. |
| Escalated concurrency, recovery, authority, or cross-component diagnosis | `gpt-6-astra` | `xhigh` | Challenge an unresolved causal model or contract, rather than merely repeat a failing run. |
| Consequential adversarial review, where an OpenAI reviewer is permitted | Separate `gpt-6-astra` reviewer | `xhigh` | Challenge the composed result from a separately established review context. |

These are project recommendations, not measured performance guarantees or
mandatory phases. A small change need not visit every role. Sol can plan a
well-bounded implementation; Astra can implement when the hard part remains
inseparable from architectural reasoning. Reasoning effort is also a starting
point, not a claim that maximum effort always improves the result.

The [October effort measurements](../notes/post-s10-direction-20261019.md#2c-probe-pre-registered-sol-medium-comprehension-probe-2026-10-19)
found similar weighted tokens per call across Sol effort levels, with cached
context accounting for most of that cost. They do not establish equal builder
quality or lower total cost at medium: calls to acceptance, correction rounds,
and escaped defects still determine the task-level result. Use those outcomes
to evaluate this selection rather than treating reduced effort as proven savings.

OpenAI describes [Astra](https://developers.openai.com/api/docs/models/gpt-6-astra)
as its most capable model for demanding work and
[Sol](https://developers.openai.com/api/docs/models/gpt-6-sol) as built for complex
coding and agentic workflows. Official OpenAI documentation supports that broad
positioning; it does not establish this repository's role allocation. We have
not benchmarked these two models on matched Work Engine tasks. Availability,
pricing, and supported settings should be checked when they affect a decision;
API prices do not establish subscription quota consumption.

## Preserve implementation and review ownership

The builder owns implementation through integration, validation, and ordinary
remediation within its authority. Sol is not merely a code generator whose
unfinished work automatically transfers to Astra. Retaining a useful builder
context avoids repeatedly reconstructing the problem.

Escalation is useful when evidence exposes an unresolved ownership boundary,
contradictory assumptions, or a causal explanation the current investigation
cannot establish. A compiler error, capacity error, or fixed number of failed
tests is not by itself a reason to change models. A model change does not repair
a harness, test, or contract defect, and the historical failure count alone does
not establish that the model caused those failures.

Review isolation follows the owning review claim. A fresh context without an
inherited builder/designer conversation is the default for an initial adversarial
challenge, not proof of independence by itself. Retain a useful reviewer through
bounded remediation unless the review premise or required isolation changes.
The Astra designer does not become an independent reviewer of its own design by
being renamed. Astra and Sol are both OpenAI models: this pairing does not meet
a cross-provider requirement. Any change to that requirement belongs to its
authority owner, not this guidance.

## Make selection and execution distinguishable

Inherited defaults can silently select a different model from the one intended.
For a delegated task, explicitly select the model and effort using the launch
mechanism available, and preserve requested versus observed configuration in
the existing task record or receipt. Useful evidence includes the task/role,
requested model and effort, effective identity reported by the runtime,
session/run reference, and any actual substitution. An unavailable effective
identity is unknown, not verified merely because a configuration file names it.

For contract-bound selections, an observed mismatch leaves the requirement
unmet; an unobserved identity leaves it unestablished when identity evidence is
required. Capacity failure does not authorize a silent
fallback. Where retry is authorized and safe, wait/retry the same selected model
within the applicable deadline and budget; otherwise report the limitation.
Retrying a launch must not replay already-entered work. If a different model is
permitted, preserve the reason and actual change; if selection is fixed, obtain
the owning authority before changing it. No automatic upgrade to a newer model
is implied by this document.

## Learn from outcomes without creating a new gate

Use existing receipts to compare accepted outcomes, escaped defects, review
repairs, total time, token/cache costs where observed, and operator intervention.
Distinguish model performance from differences in task, context, tools, harness,
and validation. Missing measurements remain unavailable. Revise the defaults
when evidence changes their value; collecting a new benchmark is not a
prerequisite for ordinary authorized work.

For the Rust lifecycle effort, the proposed application is Astra for the bounded
M1 plan, Sol for the complete implementation after plan acceptance, and a
separate Astra review where the review contract permits it. That is an example
of this guidance, not authorization to start M1 or a dependency of the lifecycle
runtime on either model.
