# Pi Durable + pi-env as the per-agent execution substrate: investigation, 2026-11-17

Standing: an investigation answering the operator's handoff
`pi-durable-work-engine-architecture-handoff-v2.md` (2026-11-16, held outside
the repository). It is not a decision, not authorization to implement or
migrate, and not independently reviewed. Work Engine's orchestration stays
deterministic and in Rust; no Pi subagents are considered.

Inspected:
- upstream `earendil-works/pi` at `1f6f319bb28992a40b99c147fffb23b65a23129d`
  (2026-11-17T01:00+01:00); `@earendil-works/pi-durable` 1.5.2 and
  `@earendil-works/pi-env` 1.5.2. Clone at
  `~/.local/state/work-engine/upstream/pi`.
- Work Engine at `252b953e`: `rust/crates/work-engine-manual-host` (Pi custody,
  coding profiles, the hosted transport) and `rust/crates/pi-pressure-context-host`
  (the coding lifecycle gate).

Nothing was built or run. "V" means read in source or upstream docs at those
revisions. "A" means assumed, not checked. "P" means proposed.

## 1. Answer in brief

- **pi-env fits our boundary well.** It is a small Rust daemon with a framed
  stdin/stdout protocol. Its client starts it with any command, so
  `docker exec -i`, or a Work Engine broker wrapping it, works. It kills its
  process groups when the connection is lost.
- **Pi Durable fits less well than the handoff hopes.** It is an autonomous
  durable scheduler. It resends model requests after a close, retries them
  after a crash (unless retries are disabled), and turns an interrupted tool
  call into an error result that the model then answers in a new request.
  Without separate deterministic gates it would also run background
  compaction requests and resume all pending work when its scheduler starts.
  None of this is fatal. Each behaviour can be disabled, or vetoed at a
  boundary Work Engine owns (the model HTTP endpoint, and the pi-env wire).
  But the veto must live in Rust at those boundaries, not in Durable's hooks:
  a throwing `beforeProviderRequest` hook is skipped, not enforced.
- **The simplification is real but narrower than "replace our Pi plumbing".**
  These parts would go:
  - the in-container model runtime and its receiver shims;
  - the container↔Host stream-frame protocol;
  - export and rehydrate of the Pi session across container replacement;
  - credential projection into containers.

  These parts must stay in Rust: the lifecycle gate, the attempt ledger,
  access screening, SSE retention and the Docker custody journal. New parts
  would be needed: an HTTP/SSE provider endpoint, a pi-env wire broker,
  reconciliation of Durable's state after a crash, and a TypeScript adapter.
- **Durable may not be needed.** The Pi coding agent we already run supports
  pluggable tool operations (`BashOperations`, `ReadOperations`, …) and keeps
  sessions as files. Running it on the host (or in its own long-lived
  container) with pi-env-backed operations gives persistent context without
  Durable's scheduler. That is the comparison the experiment should decide.
- **Recommendation: defer adoption; run one bounded offline A/B experiment**
  (section 7) if the operator wants to pursue it. It needs no live inference.

## 2. Fact table

