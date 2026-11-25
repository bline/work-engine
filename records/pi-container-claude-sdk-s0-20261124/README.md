# S0: Claude Agent SDK wire in the Python Pi container (offline)

2026-11-25. Stage S0 of the [implementation plan](../../planning/pi-container-claude-sdk-implementation-plan-20261124.md).
All of it is offline. There is no real credential and no network. Claude Code
inside the container holds only the public placeholder
(`pi_container.CLAUDE_SDK_PLACEHOLDER`), and a host mock serves the relay's Unix
socket. This record is evidence of the wire against a mock. It is not live route
evidence and says nothing about billing or quota.

## Pinned bundle

| Item | Value |
| --- | --- |
| Pi | snap `pi-coding-agent` revision 91 (Pi 1.5.2); re-run on revision 93 (Pi 1.6.0) after the snap updated mid-session, same wire (`evidence/rev93-*`) |
| Provider extension | `pi-claude-subscription` 0.2.4 |
| Agent SDK and bundled `claude` | `@anthropic-ai/claude-agent-sdk` and `-linux-x64` 0.3.325; `user-agent: claude-cli/2.1.325` |
| MCP SDK | `@modelcontextprotocol/sdk` 1.32.2 |
| Closure | 94 packages from the declared dependencies; bundle digest `e6e4ad42fc1ff4c4…` (`claude_sdk_bundle_identity`) |
| Container | `pi_container.run_profile` profile, image `node@sha256:4b150b48…`, `--network none`; package directories mounted read-only at their host paths |

[probe.py](probe.py) reproduces the runs (`probe.py RUN_DIR [--fault status500]
[--no-lane-env]`). Scrubbed outputs are in [evidence/](evidence/). The `rev93-*` runs repeat the normal, 500 and P7 runs on Pi 1.6.0 with the same betas, body keys, bundle digest and single-request 500 outcome. Only header
names, selected public header values, body keys and shapes, and digests were
kept. No prompt or response text was kept.

## Observed wire (frozen for S1)

- **Route.** Every request was `POST /v1/messages?beta=true`. No other path, no
  token counting, and no traffic outside the base URL.
- **Auth.** `Authorization: Bearer <placeholder>` and no `x-api-key`. Claude Code
  chose its OAuth form from the synthetic `~/.claude/.credentials.json`.
- **Headers.** `accept`, `accept-encoding`, `anthropic-beta`,
  `anthropic-dangerous-direct-browser-access`, `anthropic-version: 2023-06-01`,
  `authorization`, `connection`, `content-length`, `content-type`, `host`,
  `user-agent`, `x-app: cli`, `x-claude-code-session-id`,
  `x-client-app: pi-claude-subscription/0.2.4`, and `x-stainless-arch`, `-lang`,
  `-os`, `-package-version`, `-retry-count`, `-runtime`, `-runtime-version`,
  `-timeout`.
- **Betas.** `oauth-2025-04-20`, `interleaved-thinking-2025-05-14`,
  `context-management-2025-06-27`, `claude-code-20250219`,
  `per-turn-control-2026-07-01`, `mid-conversation-tool-changes-2026-07-01`,
  `thinking-token-count-2026-05-13`, `effort-2025-11-24`,
  `thinking-display-updates-2026-08-18`, `extended-cache-ttl-2025-04-11`.
- **Body.** The keys were `context_management`
  (`clear_thinking_20251015`), `max_tokens` (128000), `messages`, `metadata`
  (`user_id`, a JSON string with `account_uuid` empty, `device_id`,
  `session_id`), `model`, `output_config` (`effort`), `stream: true`, `system`,
  `thinking` (`adaptive`, `display: updates`) and `tools`.
  - `system` has three text blocks: the per-request
    `x-anthropic-billing-header: …; cc_entrypoint=sdk-ts;`, the SDK identity
    (`You are a Claude agent, built on Anthropic's Claude Agent SDK.`), and Pi's
    Claude prompt. The last two carry one-hour `cache_control`.
  - `tools` is `DeferredToolPlaceholder` (`defer_loading`) plus `mcp__pi__<tool>`.
  - `messages` includes `role: "system"` entries (the per-turn-control feature).
