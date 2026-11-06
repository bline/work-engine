# Live Sol/high development pilot: proposed budget

Recorded 2026-11-06 UTC. **Proposed; no live execution grant bound.**
Prepared from the user's “great. let's continue with your next test.” at
04:43:55.259Z. The completed provider-free round remains closed:
[result](../records/repeatable-development-sol-20261106/README.md).
Its [budget](repeatable-development-sol-and-risk-budget-v2-20261106.md)
explicitly reserved zero for live provider execution. This proposal binds a
new live scope; unused earlier allowances do not carry forward.

## Intended result and task

Run **one supervised real coding job with `gpt-6.1-sol`, high reasoning** through
the merged preparation, MCP/command/server, Host, Pi and Docker path. Establish
actual account/model access, an observed response, one workspace effect, the
deliberate SDK reset and recovery, independent acceptance, and repeat/restart
readback of the original operation. Catalog and synthetic-wire evidence from
the previous round do not establish these live properties.

The job appends a `fraction_regressions` Rust test module to
[`timestamp.rs`](../rust/crates/context-pressure/src/timestamp.rs), preserving
every original byte as a prefix. It covers fractional-second truncation beyond
milliseconds, the midnight rule that all fractional digits must be zero at
hour 24, and TimeClip boundaries with explicit offsets. This is useful regression
coverage of currently correct code, not a claimed production bug or a general
coding-quality benchmark.

Source baseline is `d38900b975c3f2b95ec8edc61b70bf809fef3780`; the selected
module is 6,766 bytes, SHA-256
`589ecffa40f3667189f0ba7c8f5413f24e3f0b2285f401803b77364383291684`.
The draft task including that source is 7,501 bytes, within the existing
8,192-byte task limit. The independent verifier and mutation controls are
**not frozen or executed yet**; that is a pre-admission requirement below.

## Budget and controls

**$45 maximum / 120 minutes from actual approval.** All preparation from the
04:43:55.259Z continuation request, all actors, failures, review, repair and
closure count once inside the dollar ceiling. Bind absolute deadlines and the
exact approved subject before execution. Earlier-round spending is not charged
again. Do not interpret this proposed document as approval.

| Initial allowance | USD |
| --- | ---: |
| Preparation, verifier controls and admission checks | 6.000000 |
| Full conservative four-request product hold | 17.680004 |
| Runtime supervision, independent checks and integration | 8.000000 |
| Separate Astra/high/default review and retained remediation review | 6.000000 |
| Bounded repairs, publication and closure reserve | 7.319996 |
| **Total** | **45.000000** |

Allocations are adjustable within the total. The product hold stays fully
reserved until the owning budget contract permits release or reconciliation;
an expected token estimate cannot replace it. Count a hold and its reconciled
usage once, not twice. Model-call accounting uses the existing speed-adjusted
API-equivalent proxy with raw usage and outstanding-call reserves. Report actual
billing and subscription-quota observations separately; unknown is not zero.

- **First gate: $8 or 20 minutes**, whichever comes first. Freeze a feasible
  task/verifier, exact source and runtime inputs, payer and admission bindings.
  Before credential/provider admission, check current capacity, credential
  validity through the existing owner and the full remaining reservation. A
  failed prerequisite remains held with evidence rather than widening the lane.
- **Stop new productive launches at $30 or 75 minutes**, whichever comes first,
  including the outstanding product hold. Protect at least $15 and 45 minutes
  for independent review, bounded repairs and collection/closure.
- **Hard stop: $45 or 120 minutes**, whichever comes first. No automatic
  extension. Every admitted process, model turn and cleanup deadline must fit
  the remaining allowance. Reserve for closing tails before admission.
- **One live job, at most four product provider requests.** The existing
  one-workspace-effect/two-SDK-session profile remains fixed. A failed job is
  not permission for a second live attempt. Exact transport repeat reads the
  first operation and cannot admit replacement work.

## Runtime and credential scope

Use the already qualified, separately versioned
`work-engine.pi-coding-lifecycle-sol-high.v1` profile, runtime admission v2,
`work-engine.sol-high.v1` tuple, lifecycle v5 and Sol/high continuity checkpoint.
Keep `openai-codex`, `openai-codex-responses`,
`https://chatgpt.com/backend-api`, high reasoning and the exact payer bound
together. Four requests reserve 17,680,004 microUSD under the pinned algorithm.
The [preparation reference](../rust/crates/work-engine-server/docs/coding-preparation.md)
describes the current operator boundary; preparation's `allow_provider:false`
cannot mint the separate live admission.

