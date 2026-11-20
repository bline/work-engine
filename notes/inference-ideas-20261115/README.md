# Inference ideas extracted from scratch notes

Recorded 2026-11-15. This collection organizes the ideas in
[notes/scratch.md](../scratch.md). It preserves a conversation's proposals,
questions, and reported findings; it does not establish implementation status,
accept a design, or authorize provider calls or spending. The original notes
remain unchanged.

The central idea is to make inference a governed Work Engine capability.
The conversation develops three cooperating responsibilities: optimization
chooses suitable inference candidates, resource management determines which
authorized funding routes are available, and a trusted host gateway executes
requests. Cache behavior connects all three because changing an otherwise
eligible route may sacrifice accumulated efficiency.

A separate [comparison with the current Rust implementation](rust-implementation-comparison.md)
records a source assessment at HEAD `01e41b04` on 2026-11-15. It distinguishes
existing mechanisms, integration gaps and unfinished qualification; it does
not change the standing of the extracted ideas below.

## Topic documents

| Document | Ideas extracted |
| --- | --- |
| [Trusted host inference gateway](host-inference-gateway.md) | Credential-free workers; host authentication and dispatch; reuse of native Pi IPC; shared integration boundary |
| [Inference resource management](inference-resource-management.md) | Four funding streams; preferences versus hard failure policy; bounded fallback; admission versus model substitution |
| [Inference optimization and routing](inference-optimization-and-routing.md) | Quality, effort, cost, and cache tradeoffs; ranked candidates; operator, policy, and research modes; proposed deterministic decisions |
| [Cache affinity and effort adaptation](cache-affinity-and-effort.md) | Route affinity; switching costs; session versus prefix identity; changing effort within a model; provider and billing boundaries |
| [Usage and attempt evidence](usage-and-attempt-evidence.md) | Raw and normalized cache telemetry; missing versus zero; transport-specific instrumentation; interrupted attempts and actual-route provenance |
| [Research and compiled inference policy](research-and-compiled-policy.md) | Plan IR; qualified research becoming policy; reproducible routing; regression, review, and hosted-use connections |
| [Qualification experiments and open questions](qualification-experiments.md) | Offline gateway qualification; live-route compatibility; cache controls and switching experiments; decision-relevant measurements |

## Attribution and evolution

The source interleaves operator remarks, assistant responses, and a passage
introduced as coming from the model doing the development vertical. Its export
does not carry formal message-role metadata. Attribution below follows explicit
wording and conversational context rather than reconstructing missing metadata.

The operator identifies the long-term value of the gateway, raises the cache
cost concern, asks whether cache usage is observable, separates funding
availability and failure policy from efficiency and quality, and proposes
investigating cache reuse across boundaries. Component names, detailed gateway
mechanics, the fully deterministic router proposal, and most provider-specific
claims appear in responses. The quoted development-model passage refines the
gateway around existing IPC, separate semantic owners, separate dollars and
quota evidence, and attempt-level failover.

Later passages qualify earlier ones. Choosing the cheapest route becomes a
cache-aware decision. A general router becomes separate resource and
optimization responsibilities. Broad claims about effort changes are narrowed
to particular update mechanisms and supported configurations. These refinements
are retained together rather than flattened into unconditional conclusions.

Provider documentation, prices, model support, and repository observations in
the source were **not independently reverified for this extraction**. They are
carried as reported findings or hypotheses. Illustrative percentages, budgets,
policies, and usage values are examples, not measurements or configured limits.
The source also states that implementation is paused; extraction does not
change that recorded disposition.

## Source coverage

The extraction used the 919-line, untracked source file with SHA-256
`66d110f54c80d047b578fce969c7f4f317a486de8741c38ca3cae15f991e3fc2`.
Line ranges refer to that exact content; the extraction date does not establish
the dates of the underlying conversation or its claimed observations.

| Source lines | Main destination |
| --- | --- |
| 2–42 | Usage and attempt evidence |
| 46–191 | Host inference gateway; resource management; qualification experiments |
| 195–217 | Host inference gateway; usage and attempt evidence |
| 221–301 | Research and compiled policy; host inference gateway |
| 305–430 | Cache affinity and effort; qualification experiments |
| 434–479 | Usage and attempt evidence |
| 484–591 | Inference optimization and routing; research and compiled policy |
| 595–761 | Resource management; inference optimization and routing |
| 765–918 | Cache affinity and effort; qualification experiments |

Repeated questions and export artifacts such as favicon labels and flattened
interactive controls are consolidated into their substantive ideas. No current
code audit or formal proposal evaluation is implied by the source map.

Related earlier direction: [named provider connections and configurable auth
fallback](../provider-auth-connections-and-fallback-direction-20261106.md).
That note has its own attribution and scope; this collection does not reconcile
or supersede it.
