# P0: sizing a Pi realization under the task host for H2

2026-11-17, revision 2 (review disposition at the end). **Read-only sizing. It decides nothing, accepts no
architecture, grants nothing and changes no code.** This is row P0 of the
[H2 route comparison](research-lifecycle-h2-route-comparison-20261117.md)
(sections 2, 3 and 5). Author: dev-vertical Pi session
`01a1e2a4-f353-743a-ba23-f42394958125`, after the operator's "proceed with
slice 4". Source read at `9280741b`; line numbers are from that commit. Line
counts below are estimates from reading the source, not measurements of a
prototype.

## Short answer

The P2 piece is smaller than the comparison feared, and most of it falls on the
Pi Host, not on `lifecycle-service`.

1. **The actor/runner boundary already exists.** `lifecycle-service` drives the
   native through a runner (`operations/mod.rs:157-850`) that talks to the
   lifecycle actor only through eight `NativeMessage` values (`:28-71`:
   `BindThread`, `PrepareTurn`, `BindTurn`, `Ingress`, `Tool`, `CaptureInput`,
   `Settle`, `Done`). The actor (`actor.rs:1289-1636`) owns turn claims, send
   intent, ingress recording, tool execution, input retention and settlement.
   About 8 000 lines of actor plus operations do **not** need to be mirrored.
   The Codex coupling is the runner's startup handshake, the harness type,
   two digest constants, the model constant, and the type carried by
   `CaptureInput`.
2. **Tools are executed by the actor, not by the native.** `snapshot_read`,
   `question_open` and `result_publish` arrive as `item/tool/call` server
   requests and the actor executes them against the store and the task owner
   (`actor.rs:2019-2320`). Under the task host, the Pi Host therefore must
   **relay** tool calls to its controller, not serve them as the hosted
   analysis profile does today. This is a new Host mode, not a reuse of
   `work-engine.pi-analysis-hosted.v1`'s tool serving.
3. **The store's native grammar and the delivery verifier are Codex
   app-server shaped.** Method names and JSON paths (`turn/completed`,
   `item/tool/call`, `params.callId`, `params.turn.status`) are in SQL and
   checks in `lifecycle-store` (`native_input.rs:136-157`,
   `native_causal.rs:94-104`, `native.rs:936-941,1026,2737,2834-2846,3043-3068`)
   and in the task host's `verify_delivery` (`research_delivery.rs:414-475`).
   The cheapest placement in code is for the **Pi Host to speak the required
   subset of that protocol** under its own protocol identity, so the store and
   the actor stay nearly unchanged. But the store owner has already drawn a
   Pi-versus-native boundary: `settle_native_turn` refuses every known Pi
   protocol and profile digest (`native.rs:3021-3037`). Placement A works only
   by minting a new Pi Host identity that passes that fence, which is the store
   owner's decision, not a free choice. A protocol-neutral grammar, or
   producing the consumer's records from the existing Pi grammar, is the
   alternative (B); it touches the most qualified owner and re-qualifies the
   Codex path.
4. **Estimate:** about 3 300-5 800 changed lines across five owners, including
   a composed offline qualification, on placement A (roughly 800-1 500 more if
   the store owner requires B). For scale, H2 slice 3 (the hosted analysis
   profile) was 2 670 lines in one lane. This keeps P2 inside the comparison's
   $50-120 range, most likely in its upper half because of multi-owner review
   and the composed qualification harness, which is the least certain piece.
5. **C1 consumption cannot be sized apart from turn mapping on this
   placement.** The causal record joins the native ingress row of the
   `result_publish` call and the actor's task-operation row
   (`native_causal.rs:39-125`). Those rows exist only if Pi turns run through
   the actor. The C1-specific work (capture artifact, checked Pi input value,
   projection change) is about 650-1 000 lines on top of turn mapping.

Nothing here changes the comparison's decision structure: R7′ (whether a Pi
realization under the task host is the widening the M4 exit excludes), R7″
(standing of the hosted path), R8′ (grant derivation) and the shared G4 work
remain where they were. Owner acceptance of a placement is required before any
code in `lifecycle-service`, `lifecycle-store`, `work-engine-task-host`,
`retained-task-workflow` or `research-register`.

## 1. What the task host consumes, and where Codex enters