| # | Fact | Status | Basis |
| --- | --- | --- | --- |
| F1 | Durable is marked experimental; "the API changes without notice between releases". | V | `packages/durable/README.md` |
| F2 | Durable is a separate harness from the Pi coding agent (`pi-coding-agent` is built on `pi-agent-core`, not Durable). | V | `packages/coding-agent/package.json` |
| F3 | Durable commits every model turn, tool call and document change to storage (SQLite, JSONL, memory, IndexedDB, Durable Objects) before showing it. One writer per storage; no cross-process lock. | V | README "Storage" |
| F4 | A model request cut off by a **close** is sent again on recovery, with the same committed messages and model. | V | spec §8.3 `request` |
| F5 | A model request cut off by a **crash** counts as a failed attempt. It is retried under `settings.retry` (`enabled`, `maxRetries`). With no retry left, the run ends with `runFailure: interrupted`. `restartLimit` (default 5) bounds crash loops. | V | spec §8.3; README "Failures and Recovery"; `src/harness/types.ts:660-786` |
| F6 | Retryable provider errors are retried within the retry policy. pi-ai may rebuild the request body "such as a retry within one attempt". Whether a given provider SDK also retries internally was not checked. | V / A | spec §8.3; `types.ts:1108` |
| F7 | `beforeProviderRequest` can replace a request body but cannot veto: a throwing hook "is reported and skipped". | V | README "Errors" rule 3 |
| F8 | Every model call goes through the host-supplied `Models` (`runtime.models.streamSimple`), so the host can wrap or redirect it. | V | `src/harness/generation.ts:811`; `ai/src/models.ts` |
| F9 | Durable's usage totals in `pi.usage` are not conservative. A stream that crashes in its first second "may leave no aborted entry, and that attempt's usage is not recorded." | V | README "Watching a Conversation" |
| F10 | Each tool call is a durable task whose intent is committed before `execute()`. After a crash, a tool that is not `replay: "safe"` is not rerun: the model gets an `interrupted` error result. The built-in `read`, `write`, `edit` and `bash` declare no replay policy. | V | README "Tools"; `src/tools/*.ts` |
| F11 | After an interrupted tool call the run continues: the generation sends the next request with the error result, so the model decides what to do about an uncertain effect. | V (derived) | spec §8.3, §8.5 |
| F12 | Nothing runs until `resume()`, `submit()` or `wait()` starts the scheduler. A `readOnly` open recovers nothing and rejects all writes. | V | README "Persist and Resume" |
| F13 | Automatic compaction is itself a model request (threshold, background and overflow). It can be disabled per Harness or per conversation; manual compaction always works. | V | README "Compaction" |
| F14 | Subagents, child conversations, background tasks and code mode exist only if an extension provides them. The registry is host-owned; by default a conversation selects every installed extension. A `beforeTool` hook that throws blocks the call. | V | README "Extensions", "Abort and Subagents", "Errors" |
| F15 | The built-in tools reach files and processes only through the call's `ExecutionEnv`. With no environment they fail; there is no local fallback. The `env` resolver is host code, called per tool call and section render. | V | README "Environment"; no `fs` or `child_process` in `src/` outside env, storage, images and testing |
| F16 | pi-env's `Connection` spawns `command + ["serve", "--token", <hex>]` and talks framed stdin/stdout. `command` may be an async function, evaluated at each daemon start. A lost connection fails requests in flight with `lost: true` (a mutation's outcome is unknown), and the next request starts a new daemon. | V | `packages/env/src/connection.ts:78-460`; `docs/protocol.md` |
| F17 | The pi-env daemon kills every process group it started, and exits, when stdin ends, after 30 s without bytes, or on SIGTERM, SIGINT or SIGHUP. | V | `docs/protocol.md` "Liveness" |
| F18 | Besides file and `exec` operations, pi-env offers `pty`, `process` (pipes for MCP-style servers) and `watch`. The built-in tools use file and `exec` only. | V / A | protocol; tool use of `pty` and `process` not traced |
| F19 | The Pi coding agent's tools accept pluggable operations (`BashOperations`, `ReadOperations`, `EditOperations`, `WriteOperations`, `Find`, `Grep`, `Ls`). | V | `packages/coding-agent/src/core/tools/*.ts` |
| F20 | pi-ai has an `openai-codex` provider (the subscription route our hosted profile targets) and an `anthropic` provider. Whether they work against a loopback proxy that injects access, as our hosted transport does, was not checked. | V / A | `packages/ai/src/providers/` |
| F21 | Today the model runtime (Pi coding agent plus our receiver) runs inside a network-none inference container. Model requests leave over stdio frames (`bridge_provider`, `provider_fetch`, pull, cancel, ack) to the Rust Host, which gates, reserves, screens, relays and retains them. The one tool, `workspace_exec`, is dispatched by the Host into a separate tool container. | V | `work-engine-manual-host` `receiver/*.mjs`, `src/pi_coding*.rs`, `src/pi_stream.rs` |
| F22 | Lines of Pi-specific per-agent code today: `work-engine-manual-host` `pi_*.rs` about 13 500 Rust plus about 700 receiver JavaScript; `pi-pressure-context-host` about 1 900. Pi-related Rust elsewhere is about 7 000 (`review-execution-evidence` Pi module, `session-task-admission` `pi_pilot`, `pi-context-*`, `pi-runtime-pressure`). Much of it is tests and superseded profiles. | V (line counts) | `wc -l` at `252b953e` |

## 3. Responsibility and trust boundaries (proposed)

| Concern | Owner |
| --- | --- |
| Admission, grants, budgets, attempt ledger, failover, acceptance, next work | Work Engine (Rust), unchanged |
| Model HTTP egress: admission per HTTP request, access injection, screening, SSE retention, raw-usage settlement | Work Engine Rust **provider endpoint**: the hosted transport's relay, ledger and gate behind a loopback HTTP/SSE listener instead of container frames (P) |
| Tool effects: which operations, which paths, how many, and which container | Work Engine Rust **pi-env wire broker**, as the `Connection` command. It checks the live lease at every daemon start, allows only file operations under `/workspace` plus `exec`, refuses `pty`, `process` and `watch`, counts effects, and runs `docker exec -i <leased id> pi-env serve …` (P) |
| Conversation, context, turn mechanics | Pi Durable **or** the Pi coding agent, in a long-lived Work Engine-owned process or container with no network, no credentials and no workspace (P) |
| Shell, files, processes | pi-env daemon in the disposable container |
| Durable or Pi session storage | A store Work Engine owns and opens; subordinate to the Work Engine journal, never an authority (P) |

