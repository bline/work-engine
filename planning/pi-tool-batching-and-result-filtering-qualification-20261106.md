# Pi tool batching and result filtering: qualification workstream

Recorded 2026-11-06 UTC against `780fcc61eb740a1e936493108e26d45e38b6cdc5`.
Status: research-informed test proposal; source inspection only. No performance
result, runtime activation, implementation budget or additional live job is
established by this document. The preceding $45 pilot remains closed.

## Question and current evidence

Can Pi batching and selective result projection reduce total resources per
independently accepted task without losing evidence or widening authority?
This extends the [Pi risk register](../docs/pi-risks.md) and the
[failure qualification plan](pi-unattended-development-qualification-tests-20261105.md).

The maintainer supplied the research. This pass checked the tagged upstream
[agent loop](https://github.com/earendil-works/pi/blob/v1.5.0/packages/agent/src/agent-loop.ts),
[Codemode documentation](https://github.com/earendil-works/pi/blob/v1.5.0/packages/coding-agent/docs/codemode.md)
and installed package source:

- Native Pi accepts multiple calls in one response. Its installed `Agent`
  defaults to parallel execution; a sequential tool makes the batch sequential.
  The parallel executor prepares calls in order, dispatches via `Promise.all`
  and returns results in call order. That dispatch path has no general
  concurrency limiter. This is bounded source inspection, not an assertion
  that every execution surface is unbounded.
- Optional Codemode runs JavaScript in QuickJS, calls tools and projects script
  output back to the model. Its script deadline is unset by default. A failed
  script does not undo completed calls. It also exposes non-LLM model calls;
  those require separate inference admission. Worker limits are distinct from
  limits on nested tool calls.

The installed `pi-agent-core`, `pi-coding-agent`, `pi-codemode` and `pi-ai`
packages all report **1.5.0**. Installation alone establishes no active tool.
Our [receiver](../rust/crates/work-engine-manual-host/receiver/pi-coding.mjs)
lines 116–143 explicitly marks `workspace_exec` sequential, selects custom
tools and disables builtin tools. Optional claims tools are also sequential
and restricted to the provider-disabled fixture profile. Codemode is absent
from that selected set. Changing stock CLI settings alone does not qualify
this SDK route. The last live pilot admitted only one workspace effect and
never reached inference; it supplies no batching measurement.

Batching, concurrency and filtering are different variables. Multiple calls in
one response can reduce model round trips even when executed sequentially.
Parallel dispatch changes tool latency; it does not inherently reduce model
requests. Filtering changes returned context and may increase later retrieval.
The model's actual use of each capability must be measured.

## Smallest useful comparison

Begin with deterministic SDK/Host cases using recorded assistant calls and
fixed tool outputs. Compare the same calls emitted one per response and as one
batch to test round-trip accounting. Replay an identical batch sequentially
and concurrently to test dispatch, result order and limits. Label scripted
inference as substituted; it cannot establish model efficiency or quality.

For a later funded live comparison, hold the model/effort, provider/auth/payer,
repository snapshot, task, resources, acceptance and total allowance constant.
Pin the actual tool schemas, prompt, SDK and Host profile. Use clean sessions;
record unavoidable prompt/schema differences rather than calling them equal.

| Cell | Tool dispatch | Model-visible result | Comparison |
| --- | --- | --- | --- |
| A: Native calls | Sequential; batching allowed | Full bounded tool results | Baseline with observed calls per response |
| B: Native calls | Host-bounded parallel reads | Same result policy as A | Concurrency, with batching rate reported |
| C: Codemode | Same bounded read backend | Full bounded results through script | Script/schema overhead relative to B |
| D: Codemode filtering | Same backend and limits as C | Selected findings with retrievable source references | Filtering relative to C |

Do not describe A as Pi's stock default or presume B uses fewer requests.
An optional one-call-per-response control isolates batching but changes the
available interaction surface; label that constraint. Freeze the selected
cells, tasks, repeats and useful effect/quality margins before collecting live
results. A small pilot reports uncertainty rather than general harness rankings.

Start with read-heavy Rust tasks: inspect several modules for a verifiable
answer, find a defect-relevant definition and its callers, and reconcile a
large search result containing both decisive evidence and distractors. Include
a case where further action requires interpreting new evidence. Check answers
and exact source references independently, including deliberately planted
counterevidence. A model-written summary does not establish completeness.

The initial efficiency trial is read-only. Write qualification is a later
extension: one bounded Rust change with independent tests/review, conflicting
write cases and interrupted-effect recovery. The model's report and a completed
Codemode script cannot establish task acceptance.

## Host acceptance and fault cases

The execution owner must bind an admissible tool surface before experiments.
Whether a bounded read tool belongs in the current receiver or an existing
adapter is an implementation placement decision, not settled here. Reuse the
current admission, custody and effect owners; avoid a second workflow ledger.

| Case | Required observable outcome |
| --- | --- |
| Read fan-out beyond cap | Host bounds active calls and queued work; excess requests follow the declared queue/refusal policy; cumulative call/output/time limits still apply |
| Long preparation or queued call | Authority, deadline and reservation are checked at actual dispatch; expired work cannot execute using an earlier preflight decision |
| Out-of-order completion and one denial | Stable call/result attribution; allowed and denied calls retain their own outcomes; a batch is not assumed atomic |
| Nested Codemode call or hidden tool discovery | Every nested call crosses its owning Host checks; discovery exposes only admitted capabilities; no alternate tool, credential or inference route appears |
| Script timeout, abort or failure after a successful call | Bounded host termination and collection; completed, cancelled, pending and unknown effects remain distinct; restart does not replay completed/uncertain effects |
| CPU loop, unresolved promise or abandoned tool promise | Absolute host bounds and cleanup remain effective independently of script cooperation or output activity |
| Filter drops an error or counterexample | Independent acceptance rejects the unsupported answer; raw outcome remains available outside the projection |
| Large or truncated output | Explicit omissions, full-result digest/reference and bounded retrieval; no completeness assertion from a prefix or selected summary |
| Conflicting/dependent writes in the later extension | The owner serializes, fences or refuses conflicts; all prior effects remain attributable after a script failure |

A script-side semaphore or optional `timeout_ms` is not a Host enforcement
boundary. Tune concurrency inside the admitted resource envelope; freezing one
test cap does not make that number universal doctrine. Codemode's model helpers
stay unavailable unless their exact route, payer and reservation are separately
admitted. Test that refusal before enabling the surface.

Store raw tool outcomes and evidence with their existing owners. Bind each
projection to the source digest and query/range, record filtering/truncation and
keep a permitted retrieval route. Preserve errors and negative evidence needed
for acceptance. The projection is a consumer view, not a new canonical fact.

## Measurement, advancement and ownership

Count provider requests, tool calls per assistant response, nested calls,
peak active calls, queue time, tool time, wall time through acceptance, raw and
returned result bytes, actual input/output/cache tokens, script/schema overhead,
omissions, retrievals, failures, repeated operations, review and repair. Include
all attempted tasks, rejected answers and aborted runs. Report resources per
accepted task; separately report resources per accepted change for write work.
Zero acceptance makes that ratio undefined.

Keep API-equivalent estimates, actual billing and attributable subscription
quota separate. A shared account percentage is not an isolated task's quota;
unavailable attribution stays unavailable. Fewer bytes returned does not prove
fewer total tokens or lower total accepted-work cost.

Advance only when Host-bound safety cases pass and the selected live cells
preserve predeclared independent quality margins. Adopt a mode only for the
workloads where evidence supports its accepted-work benefit. A rejected or
inconclusive optimization leaves the previous qualified surface available.

The runtime/tool owner owns bounded execution and nested-call enforcement; the
acceptance owner owns verifier/review decisions; the integration/accounting
owner owns matched setup and measurements. This session can prepare that
qualification work. S10's separately approved development-loop composition
remains with its owner. This workstream does not amend that scope or require
Codemode for the next live correctness proof. Execution, implementation and
live comparisons need a concrete later budget; no new actor is launched here.

## Inspection provenance and limits

Repository intent: targeted find/trace, Tier 2; graph project
`home-bline-code-work-engine`, generation `2026-11-06T05:38:17Z`.
Broad candidate search was not exhaustive. Exact receiver `bindBridge` source
and its one-hop factory caller were inspected, together with receiver lines
1–143 and 230–310. Exact tool-symbol search in `pi_coding.rs` exhausted its 27
matches; `Host::coding_prepare_tool` source was read. Coverage reported these
two source files content-verified with unresolved call edges; material ranges
were read directly, and no complete call-graph or runtime claim follows. The
risk register, layout and prior failure plan had no recorded coverage issue;
that is best-effort metadata, not proof of completeness.

Installed SDK source is outside this repository graph and was read directly
under `/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi/node_modules/@earendil-works/`:

| Inspected asset | SHA-256 |
| --- | --- |
| `pi-agent-core/dist/agent.js` | `f45ae001e2b14fced8ae151f0a00303b5fbdb5fc4b25418d45de6a5546f48a8c` |
| `pi-agent-core/dist/agent-loop.js` | `22ee81f9977e211044c1ad41a763876c793ca7fc1c5b39b9fa7f76f3cb281129` |
| `pi-coding-agent/docs/codemode.md` | `3a96ee10da36395bbd22beb60ac029f8ca8a859f2fd3c3104cfc38f9d5a1f95a` |
| Repository receiver `pi-coding.mjs` | `afba55bdd9393acf517fab6a47da7735545c146856c0043d1ca6dfcf8d1d04ed` |

This inspected installed behavior and tagged upstream text separately; it does
not certify package reproducibility or byte equality to the Git tag. No
credentials were read, SDK experiment run or provider request made by this pass.