| Consumer step | Code | Codex-specific? |
| --- | --- | --- |
| Task host launches `lifecycle-service` with a `NativeConfig` | `work-engine-task-host/src/lib.rs:104-120` (`ControlSelection::Native {}`), `:495-565` | Config is realization-neutral except `provider_route` and `chatgpt_access_source` |
| Runner spawns and drives the native | `lifecycle-service/src/operations/mod.rs:852-1090` (`run_native`), `:417-720` (`run`), `:739-850` (`run_turn`) | Yes: `NativeHarness` (bwrap, codex home), `initialize` user-agent check, `config/read` provider and policy validation, external ChatGPT login, `thread/start` with `dynamicTools`, `turn/start`, `thread/read` |
| Thread start request | `operations/mod.rs:99-155` | Model fixed at `NativeProviderRoute::MODEL` (`:138`); tool descriptions name the code-mode route |
| Session reservation | `actor.rs:1047-1048,1087,1321-1322` | Binds `NATIVE_BINARY_SHA256` and `SCHEMA_THREAD_START_SHA256` constants |
| Turn settlement | `lifecycle-store/src/native.rs:2998-3460` | Refuses sessions bound to any existing Pi protocol or profile digest (`:3021-3037`); requires a `thread/read` history response of a specific shape (section 3, H1) |
| Ingress classification | `actor.rs:1801-2017` | Classifies Codex methods and item types; unchanged if the native emits only qualified methods |
| Tool execution | `actor.rs:2019-2320` | Takes `lifecycle_codex::NativeToolCall`; logic is store and task-owner work |
| Input retention | `actor.rs:1506-1562`; `lifecycle-store/src/native_input.rs:88-280` | Message carries `lifecycle_codex::AcquiredNativeInput` (private construction); store requires a `turn/completed` terminal ingress row |
| Causal record | `lifecycle-store/src/native_causal.rs:39-210` | Joins the `item/tool/call` ingress row and the `result_publish` task-operation row; the record's scope literal names "direct/same-turn-code-mode" (`:105`) |
| Causal-read fence (the comparison's "preparation fence") | `actor.rs:2700-2760` (rejects `!input.eligible` at `:2726`); store refusals at `native_causal.rs:55-61,197-201` | No |
| Delivery verification | `work-engine-task-host/src/research_delivery.rs:42-124` (`acquire`), `:366-648` (`verify_delivery`) | Reads ingress `params.*`; `snapshot_projection` requires `namespace == "workspace"` (`:248`); payload `inference_call_id`/`physical_send`/`wire` |
| Startup hold | `lifecycle-service/src/native_startup.rs:113-137` | `OwnedNativeObservation` from `lifecycle-codex`; `publish` refuses any non-controlled observation on every route (`:122-123`) |
| Install predicate | `work-engine-task-host/src/research_installation.rs:1342-1363` | Requires `is_native()`, `OpenaiChatgptAccessLeaseV1` and access isolation |
| Admission association | `retained-task-workflow/src/research_admission.rs:442-490` | No: digests (`native_realization_digest`, `protection_evidence_digest`) and a `native-currentness-v1` artifact |

The Direct control (`ControlSelection::Direct`, `lifecycle-semantic/src/direct_control.rs:153-168`)
is a second realization under the task host and a precedent for a port owning
its evidence class, but it is tool-free semantic compile/verify and carries no
domain turns, so it is not a route for H2.

## 2. Placement alternatives

**A. Pi Host as a native speaking an app-server subset (recommended for
sizing).** The Pi Host gains a controller mode: a long-lived child process on
stdio that answers `initialize`, `thread/start`, `turn/start` and
`thread/read`, sends `item/tool/call` server requests, and emits `turn/started`,
`item/completed` (`agentMessage`, `dynamicToolCall`) and `turn/completed`. These
frames are what the Host actually emits, so `native_ingress.raw` stays truthful;
the session binds a Pi Host protocol digest and a Host/receiver/image identity
in place of the Codex constants. That identity must differ from the Pi digests
`settle_native_turn` refuses (`native.rs:3021-3037`), so A exists only if the
store owner accepts a Pi realization passing that fence. One store literal
would still be inaccurate: the causal record's scope names
"direct/same-turn-code-mode" (`native_causal.rs:105`); a Pi record would carry
it unless the owner changes that literal (a one-line store change). Pi's own RPC frames,
provider bodies and responses stay in the Host's retained artifacts and are
named by digest in the capture metadata. The store, the actor and the causal
record are unchanged. `lifecycle-service` needs a runner that is generic over
the harness (or a Pi runner beside the Codex one) and a realization identity.

**B. Protocol-neutral grammar, or consumer records from the Pi grammar.**
Either replace the Codex method and path literals in `lifecycle-store` and
`verify_delivery` with a realization-supplied grammar, or have the existing Pi
grammar (which the fence keeps separate, and whose analysis variant already
binds each tool result to a Host delivery) produce the input and causal
records the task host reads. Either respects the boundary the fence draws and
is cleaner if a third realization appears, but it changes the most qualified
owner and requires re-qualifying the Codex path. The extra size over A
(roughly 800-1 500 lines) is a judgment from the number of literal sites listed
in section 1, not an itemized estimate. A is simpler in code; whether it is
the right boundary is the store owner's call given the fence.

**C. Embed the Pi Host in-process** (implement the Host's `CodingEffectGate`
inside `lifecycle-service`). It would put provider credentials in the
lifecycle process (the Codex route keeps them in the native child) and bridge
the Host's blocking loop into the async actor; it would also make
`lifecycle-service` depend on `work-engine-manual-host` and its dependencies
(not sized). Not recommended.

## 3. Work items on placement A

| # | Item | Owner | Lines (est.) |
| --- | --- | --- | --- |
| H1 | Controller mode: app-server subset over stdio; one container per thread; one Pi prompt per turn. The settlement shape is fixed by the store: `turn/completed` with `params.turn.error` present and `null` (`native_input.rs:158-164`); `thread/read` answered as a Response with `historyMode: "legacy"`, one matching turn with `status: "completed"` and `itemsView: "full"`, at most 1 024 items, each `dynamicToolCall` item `completed` with `success: true`, matching arguments and `contentItems` equal to the actor's result, completed `agentMessage` items equal to their `item/completed` counterparts, exactly one carrying the final text (`native.rs:3137-3212,3351-3454`) | `work-engine-manual-host` (dev vertical) | 600-900 |
| H2 | Relayed tools: map `dynamicTools` to Pi custom tools (flat names; Pi sends no `namespace`), pin the tools digest from them, forward each Pi `tool_request` as `item/tool/call` with `threadId`, `turnId`, `callId`, `tool`, `arguments` and a Host-added `namespace` (`workspace` or `task`; without it the actor classifies the call unsupported, `actor.rs:1823-1852`), and relay the controller's result unchanged as the Pi tool text; receiver change | manual-host and receiver | 250-400 |
| H3 | Per-turn input capture: every physical send's exact body (`wire`, `identity`), `inference_call_id` = request, `physical_send` = attempt, metadata naming Host artifacts by digest, a calls bundle with the keys the store and verifier read (`call_id`, `inference_call_id`, `physical_send`, `namespace`, `tool`, `arguments`; `response_id` echoed; `lifecycle-codex/src/causal.rs:22-34`, `native_causal.rs:98-104`, `research_delivery.rs:453-482`), an outcome and an eligibility flag (completed terminal; every send's body retained with a matching digest; tools digest checked on every request; no Uncertain attempt). Complete when the Host emits the terminal, because the runner acquires input on the terminal frame before `thread/read` (`operations/mod.rs:797-813`); also produced, ineligible, for failed or interrupted turns (`:264-283`) | manual-host | 300-450 |
| H4 | H2 bounds inside the consumer limits: total payload bytes per turn at most 8 MiB (`native_input.rs:104-116`, `research_delivery.rs:15,495`), at most 64 physical sends, terminal at most 1 MiB; a Host deadline above today's 300 s job cap | manual-host | 50-100 |
| H5 | Docker protected-surface observation and currentness guard (container identity, running, `NetworkMode=none`, no `/auth`), exposed to the startup hold | manual-host | 150-250 |
| L1 | Runner generic over a harness (`receive`, `send_request`, `send_notification`, `send_result`, `deny_request`, `terminate_and_reap`, `entry_allowed`, `pid`, `captures_native_input`, `acquire_native_input`, `observe_protected_surface`; lease methods Codex-only), Codex startup handshake behind the Codex realization, `code_mode_host` off for Pi (it drives `verify_code_mode_host`, `ExpectedToolSet` and policy validation), per-realization event timeouts (today 20 s and 30 s, `operations/mod.rs:178,365,793`), successor refused (fresh start only, D7) | `lifecycle-service` | 300-500 |
| L2 | Realization identity: session binary and protocol digests, model, tool descriptions, provider route variant | `lifecycle-service`, `lifecycle-codex` constants | 150-250 |
| L3 | Checked Pi input value: a `lifecycle-pi` crate (or module) that verifies H3's artifact (digests, bounds, joins against the Host ledger) and builds the payloads, metadata, calls bundle and eligibility; `CaptureInput` carries a realization-neutral value | `lifecycle-service` plus new crate | 300-500 |
| L4 | Startup hold for a Pi observation and guard (today `OwnedNativeObservation` and `NativeCurrentnessGuard` from `lifecycle-codex`) | `lifecycle-service` | 200-400 |
| T1 | `snapshot_projection` and the publication checks accept the Pi request shape (no `namespace`, flat tool names) for a Pi realization | `work-engine-task-host` | 40-80 |
| T2 | Install predicate admits the Pi realization and its protection evidence | task host | 30-60 |
| T3 | Selected-startup composition and validation for a Pi `NativeConfig` | task host | 150-300 |
| Q | Composed offline qualification: task host, `lifecycle-service`, Pi Host and loopback provider, through a controlled research delivery and causal read | all, harness | 800-1 500 |

