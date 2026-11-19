# OpenRouter lane: host DeepSeek V4 Flash (`:floor`) on the Pi Host

2026-11-19. Dev-vertical slice on branch `openrouter-lane-20261119`, cut from
`df5ec90f` (the shared head with the relay-500k and O7 work merged). Status:
design, not yet implemented. Owned for the research lane / Pi Host dev vertical.
The goal behind the lane: put the current session's model
(`~deepseek/deepseek-v4-flash-latest:floor`, provider `openrouter-floor`) behind
the Pi Host's credential custody and isolation, so the E1a builder battery can
run a DeepSeek arm against the same battery that measured Sonnet/Opus/Haiku/Terra.

## Operator decisions (2026-11-19)

| Who | Decision |
| --- | --- |
| Operator | Use OpenRouter `:floor` routing: same model (`deepseek-v4-flash-latest`), cheapest available provider via the `:floor` suffix in the request `model` field. Effort `high`. One run now; three if the first is clean. No hard budget needed (operator monitors OpenRouter spend externally); keep `budget_microusd` as a generous safety bound because it is an architectural invariant. Billing: OpenRouter meter (the `~deepseek` / `openrouter-floor` key). |
| Operator | The container must run Pi **plugins**, including the process-management plugin used in this instance (default Pi process management is poor). Provider `openrouter-floor` and model `~deepseek/deepseek-v4-flash-latest:floor` resolve only through the `pi-openrouter-floor` plugin. |

## Plugin support (new scope)

The hosted receiver builds a `ModelRuntime` directly with `noExtensions: true`; `resourceLoader` sets `noExtensions:true, disabledBuiltinExtensions:['mcp'], noSkills:true, noThemes:true, noContextFiles:true`. Supporting plugins in the container is genuinely new. The established, working precedent is the `claude-sdk` probe
(`records/dev-vertical-h2-route-20261117/pi-claude-probe/forwarder.mjs`): extension packages mounted read-only at `/ext` and given to Pi via
`-e /ext/node_modules/<pkg>`; Pi registers the provider from the extension at load.

- **`pi-openrouter-floor`** must be carried: it is what makes provider `openrouter-floor` and model `~deepseek/deepseek-v4-flash-latest:floor` exist (the extension clones the OpenRouter catalog at startup and registers `openrouter-floor` with `api=openai-completions`, `baseUrl=https://openrouter.ai/api/v1`).
- **Process-management plugin**: `pi-better-background-tasks` 0.18.0 is the installed package (`settings.json` `packages`; npm r0.18.0). `pi-processes 0.4.3` survives only as a `settings.json` `extensionChangelogVersions` entry and a stale jiti cache (`process-manager`/`process-panel`/`process-widget`), not as an installed package — ruling on whether it is the intended one is pending (below).
- The receiver then needs a **`chat/completions` forwarder** (an `openai-completions` counterpart to the claude-sdk `forwarder.mjs` loopback listener), plus the plugin's provider transport wired to the Host's framed bridge.

Open item / decision for the operator:
- Confirm whether the process plugin to carry is `pi-better-background-tasks` (installed) or a `pi-processes` package to locate. The claude-sdk `-e /ext/node_modules/<pkg>` mounting mechanism is the agreed prototype; the decision is which packages go into `/ext`.

## What the lane is

A third provider wire family in `work-engine-manual-host`, alongside `Wire::Responses`
(openai-codex / OpenAI API) and `Wire::Messages` (claude-sdk / Anthropic API):
**`Wire::Completions`** — OpenAI `chat/completions` over HTTPS SSE, the format
Pi's `openrouter` / `openrouter-floor` providers use. Only the OpenRouter origin
and api key are admitted. It preserves the existing single shared attempt
ledger, funding failover, retention, screening and per-turn bounds.

## Confirmed facts (read 2026-11-19)

