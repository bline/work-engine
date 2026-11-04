# S10 Rust reuse map — 2026-11-03

Status: source-verified planning evidence and recommended integration order.
No implementation, provider login/inference, Docker run or release is authorized
by this record. It supplements the selected
[Pi/Docker convergence plan](s10-pi-docker-convergence-amendment-20261103.md).

## Start from the deterministic work already present

[`work-engine-manual-host`](../rust/crates/work-engine-manual-host/src/lib.rs)
is the first mechanical reuse anchor. It already performs many operations for
which the earlier S10 work repeatedly spent supervisor inference. Reuse its
admission, observation, collection and receipt contracts rather than build a
second general orchestration ledger.

This is a recommendation to adapt existing behavior, not to deploy the current
binary unchanged. The current host composes pinned Python/native Codex adapters,
uses foreground observations and returns decisions to an outside root. Its
manifest has no lifecycle/task/review dependencies. Those separate owners are
available, but their joined Pi/Docker path is not already implemented.

| Implemented manual-host behavior | Source in `src/lib.rs` | Consequence for the selected base |
| --- | --- | --- |
| Validate objective, authority, packet/tool/input identities and operation replay; preserve uncertain creation/start | `Host::dispatch`, line 514 | Mechanical job admission and replay checks need no model turn. |
| Prepare before start, verify returned session/read-only state, bind budget/profile, check retained ownership/settings | `Host::admit_prepared`, line 978 | Preserve launch admission and identity binding; replace Codex-specific realization for Pi. |
| Read owned context, budget and scope evidence; require established headroom for admission | `Host::observe` / `headroom`, line 1167 | Reuse accounting and unavailable-state semantics; add attributed Pi observations. These are not exact billing caps. |
| Wait locally for the exact session/turn, terminal state, new pressure, unavailable observation or bounded stop | `Host::collect`, line 1314 | Routine waiting can remain outside inference. The current implementation does not interrupt a running native turn. |
| Validate and snapshot result/artifact/custody identities, then recheck source and snapshot hashes | `Host::collect_artifacts` / `verify_snapshot`, line 1415 | Collection is deterministic; terminal output still needs the contract's semantic acceptance. |
| Persist content-addressed receipts and require exact-ID acknowledgment without replaying effects | `Host::receipt` / `acknowledge_receipt`, line 1540 | Separate delivery acknowledgment from acceptance, integration and task execution. |
| Lock and atomically save state; record adapter process identity, exit, timeout and leftovers; retain uncertainty on restart | `Host::open` / `run` / `verify_processes`, line 373 onward | Reuse custody bookkeeping. Immediate-child/process-group evidence does not establish Docker daemon workload termination. |
| Gate closure on authority, collected effects/artifacts and acknowledged receipt; reconcile accounting separately | `Host::close` / `reconcile_accounting`, line 1583 | Keep unresolved work visible without rerunning the task to refresh its cost record. |

The current receipt's `root_owned` acceptance/integration meaning does not
require a standing outside supervisor model in the new base. The job contract
can bind local semantic decisions to the authorized container role and reserve
acceptance for its named owner. The outside host applies only configured,
mechanically decidable transitions; a missing semantic decision remains pending
with its owner. Parsing a model verdict cannot independently establish it.

## Reuse owners, with their actual boundaries