- **Session continuity.** Turn 1 made a `read` tool call and then answered: two
  requests. Turn 2 was a new container on the same Pi session ID and made one
  request. The extension rebuilt Claude Code's conversation from Pi's history,
  because the turn-1 Claude Code state was in the old container's tmpfs home.
- **Retries.** These are the deciding finding for the S1 design:

| Run | Lane switches | First request answered 500 | Requests in turn 1 |
| --- | --- | --- | --- |
| `run-500` | none | Claude Code resent the request with `x-stainless-retry-count: 0` | 3 (500, then resent and succeeded) |
| `run-500-laneenv` | `CLAUDE_SDK_CONTAINER_ENV` | Claude Code surfaced the error; **Pi's own retry** resent | 3 |
| `run-500-laneenv2` | env + `CLAUDE_SDK_PI_SETTINGS` (Pi retry off) | error surfaced, no resend | **1** |
| `run1-laneenv` | env + settings | no fault: tool round and resume work | 2 + 1 |

`run-p7-notools` used `claude-opus-5-5`, `--thinking high`, `--no-tools` and
`--system-prompt`. It sent `tools: []` (no deferred placeholder) and effort
`high`. The replaced prompt was in the third system block with a short Pi suffix
(the probe kept a yes/no, not the text). Each turn made one request. See the
[P7 addendum](../../planning/pi-container-claude-sdk-p7-addendum-20261125.md).

So the retry header is not evidence that a request is not a retry. S1
therefore disables both retry layers and halts the turn at the proxy after any
exchange that does not complete. No later request in that turn is forwarded: the
halt is recorded before any queued request is woken (review P1-1).

The 2026-11-17 Rust Host probe ([pi-claude-probe](../dev-vertical-h2-route-20261117/pi-claude-probe/README.md))
used older versions and saw the same request form. S0 checked its allowlists
against the current wire rather than copying them.

## Producer contract (host-only, consumed by `status` and `usage_ledger.py`)

- `host/turn-N.intent.json` is written with `O_EXCL`, fsync and a directory
  fsync before the runner starts. It contains `kind: claude_sdk_turn_intent`,
  `session_id`, `turn`, `event`, `model`, `container`, `socket_dir`,
  `proxy_log`, `runner_record`, `spec`, `max_requests` and
  `fingerprint_sha256`. A failed write starts nothing.
- In `host/turn-N.proxy.jsonl`, `claude_sdk_admitted` is fsynced before any
  request byte goes upstream. It contains `n`, `model`, `effort`, `thinking`,
  `betas`, `body_sha256`, `system_sha256`, `billing_block_sha256`,
  `tools_sha256`, `tool_names` and `messages`. A failed write means no forward.
- The disposition is a `request` row with the same `n`, `provider: claude-sdk`
  and a `disposition`: `completed`, `upstream_error`, `dispatched_no_response`,
  `billing_refused`, `billing_not_established`, `response_model_*`,
  `terminal_missing`, `stream_error*` or `usage_not_established`. It also
  carries `billing_verdict` (`verdict`, `unified_status`, `overage_status`),
  `response_model`, `response_id`, `usage`, `usage_status` and
  `output_released`. There are no raw headers, bodies or bearer.
- Dispatch is read only from admissions and their dispositions. `refused` rows
  are not keyed to requests: they occur before dispatch and also after some
  dispatched exchanges (upstream unreachable, upstream status, billing or model
  refusal).
- The joint reading is `pi_container.claude_sdk_turn_summary`, which the ledger
  implements independently:
  - `complete`: every admission is settled;
  - `unresolved`: an admission has no disposition, so the request was possibly
    dispatched and possibly billed;
  - `corrupt`: unreadable or inconsistent rows, or a partial final row;
  - `missing`: no log, because the proxy (sole holder of the credential) never
    started.

  `complete` means every admission has a disposition, not that every outcome is
  known: `outcome_known` is true only for `missing`, or `complete` with every
  disposition `completed`. A `dispatched_no_response`, truncated, unverified or
  usage-less exchange remains unknown per request. Neither `unresolved` nor
  `corrupt` is ever zero. Sources are joined by `n` within one log; the
  ledger's call id is `claude-sdk:<log>:<n>`.