- Floor routing is a request-time directive to OpenRouter: `model =
  "deepseek/deepseek-v4-flash-latest:floor"` makes OpenRouter sort the model's
  providers by price (incl. flex-tier). It is not a separate API option, and
  it is *not* a distinct model. Superseded by the **three-layer identity
  contract** in Revision 2: OpenRouter resolves the `latest:floor` slug to a
  concrete checkpoint (e.g. `deepseek-v4-flash-0731`), so the request slug and
  the resolved `backendModel` differ. Identity equality holds **within** a layer,
  not **between** the transmitted slug (Layer B) and the resolved checkpoint
  (Layer C). See "P1 — Model identity: separate the three layers".
- DeepSeek V4 Flash is not in the `pi-openrouter-floor` `:floor` twin list from
  the live catalog alone; the model resolves in Pi's catalog as
  `~deepseek/deepseek-v4-flash-latest` (alias id).
- `openrouter.json` provider data ships in the runtime at
  `node_modules/@earendil-works/pi-ai/dist/providers/data/openrouter.json`
  and has `openai-completions` entries for `~deepseek/deepseek-v4-flash-latest`
  (api `openai-completions`, provider `openrouter`, base `https://openrouter.ai/api/v1`,
  cost input 0.07 / output 0.5 / cache-read 0.008 / cache-write 0, window 1M,
  max out 943 718, reasoning true). Caveat: `pi --list-models` shows a different
  price (0.05/0.30/0.035) than the pinned catalog row (0.07/0.5/0.008); the
  pinned catalog row is authoritative for wallet-style reservations, while the
  effective charge may change with floor routing. The lane records both.

## Design (touch points, all in `rust/crates/work-engine-manual-host`)

### 1. `Wire::Completions` (`pi_stream.rs`, `pi_coding_hosted.rs`, `pi_coding.rs`)
Chat/completions SSE: `data:` deltas, terminal `data: [DONE]` (no typed
`response.completed`/`message_stop`), usage from the final chunk/finish
(OpenRouter reports provider-billed cost in the stream). Add `terminal_event`,
`terminal_is_last_for`, `usage_for` handling.

### 2. `Endpoint::OpenRouter` (`pi_coding_hosted.rs`)
- `parse` maps `"https:openrouter-completions"` (or `https:openrouter`).
- `host()` → `openrouter.ai`; `target()` → `POST /api/v1/chat/completions`.
- `serves(Wire::Completions)`.
- `forwarded()` allowlist for chat/completions (accept, content-type,
  user-agent; drop billing/session-context headers).

### 3. Credential custody (`pi_coding_https.rs`)
- `CredentialSource::{OpenRouterApiKeyEnv, OpenRouterApiKeyFile}` (an `sk-or-`
  key), `Credential::OpenRouterApiKey`, `endpoint()` → `Endpoint::OpenRouter`,
  auth `Authorization: Bearer <key>`. Never enters the container or journal.

### 4. Body policy (`pi_coding_https.rs` request validation)
Chat/completions exact carry which fields from the container (model, messages,
stream, tools as `function`/`custom`, tool_choice, temperature, max_tokens,
reasoning/thinking params OpenRouter uses for DeepSeek). Allowlist and ship only
those; refuse everything else and refuse `service_tier`. Note: Pi's openrouter
uses `openai-completions`; the frame/body the container presents must be
validated against this policy before forwarding, and the model id pinned to
`deepseek/deepseek-v4-flash-latest:floor`.

### 5. Provider plan lane (`pi_coding_provider.rs`)
- `wire()` adds `"openrouter" => Wire::Completions`.
- `selected()` binds the OpenRouter catalog at
  `node_modules/@earendil-works/pi-ai/dist/providers/data/openrouter.json`,
  openai-completions family.
- `request_reservation()` a completions full-catalog reservation.
- `CLAUDE_MODELS`-style allowlist for the completions family (the pinned
  DeepSeek model id), and a completion-family analysis budget constant.
