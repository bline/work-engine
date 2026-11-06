# Repeatable development: stronger model and risk qualification, revision 2

Recorded 2026-11-06 UTC. Status: **proposed; execution not approved**.
This supersedes the proposed $35/two-hour
[first budget](repeatable-development-submission-and-failure-tests-budget-20261106.md)
at `d204cc0b`, following the user's requirement for a stronger test model and
their addition of [Pi risk research](../docs/pi-risks.md). The first budget and
prior accepted pilot records remain historical. The prior $45 pilot is closed;
neither unused allowance nor its live authority carries into this proposal.

## Intended result

Prepare the existing Work Engine job path for **`gpt-6.1-sol`, high reasoning**,
make operator submission reproducible and qualify the first deterministic
deadline/interruption/restart cases. Keep the current `openai-codex` OAuth route
explicit. Meaningful subsequent coding-quality trials use the stronger tuple;
the Luna pilot establishes a bounded execution/task outcome, not quality parity
for normal engineering work.

Sol/high is an initial qualification target, not a demonstrated optimum. It is
consistent with the repository's Sol engineering direction and the user's
stronger-model requirement. The current official
[model page](https://developers.openai.com/api/docs/models/gpt-6.1-sol)
documents high reasoning and Responses tool calling. The locally pinned Pi
catalog contains the exact model. Neither observation proves eligibility or
successful execution through this account and route; those remain live-test
requirements. Astra stays the separately established reviewer.

## Verified prerequisites, not a configuration-only switch

Bounded source inspection found these concrete constraints:

- The Host's `ProviderPlan::validate` in
  `rust/crates/work-engine-manual-host/src/pi_coding_provider.rs` admits only
  `gpt-6-luna` and at most 1,000,000 microUSD per job.
- `verify_live_job` in `rust/crates/work-engine-server/src/job.rs` independently
  admits only that model, exactly four requests and at most 700,000 microUSD.
- The pinned `receiver/pi-coding.mjs` checks the Luna model and configures the
  SDK with `thinkingLevel:'off'`. A stronger model needs explicit reasoning
  selection and evidence of the actual serialized request, not only a label
  in its manifest.
- The existing full-reservation algorithm uses the catalog's maximum input,
  cache and output rates across all tiers and its full context/output bounds.
  Applying it to the pinned Sol 6.1 entry gives **4,420,001 microUSD per request**,
  or **$17.680004 for four requests**. Luna gives $0.656004 for four requests.
  The Sol calculation is a counterfactual use of that algorithm on catalog data;
  current admission refuses it. It is a conservative hold, not expected usage,
  actual billing or measured subscription-quota consumption.

The catalog is the pilot-pinned `openai-codex.json`, SHA-256
`dd30453068f310317abf35d9adc6d841339a1351f0c6925a6aff45d975e0257c`.
Do not substitute public API context/pricing assumptions for the pinned route.
Any smaller reservation needs an enforceable request bound and reviewed cost
proof; a cheaper estimate alone cannot replace the current reservation contract.

The model/effort/cost contract must compose across admission, job identity,
receiver requests, lifecycle evidence and receipts. Add the smallest explicitly
versioned supported tuple rather than widening old profiles by string replacement.
Preserve existing encoding, profile and receipt fixtures. Inspect whether real
reasoning response shapes need a separately versioned proof before claiming
support; do not bypass the lifecycle validator.

Graph evidence is Tier 2, generation `2026-11-06T02:56:09Z`. Selected source paths
are content-verified but have unresolved call edges; direct reads of the material
validation, reservation and receiver ranges govern these findings. This is not
an exhaustive inventory of every model-dependent assumption.

## Proposed budget and limits

**$45 maximum / 150 minutes from approval.** The additional $10 and 30 minutes
cover explicit model/effort/admission/reservation work missing from revision 1.
This remains a preparation and deterministic-qualification round.

| Initial allocation | Allowance |
| --- | ---: |
| Route, contract and risk mapping; feasibility evidence | $8 |
| Stronger-model preparation, repeatable submission and fault tests | $20 |
| Separate Astra review and retained remediation review | $8 |
| Integration, repairs, evidence, local publication and closure | $9 |
| **Total** | **$45** |

Count preparation from the original budget request, all bound actors, failures,
review, repairs and closing tails once inside this ceiling. Use the existing
speed-adjusted API-equivalent proxy, preserve raw usage and reserve for outstanding
calls. Billing and subscription quota remain separate observations. Allocations
are adjustable supervisor controls, not independent grants.

- **First gate: $8 or 25 minutes**, whichever comes first. Establish a bounded
  owner-supported Sol/high contract and provider-free qualification route. Name
  any lifecycle, SDK or authority blocker that prevents a credible fit.
- **Stop new productive launches at $30 or 100 minutes**, whichever comes first.
  Protect at least $15 and 50 minutes for review, bounded repairs and closure,
  including outstanding calls and process collection.
- **Hard stop: $45 or 150 minutes**, whichever comes first. Bind absolute times
  from the actual approval before launching work. Admit only subprocesses whose
  deadline and cleanup fit inside the remaining window. No automatic extension.
- **Live provider execution reservation: zero in this round.** The $17.68 Sol
  hold above is input to a later live budget, not permission for a provider trial.
  Builder/reviewer model inference is included in this round's $45.

## Deliverables and exact evidence limits

1. **Stronger-model contract and preparation.** Offline tests establish the
   exact model, high reasoning, provider/endpoint, request limits, reservation
   and authority bindings through the intended existing owners. Capture the
   actual adapter's serialized request with a deterministic transport substitute;
   reject unadmitted model/effort/payer changes and preserve old profile behavior.
   If no permitted offline route can establish this, report the remaining
   prerequisite rather than claiming model support from catalog presence.
2. **Repeatable operator submission.** A repository-owned Rust entry or bounded
   extension prepares/validates the existing job bundle from explicit task,
   workspace, validator, limits and separately trusted authority inputs. Use
   existing MCP/command/server submit/read operations. Preparation executes no
   inference and cannot mint provider permission. Reproduce two independently
   authorized deterministic jobs without copying the old private launcher or
   editing generated JSON; fresh jobs have isolated state and identities.
3. **Deadline, quiet control, interruption and replay.** Retain revision 1's
   required hanging-child, healthy-silent-command and controlled interruption/
   restart cases, with real caller/owner execution and independent effect-count
   observations. Preserve partial output, exact receipts and known/unknown effect
   state. Exact repeat reads the same operation; reconciliation cannot launch
   replacement work. Name each fixture and tested layer. Collector termination
   does not establish workspace-command deadlines or container closure.
4. **Risk-to-evidence result.** Reference the register's IDs and report observed,
   unmet and out-of-scope properties with exact test/receipt evidence. Neither
   this plan nor a successful test closes unrelated risks.
5. **Review and publication.** Run relevant tests, formatting and affected lint
   checks on the exact subject. Establish a separate read-only Astra/default
   review, using high for bounded code and xhigh where the model/admission
   contract remains an architecture subject. Preserve the realized grant and
   exact reviewed revision; retain that reviewer through bounded repairs. Root
   accepts the exact result and collects every actor/process before closure.

If a required property cannot fit, the result may be reviewed prerequisite work
with an explicit held requirement. It is not acceptance of the full outcome.

## How the risk research changes qualification

Research input is `docs/pi-risks.md`, SHA-256
`74f4c7a32f559fa1f414d337d47a9edea7fd0b6e6f7b0973ec7549347a741efe`.
Its tagged-source, benchmark and policy statements retain their attributed
standing; this pass did not independently reproduce every cited source or result.
It supplies test hypotheses and proposed readiness conditions, not execution
authority or proof that our particular SDK/tool route has each cited behavior.

| Register IDs | Treatment |
| --- | --- |
| A07, Q01 | Sol/high tuple and request preparation in this round; actual account/model access and quality remain unestablished until live qualification. |
| E01–E02, E07–E08; F01–F04, F16, F18 | Selected deadline/quiet/interruption/restart/replay cases in this round. Publish exact owner/layer coverage; partial coverage does not close the whole ID. |
| E04–E05; F05–F06 | Actual SDK truncated-tool and truncated-prose injection remains required later; generic malformed-wrapper tests are insufficient. |
| E06, Q04; F07–F08 | Independent acceptance remains mandatory. Later quality trials add withheld acceptance/false-success controls; model assertions and exit zero cannot establish acceptance. |
| S01–S03, S06; F15, F17 | Preserve effect, mount, credential and delegation boundaries here. A full alternate-tool/extension/context-contamination bypass suite remains open. |
| A01, A08, Q06; F09 | Pin payer, preserve all actor/retry/review usage and prohibit silent fallback. Full fault and actual-quota characterization remain open. |
| A02–A04; F10–F11 | New public `openai`/ChatGPT OAuth is a separate candidate route. Its scope/field/proxy tests do not automatically describe our existing `openai-codex` lane; qualify applicable cases per route. |
| E03, Q03, Q05; F12–F14 | Busy-without-progress, evidence invalidation, budget expiry and long-context/repeated recovery require later qualification. |

Apply P0 readiness conditions to the exact selected execution and inference route
before claiming bounded unattended readiness. Keep open rows visible. The source
document's suggested G0–G5 sequence is a candidate route, not a retroactive
condition invalidating the completed supervised pilot or authority to run A/B.

Transport replay and inference retry are distinct: an exact request repeat
retains the existing operation; a separately admitted provider request/retry has
its own linked call evidence and cost. Resume must preserve task/effect identity
and cannot justify issuing another effect under a new attempt label.

Next live quality work should use the same exact Sol/high tuple, repository
snapshot, capability/effort limits and withheld verifier in Pi and native Codex,
with all attempts, review and repair counted. Run-to-run spread and matched
repetitions belong to that separately budgeted comparison; this round is not A/B.

## Ownership and proposed authority

This session owns integration, model/effort binding, accounting, evidence,
acceptance and local publication. Root may implement or delegate one bounded
Rust builder, obtain bounded existing-owner consultation and establish the
separate Astra reviewer under applicable orchestration contracts. All paid
turns count here. Preserve this Root's model/settings. Claim exact resources
before edits and isolate changes from foreign source/index work.

The claims reader at `210e0955` is available. Full claims-aware SDK/lifecycle
continuation remains a separate incomplete workstream; this round neither
depends on it nor takes over its source. Agree lifecycle ownership before any
needed model-proof extension, rather than editing another session's work.

Approval would authorize reversible implementation, fresh owned provider-free
local subprocess/container tests with existing approved assets, bounded repairs
and local commit/merge of accepted owned changes. No provider/native-credential
access, auth-route rollout, Pi upgrade, push, deployment, global activation,
live comparison or unattended authorization is included. Future live authority
must bind the stronger tuple and its conservative reservation separately.