Retain Pi 1.5.0, the existing offline Rust 1.92.0 toolchain and Docker image
`node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da`.
The pinned `openai-codex.json` catalog digest remains
`dd30453068f310317abf35d9adc6d841339a1351f0c6925a6aff45d975e0257c`.
Freeze immutable producing binaries, full asset manifests, task and validator
digests, deadlines and job identity before admission. A changed material input
requires renewed binding; a mutable working-tree path is insufficient.

Approval would include the existing Host owner's bounded projection of the
already configured Codex OAuth access token and account identity, from the
native credential reference into the private job scope. Retain the existing
expiry/deadline checks and secret handling. Do not print credentials, copy
refresh or ID tokens, silently refresh/re-authenticate, change payer, or fall
back to another auth route. Expiry or access rejection may end this round with
a useful blocker. The future
[named connections and fallback direction](../notes/provider-auth-connections-and-fallback-direction-20261106.md)
is a separate product scope; this pilot does not implement those interfaces.

## Acceptance and evidence

Before provider admission, freeze a Rust acceptance verifier outside the
implementing agent's workspace and capabilities. Confirm that a valid local
control passes and invalid/empty output fails. Require an unchanged original
source prefix and tests only in the appended module. Acceptance cannot come
from the implementing model's final statement or exit zero alone.

The new tests must pass against the original implementation and catch two
meaningful defects: checking only the first three fractional digits for hour
24, and rounding the fourth fractional digit instead of truncating it. Run
the new module's tests against each mutant so existing tests cannot supply the
mutation credit. Retain the original crate's relevant tests and formatting
checks on the exact integrated candidate. The private host Node/V8 reference
observations are task preparation only; they are not yet verifier or mutation
evidence and are not container-runtime observations.

Actual run evidence must bind model, reasoning, payer, request count, bounded
tool execution, checkpoint/reset/reopen, exported patch and original receipt.
Use the existing bounded command and collector limits with an outer custody
deadline and explicit cleanup grace; record exact selected values before launch.
Preserve partial or truncated responses and failed attempts. An incomplete or
unverified result cannot be classified as accepted completion. This is not an
injected output-cap fault test.

Repeat and restart reads must recover the same operation/receipt without
repeating the workspace effect. Native `owner_pending` acceptance and domain
settlement retain their existing meanings. Only directly observed process and
container collection can establish closure.

Establish a fresh separately scoped, read-only **Astra/high/default** review of
the exact patch, acceptance evidence and limitations. Independently preserve
its realized capability grant and retain that reviewer through bounded
remediation. Root owns acceptance and local publication; report any unmet
condition explicitly rather than silently weakening acceptance.

## Ownership, authority and limits

Root owns the budget, admission, supervision, evidence, integration and closure,
and preserves its current model/effort/service settings. Following approval it
may use one bounded preparation builder where useful and the separate Astra
reviewer under the existing orchestration contracts. All turns count here;
collect every launched actor/process and release claims at terminal closure.

Approval would authorize the single live job and bounded existing credential
projection above, reversible owned local preparation/validation/repairs,
separate review and local commit/merge of accepted owned work. It includes no
second live job, model/auth fallback, Pi upgrade, A/B comparison, push, deploy,
global activation or unattended-development authorization. Preserve foreign
worktree/index changes and other sessions' source ownership.

The [Pi risk register](../docs/pi-risks.md) retains open requirements for actual
truncation injection, busy-without-progress detection, auth alternatives,
quota/failure characterization and long-context/repeated recovery. Passing
this pilot establishes only the observed bounded live lane and accepted task.

Preparation evidence is under
`/home/bline/.local/state/work-engine/sol-high-live-pilot-budget-20261106T0444/`.
Source discovery used Tier 2 graph generation `2026-11-06T04:41:40Z`;
`timestamp.rs` content matched, with unresolved call edges, and its complete
source was read directly. No exhaustive test-gap or caller-coverage claim is
made. No provider request, credential read or paid child has been launched for
this proposal.