- `validate()` extended for the new wire/catalog/budget/context bounds.
- `native_model_matches()` for the openrouter provider/api.

### 6. Receiver routing
Point the in-container Pi's `openrouter-floor`/`openrouter` provider transport
at the Host's framed bridge (the fetch-interception path, not the Claude-CLI
path), so provider traffic is Host-served. Container holds a dummy key/base;
the Host injects real access only within an admitted dispatch.

### 7. Catalog / pricing / receipts
- The OpenRouter catalog artifact `openrouter.json` is pinned like the other
  family catalogs; full-catalog reservation for `budget_microusd`.
- Receipts record pinned-catalog cost and, separately, the provider-reported
  cost OpenRouter returns in the stream (floor may change the effective rate).
- A DeepSeek price/rate row for `usage_ledger.py` (hi: high-level, this is an
  OpenRouter meter; ledger entry is for Work Engine's own cost records).

## Sequence

1. **Offline-qualifiable core** (this branch): Wire::Completions, Endpoint,
   credential, body policy, catalog binding, provider-plan lane, mock
   completions upstream fixture, refusal cases, unit tests (`pi_coding_https`,
   `pi_coding_provider`, `pi_stream`). No live inference. Cargo builds + tests.
2. **Fable review** of the lane (credential-custody and fail-closed changes).
3. **Live qualification** on a pinned deepseek id (offline H2 treatment or a
   small non-battery task), then the **E1a builder battery** DeepSeek arm(s):
   `arm_run.py prepare` (E1a fixed inputs) → Pi Host coding job pinned to the
   floor model → commit + `completion.json` → F3.1 counts-only scoring.

## Acceptance (this slice)
- Exact model id pinned; OpenRouter floor routing exercised; fee/identity both
  verified in qualification (backend model echo).
- No regression in Responses/Messages lanes (existing tests pass).
- The lane is reachable from a Pi Host coding job (offline fixture, then live).

## Open items / decisions to confirm while building
- Exact pinned model id string: `deepseek/deepseek-v4-flash-latest:floor`
  (host-pinned) vs the catalog alias `~deepseek/deepseek-v4-flash-latest`.
  Qualify which OpenRouter accepts/echoes; align the catalog entry and the pin.
- Whether `:floor` is present in the pinned catalog as a `chat:` key or must be
  derived from the base entry (the Host's `selected()` needs an exact
  `chat:<model>` key; the catalog today has `chat:~deepseek/deepseek-v4-flash-latest`
  but no `:floor` variant — floor is request-time, so the pin likely binds the
  base model id and the `:floor` suffix is added at dispatch).
- `budget_microusd` safety value (proposal: ~ $5) given floor cost variability.
- Process plugin to carry: **`pi-better-background-tasks` 0.18.0** (installed),
  not the stale `pi-processes` changelog entry. Operator confirmed 2026-11-19.

## Revision 2 — review findings and their dispositions (2026-11-19)

Independent review of Revision 1. Verdict: accept direction, request changes.
Scores: 7.5 -> 8.0 / 10. This section records each finding and the concrete,
code-grounded disposition. This lane is not authorized to run live until the
audited dispositions are met and the offline fixture + independent review are
done.

### P1 — Extension tools must remain under deterministic Host control

Finding: the process-management plugin may register tools that spawn/kill
processes, creating a second execution path around the Host's tool gate.

Facts (read 2026-11-19): in the hosted receiver every side-effecting tool is a
`customTool` whose `execute` does `emit({type:'tool_request'|'claims_request'|'snapshot_request', ...})` and returns only once the Host answers; `noTools:'builtin'`, `tools:[activeNames]`. Tool approval is Host-mediated, not extension-mediation.

Disposition: any extension tool is opted in per name in `activeNames`, and
every side-effecting extension tool must be routed through the Host's existing
`tool_request` grant or be excluded from `customTools`. `pi-better-background-tasks`
will be loaded with its tools **excluded** for the E1a arm (a completed arm has no
need to spawn durable processes); qualification includes a **negative test** that
a process-management tool cannot start an unapproved command (grant withheld ->
tool errors / not started), mirroring the existing workspace_exec gate.

### P1 — Embedded plugin loading is the integration contract (not the CLI child)

The earlier probe launched the Pi CLI as a child with `-e /ext/...`; the hosted
receiver instead constructs an embedded SDK session.

Facts (read 2026-11-19): `receiver/pi-coding.mjs:404` already loads a pinned
claude-sdk extension through the embedded factory:
`createAgentSessionServices({...resourceLoaderOptions:{..., additionalExtensionPaths:[EXTENSION]}})`
where `EXTENSION='/pi/node_modules/pi-claude-subscription'`. This is the
established embedded loading path; the CLI `-e` child was only the probe.

Disposition: the OpenRouter lane carries the approved extensions the same way:
`additionalExtensionPaths:[OPENROUTER_FLOOR_PATH, PBBT_PATH]`, each mounted
read-only at `/ext/node_modules/<pkg>` and **pinned by package version and
sha256 digest** (the Job's immutable runtime tree already pins digests). The
receiver verifies each loaded extension's identity; any additional extension
is refused. The catalog (from the live OpenRouter JSON, cloned at extension
start) is bound like the other family catalogs.

### P1 — Model identity: separate the three layers

Finding: the plan inconsistently mixed the Pi provider id, the native model id,
the transmitted OpenRouter slug, and the resolved checkpoint. OpenRouter
reports `deepseek-v4-flash-0731` for a `latest:floor` request.

Facts: `provider_observation` currently refuses `backendModel != plan.model`
(pi_coding_provider.rs), and `frame.model != plan.model` refusal is in
`provider_request`. These strict equalities are correct **within** a layer but
wrong **between** layers.

Disposition:
- Layer A (Pi catalog id): `~deepseek/deepseek-v4-flash-latest`.
- Layer B (transmitted model slug): `deepseek/deepseek-v4-flash-latest:floor`
  (base id + `:floor`, appended at dispatch per operator).
- Layer C (resolved checkpoint): OpenRouter's reported `model` (e.g.
  `deepseek/deepseek-v4-flash-0731`).
The Host pins and strictly checks Layer B for the request body it sends
(`request_model == plan transmitted slug`). `backend_model` from the response
is **recorded** as the resolved checkpoint (Layer C) and must belong to the
pinned model's known floor family; it is **not** required to equal the
request slug. The `provider_observation` equality is relaxed only for the
Completions family and only between B and C; requests still require the exact
catalog pin, and the receipts carry all three ids.

### P2 — Accounting and streaming specifics

Finding: a full-catalog reservation at the pinned prices is not trivial; a ~$5
bound cannot admit 32 fully-reserved requests; `usage`/`stream_options` and the
`high` reasoning carrier were not in the body policy.

Disposition:
- Reservation: completions full-reservation uses the pinned catalog cost
  (input/output/cache) times the max context (1M) and max output (943 718) for
the plan's max_requests, computed by the same full_reservation math as
Responses/Messages. `budget_microusd` for the OpenRouter analysis profile is
its own named constant large enough to admit max_requests full reservations
(as MESSAGES_ANALYSIS_BUDGET does for Messages), not a hard external cap; the
operator monitors spend externally. A live lane that cannot admit its own
max_requests full reservations fails validation, as today.
- Body allowlist keeps `stream:true`, `stream_options:{include_usage:true}`,
  and the reasoning field OpenRouter expects for `high` (DeepSeek
  `thinkingLevelMap high->"high"`), so the `high` effort set in providerOptions
  is verified in the transmitted body (a test asserts it, not just the plan).
- Settlement: usage from the terminal chunk; uncertain reservations retained
  when terminal usage is missing, as Messages does.
- A test records the actual Pi-generated request body to confirm the required
  fields reach OpenRouter.

### P2 — Process lifetime scope

Finding: background processes spawned by the plugin in an ephemeral container
need a lifetime decision.

Disposition: processes are **scoped to one Docker execution**. The container's
PID 1 exit ends every process (a documented property of the pinned image), so a
process the plugin starts cannot outlive the arm. Durable cross-execution work
remains the outer control plane's job; the plugin in an arm does not claim it.
Qualification asserts container exit kills the process tree (existing
`pi-docker` custody already observes this), and the E1a arm carries no durable
task.

### Required changes (from review) — status

1. Embedded plugin loading spec (versions+digests, SDK binding, provider proof) - DISPOSED above; to be written as an offline-fixture requirement.
2. Tool authorization preserved (negative test: unapproved process tool start refused) - DISPOSED; offline negative fixture.
3. Correct model identity contract, tested - DISPOSED; offline identity tests + live qualification.
4. Inference contract: high reasoning, body fields, SSE termination, usage/cost settlement, sufficient reservation bounds - DISPOSED; offline tests.
5. Full execution lifecycle: offline positive/negative fixtures, plugin process cleanup, live qualification, E1a battery scoring unchanged - SEQUENCE in this plan.

Remaining before authorization to run live: implement offline-qualifiable core
with the dispositions above, add mock-completions fixture + positive/negative
refusal cases + the identity/body/budget tests, cargo build + test green, and
the plan author's self-review before an independent review of the slice.

## Revision 3 — receiver plugin finding (2026-11-19, after commit e44d7f0e)

Pinned plugin packages (live checkout v the container's `/ext`):
- `pi-openrouter-floor` 1.0.2
  (`/home/bline/.pi/agent/git/github.com/herijooj/pi-openrouter-floor`, git).
- `pi-better-background-tasks` 0.18.0
  (`/home/bline/.pi/agent/npm/node_modules/pi-better-background-tasks`, npm).
The Job's immutable runtime/manifest pins digests, so `/ext` copies carry
version+digest identity exactly like the claude-sdk extension.

Finding (read 2026-11-19): `pi-better-background-tasks` is a full `ExtensionAPI`
integration, not a passive provider. `src/index.ts` calls `pi.registerTool`
(so its tools land in Pi's own tool registry), subscribes to `before_agent_start`
and `session_start`, and `src/runtime.ts` `spawnCommand`s real child processes
and may host a runtime server. That is a second execution path around the Host's
`customTool`-`tool_request` gate and the container's networkless/read-only/-no
long-lived-daemon model.

Disposition (unchanged in intent, now with the concrete mechanism): the
process-management plugin is **functional in the arm**: its background
spawn/status/log/cancel/async-completion capabilities are exposed to Pi, but
**each useful operation is mediated by the Host** through the same gate as
`workspace_exec`. The receiver does not let the plugin's in-process
`spawnCommand`/server run directly; it presents the plugin's tools as
`customTool`s whose `execute` `emit(tool_request)` and returns only on Host
approval.

Receiver changes needed (next) — **corrected 2026-11-19**:

Key correction: `pi-openrouter-floor` fetches OpenRouter's live `/models` catalog
at pi startup and **silently skips registration when the network is unavailable**
(a documented property of the extension). The Host's container is `--network
none`, so the extension can NOT be what registers `openrouter-floor` in the
container; neither can `additionalExtensionPaths:[EXTENSION]` by itself make the
model resolvable. Instead:

- The Host **stages a `/config/models.json`** catalog with the pinned
  `openrouter-floor` entry (`~deepseek/deepseek-v4-flash-latest`,
  `api=openai-completions`, baseUrl -> the in-container completions forwarder),
  exactly as the OpenAI lane is resolved off `modelsPath` by
  `getModel('openai-codex', config.model)`.
- The receiver adds a **chat/completions loopback forwarder** (the claude-sdk
  `ANTHROPIC_BASE_URL -> forwarder` pattern, ported to `POST
  /v1/chat/completions`): at most one request per grant, only while a grant is
  open, else refused locally; each admitted request becomes one `provider_fetch`
  under the grant's stream identity; the Host validates policy + dispatches with
  real access (Rust core already has this endpoint/custody). The staged
  baseUrl points Pi's `openrouter-floor` provider at the forwarder.
- The receiver resolves the model off `createAgentSessionServices().modelRuntime`
  by `getModel('openrouter-floor', config.model)` and rejects when absent or
  not the pinned one — mirroring both `getModel('openai-codex',...)` and the
  claude-sdk check.
- `pi-openrouter-floor` is still mounted/pinned (its model definitions set the
  openai-completions api + reasoning map Pi needs), but its live-catalog fetch
  must be seedable/bypassed; the staged models.json is authoritative.

Model-id convention (settled 2026-11-19, operator): **the wire model always
carries the `:floor` suffix, appended at dispatch.** Option B. The plan and
catalog bind Layer A (`~deepseek/deepseek-v4-flash-latest`, `plan.model`,
`selected()` key, reservation); `transmitted_model()` derives Layer B
(`deepseek/deepseek-v4-flash-latest:floor`), which `completions_policy`
requires. `:floor` is appended to the **model**, not to a separate provider.

Provider resolution (probed against the real Pi SDK 2026-11-19):
- `openrouter` / `~deepseek/deepseek-v4-flash-latest` -> resolves OFFLINE (Pi
  ships `openrouter.json`; api=openai-completions, base openrouter.ai/api/v1).
- `openrouter-floor` / `deepseek/deepseek-v4-flash-latest:floor` -> NOT
  resolvable offline; that provider only exists from the extension's live
  `/models` fetch, which skips silently when networkless.
Therefore the in-container receiver runs the **built-in `openrouter`** provider
(no staged models.json, no extension fetch), `ProviderPlan.provider="openrouter"`
-> `Wire::Completions`, and `:floor` is appended to the model at dispatch.
`openrouter-floor` is how the operator's free-standing Pi names floor routing;
in the Host, `openrouter` provider + `:floor` model is the same route.

The OpenRouter provider is `openai-completions`, so the receiver routes it
through the **OpenAI-lane `options.fetch` override** (emit `provider_fetch`),
NOT the claude-sdk forwarder; no new loopback server. The Host validates
policy + dispatches with the real key (Rust core). `pi-openrouter-floor` is
still pinned/mounted for its model definitions/reasoning map but its live fetch
is not the registration path.
- `pi-better-background-tasks` is mounted/pinned and its tools are exposed
  **through the Host gate**: every side-effecting extension tool is a
  `customTool` whose `execute` `emit(tool_request)` and returns only on Host
  approval; negative test that an unapproved process start is refused. Its own
  in-process `spawnCommand`/server is not exposed directly to Pi.

This is the largest remaining offline piece and the one the P1 review calls
out; it will get its own offline fixture + independent review before any live
run.

## Revision 4 — shell/background capability authorization (operator, 2026-11-19)

Operator decision applied to this lane. Work Engine authorizes **capabilities
at admission**, not per function within an already-authorized capability.

Capabilities (bound to the admitted execution authority, never model-selected):
- `allow_shell`: command execution within the admitted Docker execution.
- `allow_background`: managed asynchronous execution (process spawning,
  concurrent tasks, status/log inspection, cancellation, completion
  notifications, waits).

Contract matrix:
| allow_shell | allow_background | behavior |
|---|---|---|
| false | false | no shell or background tools |
| true | false | foreground shell only; background plugin NOT loaded |
| true | true | shell + the full approved background plugin |
| false | true | INVALID; refuse admission |
Deterministic dependency: `allow_background => allow_shell`.

Plugin integration when both true:
- Load version-pinned `pi-better-background-tasks` 0.18.0 via Pi's existing
  embedded `additionalExtensionPaths` mechanism.
- Expose its normal tools (bg_task_spawn/watch/status/log/stop/list, bg_task,
  bg_status, bg_task_spawn_batch, wait_for_operations, acknowledge_operation,
  bg_task_wait_cancel/list) incl. completion notifications.
- Let the plugin manage its own internal processes WITHOUT routing all 13
  tools through Host `tool_request` one-by-one. Preserve its own supervision,
  logging, status, cancellation, async-notification. Do NOT recreate a process
  manager. (Supersedes Revision 3's per-tool `tool_request` disposition.)
- Keep the plugin and its process service confined to the admitted Docker
  execution boundary.
- When `allow_background` is false, do not load/expose the plugin.

Security boundary: grants process execution only inside the existing sandbox;
no FS beyond the workspace, no network/remote SSH, no Host credentials or
privileged Host interfaces, no execution beyond deadline/container lifetime,
no authority over processes outside the admitted execution. If the plugin
needs a Host-side service beyond the boundary, that is an ARCHITECTURAL
BLOCKER to report, not something to bypass. No new per-tool authorization
machinery unless a demonstrated security requirement demands it.

Semantic distinction (do not overclaim): `allow_background=false` withholds
managed background-task capability; it does NOT promise a universal
prohibition of shell-level background syntax, which may still exist per the
shell's own policy.

Home in the code (per "inspect current contracts"):
- `Admission` struct (`pi_coding_provider.rs`, schema
  `work-engine.pi-coding-runtime-admission.v1`) currently carries
  `allow_provider`. Add `allow_shell` and `allow_background` there, validated
  in `ProviderPlan::validate` (refuse the invalid `(false,true)`; enforce
  `allow_background => allow_shell`), defaulting to preserve existing runs.
- `CodingEffectGate` is the Host execution boundary; foreground shell
  (workspace_exec) already goes through it. Background stays inside the
  Docker boundary the same way.
- The receiver loads the plugin only when both are true, via the existing
  embedded `additionalExtensionPaths` mechanism (mounted /ext, version+digest
  pinned).

Offline acceptance tests (required before live):
1. All four combos incl. refusal of the invalid one.
2. Plugin absent when allow_background false.
3. Plugin loaded with normal tools when both true.
4. Two authorized background commands run concurrently without blocking Pi.
5. Status, logs, cancellation, completion notifications work.
6. Background cannot escape FS/network/credential/lifetime boundaries.
7. Existing foreground shell + provider lanes unaffected.

No live inference / E1a until the offline implementation + these tests pass
the independent Fable xhigh (provider `claude-sdk`) review.
## Revision 5 — review remediation status + migration note (2026-11-19)

Code-review (DeepSeek) findings at fe6e7f14, dispositioned and committed:

| Finding | Commit | Status |
|---|---|---|
| P1 Host allow_shell enforced on tool_request (no receiver bypass) | ab79b475 | done; test |
| P1 Background tools in activeNames + Host readiness surface | ab79b475 | done; BG_TOOLS shared |
| P1 OpenRouter URL {base}/chat/completions (not doubled /api/v1) | ab79b475 | done; test |
| P1 Budget ceilings admit realistic multi-turn plans (\$4 coding / \$20 analysis) | ab79b475 | done; test |
| P2 Resolved checkpoint must belong to admitted model family | e2353906 | done; test |
| P2 Analysis/relay lanes reject shell+background at admission | acc4fd5d | done; test |

Serde-default migration note: the new capability fields default to `false` via
`#[serde(default)]`, so a previously-admitted plan without them loses shell
access. That is the secure default and, for this dev-vertical lane, there are no
production plans that carry the pre-Revision-4 authority; no migration is
required. If an old authority is ever replayed with execution expected, it must
be re-admitted with `allow_shell`/`allow_background` explicitly. The operator
monitors live spend separately; these ceilings are finite Host limits.

Remaining before independent Fable xhigh review:
- Background-execution boundary: qualify whether pi-better-background-tasks'
  managed service can run inside the Docker/workspace boundary or is an
  architectural blocker (report, do not bypass).
- Offline acceptance tests (7 from Revision 4), clippy -D clean, full
  integration suite, self-review.

## Revision 6 — background execution boundary: architectural blocker (2026-11-19)

Investigated the installed pi-better-background-tasks 0.18.0 (`~/.pi/agent/npm/
node_modules/pi-better-background-tasks`). Its mechanical execution is NOT a
passive in-process tool set. It carries two execution routes:

1. In-process spawn: `bg_task_*` tools call `spawnTask`/`spawnCommand`
   (`src/runtime.ts`) in the same process tree as Pi. These run inside whichever
   container holds Pi (the inference container), which has NO mounted writable
   coding workspace; the workspace lives in the separate coding-tool container.
   So even the in-process route does not execute in the `workspace_exec`
   environment by default.
2. Managed/durable/delivery paths (`requestBackgroundTool`,
   `managed-client.ts`) connect over a Unix socket
   (`$serverDir/pi-server.sock`, from `server-registration-v1.json`) to a
   SEPARATELY-STAGED LONG-LIVED DAEMON: `runtime-client/src/runtime/client.mjs`
   `spawn(runtime.node, [runtime.entrypoint, "--agent","pi"],
   {detached:true, ...})` installing its own private Node runtime
   (`stageRuntime`), daemon owner, and server. This is a Host-side process
   service outside the Docker execution boundary.

CONCLUSION — ARCHITECTURAL BLOCKER (report, do not bypass): loading the plugin in
the Pi container exposes tools that, for their full lifecycle, require a
separate detached Host daemon over a Unix socket. Wiring that socket/daemon would
create the unrestricted Host-side execution route the operator and review both
explicitly prohibited, and running the plugin's in-process spawn in the inference
container does not share the coding workspace boundary.

DISPOSITION for this lane (offline): keep `allow_background` admit/load gating
(Revision 4/5) but, in the arm, expose the plugin's tools only where their
mechanical execution can run inside the authorized workspace boundary. Since the
plugin's service placement is unresolved and would expand the execution surface
beyond the coding tool container, the lane raises background to a documented
blocker pending a boundary design decision (e.g. a workspace-bounded relay or a
dedicated process host). No live E1a arm exercises background until that is
resolved. `allow_background=false` remains the default, secure posture.

## Revision 7 — background deferred; foreground-only lane for review (2026-11-19)

Operator direction:
- Keep `allow_background=false` for the OpenRouter E1a lane.
- Add an explicit RUNTIME refusal if background is requested until its execution
  boundary is implemented (done: `background_gate` in `ProviderPlan::validate`
  refuses any admission with `allow_background=true`, reason names the pending
  boundary; `background_admission_is_refused_until_implemented` test, 126 lib
  tests).
- Defer the Revision-4 background-dependent acceptance tests (items 2, 3, 4, 5
  -- plugin loaded with normal tools when both true; two concurrent background
  commands non-blocking; status/logs/cancel/notifications; no boundary escape for
  background) into a SEPARATE CAPABILITY QUALIFICATION, NOT reported as passing
  and NOT waived. They are recorded as `deferred` pending the workspace-bounded
  process-host implementation. Items 1, 6 (foreground), 7 remain valid for the
  foreground-only lane.
- Future design: host the existing plugin's RUNTIME inside a workspace-bounded
  process container, not on the unrestricted Host.

Hence the OpenRouter E1a lane is FOREGROUND-ONLY (allow_shell honored, bg refused)
and is submitted for independent Fable xhigh review with background execution
declared OUT OF SCOPE.