| Existing owner | Useful behavior | Integration limit |
| --- | --- | --- |
| [`lifecycle-core`](../rust/crates/lifecycle-core/src/lib.rs), [`lifecycle-provider`](../rust/crates/lifecycle-provider/src/lib.rs), [`lifecycle-store`](../rust/crates/lifecycle-store/src/lib.rs), [`lifecycle-runtime`](../rust/crates/lifecycle-runtime/src/executor.rs), [`lifecycle-service`](../rust/crates/lifecycle-service/src/semantic.rs) and [`lifecycle-semantic`](../rust/crates/lifecycle-semantic/src/lib.rs) | Durable admission/claims, owned execution/results, checkpoint/revalidation and uncertainty/recovery distinctions. Runtime entry requires store authorization and matching capacity. | Preserve this continuity owner. Existing native/direct realizations are not Pi adapters; process exit does not settle domain effects. |
| [`retained-task-workflow`](../rust/crates/retained-task-workflow/src/lib.rs) and [`work-engine-task-host`](../rust/crates/work-engine-task-host/src/lib.rs) | Canonical task meaning, exact lifecycle command binding/readback, idempotency and replacement authority. | Current profile is retained repository analysis. It does not authorize an arbitrary engineering mutation job. |
| [`work-engine-context-status`](../rust/crates/work-engine-context-status/src/control.rs) and [`context-pressure`](../rust/crates/context-pressure/src/controller.rs) | Session/model/window/epoch observations, unavailable states, replay checks and hysteresis. | Pi needs normalized evidence. `context-pressure/src/lib.rs:32` explicitly leaves transport-session sequence versus lifecycle ordering unresolved. Pressure is evidence, not context-switch authority. |
| [`bounded-inference`](../rust/crates/bounded-inference/src/direct.rs) | Bounded, attributed direct compile/verify requests and durable attempt tracking. | Its direct path is tool-free. It is not a replacement for Pi's whole coding-agent job. |
| [`review-workflow`](../rust/crates/review-workflow/src/lib.rs), review episode/execution crates, [`slice-campaign`](../rust/crates/slice-campaign/src/lib.rs) and [`work-engine-review-host`](../rust/crates/work-engine-review-host/Cargo.toml) | Existing review, campaign and evidence owners, to compose when the selected job needs them. | Manifest/header-level inventory here; their Pi/Docker join and any required reviewer independence remain unqualified. |
| [`work-engine-completion-host`](../rust/crates/work-engine-completion-host/src/lib.rs), [`workspace-publication`](../rust/crates/workspace-publication/src/lib.rs) and [`claim-evidence`](../rust/crates/claim-evidence/src/lib.rs) | Existing completion, publication and exact evidence boundaries. | Keep their authority; a Pi completion event supplies neither publication permission nor acceptance. |
| [`session-bridge`](../rust/crates/session-bridge/src/lib.rs), [`mcp-auth`](../rust/crates/mcp-auth/Cargo.toml), compiler/types crates | Candidate event-edge, authentication and contract/value components. | Inventory candidates, not an accepted generic Pi bridge or a prerequisite compiler project. |

## Small first integration

The proposed first slice is one whole-job host-to-container-to-result path,
using the existing convergence plan's early feasibility stop and one total cap.
The useful sequence is a dependency recommendation for that slice:

1. Preserve manual-host's mechanical admission/identity/receipt/collection
   behavior and choose the smallest composition with existing domain owners.
   Avoid a wholesale script rewrite before a useful job works.
2. Add the selected Pi/Docker realization: actual container identity and daemon
   wait/state/removal, durable exported artifacts, Pi settled/session boundaries,
   permitted tools/effects and role-specific exposure. Preserve failed/uncertain
   launch evidence rather than retrying blindly.
3. Join the existing event capabilities to the actual recipient and Rust-owned
   continuity/admission/cancellation. Demonstrate outside pressure reaching the
   active lane and Rust-governed continuation, with unavailable/stale observations
   and pending/uncertain work preserved.
4. Use applicable task/review/completion owners for the same job. Keep engineering
   judgment and most supervision inside its container; leave no routine outside
   model polling/dispatch loop.

These are parts of one bounded integration, not four automatically funded
projects. Exact scope, numeric ceiling, acceptance owner and permitted effects
belong in its execution assignment. Original S10/M6 and release conditions remain
as stated in the convergence plan; this inventory adds no new release gate.

## Event integration standing

The event-path owner clarified current state on board sequence **4302**:
existing Events MCP facade/transcript subscriptions/callback delivery and the
local Git-ref CAS chatboard are separate surfaces. Chatboard `--wake` marks a
listener signal; it does not launch a session. Manual-host currently uses
foreground readers and native role dispatch, not an MCP subscription.

The unified service-owned board/context inbox, delivery without an agent-managed
wait, consolidated completion/budget route and actual Pi recipient are unfinished.
The reviewed Claude stdio fixture is bounded offline evidence, not that cutover.
Reuse the existing event work and its owners; qualify the missing delivery join
rather than assume a ready unified push system or create a parallel service.
The owner's attempted live status read was unauthorized, so current daemon health
also remains unverified. No reconnect or live probe was attempted in this audit.

### Shared delivery contribution and receiver boundary

