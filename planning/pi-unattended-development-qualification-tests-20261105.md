# Pi unattended development: proposed failure qualification

Recorded 2026-11-05 from the maintainer's research followup. Status: test plan
candidate. Writing this plan fits the current documentation allowance; running
these experiments or implementing new runtime behavior needs a later scoped
assignment. It does not expand the current one-trial grant.

The question is whether Work Engine can supervise a useful development task
through failure, preserve its evidence and obtain independently verified work.
Benchmark rankings alone cannot answer that question. The deterministic host
owns observation and bounded effects; the model chooses recovery inside its
authority; the acceptance owner judges the result using independent evidence.

## Evidence standing

The [earlier research note](../notes/pi-harness-benchmark-evidence-20261105.md)
records a bounded primary-source check. Keep that dated record unchanged.
The maintainer subsequently supplied these additional verification targets:

- [Personal Agent Benchmark](https://github.com/treygoff24/personal-agent-benchmark):
  compare the published scoreable denominators with all 216 recorded attempts
  per harness. Using the supplied counts, 158/216 is 73.1% for Claude Code and
  Codex, and 154/216 is 71.3% for Pi. That arithmetic does not independently
  verify the counts or resolve which exclusions suit a particular comparison.
- [Finding the Right Fit](https://arxiv.org/abs/2610.00917): check the supplied
  aggregate Astra cost rows against the paper, including the Terminal-Bench 4
  counterexample to a universal Pi advantage. Aggregate API-equivalent cost
  does not measure subscription consumption.
- Pi **1.5.0** [bash tool](https://github.com/earendil-works/pi/blob/v1.5.0/packages/coding-agent/src/core/tools/bash.ts)
  and [agent loop](https://github.com/earendil-works/pi/blob/v1.5.0/packages/agent/src/agent-loop.ts):
  verify the supplied distinction between optional command timeout, truncated
  tool-call handling and truncated text-only termination. Inspect print mode
  separately. Trace the actual Work Engine SDK/tool route: behavior in Pi's
  stock bash tool need not describe our custom execution backend.
- [Orbi's article source](https://github.com/orbi-build/orbi-website/blob/beta/content/blog/pi-coding-agent-vs-claude-code.md):
  retain the tiny-task/default-versus-bare caveat already recorded in the note.

These new paper and Pi implementation claims are attributed research input,
not independently verified repository findings in this planning pass. They
motivate tests without making the test contract depend on their accuracy.

## Proposed acceptance cases

Pin the SDK, provider adapter, Rust owner, profile, validator, fixtures and
effect policy for each case. Begin with deterministic local fault injection;
use a real provider only for behavior that the local substitute cannot prove.
Store the exact input, event sequence, terminal status, artifacts, raw usage,
cleanup observations and acceptance decision. An agent's success message is
never the acceptance oracle.

| Case | Injection | Observable result to establish | Owner |
| --- | --- | --- | --- |
| Command deadline | Omit the model's timeout; run a hanging command with a child that ignores termination | Host-enforced deadline, collected process tree/container, explicit timeout result, retained partial output and no replacement execution | Runtime/tool host |
| Stalled execution | Stall a tool, provider stream or lifecycle handoff without further events | An observer outside the model detects the declared liveness failure, records which signal stopped, and reaches a bounded terminal or recoverable state | Runtime supervisor |
| Quiet but healthy work | Run a valid silent command within its declared bound | Silence alone does not become evidence of semantic failure; distinguish heartbeat, output activity, useful progress and absolute deadline | Runtime supervisor |
| Truncated tool call | Supply output-limit termination with incomplete tool arguments | No partial or malformed effect executes; the failure is visible and any continuation consumes explicitly admitted requests | SDK adapter plus effect owner |
| Truncated text | Supply output-limit termination without tool calls, including print and SDK routes | Incomplete output cannot become successful completion; bounded recovery or explicit failure preserves the stop reason | SDK adapter plus job owner |
| Interrupted agent | Terminate during an effect, after tool completion, and before receipt delivery | Preserve artifact digests, operation identity and known/unknown effect state; recovery does not duplicate an effect or resolve pending human authority | Custody/continuity owner |
| False success | Agent claims completion while the original validator fails; also attempt to change or bypass the validator | Immutable acceptance inputs and independently observed verifier result control acceptance; failure remains failure | Acceptance owner |
| Receipt delivery | Deliver/replay the same receipt and restart its consumer | Delivery acknowledgement preserves the same receipt and operation; acknowledgement alone cannot turn `owner_pending` into accepted | Job/receipt owner |
| Workflow cost | Include a failed attempt followed by separately admitted repair and review | Count all builder, reviewer, recovery and coordination inference once per independently accepted change; report failures as well as successes | Accounting/integration owner |

For liveness, choose and publish the signals and time bounds before execution.
There is no universal deterministic proof of useful model progress. A watchdog
must not silently turn an observation heuristic into task acceptance or grant
automatic retries. Specify the recoverable timeout signal that the model will
receive, and test that recovery itself stays within the remaining allowance.

For cost, retain raw calls, input/output/cache tokens, elapsed time, configured
price basis, reservations and unknown tails. Report API-equivalent dollars,
speed-adjusted objective cost and attributable subscription quota separately.
When no change is independently accepted, cost per accepted change is
undefined; publish total cost and the zero acceptance count. Unknown quota is
unavailable, not zero.

## Order and ownership of later work

1. **Runtime owner:** map each case to the actual pinned SDK/custom-tool path
   and existing timeout, custody and continuation capabilities. State what is
   already covered and where source-bound tests are missing. Prefer composing
   current owners over adding a parallel execution ledger.
2. **Job/acceptance and integration owners:** bind the verifier and receipt
   semantics, then run deterministic failure cases through the actual caller
   path. Check externally visible failures and cleanup, not only internal
   helper return values.
3. **Integration owner:** propose a separate budget for the smallest useful
   live task and required failure/recovery cases. Obtain that approval before
   extra provider runs or implementation. Independent review requirements
   follow the accepted change's contract.
4. After that path qualifies, consider the matched Pi/Codex comparison in the
   earlier note. Pin the same underlying model, snapshot, task, capabilities,
   limits and independent acceptance; compare total accepted-workflow cost.

The current seeded one-line edit can establish actual tool execution,
validator success, context reset/recovery and receipt delivery. Its prescribed
edit and tools-disabled successor do not establish open-ended coding quality,
multi-step development after compaction, unattended liveness or readiness to
move daily development into Work Engine.
