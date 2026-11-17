# H2 on a Claude model: transport probe and placement

2026-11-17, revision 2 (headless prototype added). **Probe record and sizing. It accepts no
architecture and changes no other owner's code.** It follows the
[P0 sizing](research-lifecycle-h2-pi-realization-sizing-20261117.md) after the
operator's decisions of 2026-11-17 (relayed on the board, `aspect:research-lifecycle`
sequence 5845). Author: dev-vertical Pi session
`01a1e2a4-f353-743a-ba23-f42394958125`. Not independently reviewed.

## Why

The operator stated that the Codex quota is exhausted, there are no OpenAI API
credits, development will not wait for Friday 2026-11-21, and Claude inference
is available. Every OpenAI route (Codex native, the Pi Host's `chatgpt-codex`
and `openai-responses`) is therefore unavailable this week, and the operator
accepted one Claude model for both H2 arms (R6). The Pi Host has no Claude
route. The only subscription route to Claude here is Claude Code through the
Agent SDK (Pi's `pi-claude-subscription` provider, `claude-sdk/` models); the
`anthropic/` API key has limited credits, and calling the Messages API directly
with the subscription login is the use the Agent SDK route exists to avoid.

## Probe (2026-11-17)

Method: a local recording reverse proxy for `api.anthropic.com`
([recproxy.py](../records/dev-vertical-h2-route-20261117/claude-probe/recproxy.py))
retained each request body byte-exact, header names with credential values
replaced by their lengths, response headers and the streamed response. Pi ran
headless with only the `pi-claude-subscription` extension, the built-in `read`
tool on a two-file fixture, model `claude-sdk/claude-haiku-5-5`,
`ANTHROPIC_BASE_URL` pointing at the proxy. Three runs: the extension's Claude
prompt, a custom `--system-prompt`, and Pi's default prompt
(`PI_CLAUDE_SDK_PROMPT=pi`). Six requests, all HTTP 200. Raw artifacts stay
outside the repository because they contain the account email and identifiers
(`~/.local/state/work-engine/dev-vertical-h2-route-20261117/claude-probe/run1`).

Findings:

1. **Exact request capture works under the subscription.** Claude Code honours
   `ANTHROPIC_BASE_URL` with its OAuth login (the extension clears only API
   keys and cloud switches), so a Host-side proxy sees and can retain every
   final `/v1/messages?beta=true` body before forwarding, and can refuse to
   forward. The proxy also sees the bearer token in transit; it must never be
   retained.
2. **Billing stays in the plan for all three prompts**, including a custom
   system prompt: `anthropic-ratelimit-unified-status: allowed` on every
   request. Extra usage is disabled for the organization
   (`overage-status: rejected`, `overage-disabled-reason: org_level_disabled`),
   so a request Anthropic classed as third-party would be refused, not billed.
   The extension README's 2026-10-04 observation of a 400 with Pi's default
   prompt did not reproduce.
3. **Request shape** (Anthropic Messages, streamed): `system` is three blocks
   (a billing header naming `cc_entrypoint=sdk-ts`, an Agent SDK identity line,
   then the prompt); `tools` is a `DeferredToolPlaceholder` (`defer_loading:
   true`, "never call") plus the declared tools as `mcp__<server>__<name>`;
   `messages` carries a Claude Code system-reminder with **the account email**,
   an environment message with the working directory, and a remaining-tokens
   note; `thinking: adaptive`, `max_tokens: 128000`, `context_management`
   clearing thinking, `output_config.effort`. The request is stateless and
   resends the whole history, with tool results as `tool_result` blocks, so
   the request that produces a result contains every earlier read.
4. **Headers** name the client (`claude-cli/… sdk-ts, agent-sdk/…,
   client-app/pi-claude-subscription/…`) and a Claude Code session id.

Not probed: traffic Claude Code may send to hosts other than the base URL
(only `/v1/messages` passed through the proxy); suppressing the email reminder.

### Headless prototype (shape 2)

Prototype scripts: [mcp_relay.py](../records/dev-vertical-h2-route-20261117/claude-probe/mcp_relay.py),
[fixture_upstream.py](../records/dev-vertical-h2-route-20261117/claude-probe/fixture_upstream.py),
the proxy above with a selectable upstream. Claude Code 2.1.326 ran as
`claude -p --input-format stream-json --output-format stream-json --verbose
--tools "" --strict-mcp-config --mcp-config <one stdio server> --allowedTools
mcp__we__snapshot_read --setting-sources "" --no-session-persistence
--system-prompt <text>`, with `ANTHROPIC_BASE_URL` at the proxy and API keys
cleared.

- **Live** (`claude-haiku-5-5`, subscription): the init event listed exactly
  one tool, `mcp__we__snapshot_read`; the model issued two parallel reads, the
  relay served both, and the answer followed. Two `/v1/messages` requests, both
  retained; `tools` was `DeferredToolPlaceholder` plus the relay tool; the user
  agent was `claude-cli/2.1.326 (external, sdk-cli)`; the stream's rate-limit
  event reported `allowed`, `isUsingOverage: false`. Cost reported by Claude
  Code: about $0.001 API-equivalent.
- **Offline**: with the proxy's upstream set to a scripted SSE fixture on
  loopback, the same Claude Code ran the full turn (tool call, relay result,
  final text) with no inference. Composed qualification can therefore run
  without quota.

## Placement for a Claude model

The P0 sizing assumed Pi runs the agent loop in the container and the Host
makes the provider calls. With Claude Code as the only subscription transport
that no longer fits cleanly. Three shapes:

| | 1. Pi Host + Host-side Claude transport | 2. Claude Host (recommended) | 3. Claude Code directly as the native |
| --- | --- | --- | --- |
| Shape | Pi stays in the no-network container; its provider calls cross the bridge to a Host-side port of the `claude-sdk` provider, which drives Claude Code behind the proxy | A Rust Host process is the native for `lifecycle-service`. It runs Claude Code headless (`--print`, stream-json in and out, `--tools ""`, `--strict-mcp-config`, `--setting-sources` limited, `--system-prompt`, no session persistence) with one MCP server that relays `snapshot_read`, `question_open` and `result_publish` to the Host, and with `ANTHROPIC_BASE_URL` at the Host's recording proxy | `lifecycle-service`'s runner spawns Claude Code itself |
| Truthful ingress | Yes (Host frames) | Yes: the Host emits its own frames; Claude Code's stream-json and the proxied requests and responses are Host artifacts named by digest | Only under placement B: raw frames would be stream-json, so app-server-shaped records would be synthesized |
| Main cost | Porting the extension's session and tool-parking logic (~1 750 lines of TypeScript) across the container bridge; two agent loops in series | New Host: Claude Code driver, MCP relay, proxy; no container or Pi | Forces B (protocol-neutral store grammar) now |
| Carries over | Docker containment, Host ledger and retention, most of the dev-vertical qualification | Retention, attempt-ledger pattern, cancel and reconcile, secret screening, the controller-mode and capture design (H1-H3) | Little |

**Recommendation: shape 2.** It keeps placement A's argument (the native's raw
frames are what it emits; the request evidence is held by the Host before
dispatch) without Pi's container bridge, and its pieces that do not depend on
A versus B (Claude Code driver, MCP relay, recording and enforcing proxy,
per-turn capture) can be built in this lane without touching any other
owner's crate. The research-lifecycle owner's A-versus-B ruling decides only
the frame vocabulary the Host speaks to `lifecycle-service`.