Board **4303**, from session `01a195d4-b7a7-77f2-ba65-277669c0ec97`,
records its next bounded contribution: service-owned board observation, durable
delivery inbox/state, restart/deduplication and existing MCP subscription/status/
acknowledgment, starting with an isolated receiver. S10 retains Pi/Docker
execution, containment, session controls and lifecycle integration. This is
announced ownership and intended work, not a delivered capability. Crate
placement and the exact receiver contract remain to be agreed; this audit
neither dispatches that work nor changes its funding.

The following is a proposed interface boundary for that alignment, not an
accepted wire schema or a new task/lifecycle store:

- Bind each delivery to a stable event identity and the canonical job/lane,
  session and recipient generation. Transport retry/restart preserves event
  identity; a context successor needs an explicit authorized binding rather
  than silent delivery to whichever process is current.
- Carry the producer/source epoch, observation time and freshness basis.
  Context readings retain model/window identity; budget readings retain their
  objective/pool and units. Resolve the pressure crate's transport-session
  sequence versus lifecycle ordering distinction at this join.
- Separate inbox persistence, transport receipt, actual target observation
  and application/completion. An ACK advances only its defined stage; it
  supplies no task authority, acceptance or effect-settlement evidence.
- Preserve pending, busy, unavailable and uncertain outcomes across restart.
  Unestablished observation stays pending/uncertain. Redelivery cannot itself
  authorize repeating a task effect. Hard-limit enforcement remains outside
  the container and cannot depend on the busy model acknowledging a warning.

The delivery owner's first acceptance is restart-safe delivery solely to its
bound test receiver without an agent-managed board wait. Actual Pi receipt,
session generation changes and Rust continuity remain a later joint proof
within an exact execution assignment. Manual-host supplies useful mechanical
contracts, but has no existing event subscription/receiver contract to transplant.

## Evidence and limits

This was Tier 2 Verify retrieval at graph project
`home-bline-code-work-engine`, generation `u89423602d0393f71g2`, indexed
2026-11-03 22:22:35 UTC. All 43 current crate manifests were inventoried. Deep
source inspection concentrated on manual-host, lifecycle/runtime/store/service,
retained-task/task-host, context and bounded-inference. Other rows are explicitly
candidate classifications. All 79 relied file identities matched at parent
collection; coverage reported no recorded issue/content verified, a best-effort
signal rather than completeness proof. Ambiguous short-name graph edges were
rejected in favor of exact source/manifests. Selected test bodies were inspected;
no builds or tests were rerun.

Observed repository HEAD: `0d51c0b8668d2497c0f7b31d5aac9b169faa60b5`.
Manual-host `src/lib.rs` SHA-256:
`9d61aec37746d65a8d7704b4d3867880d31a78a964df4bb1e7e2c4eeede63611`.
The mutable worktree was file-hash-bound, not frozen. Full query/coverage/source
identities and collection checks are under the private audit directory
`/home/bline/.local/state/work-engine/s10-supervision-20261103/reuse-audit/`.
The event-path clarification is attributed owner evidence, not an independent
source audit of the separate delivery branch.

Historical [Docker mechanics](../metrics/probes/manual-host-docker-20261103.md)
support retained Codex conversations, real artifact changes, exports and teardown;
they leave production exposure, Pi and this integrated host unqualified.
The [controlled trial](../metrics/probes/manual-host-controlled-20261103-qualification.md)
failed admission and produced no new efficiency estimate.
The [manual-host retrospective](../metrics/manual-host-efficiency-20261103.md)
supports less generated glue in unequal historical cohorts, not a causal claim
of net cost or time savings. No new savings estimate or release date is inferred.

## Manifest inventory

Complete for current `rust/crates/*/Cargo.toml` enumeration only. Classification
outside the deep scope above does not establish implementation completeness,
provider compatibility, dead code or immediate migration need.