Trust: the handoff puts the agent runtime on the trusted host. That moves
third-party code that parses model output out of the container boundary it
has today. Running it in its own long-lived, network-none container, with its
store on a mounted volume and egress only to the two Rust endpoints, keeps
today's isolation and still decouples context from workspace (P).

## 4. Side by side

| Aspect | Current (Pi in inference container, Host-brokered tool container) | Candidate (runtime outside, pi-env in tool container) |
| --- | --- | --- |
| Context across container replacement | Pi session exported in band, then rehydrated into a new container (`pi_coding_export.rs`, reset and rehydrate protocol) | Context stays in the runtime's store; only the tool container is replaced. Rebinding to a new container is an explicit lease change. |
| Model request path | Container stdio frames → `pi_stream` session → hosted relay → upstream | Runtime HTTP → Rust loopback endpoint → the same relay, ledger and screen |
| Tool path | One custom tool (`workspace_exec`), each call a Host-gated effect in the tool container | Built-in read, write, edit and bash over pi-env, gated per frame by the Rust broker; per-tool-call gating needs `beforeTool` or broker counting |
| Replay risks | Host owns every dispatch; nothing in the container can resend | Durable resends after a close and retries after a crash. Contained by the provider endpoint refusing any attempt while an earlier one is unsettled (the existing ledger rule), and by configuration |
| Crash of the Host or runtime | Pending attempt → Uncertain at reconcile; no replay (qualified offline) | The same at the provider endpoint, plus reconciling Durable's store: interrupted tool tasks and generations must be found **before** anything resumes |
| Accounting | Host raw usage from the relayed bytes | Unchanged; Durable's `pi.usage` is display only (F9) |
| Language | Rust Host; about 700 lines of receiver JavaScript | Rust endpoints; a TypeScript adapter for the runtime (third-party runtime, first-party glue) |
| Upstream stability | Pi 1.5 coding-agent SDK, pinned by digest | Durable is experimental (F1); pin it and expect churn |

Likely removable (code-based estimates, to be confirmed by the experiment):
- the receiver shims (about 700 lines);
- `pi_coding_export.rs` (about 960);
- the container side of `pi_stream` and the stream-frame half of `pi_coding_hosted.rs` (about 1 500);
- `pi_native`, `pi_native_memory` and `pi_coding_auth` (about 1 500);
- superseded profiles (fixture, signed, sol-high), which could be retired regardless under the pre-release policy.

New code: the provider HTTP endpoint (about 600–900), the pi-env broker (about
500–800), Durable reconciliation (about 400–700) and the adapter (about
400–800 TypeScript). Net reduction: plausibly 2 000–4 000 lines, mostly in the
most complex code (container frames and export). That is a meaningful gain,
not a dramatic one, and it trades code that has been reviewed twice for
upstream semantics that have not been qualified.

## 5. Failure matrix (candidate)

| Condition | Required outcome | Where enforced |
| --- | --- | --- |
| Runtime crashes during a model request | Attempt Uncertain, reservation kept. Durable's retry after reopen is a new HTTP request: refused while the attempt is unsettled. | Provider endpoint (existing ledger rule); `retry.enabled: false` |
| Runtime **closes** during a model request | Durable resends (F4); treat as above. The endpoint never assumes a resend is the same attempt. | Provider endpoint |
| Retryable error or SDK-internal retry | Each HTTP request needs its own admission. A refusal-class status is definitive only under the existing rules. | Provider endpoint |
| Background, threshold or overflow compaction | Disabled, or admitted as its own request class. Never an unadmitted request. | Configuration plus endpoint |
| Crash between tool dispatch and its result | Effect unknown. Durable gives the model an `interrupted` result and continues (F11). Work Engine must open the store read-only, find interrupted tool tasks, and decide (stop, or explicitly resume) **before** `resume()`. | Rust reconciliation; never auto-resume |
| pi-env connection lost mid-write | In-flight mutation unknown (F16). Daemon kills its groups (F17). Broker records the loss. A restarted daemon is allowed only on the same live lease. | Broker |
| Container gone or lease revoked | Broker refuses the next daemon start. Tools fail; the context remains. No rebinding without a new admitted lease. | Broker |
| Model asks for another agent, MCP, a host path or a terminal | No such extension installed. Broker refuses `pty`, `process`, `watch` and paths outside `/workspace`. | Registry plus broker |
| Run completes | A completion event to Work Engine; acceptance is validated separately. | Adapter event plus Rust validation |
| Duplicate or late events | Submissions are keyed by Work Engine's `requestId` (Durable deduplicates). Events carry Work Engine identities. | Adapter |