Containment changes. Shape 2 has no Docker sandbox around the agent loop:
Claude Code runs as the user, holds its own login, and has no built-in tools.
The protection argument would rest on (a) the proxy refusing any request whose
`tools` array differs from the pinned set (prevention observed on the wire, the
R8″ check) and request and byte bounds enforced there, and (b) Claude Code's
configuration. Running Claude Code inside a sandbox that reaches only the
proxy is possible later. This is a new question for the protection owner
(R9′).

## Sizing against P0

| Item | Change from the P0 sizing |
| --- | --- |
| Host (H1-H5) | Replaced by the Claude Host: Claude Code driver and stream-json mapping 400-600 lines, MCP relay 200-300, recording and enforcing proxy (reusing `pi_coding_https` patterns) 300-500, controller frames, `thread/read` and per-turn capture 600-900, redaction and retention 150-250; about 1 650-2 550, similar to H1-H5 |
| L1-L4 (`lifecycle-service`) | Unchanged in kind; the realization is a Claude Host instead of a Pi Host |
| T1 (task host) | Grows: `verify_delivery`'s projection must read Anthropic Messages (`tool_use` and `tool_result` blocks, `mcp__…` names), about 150-300 lines instead of 40-80 |
| Q (qualification) | Easier offline: the proxy can point upstream at a loopback fixture, so composed runs need no quota, if Claude Code accepts a fixture upstream (not yet probed) |
| Store (A versus B) | Unchanged question |

Total stays about 3 300-5 800 lines.

## Open points

| Who | Point |
| --- | --- |
| Operator | H2 model and effort (R6 names Claude; which model is open). Whether retained request bodies may contain the account email locally (they must, for exact-request evidence, unless Claude Code can drop the reminder). R5 (does Claude quota count against the objective budget). |
| Research-lifecycle owner | Shape 2 versus 1 or 3; A versus B; whether anything in the S2 drafts or LC-7 protocol assumes an OpenAI model or the Responses format. |
| Protection owner | Claude Code without a sandbox, bounded by a wire-level tools check and configuration (R9′). |
| Dev vertical | Next, in this lane only: the Claude Host core (driver, MCP relay, proxy, capture) with an offline fixture upstream. |