| Crate | Role | Reuse relevance |
| --- | --- | --- |
| `bounded-inference` | Bounded request/stream/auth/journal; tool-free direct compile/verify | Conditional: useful inference primitives, not whole Pi agent actuator |
| `claim-evidence` | Exact claim/reliance lineage and evidence storage | Reuse at evidence boundary; manifest inventory only |
| `context-pressure` | Pure projection/hysteresis/replay validation | Reuse evidence; ingress sequence/epoch join remains open |
| `decision-readiness` | Mechanical readiness profiles/observations | Later consumer; never semantic verdict |
| `experiment-control` | Pure experiment policy/transitions | Conditional pilot policy; not general job owner |
| `historical-evidence-client` | Historical read client | Later diagnostics |
| `historical-evidence-read` | Historical read implementation | Later diagnostics |
| `historical-evidence-wire` | Historical read DTOs | Later diagnostics |
| `lifecycle-admin` | Lifecycle custody/fencing/adoption administration | Reuse existing recovery authority; manifest only |
| `lifecycle-client` | Lifecycle wire client | Reuse transport where fit; manifest only |
| `lifecycle-codex` | Codex native provider realization | Existing backend, Pi-specific adaptation required |
| `lifecycle-core` | Pure lifecycle contract values/reducer | Reuse canonical lifecycle vocabulary |
| `lifecycle-proof` | Lifecycle proof tooling | Validation support; manifest only |
| `lifecycle-provider` | External execution fact interface | Candidate actuator boundary; cannot grant admission |
| `lifecycle-runtime` | Owned execution after store admission | Reuse execution/result retention/drain semantics |
| `lifecycle-semantic` | Pure semantic preservation values/checks | Reuse continuity contracts; models still judge meaning |
| `lifecycle-semantic-replay` | Semantic replay utility | Validation support; manifest only |
| `lifecycle-service` | Lifecycle composition and route-bound native/direct execution | Primary continuity integration owner; Pi route unqualified |
| `lifecycle-store` | Durable transactional lifecycle owner | Reuse claims/recovery/journal; no parallel lifecycle store |
| `lifecycle-test-support` | Lifecycle fixture helpers | Validation support; manifest only |
| `lifecycle-wire` | Lifecycle commands/snapshots wire values | Reuse DTOs where fit; manifest only |
| `mcp-auth` | MCP authentication primitives | Conditional existing MCP edge; manifest only |
| `portfolio-decision` | Mechanical portfolio record validation | Later strategy, no immediate runtime consumer |
| `proposal-evaluation` | Mechanical proposal estimate validation | Later strategy, no immediate runtime consumer |
| `retained-task-workflow` | Durable retained repository-analysis task meaning | Reuse domain pattern/owner; profile is narrower than engineering job |
| `review-episode` | Review episode composition | Reuse existing review semantics; manifest only |
| `review-episode-core` | Pure review episode values | Reuse existing review semantics; manifest only |
| `review-episode-store` | Durable review episode owner | Reuse existing review state; manifest only |
| `review-execution-custody` | Read-only checked execution evidence reader | Preserve independent evidence consumption; header inspected |
| `review-execution-evidence` | Exact bounded process evidence producer | Conditional provider adapter; no campaign/provider authority |
| `review-workflow` | Controlled SC0 initial-review composition | Reuse review join, not universal Pi readiness |
| `session-bridge` | Local-only session-events admission/receipt bridge | Conditional existing event adapter; not MCP replacement |
| `slice-campaign` | Private-root initial review campaign domain owner | Conditional campaign job; preserve its admission/receipt owner |
| `terminal-ui` | Terminal operator interface | Later projection, no immediate headless host consumer |
| `work-engine-compiler` | Contract/config compiler | Conditional typed assembly; manifest only |
| `work-engine-compiler-cli` | Compiler CLI/custody utilities | Mechanism; manifest only |
| `work-engine-completion-host` | Completion application composing campaign/publication owners | Reuse final domain checks/publication route when applicable |
| `work-engine-context-status` | Codex/Claude observation/control projection CLI | Reuse observations; Pi normalization/freshness adapter required |
| `work-engine-manual-host` | Deterministic manual dispatch/observe/collect/receipt executor | First reuse anchor; Codex adapter and root judgment assumptions need change |
| `work-engine-review-host` | Review workflow application host | Conditional review composition; manifest only |
| `work-engine-task-host` | Permanent retained repository-analysis task host | Reuse admission/readback/replacement pattern, bounded existing profile |
| `work-engine-types` | Shared canonical types/digests | Reuse vocabulary where existing owners expose it; manifest only |
| `workspace-publication` | Local Git publication owner | Reuse authorized publication boundary; no implicit publish authority |
