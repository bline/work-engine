# Running a role on Pi

Pi (`pi-coding-agent`) is a third host for a worker or reviewer that a Claude
Code or Codex supervisor launches. The roles, packets, evidence rules and the
retained-or-fresh decision are unchanged. Load this when you launch, continue
or collect a Pi role. Root rotation is not established for Pi:
`root_rotation.py` accepts only Claude and Codex. A Pi session can also be the
supervising root and delegate through pi-subagents' `subagent` tool;
[Pi as the supervising host](#pi-as-the-supervising-host) covers what differs.

Host facts and their evidence are in the
[Pi host probe](../../../metrics/probes/pi-cli-host-20261109.json) (Pi 1.4.0,
local fake endpoint, n = 1 per cell).

## Choose a lane

Every Pi launch names how it is paid for:

| `--auth` | Pi credential for the provider | paid by | service tier |
| --- | --- | --- | --- |
| `api` | API key (environment or Pi's `auth.json`) | the provider account, per token | allowed for provider `openai` |
| `subscription` | Pi OAuth, or existing host Codex ChatGPT auth in Codex container mode, or the existing host Claude Code login in claude-sdk container mode | the subscription's session quota | refused |

For host launches and API containers, before each turn the launcher runs `pi auth check --provider <p> --json
--no-refresh` and refuses unless Pi's credential type matches the lane
(`api_key` or `oauth`). It never prints, refreshes or copies the credential.
On the api lane with provider `openai`, a non-`sk-` `OPENAI_API_KEY` is also
refused: Pi would send it as a ChatGPT sign-in token.

An api-lane turn spends money. It needs the same authority as any other paid
operation; read the board first, as [AGENTS.md](../../../AGENTS.md) requires.
The owner stated on 2026-11-09 that service tiers do not apply to
session-quota requests, so a subscription launch with a tier is refused.

## Launch

`<skill-dir>` is this skill's directory. The packet goes in a file; Pi reads
it on stdin.

```bash
python3 <skill-dir>/scripts/role_host.py launch pi \
  --cwd <repo-or-worktree> --name <role-name> --prompt-file <packet> \
  --parent <your-session-id> --auth api --service-tier priority \
  --host-arg=--model --host-arg=openai/<model-id> \
  --host-arg=--thinking --host-arg=<level>
```

- `--model` is required and must be one exact `<provider>/<model-id>`, not a
  pattern. Give the thinking level with `--thinking` (`off`, `minimal`, `low`,
  `medium`, `high`, `xhigh`, `max`), not a `:level` suffix. Choose the model
  and effort from [model selection](../../../docs/model-selection.md).
- Accepted pass-through flags: `--model`, `--thinking`, `--tools`/`-t`,
  `--exclude-tools`/`-xt`, `--skill`, `--append-system-prompt`,
  `--extension`/`-e`, `--no-mcp`, `--no-skills`, `--no-extensions`,
  `--no-context-files`, `--approve`, `--no-approve`. Session, mode, provider
  and credential flags (`--session*`, `--continue`, `--fork`, `--mode`,
  `--provider`, `--api-key`) are refused; they belong to the launcher.
- The launcher generates the session ID, starts
  `pi --mode json --session-id <id>`, and returns only after the turn's first
  record is the session header for that ID. That ID is the identity to bind.
  `identity pi <id>` returns the verified session file.
- Pi loads `AGENTS.md` and `CLAUDE.md` from the working directory. Tell the
  child to read `skills/manual-orchestration/SKILL.md` by path, as for any
  child.

Each turn is a detached process in its own process group. Its prompt, event
stream, stderr, exit status and tier record are under the role-host state
directory, at `pi/<session-id>/turn-N.*`.

## Service tier

`--service-tier` (`auto`, `default`, `flex`, `priority`, `scale`) is accepted
only on the api lane for provider `openai`. Pi 1.4.0 has no tier setting, so
[`pi_service_tier.ts`](../scripts/pi_service_tier.ts) adds `service_tier` to
each request for that exact model on the `openai-responses` API. It reads the
tier from its own process environment, so concurrent roles on the same model
can run at different tiers, and it writes no shared configuration.

The extension records each request's injected or skipped decision and the tier
OpenAI reported on the response. `status` reports these as
`last_turn.service_tier`:

| status | meaning |
| --- | --- |
| `applied` | every response reported the requested tier (any tier for `auto`) |
| `mismatch` | a response reported a different tier |
| `not_applied` | no request carried the tier (see `skipped` reasons) |
| `not_established` | the tier was sent, but no response reported a tier |
| `not_requested` | no tier was requested |

Pi reports a throwing hook and sends the request without the tier, so the
injected and observed records are the evidence, not the request. Record a
mismatch; do not relaunch on it automatically. Whether compaction requests
carry the tier is not established.

## Observe, continue, retire

- `status pi <id>`: `running` while the turn's process is alive (its pid is
  checked against the recorded start time), then `idle`, or `stopped` after
  retirement. `last_turn` gives `outcome` (`completed`, `failed`, `aborted`,
  `incomplete`, `interrupted`, `not_established`), the requested and served
  models, Pi's reported cost and the tier observation. JSON mode exits 0 even
  when the response failed, so the outcome is read from the events. The cost is
  Pi's catalog price times Pi's tier multiplier, not billing.
- `deliver pi <id> --cwd <launch cwd> --message-file <file>`: refused while a
  turn runs. It reuses the launch's model, thinking, lane and tier, and checks
  the credential again. Pi finds `--session-id` within the working directory, so
  delivery from another directory is refused rather than starting a new session.
  To change settings, launch a new session.
- `retire pi <id>`: an idle session is released by a ledger row. A running turn
  needs `--force`, which sends SIGTERM to its process group. Pi then kills the
  tool processes it started (it starts them detached); SIGKILL would leave those
  running, so the launcher does not send it. The session keeps the interrupted
  user message without a reply.
- `children <your-session-id>` lists Pi children launched with `--parent`.
- From inside a Pi turn, `role_host.py whoami` reads `PI_SESSION_ID`. Pi's
  session variables are not passed to sessions launched from it.

## Directed coordination

For enrolled, persistent Pi sessions, use
[Pi Chatboard](../../coordination-channels/references/pi-chatboard.md) for
directed coordination. Verify the actual session and expected board/group/lane
with `/chatboard config` and `/chatboard status`. The extension owns the
message waiter; do not arm a duplicate at startup or after `new_context`.
Reload and same-session recovery preserve compatible enrollment, not semantic
recovery or telemetry. Do not assume a detached turn or subagent has the
extension loaded, an enrollment, or a running recipient after the process
exits. Use an owned alternative when the route is not established.

## Context

`child_context.py` has no Pi reader yet. A Pi session file records each
assistant message's `usage`; the input of the latest call is
`input + cacheRead + cacheWrite`. Use [session control](context-control.md)
for verified local pressure delivery, a context-bound CLI wait, or the pinned
`work-engine-context-status pi` check. Pi Chatboard carries no pressure events.
Near the model's limit Pi compacts automatically, or, with `pi-context-epochs`
loaded, rolls over without a summary ([native recovery](native-context.md#pi)).
Make the retained-or-fresh decision from that reading as
[context boundary](context-boundary.md) describes, and record it.

## Pi as the supervising host

A Pi root launches children with the pi-subagents `subagent` tool rather than
`role_host.py`. Roles, packets, review and the retained-or-fresh decision are
unchanged; these properties of the mechanism differ.

- **The `task` field is the packet.** The child's first user message is
  `Task: <task>` plus any output-path instruction, and a launch without `task`
  is not rejected. On 2026-11-19 a review launched without one received only
  its output path, spent its turn trying to infer a subject from repository
  state, and produced nothing. Read the child's first user message to confirm
  the packet arrived.
- **Identity.** A child's session file is
  `<parent session file without .jsonl>/<run id>/run-<n>/session.jsonl`; its
  header names the parent in `parentSession`. Bind the child's context
  observation to that file and its header ID (`--context-host pi` for a CLI
  listener). Async run state and
  `events.jsonl` sit in the run directory that `subagent` status reports.
- **Tools are the agent's list.** An agent with an explicit `tools` list gets
  only those tools: MCP tools need `mcp:<server>` or `mcp:<server>/<tool>`
  entries, and `new_context` and `context_notes` are absent unless listed.
  User-level `subagents.agentOverrides` in `~/.pi/agent/settings.json`
  currently give `worker`, `scout` and `reviewer` the Codebase Memory read and
  index tools and the context-epoch tools. A foreground child loads no ambient
  extension, so a listed extension tool without its provider fails the launch;
  the overrides therefore also name the `pi-context-epochs` entry path.
- **Rollover inside a child.** A child with the epoch tools can replace its
  own context. In three 2026-11-20 probes (background without and with the
  provider path, and foreground with it), each child's `new_context` produced one
  boundary and one continuation, notes survived, and pi-subagents completed
  the step only after the post-rollover run, returning that run's answer. The
  context reader followed the child into its new epoch. The new epoch quotes
  the child's task verbatim, so a child that records its progress in
  `context_notes` before calling `new_context` can tell continuation from a
  fresh start; without that record it can repeat finished work.
- **Stalls.** A `claude-sdk` child can stop inside one model call: its session
  ends with an assistant message holding tool calls and no tool results, while
  status reports no activity. Two Fable reviewer runs on 2026-11-19 stalled
  this way, for 71 minutes (a `resume` then completed it) and for 120 minutes,
  until the subagent timeout. A queued steer is not delivered to a stalled
  call. A no-activity alarm is a reason to read the child's session file, not
  evidence of long thinking.

## Container mode

`--container` runs each turn in a Docker container with no network. It is
available on the api lane for providers `openai` and `anthropic`, and on the
subscription lane for `openai-codex` using existing host Codex ChatGPT auth and for
`claude-sdk` using the existing host Claude Code login
([Claude subscription container](#claude-subscription-container)). Host facts: [Pi Docker probe](../../../metrics/probes/pi-docker-host-20261109.json)
(Docker 29.7.2, local fake endpoint, n = 1 per cell).

```bash
python3 <skill-dir>/scripts/role_host.py launch pi --container \
  --cwd <work-dir> --name <role-name> --prompt-file <packet> --parent <your-session-id> \
  --auth api --service-tier priority \
  --mount-ro <inputs> --mount-rw <output-dir> [--codebase-memory] \
  --host-arg=--model --host-arg=openai/<model-id> --host-arg=--thinking --host-arg=<level>
```

What the agent gets, enforced by the container rather than by instructions:

- **Your own tools at your own paths.** The host's `/usr` and `/etc` are
  mounted read-only, so the user, git, Python and other tool versions match the
  host. Listed folders are mounted at their host paths: `--mount-ro` read-only,
  `--mount-rw` read-write. The working directory is read-write unless a listed
  mount covers it. Everything else is absent, including the rest of your home
  directory. Home itself, `/`, system paths and role_host's state cannot be
  mounted.
- **Protected host code.** Writable mounts cannot overlap the active host
  implementation, Python runtime/import roots, command search directories or
  resolved host executables. Launch and continuation both check this boundary;
  implicit writable state/socket mounts are checked too. The credential-bearing
  runner uses Python isolated mode (`-I`), excluding script/current-directory,
  `PYTHONPATH` and user-site imports. To edit this checkout, run an independently
  installed host outside the writable checkout. Read-only code mounts remain
  available. This protects host loading paths; it does not attest arbitrary
  pre-existing hard-link or bind-mount aliases created by the operator.
- **No network.** The container has only loopback. Network access fails at once
  with "Network is unreachable"; there is no web access from tools.
- **No credentials or host environment.** Only the variables role_host sets
  reach the container. The API key stays on the host: the container holds a
  placeholder.
- **One way out.** A relay in the container forwards Pi's model requests over a
  mounted Unix socket to an egress proxy that role_host runs on the host. The
  proxy accepts `POST /v1/responses` for the launched model only. It forwards
  only a fixed set of top-level request fields (so stored prompts, previous
  responses, conversations and background mode are refused), refuses hosted
  (server-side) tools declared in any `tools` list and in `tool_choice`, and
  refuses `store: true`. It sets the requested tier (or removes one),
  substitutes the real key, and logs the tier and usage the provider reported.
  `--max-requests N` caps forwarded model requests per turn.
- **API dollar limit.** On the API lane, `--max-usd X` limits the whole session's modelled API
  spend. The proxy forces `max_output_tokens` to `--max-output-tokens`
  (default 16000 with `--max-usd`), then sets aside each request's worst
  case before forwarding it: every input token at the dearer of the input and
  cache-write rates, plus the full output cap, at the requested tier. Input is
  bounded by the previous request's reported `input_tokens` plus the bytes of
  the items appended to it when the request extends that input, otherwise by
  the bytes of the whole body (a token is at least one byte); above 272k the
  long-context multipliers apply. If spend plus that amount would pass the
  limit, the proxy refuses with 429, refuses every later request, and the
  runner stops the container. After the response the reservation is replaced
  by the charge from the provider's final usage at the tier it reports. A
  response with no usage keeps the full reservation; an error status before
  any stream is charged nothing (assumed unbilled, not verified). A response
  served at another tier is charged at that tier and later reservations use at
  least the dearer of it and default; one reporting no tier is charged at the
  dearest multiplier. `--usd-strict-tier` sets aside at the dearer of the
  requested and default tier from the start. Files, and images not sent
  inline, are refused because their bytes do not bound their tokens. Rates
  are `scripts/openai_api_rates.json` (a test keeps them equal to the usage
  ledger's) and are fixed in the environment at launch. Each turn starts from
  the spend recorded in the session's earlier proxy logs, counting any
  reservation that was never settled; a delivery is refused with
  `usd_limit_reached` once that spend reaches the limit. `status` reports it
  under `last_turn.egress.usd`. Stopping early is the cost of a hard limit:
  the limit can stop a run while up to one worst case of it is unspent.
- **The proxy is bounded too.** It runs on the host, outside the container's
  limits, so it accepts at most 8 connections at once (more are closed
  without a thread), requires a `Content-Length` of at most 64 MiB, refuses
  chunked bodies, checks the request cap before reading a body, and drops a
  connection that stalls for 60 s.
- **Nothing outlives the turn.** The container's entry process is its PID 1;
  when it exits, the kernel ends every process left in the container.
- Limits default to `--container-memory 4g`, `--container-pids 512` and
  `--container-cpus 2`.

The proxy log and runner record (`pi/<session>/host/turn-N.*`) are host-only.
The events and the extension's tier record are written inside the container,
where the agent could alter them, so `status` takes the tier, request count and
usage from the proxy (`last_turn.egress`, `service_tier.observed_by`). The proxy
log records input, cached, cache-write, output and reasoning tokens per call;
`metrics/scripts/usage_ledger.py build` reads it as provider `pi`, marks each
call with the tier the provider reported, and prices it at that tier. The turn
container is removed after each turn; `retire --force` stops it. If the runner
dies before its cleanup, `status` lists the container and socket folder it left
under `leftovers`, and `retire` (forced or not) removes them.

**Environment fingerprint.** The launch records a fingerprint of what decides
what the agent sees and can do: the image; host OS, kernel, package database
and `/usr/local` metadata; the Pi runtime (an immutable snap revision by path,
any other runtime by a digest of all its files); each mount's path, mode,
device and inode; limits and user; model, tier and Pi arguments; the container's
complete environment; and the digests of the entry script, tier extension,
`pi_container.py` and codebase-memory binary. Each later turn re-validates the
mounts (a folder replaced by a symlink plans differently) and recomputes the
fingerprint, and refuses with `environment_changed` if anything differs. Not
covered: hand edits under `/etc` that change no package, and an edit under
`/usr/local` that keeps size and modification time. Launch a new session to
run in a changed environment.

**codebase-memory.** `--codebase-memory` indexes the working directory into the
session's own cache before launch, so the agent sees no other project's index
and nothing indexed from outside its mounts. role_host then runs `index_status`
inside the same container profile, with no model, and refuses the launch unless
it reports `ready` for the working directory's path. Pi gets the server's
tools declared directly (17 in 0.11.0), named `mcp__codebase_memory_mcp__<tool>`
(Pi turns the dashes into underscores). That name comes from the container's
server entry; on the host, `~/.pi/agent/mcp.json` names the server
`codebase-memory`, so its tools are `mcp__codebase_memory__<tool>` with the
configured exposure, and a Claude provider bridge prefixes them `mcp__pi__`.
The container index records no revision: it
matches the tree because it was built from it at launch. Whether it refreshes
after the agent edits files inside the container is not established.

### Codex subscription container

`--container --auth subscription --host-arg=--model
--host-arg=openai-codex/<model-id>` uses the existing `~/.codex/auth.json`
ChatGPT credential on the host. It does not require Pi OAuth state, run login,
refresh tokens, or change either application's auth file. Host subscription
launches without `--container` continue to use Pi's own OAuth checks.

The adapter validates the host record's `chatgpt` auth mode and JWT issuer,
expiry and account claim before launch and each delivery; the backend
authenticates the JWT. The container receives a public synthetic JWT, and its
private Pi settings select SSE. The host proxy decodes Pi's zstd request with
the host `zstandard` Python module, accepting exactly one frame; the frame window,
encoded and decoded bodies all have the
proxy's 64 MiB bound. It applies the existing request/model/tool/store
filters, rebuilds auth/account/originator/beta headers from host state, and
forwards to `https://chatgpt.com/backend-api/codex/responses`. The
`ROLE_HOST_PI_UPSTREAM_CODEX` override supplies the base through `/codex` for
local transport testing. Mount planning and retained-turn validation refuse the resolved host auth file
and its ancestors, including aliases, while permitting nonoverlapping skill
folders. No host credential or account identity enters the
container, models file, fingerprint, spec, or proxy log.

This native transport has no hard output-token-cap field. `--max-usd X`
opts into an **API-equivalent cost-proxy stopping threshold**, using the existing
OpenAI rates pinned at launch and default API valuation without a subscription
service tier. It is not a hard pre-request cap: one admitted response can
overshoot. The proxy allows one request in flight, persists admission before
forwarding, and stops the container when accumulated proxy cost reaches the
threshold or accounting becomes unknown. A concurrent request is refused before
forwarding. Failed, interrupted, identity-unverified or usage-invalid exchanges
leave an unresolved admission; a later delivery is refused. Malformed logs or
partial final records also block delivery while retaining prior known costs.
Required input/output counts must be nonnegative integers; when optional cache
counts are absent all input is conservatively valued at the dearer input/cache-write
rate. Status, environment and receipts label the units and the overshoot limitation.
`--usd-strict-tier`, `--max-output-tokens` and service tiers remain refused on
this lane; `--max-requests` still applies. API lanes retain their dollar
reservations and output caps. Proxy logs identify `openai-codex` and the
subscription lane and retain provider-reported token usage, including native
`response.done` completions. Forwarded requests missing a completion remain
unknown accounting calls. Billing and quota
are unknown. The usage ledger places these calls in the Codex subscription
pool with unknown dollar/quota values rather than API-billed `pi_api` spend.
It retains separately labelled per-request proxy measurements, including
unresolved admissions, without settling subscription invoice dollars or quota.
`--objective-record` API-dollar admission cannot be combined with this proxy limit.
Validation uses synthetic credentials and native Pi SSE against a local mock;
no live quota claim is established.

### Anthropic Messages

`--host-arg=--model --host-arg=anthropic/<model-id>` routes Pi's own
`anthropic-messages` transport through the same container and proxy
(2026-11-10, offline with mocked providers; no live Anthropic request yet).
Differences from the OpenAI route:

- The proxy accepts `POST /v1/messages` (query `beta=true` only) for the
  launched model, with `stream: true`. It forwards a fixed set of top-level
  fields; `fallbacks` (another model may serve the request), `service_tier`,
  `speed`, `container`, `mcp_servers` and anything else are refused. Betas are
  limited to the ones Pi uses, without `server-side-fallback`. Tools must be
  custom (no `type`, or `custom`) in `tools` and in inline `tool_addition`
  blocks. No service tier is set; `--service-tier` is refused.
- The proxy rebuilds the headers: `anthropic-version`, the allowed
  `anthropic-beta` values and `x-api-key` with the host key from
  `pi auth print-api-key --provider anthropic` (must be `sk-ant-`, not an OAuth
  `sk-ant-oat` token). The container holds `ANTHROPIC_API_KEY` set to a
  placeholder. Upstream is `https://api.anthropic.com`, or
  `ROLE_HOST_PI_UPSTREAM_ANTHROPIC`.
- The response's own model identity is checked, fail closed. A 200 stream is
  held until its first event (pings aside) is a `message_start` whose
  `message.model` equals the launched model exactly; only then are headers and
  events sent to the container. A missing, malformed or different model, a
  different first event, a stream that ends (or holds more than 1 MiB) before
  that event, or a second `message_start` later in the stream, refuses the
  request: the upstream connection is closed, the proxy logs a
  `response_model_refused` event (reason, authorized and reported model) and
  the request row's `response_model_check`, the reservation stays charged, the
  container gets a 502 (or, after a restatement, a truncated stream), and
  nothing is retried or substituted. Error statuses carry no model output and
  pass through unchanged.
- Usage comes from the stream's `message_start` and `message_delta` events:
  input, cache reads, cache writes split into 5-minute and 1-hour lifetimes,
  and output. The proxy log row carries `provider: anthropic`;
  `usage_ledger.py build` prices it with the Claude rates (`native_usd`) in
  pool `pi_api`.
- `--max-usd` uses `scripts/anthropic_api_rates.json` (a test keeps it equal to
  the ledger's rates). The worst case prices every input token at the dearest
  of the input and both cache-write rates, plus `max_tokens` forced to the
  output cap; a budget-thinking `budget_tokens` at or above the cap is refused.
  Cache writes the provider does not split by lifetime are charged at the
  1-hour rate; a reported tier other than `standard` keeps the reservation.
- A model missing from Pi's catalog (Pi 1.4.0 lacks `claude-fable-5-2`) needs
  `--container-model-file <json>`: one Pi `models.json` model entry with that
  id, written under the provider with the relay `baseUrl`; it may not set
  `baseUrl`, `headers`, `apiKey` or fallback models.

### Claude subscription container

Qualification: **offline only** (2026-11-25, synthetic credentials and a local
mock upstream; [S0 wire record](../../../records/pi-container-claude-sdk-s0-20261124/README.md),
[plan](../../../planning/pi-container-claude-sdk-implementation-plan-20261124.md)). No live request
has been made through this route; a live smoke needs separate approval.

`--container --auth subscription --max-requests N --host-arg=--model
--host-arg=claude-sdk/<model-id>` runs Pi with the `pi-claude-subscription`
extension. The extension starts Claude Code (Claude Agent SDK) inside the turn
container; Claude Code reaches the host proxy through the loopback relay.

- **Selection.** Supported models: `claude-haiku-5-5`, `claude-opus-5-5`
  (exact response-identity checks; other models are refused until they have
  their own evidence). `--max-requests` is required, finite and positive; it is
  a per-turn request cap, not a session quota or spend cap. `--max-usd`,
  `--max-output-tokens`, `--usd-strict-tier`, `--container-model-file`,
  service tiers, `--extension` and `--objective-record` are refused. There is no
  API-key, extra-usage or model fallback.
- **Credential.** The host reads the existing Claude Code OAuth token from
  `~/.claude/.credentials.json` (or `ROLE_HOST_CLAUDE_CREDENTIALS`) before
  launch and each delivery: a regular, non-symlink file owned by the user with
  no group/other access, an `sk-ant-oat01-` access token and an expiry more
  than two minutes away. It never logs in, refreshes or writes it. The token
  goes to the runner on stdin only. Mount planning refuses that file, its
  directory, `~/.claude`, `~/.claude.json`, and the bundle root, including
  ancestors and aliases. The container entry writes a public placeholder
  credential into the turn's tmpfs home; nothing real enters the container,
  spec, fingerprint, ledger or logs.
- **Pinned bundle.** The extension and its declared dependency closure under
  `~/.pi/agent/npm/node_modules` (or `ROLE_HOST_CLAUDE_SDK_BUNDLE`) are mounted
  read-only, package by package, so an undeclared import fails. Their content
  digest is part of the environment fingerprint and is rechecked on delivery.
  Pi runs with `--no-extensions` and `--no-approve` (workspace `.pi` resources
  never apply; `--approve` is refused), plus only the pinned extensions. Before
  every turn the host rewrites the agent directory's `settings.json` (Pi retry
  off) and removes `models.json`, `claude-sdk.json` and `trust.json` without
  following symlinks. The
  container environment turns off Claude Code retries, model, non-streaming
  and refusal fallbacks, and nonessential traffic, and forces the extension's
  Claude prompt.
- **Proxy.** Only `POST /v1/messages?beta=true`. The headers, betas and body
  fields observed in S0 are allowed. The container must present the placeholder
  bearer and `x-stainless-retry-count: 0`, and the system blocks must carry the
  SDK billing header and identity. These checks catch misconfiguration; they do
  not prove who the caller is. The body must be strict JSON: duplicate keys at
  any depth and `NaN`/`Infinity` are refused, so the forwarded bytes have
  exactly the reading that was checked. Only Pi's bridged `mcp__pi__*` tools and
  Claude Code's deferred-tool placeholder are accepted; any typed (hosted) tool
  and any image or document source the provider would fetch (URL, file id) is
  refused. The proxy durably logs an admission before sending, then forwards
  the same accepted bytes. The headers are rebuilt with the host bearer. Output
  is held until the response's subscription verdict
  (`anthropic-ratelimit-unified-status` `allowed` or `allowed_warning`) and its
  first `message_start` model are both established. Upstream error bodies are
  neither relayed nor logged.
- **No replay.** Claude Code can resend a failed request above its visible
  retry counter (S0). Any refusal, upstream error, billing or model refusal,
  truncated stream or missing usage therefore halts the turn: every later
  request in it is refused and the container is stopped. The halt is recorded
  before any queued request is woken.
- **Evidence.** The host writes `turn-N.intent.json` durably before starting
  the runner. Each admission is matched by `n` to a durable disposition
  (`completed`, `upstream_error`, `billing_refused`, `terminal_missing`, ...).
  The admission row carries the effort and thinking settings, betas, a body
  digest, digests of the system prompt (without the per-request billing block)
  and tool list, and the tool names. `status` reads these host-only records.
  Its `egress.claude_sdk` evidence is `complete` (every admission has a
  disposition), `unresolved` (an admission without a disposition: possibly
  dispatched and billed), `corrupt` or `missing` (the proxy never started, so
  nothing was sent). `complete` does not mean every outcome is known:
  `outcome_known` is true only when every disposition is `completed` (or the
  log is missing); `dispatched_no_response`, truncated or unverified exchanges
  stay unknown per request. `egress.refused` lists refusals both before and
  after dispatch; the per-request dispositions say which were dispatched.
  A delivery is refused while any earlier turn is unresolved, corrupt or
  lacks a ledger row. `status` and `retire` find turn intents that never got a
  ledger row (launcher death). The runner's first host-only record is its own
  pid and start time, so such a turn reads as `running` while its runner or
  container lives; `retire --force` stops the runner and removes its container.
- **Usage ledger.** `usage_ledger.py` reads the same logs and intents: one row
  per admission in pool `claude_all_models` with `usd: null`, `billing` and
  `quota` `unknown`, and a disposition. Unresolved admissions are rows with
  unknown usage, not zero. A refused request is no row. Nothing enters
  `pi_api`. Native Pi session files of these sessions are skipped, and the
  container's Claude Code transcripts live in its tmpfs home, so no call is
  counted twice. Provider-reported token usage is kept; subscription quota
  consumption and any overage charge are not measured.

## Not supported here

- `rename`, `archive`, `unarchive`, `settings`, `prepare`, `start-prepared` and
  `exact-turn` are Codex-only.
- A Pi session started outside role_host has no observable turn process;
  `status` reports `not_established` rather than idle.
- The session directory is read from `PI_CODING_AGENT_SESSION_DIR`, else
  `PI_CODING_AGENT_DIR/sessions`, else `~/.pi/agent/sessions`. A `sessionDir` in
  Pi's `settings.json` is not read. Container sessions keep their session file
  in their own directory under role_host's state.
- Container mode does not yet offer web access (a
  mediated fetch through the proxy is the intended route), or moving a session
  between host and container.