Totals: Host 1 350-2 100, service 950-1 650, task host 220-440,
qualification 800-1 500; about 3 300-5 800 lines. On placement A, no change is
expected in `research-register` or `retained-task-workflow` (subject to the
owner's reading of `native-currentness-v1` for Docker evidence), and
`lifecycle-store` changes at most the causal scope literal, **provided the
store owner accepts a new Pi Host identity past the `settle_native_turn` Pi
fence**. If the owner requires B instead, add roughly 800-1 500 lines in
`lifecycle-store` and the verifier plus Codex re-qualification.

The comparison asked for four parts sized separately:

| Part | Items | Lines (est.) |
| --- | --- | --- |
| Turn mapping | H1, H2, L1, L2 | 1 300-2 050 |
| C1 consumption | H3, H4, L3, T1 (needs turn mapping: see short answer item 5) | 690-1 130 |
| Fence join | Pi eligibility semantics in H3/L3; the causal-read fence itself (`actor.rs:2726`) is unchanged | owner decision, no separate code |
| Install predicate | H5, L4, T2, T3 | 530-1 010 |

## 4. Facts checked against Pi's actual requests

From a retained offline analysis run's request bodies
(`~/.local/state/work-engine/dev-vertical-h2-route-20261117/a-dev/read-all/state/coding-hosted-*.body.json`):
Pi declares its tools in an `additional_tools` developer input item;
`function_call` items carry `call_id`, `name` and `arguments` with no
`namespace`; `function_call_output.output` is the tool's text. The request is
stateless (`store: false`, no `previous_response_id`, which `verify_delivery`
requires at `:568-580`) and resends the whole history, so the request that
produces `result_publish` contains every earlier snapshot read, as
`verify_delivery`'s unique-visibility check needs. Today a Pi request would be
refused at that visibility check: `snapshot_projection` skips namespace-less
calls (`:248`) and returns nothing, `verify_delivery` falls back to
`text_projection` (`:593-602`), and the request, card and reconstruction appear
there only JSON-escaped inside tool output, so they are not uniquely visible
(`:617-626`). T1 (recognizing the Pi tool identity) is the remedy, provided the
Host returns the actor's tool text unchanged as the Pi tool result.

Bounds for H2: at about 15 snapshot reads of up to 64 KiB, per-turn payload
total is estimated at about 1.5 MB, well inside 8 MiB. The analysis profile's
own maxima (32 requests of up to 600 kB) could exceed it, so H4 sets the H2
bound from the consumer limit rather than from the profile.

## 5. Risks and unknowns

- **Composed qualification (Q).** No harness drives task host, lifecycle
  service and an external native together offline except the Codex
  controlled-proof tests. This is the least certain item.
- **Startup publication is refused on every route** outside controlled
  qualification (`native_startup.rs:122-123`). A Pi observation does not lift
  that by itself; it needs the protection owner's contract (R9′), as on Codex.
- **G4 is unchanged and shared**: production basis verifier, Preparer/Adopter
  installation and dispatch beyond `research.invalidate.v1`.
- **Grant derivation (R8′).** On A the Host would enforce a per-turn attempt
  and budget fence derived from the lifecycle `native_turn` grant passed at turn
  start; the Host's own Pi coding grammar in `lifecycle-store` would not be used.
  That is the "derived fence" option and needs the operator's decision.
- **Event parsing location.** The Pi runner can reuse `lifecycle_codex::parse_event`
  on Host frames, or the owner may move `NativeEvent` to a neutral crate.
- **Protocol drift.** A Host speaking an app-server subset couples to the
  methods and paths above. The coupling is the same set the store and verifier
  already pin, and the Host's protocol digest makes the subset explicit.

## 6. What this asks of whom

| Who | Question |
| --- | --- |
| Operator | R7′, R7″ and R8′ as in the comparison; this sizing does not answer them |
| Research-lifecycle owner (`lifecycle-service`, task host) | Accept or reject placement A (or choose B); realization identity in session reservation; Pi eligibility semantics for the preparation fence; the T1 projection change |
| Lifecycle-store owner | Accept a new Pi Host identity that passes native settlement despite the existing Pi-digest fence (`native.rs:3021-3037`), and rule on the causal scope literal (`native_causal.rs:105`); or require B |
| Protection owner | Whether Docker inspection with Host-only credentials is acceptable protected-set evidence (R9′, D2′) |
| Dev vertical (this lane) | H1-H5 after placement acceptance. H3's capture artifact is useful on every Pi option, including R1(c); it is the one item within this lane's own authority that could start earlier, and only if the operator wants it |

## Evidence used

Source at `9280741b`: `lifecycle-service/src/{lib.rs, operations/mod.rs,
actor.rs, native_startup.rs}`; `lifecycle-store/src/{native_input.rs,
native_causal.rs, native.rs}` (method literals); `lifecycle-codex/src/{transport.rs,
compiled_input.rs, causal.rs}` (public surface); `work-engine-task-host/src/{lib.rs,
research_delivery.rs, research_installation.rs, selected_startup.rs}`;
`retained-task-workflow/src/research_admission.rs:425-500`;
`lifecycle-semantic/src/direct_control.rs`; `work-engine-manual-host/src/{pi_coding.rs,
pi_coding_hosted.rs, pi_coding_analysis.rs}`; crate dependency lists; retained
offline request bodies named in section 4. Not done: no build, test, prototype,
inference or code change.

## Review disposition

Review: [Fable on the Claude subscription](../reviews/h2-pi-realization-sizing-fable-20261117/REVIEW.md)
(changes required; estimates, the C1 conclusion, request-shape facts and
authority framing otherwise confirmed).

| Finding | Change |
| --- | --- |
| P1-1 `settle_native_turn` refuses all Pi digests | Short answer item 3, section 1 row, section 2 A and B, totals and the store-owner question now state the fence and make A conditional on the store owner accepting a new Pi Host identity past it; B now includes producing the consumer records from the existing Pi grammar |
| P1-2 bundle and tool-call keys | H2 states the Host adds `namespace` and the frame fields; H3 names the bundle keys the store and verifier read |
| P2-1 settlement shape | H1 lists the `thread/read` shape and `"error": null` |
| P2-2 harness surface, capture timing | L1 trait list extended, `code_mode_host` off; H3 complete at the terminal and produced for failed turns |
| P2-3 code-mode scope literal | Section 1 row and section 2 A; store-owner question |
| P2-4 failure stage | Section 4 corrected |
| P3-1 fence label | "Causal-read fence" with the store refusals |
| P3-2 B and C unsized | B's figure labelled a judgment; C's dependency claim marked not sized |
| P3-3 unverifiable figures | Slice 3 size is from `git show --stat 1051e660` (2 669 insertions, 91 deletions); no change |
| Authority note on H3 | Section 6 now says H3 could start earlier only within this lane's authority and if the operator wants it |