## 6. Answers to the handoff's questions

1. **Runtime only, orchestration absent?** Yes, by configuration. Install only
   the tools extension, disable compaction and retries, never call `resume()`
   without a Work Engine decision, and keep subagents and code mode out of
   the registry (F12–F14). Its internal scheduler remains and resumes pending
   work when started; Rust reconciliation must own that moment.
2. **All shell and file tools confined to the remote env?** For the built-in
   tools, yes (F15). Remaining paths: any installed extension, hooks, and
   section rendering (which gets the env). Enforce at the pi-env wire broker,
   not only by registry discipline.
3. **Rust owns the lease and brokers the channel?** Yes. `Connection.command`
   is any program (F16). A Rust broker can be that program and use
   `docker exec -i` itself; Node never gets Docker access.
4. **Host session state; container collected; rebinding?** Durable keeps
   transcript, documents, task records and `pi.live` in its store. A collected
   container only breaks tool calls. Binding a new container is a new lease
   the broker resolves at the next daemon start, so it must be an explicit
   Work Engine transition, recorded before the start.
5. **Checkpoint and retry behaviours that conflict?** The close resend (F4),
   the crash retry (F5), error retries (F6), continuing after an interrupted
   tool (F11), compaction requests (F13) and resume-everything on start (F12).
   F5, F6 and F13 can be disabled. F4 and F11 cannot. F4 is made harmless by
   the per-request gate; F11 needs reconciliation before resume. None needs a
   heuristic workaround.
6. **Transports and billing modes?** pi-ai has `openai-codex` and `anthropic`
   providers (F20). Routing them through our access-injecting endpoint, model
   and effort settings, and usage records all need qualification: offline
   first, then live in phase 6. Nothing here is verified against a live
   backend.
7. **What can be deleted?** See section 4. The deterministic orchestration,
   the gate, the ledger, screening, retention, custody and acceptance stay.
8. **Completion without model inference?** The adapter turns Durable's
   settled submission or `runFailure` into a Work Engine event. Work Engine
   observes process exit for crashes and reads the store read-only. This is
   the experiment's gate C.
9. **Is Durable necessary?** Probably not for persistence. The Pi coding
   agent already keeps sessions as files and accepts pluggable tool
   operations (F19). Run on the host (or in its own container) with pi-env
   operations, it has no scheduler to restrain. Durable's real advantages are
   crash-resumable tool journaling and in-process compaction, both of which
   we would mostly disable. Decide by experiment.

## 7. Smallest bounded experiment (if authorized)

Offline. The faux provider or our observer stands in for the model; no live
inference.

1. **Pin.** Upstream commit above; build the pi-env daemon from source; record
   digests.
2. **Two variants on one fixture task** (our coding continuity task, four
   requests with a reset):
   - **A:** a Durable Harness on SQLite, `CodingTools` only, retry and
     compaction disabled, env = `RemoteExecutionEnv` whose `Connection` is a
     Rust broker.
   - **B:** the Pi coding agent SDK with pi-env-backed tool operations and the
     same broker.

   Both send model requests to a Rust loopback endpoint backed by the existing
   hosted relay, ledger and gate.
3. **Gates (any failure is a hold):**
   - G1: every model HTTP request is admitted per attempt; a resend after close
     or crash is refused while the attempt is unsettled.
   - G2: no shell or file effect outside the leased container. The runtime
     process runs with a file-access audit (or in a network-none container).
   - G3: a revoked or removed container makes the broker refuse; no rebinding.
   - G4: after a SIGKILL between tool dispatch and result, Work Engine detects
     the interrupted call from the store read-only, and nothing runs until it
     decides.
   - G5: completion and failure reach Work Engine as events, with no model
     polling.
   - G6: the Rust ledger stays conservative whatever the runtime records.
   - G7: measured net code removal versus added code, per variant.
4. **Cost:** engineering only, about two to three focused sessions. No API
   spend except an optional independent review.
5. **Decision rule:**
   - Proceed with the variant that passes G1–G6 and shows the larger net
     removal.
   - If only A passes and B does not, weigh Durable's churn (F1).
   - If neither passes G1–G4, reject.

## 8. Effect on current work

The hosted transport's relay, ledger, gate, screening and retention (phase 4,
both review rounds) carry over to the candidate as the provider endpoint's
core. Only the container stream-frame layer would be replaced. Phase 6 (live
backend measurement and HTTPS) is needed under every option. Its
endpoint-facing parts could wait for this decision; its live measurements of
backend close timing and the `[DONE]` sentinel could not.
