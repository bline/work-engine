---
name: session-events
description: Deliver Codex or Claude Code turn ends (optionally with board, budget, worker-closure and successor context), budget/deadline/stall/successor alerts, and regular-expression transcript matches to an MCP Events subscriber such as OpenAI Dot, as signed webhooks with replay; and let it list, search and read local sessions and inspect budgets and delivery health. An operator-run service; it grants the subscriber no authority over the sessions.
---

# Session events

An MCP Events client (an OpenAI Dot, or a Work chat) can watch named local
Codex and Claude Code sessions for a regular expression and be called back when
a new transcript entry matches. Nothing polls on a model's behalf: the
subscriber waits for a webhook, and the daemon waits on inotify.

## What exists

```text
Dot ──JSON-RPC──▶ OpenAI tunnel endpoint ◀──long-poll── tunnel-client (this machine, outbound only)
                                                              │ stdio
                                                              ▼
                                     session_events.py serve ──writes──▶ subscriptions/
                                                                              │ inotify
Codex / Claude transcripts ──inotify──▶ session_events.py run ──signed HTTPS POST──▶ Dot's callback URL
```

- **`serve-http`** is the MCP server as stateless HTTP on loopback
  (`127.0.0.1:8787/mcp`), reached by OpenAI's `tunnel-client`. **`serve`** is
  the same server on stdio. Both declare `tools` and `events` capabilities, two
  event types and three read-only tools. They answer subscribe calls by writing
  subscription records; they never deliver events. Use `serve-http` with the
  tunnel: with a stdio target, `tunnel-client` refused ChatGPT's
  `events/subscribe` (see the live trial below).
- **`run`** is the delivery daemon. It watches the transcripts named by
  subscriptions, matches each newly completed line, and POSTs events. It runs
  independently of `serve` and of the tunnel; deliveries go out from this
  machine. It also raises alerts on timers and writes a heartbeat every minute.
- **Context sources**, all read-only and each optional, add context to
  `session.turn.ended` (envelope `full`), `session.alert` and `budget_status`:
  - the coordination board of `--board-repository`
    (`skills/durable-state/scripts/codex_chatboard.py`'s JSON document behind a
    git ref, read with `rev-parse` and `cat-file` only);
  - the usage ledger `--budget-ledger` (`metrics/scripts/usage_ledger.py`),
    run only as the bytes of `--budget-ledger-sha256`, from a private copy;
  - an index of `scope_budget` records under `--budget-root` (default
    `~/.local/state/work-engine`), built by an mtime scan every 5 minutes;
  - worker lineage from Codex's `state_5.sqlite` spawn edges, the role_host
    launch ledger, `scope_budget` bindings and Claude subagent transcripts.
- **`sessions`, `match`, `status`, `revoke`** are local CLI views: search by
  title, dry-run a pattern (prints event data, sends nothing), inspect stored
  subscriptions without secrets, and refuse a subscription.

State lives in `~/.local/state/work-engine/session-events/` (`--state-dir`,
`$SESSION_EVENTS_STATE`): `subscriptions/` (written by `serve`; holds signing
secrets, mode 0600), `delivery/` and `events/` (the per-subscription event log
used by `replay_events`), `daemon.json` (heartbeat), `gaps.jsonl`,
`budget-index.json`, `ledger/` (the pinned ledger copy), `verified/`,
`revoked/` and `title-cache.json`, all written by the daemon except the first. Transcripts are found under `$CODEX_HOME`
(`sessions/`, `archived_sessions/`) and `$CLAUDE_CONFIG_DIR` (`projects/*/`).
A session id that resolves to no file, or to more than one, is rejected.

### Event `session.entry.matched`

Subscription arguments, which are also `test_pattern`'s:

| argument | meaning |
|---|---|
| `session_ids` | 1–20 Codex or Claude Code session ids (UUIDs) |
| `pattern` | Python regular expression, at most 512 characters, searched in each entry's text |
| `roles` | any of `user`, `assistant`, `tool_call`, `tool_result`, `reasoning`, `system`, `agent_message`, `turn_end`; default `user`, `assistant` |
| `case_insensitive` | default `false` |
| `content` | `excerpt` (default): about 600 characters each side of the match; `full`: the whole entry, cut around the match only to fit the 256 KiB body limit |

### Event `session.message.matched`

The same matcher behind a schema shaped like OpenAI's own example: arguments
`session_id` and `pattern`, both required strings, with no other fields
allowed. Only `user` and `assistant` entries are matched, and the payload is
an excerpt. The payload fields are `session_id`, `session_title`, `host`,
`entry_id`, `role`, `timestamp`, `text` and `match`: all strings, all present,
no others. It exists so that a subscriber that rejects the richer schema still
has an event it can subscribe to.

### Event `session.turn.ended`

One event each time a watched session stops and waits for input. Arguments:
`session_ids`, and `envelope` (`basic`, the default, or `full`). The basic
payload carries `event_id` (the same as the
delivery's `eventId`), `session_id`, `session_title`, `host`, `entry_id`,
`turn_id`, `status`, `reason`, `ended_at`, `duration_ms` (`-1` when the host
records none), `final_message` and `final_message_truncated`. The final message
is redacted and cut to 4,000 characters, never through a secret. If the turn had
no final message, `final_message` is empty and the status is still sent.

- **Codex:** a `task_complete` record gives `completed`, or `failed` when it
  carries an `error` object; the reason is then its `codex_error_info`, such
  as `server_overloaded` or `usage_limit_exceeded`. Codex's own turn table
  records these turns as failed; 19 such records were found in 404 rollouts on
  2026-10-31. A `turn_aborted` record gives `interrupted` when its reason is
  `interrupted`, otherwise `failed`.
- **Claude Code:** an assistant text record with `stop_reason: "end_turn"`
  gives `completed`, with no duration. An interrupted Claude turn produces no
  event.
- **`lost`:** after the computer restarts, each watched turn that was open
  and whose transcript was last written before the boot is reported once
  with status `lost` and reason `host_restart: …`. It cannot finish, because
  its process died with the host. This is checked once per boot, and only for
  subscriptions older than that boot.

**Envelope `full`** adds these fields to the payload. Any context that
cannot be gathered is named in `context_errors`, and the event is still sent.

| field | content |
|---|---|
| `subscription_id`, `sequence` | per-subscription numbering for `replay_events`; a jump means missed events |
| `turn_started_at` | from `task_complete.started_at`, else read back to the turn's start |
| `board` | `cursor` (the last row considered) and `revision`. `posted` and `received` are rows posted during the turn by the session or its workers, or addressed to the session. `outstanding` covers open `request`s it made or owes and the claims it holds. Bodies are cut to 300 characters and redacted. |
| `budget` | the session's objective at the turn's end (limits, `hours_deadline`, the grant and deadline fields roots write beside `scope_budget`, `closed`), with `spend` per pool and `coverage` from `usage_ledger.py tree --until <ended_at>` |
| `closure` | the session's workers. `workers_total`, `states`, `sources`, `running` (up to 20) and `dead` (shown running, but not written to since the host booted). `workers_settled` is true when none is running. `costs` is the ledger coverage (`complete`, `partial` or `not_established`). |
| `successors` | adoptions of this session's role on the board. Each is checked: the adoption is on the successor's own topic, the predecessor posted `ending` naming the same handoff digest, and the handoff file hashes to that digest. Whether the gate committed the rotation is recorded only in the gate's state directory, so it is reported as not established. |
| `artifacts` | files the turn's board posts reference, plus the grant's result files beside its `scope_budget` record (`root-result.json`, `closure-cost.json`, `final-recorded-cost.json`, `continuation.json`). Each has sha256, size, mtime and `changed_this_turn`; JSON files also get a redacted summary of `status`, `grant`, `review`, `verdict` and similar keys. |
| `monitoring` | gaps (host restart, sleep, daemon stopped) recorded since the previous delivered event |

Enrichment costs about 0.8 s per event, mostly the ledger run (n = 3 roots, a
read-only dry run on 2026-10-31). The envelope is opt-in because a basic
subscription's payload must keep matching the schema a subscriber already
holds. Whether ChatGPT validates payloads against `payloadSchema` is not
established.

### Event `session.alert`

Raised between turn ends. Arguments: `session_ids`, `kinds` (any of `budget`,
`deadline`, `stalled` and `successor`; default all) and `stall_minutes`
(5–240, default 20). The payload has `alert_id`, `subscription_id`,
`sequence`, `kind`, `session_id`, `session_title`, `raised_at`, `summary` and
`detail`.

- **`budget` and `deadline`:** every 2 minutes, the daemon runs `usage_ledger.py
  control` with its own persisted cursor for each watched session's current
  objective. That is the latest `scope_budget` record naming the session that
  started in the last 24 h and is not closed. Each new ledger event becomes an
  alert: `hours_*` events are `deadline`, and the rest (marks, limit fractions,
  account ceiling, coverage changes) are `budget`. `detail` carries the ledger
  event, the objective and the spend snapshot. The cursor is independent of
  any supervisor's own listener.
- **`stalled`:** the session's turn is open, but neither its transcript nor
  any worker's has changed for `stall_minutes`. Reported once per turn.
- **`successor`:** a new board adoption names the session as predecessor.
  `detail` carries the successor's id and the checks listed for `successors`
  above.

The first pass records what is already true and raises nothing: adoptions
already on the board, and triggers already crossed when the subscription
began. An objective that starts later is reported from its first check.

A turn end reports only that session. A coordinating root can stop while its
workers, which are other sessions, are still running: envelope `full`'s
`closure` reports them.

An *entry* is one content block. For Codex it is a `response_item` line: a
message, function or custom tool call, tool output, reasoning summary or
inter-agent message. For Claude Code it is a block of a `user`, `assistant` or
`system` record: text, `tool_use`, `tool_result` or `thinking`. Its `entryId` is
the line's byte offset, plus `.N` when the line holds several blocks.

Event `data` carries `host`, `sessionId`, `sessionTitle`, `entryId`, `role`,
`tool`, `entryTimestamp`, `content`, `text`, `textStart`/`textEnd`/`entryChars`/
`textTruncated` (which part of the entry `text` is), `match` (offsets within
`text`) and `redactions`.

### Tools

- `list_sessions`: newest first. `query` matches every word against the title,
  or a session id prefix. Titles come from Codex's `session_index.jsonl` and from
  Claude Code's `custom-title` record (preferred) or `ai-title` record. Claude
  titles are cached by byte offset, so a transcript is only read past the point
  already seen. The first scan of this machine's 225 Claude transcripts (578 MB)
  took about 1 s and a repeat call about 0.1 s (one run each).
- `read_session`: entries around an `entryId` (`before`/`after`), or the
  latest `tail` entries. `max_chars` and `text_offset` page through a long entry.
- `test_pattern`: previews the most recent matches in the last 8 MiB of each
  session, without subscribing.
- `budget_status`: a session's current (or latest) objective, with its saved
  limits, deadline, grant and setter, its spend per pool, and the marks or
  limit fractions crossed now (`usage_ledger.py control` without a cursor).
  It starts no model turn.
- `delivery_status`: the daemon's heartbeat age (`alive` when at most 180 s),
  the last 10 monitoring gaps, and each subscription's sequence, pending or
  failing event and expiry. If the call fails, the computer or the tunnel is
  unreachable.
- `replay_events`: a subscription's logged events after a sequence number, in
  order, with full payloads and their outcome (`delivered`, `abandoned` or
  `rejected`). Up to 1,000 rows are kept once the log passes 8 MiB.

### Delivery behaviour

- A new subscription starts at the end of each transcript, or at a supplied
  cursor. A cursor that cannot be honoured is answered with the current end and
  `truncated: true`: one that cannot be decoded, is more than 64 MiB behind, or
  points past the end of the file.
- Each subscription has one event in flight at a time, in transcript order.
  Each event's `cursor` is the position just before that event, so a persisted
  cursor never passes an unacknowledged event. A refresh's response carries the
  same watermark, plus `deliveryStatus`.
- A partly written last line is not read until it is complete.
- Every attempt at an event sends the same body, with the same `eventId` and a
  fresh signature. The body, including any envelope context, is built at the
  first attempt.
- An unreachable or failing receiver (no response, `5xx`, `408`, `425` or
  `429`) is retried after 5 s, 30 s and 2 min, then every 5 minutes until the
  subscription expires. Later events wait behind it, so an outage delays
  events but does not drop them.
- A refusal (any other `4xx`) gets 5 attempts and is then abandoned. So is
  `413`, at once. `410` is not retried. An abandoned event stays in the event
  log for `replay_events`. Three abandoned events in a row suspend delivery
  until the next refresh.
- A subscription that lapses (expired, but kept 24 h) and is renewed without a
  cursor resumes where its delivery stopped, keeping its sequence numbers. A
  renewal with a cursor starts from the cursor.
- Each subscription sends at most 60 deliveries a minute. Further events wait
  and are reported as `throttled`.
- TTL: 24 h by default, clamped to between 5 minutes and 7 days. A request for
  no expiry (`ttlMs: null`) is granted 7 days. An expired subscription is kept
  24 h, then deleted.
- The callback's verification is cached for 24 h per (principal, URL). On
  secret rotation, deliveries are signed with both the old and new secrets for
  1 h.

## Invariants

1. **Only matching entries from named sessions leave the machine.**
   *Reason:* transcripts hold code, credentials and private material, and the
   subscription's session list and pattern are the consent the user gave
   through Dot. *Command:* deliver only entries that match the pattern, from
   the sessions the subscription names. `read_session` and `test_pattern`
   return text only for a session id the caller names.
2. **Known credential formats are redacted in all outbound text; identifiers
   are not.** *Reason:* a subscriber has no use for a key, and a leaked key in
   a webhook log is compromised. A generic high-entropy rule would also blank
   session ids, commit hashes and digests that the subscriber needs.
   *Command:* apply `SECRET_RULES` and the assignment rule to event text and
   `read_session` output, and change neither rule to match UUIDs or hex
   digests. Anything outside those formats is sent as written.
3. **Callbacks go only to verified, public HTTPS endpoints.** *Reason:* a
   subscriber supplies the URL. Without these checks the daemon could be aimed
   at internal services, or at a third party it would then flood. *Command:*
   resolve the host at every delivery, refuse non-public addresses, connect to
   the validated address with the hostname as SNI, follow no redirects, and
   send nothing until the endpoint has echoed a challenge.
   `SESSION_EVENTS_ALLOW_PRIVATE_CALLBACKS=1` lifts these checks for loopback
   tests; do not set it on a running service.
4. **An event grants no authority.** *Reason:* the payload is transcript text
   that any participant in the session could have written. *Command:* describe
   payloads as data, never as instructions, as the event description and
   server instructions do.

## Setup

1. Run the daemon as a long-lived user service, pointed at a released copy so
   that edits to the working tree do not change a running service:
   `python3 -I <skill-dir>/scripts/session_events.py run`. A systemd user unit
   with `Restart=on-failure` is one way to do this. For context, give the
   daemon and `serve-http` the same global options:
   `--board-repository <repo> --budget-ledger <repo>/metrics/scripts/usage_ledger.py
   --budget-ledger-sha256 <sha256 of that file>`. A changed ledger runs only
   after its new digest is configured.
2. Create a Secure MCP Tunnel in OpenAI Platform tunnel settings. The tunnel
   must be associated with your ChatGPT workspace. Then:
   ```bash
   export CONTROL_PLANE_API_KEY=...   # needs Tunnels Read + Use
   python3 -I <skill-dir>/scripts/session_events.py serve-http   # long-lived, like the daemon
   tunnel-client init --profile session-events --tunnel-id tunnel_... \
     --mcp-server-url http://127.0.0.1:8787/mcp
   tunnel-client doctor --profile session-events --explain
   tunnel-client run --profile session-events
   ```
3. In ChatGPT: Plugins → Add custom MCP server → Connection: Tunnel → the
   tunnel id → Authentication: No authentication → Create as a plugin, then
   install it. Ask a Dot to find the sessions with `list_sessions` and monitor
   `session.entry.matched`. Ask the Dot to confirm the subscription it created:
   connecting the plugin alone establishes no monitoring. Afterwards, `status`
   shows the stored subscription.

## Pull filters, for when a webhook subscription cannot be created

A subscriber that can run commands on this machine can read matches directly,
without MCP Events. `subscribe --name N` stores a filter that starts at the
current end of each transcript; the arguments are the same as the event's.
`next --name N --wait S` returns new matches as events with the same `data`, or
`status: "timeout"` after S seconds. The wait uses inotify, so no model runs
while it waits. A returned match is not returned again: the stored position
advances past it. `--peek` reads without advancing, and `--from <cursor>`
re-reads from an earlier event's `cursor`. With `--wait 0` the call returns at
once, which suits a scheduled check. These filters do not involve `run` or
`serve`.

## Live trial, 2026-10-30 (n = 1 account, through the Secure MCP Tunnel)

- ChatGPT (`openai-mcp/1.0.0`) first sent `server/discover` with protocol
  `2026-07-28`. On `-32601` it fell back to `initialize` with `2025-11-25`.
- `tools/list`, `events/list`, `list_sessions` and `test_pattern` all arrived
  through the tunnel and succeeded. The tunnel forwards `events/*` methods.
- A Dot's subscription failed twice ("The task service returned an unexpected
  error"). `tunnel-client`'s metrics show both `events/subscribe` calls, and two
  `events/unsubscribe` calls, answered `409` by `tunnel-client` itself; its log
  has no "forwarded" line for them. That matches its documented stdio
  initialization guard: calls arriving before the shared stdio child's latest
  `initialize` has been followed by `notifications/initialized` are refused with
  `409` and not written to the child. ChatGPT opened two MCP sessions back to
  back each time (13 `initialize`, 9 `notifications/initialized` in the trial).
  The guard applies only to stdio targets, so `serve-http` was added.
- Over `serve-http` the same request succeeded. The Dot's task tool rejects an
  event trigger without `schedule` and `timing_mode`, despite its own guidance
  to omit a schedule. The accepted combination was the trigger plus
  `schedule: ""`, `timing_mode: "condition_watch"` and a `prompt` (the action
  to take when an event arrives). `events/subscribe` arrived with callback host
  `connectors.api.openai.com`, the challenge was echoed, and the first event
  was acknowledged `200` about 0.1 s after the matching line was written
  (n = 1 event).
- The Dot's task-tool discovery kept serving the plugin's earlier event list
  after `events/list` had changed; a new event needs a plugin rescan before a
  Dot can use it.
- That first event matched the user pasting the Dot's own reply, which
  contained the pattern, into the watched session. A watched session that
  receives the Dot's output can trigger itself.

## Not established

- **Whether ChatGPT validates payloads against `payloadSchema`.** This is why
  envelope `full` is opt-in.
- **Closure completeness.** Workers are found only where this machine records
  them. Codex spawn edges, role_host launches (the only record of a
  `claude --bg` child's parent), budget bindings (the only record of
  direct-native threads) and Claude subagent files were all checked for S10 on
  2026-10-31 (n = 1 root, 70 workers). A worker started some other way is not
  seen.

- **The refusal reason on stdio.** The `409`s match the documented guard, and
  the same request succeeded over HTTP. The reason itself was not logged at the
  default level.
- **Refresh and long-run behaviour.** Whether ChatGPT refreshes before
  `refreshBefore` (24 h), and how it handles retries and suspension, has not
  been observed yet.
- **The numbering of every error code except `-32015`.** OpenAI documents
  `-32015` (`CallbackEndpointError`). The others (`-32011` to `-32014`) are
  inferred from the order of the earlier design-sketch revision that `-32015`
  comes from.
- **The modern `server/discover` path.** It is not implemented; ChatGPT's
  fallback to `initialize` made it unnecessary in the trial.

## Limitations

- The matcher sees only what the host writes to the transcript. In one Claude
  Code session (n = 1, 2026-10-30), several multi-paragraph assistant replies
  were stored only as a one-sentence `thinking` summary, so a pattern on their
  wording would not match.
- Claude Code subagent transcripts (`<session>/subagents/`) are not watched
  for matches. They count only as workers in `closure` and `stalled`.
- Closure is a snapshot. A worker can start a queued or recovery turn after
  the root's turn ends. A Codex thread that crashed without writing a terminal
  record looks `running` until the host restarts.
- The current objective is a heuristic: the latest `scope_budget` record that
  names the session, started within 24 h, and is not closed. Many objectives
  are never formally closed, and older ones are not followed.
- The budget index walks the whole state tree: about 1.9 million entries,
  266,105 of them JSON files, 4–7 s per scan on 2026-10-31, in a background thread every 5 minutes. A new record
  can go unseen for up to 5 minutes.
- The state tree holds FIFOs named like JSON files (13 on 2026-10-31, test
  fixtures named `plan.json`). Every file read checks for a regular file and
  opens it non-blocking; an earlier version of the scan blocked on one
  indefinitely.
- Python `re` has no timeout. A pathological pattern can stall the daemon for
  every subscription. Patterns are capped at 512 characters and each entry's
  scan at 1,000,000 characters.
- If the Dot's response writes text matching the pattern into a watched
  session (for example, by delegating to Codex in that session), each response
  can trigger another event. The rate limit bounds the rate, not the loop.
- Waking uses Linux inotify. Without it, the daemon falls back to checking the
  files every 2 s.

## Commands

<!-- cli-synopsis script=skills/session-events/scripts/session_events.py display="python3 -I <skill-dir>/scripts/session_events.py" -->
```text
# global options, given before the subcommand: [--codex-home CODEX_HOME] [--claude-home CLAUDE_HOME] [--state-dir STATE_DIR] [--board-repository BOARD_REPOSITORY] [--budget-ledger BUDGET_LEDGER] [--budget-ledger-sha256 BUDGET_LEDGER_SHA256] [--budget-root BUDGET_ROOT] [--role-host-ledger ROLE_HOST_LEDGER]
python3 -I <skill-dir>/scripts/session_events.py serve [--principal PRINCIPAL]
python3 -I <skill-dir>/scripts/session_events.py serve-http [--listen LISTEN] [--path PATH] [--principal PRINCIPAL]
python3 -I <skill-dir>/scripts/session_events.py run [--poll-seconds POLL_SECONDS] [--max-seconds MAX_SECONDS]
python3 -I <skill-dir>/scripts/session_events.py sessions [--query QUERY] [--host {codex,claude}] [--limit LIMIT]
python3 -I <skill-dir>/scripts/session_events.py match --session SESSION_IDS --pattern PATTERN [--role {user,assistant,tool_call,tool_result,reasoning,system,agent_message,turn_end}] [--ignore-case] [--content {excerpt,full}] [--limit LIMIT]
python3 -I <skill-dir>/scripts/session_events.py status
python3 -I <skill-dir>/scripts/session_events.py revoke --id IDENTIFIER [--allow]
python3 -I <skill-dir>/scripts/session_events.py subscribe --name NAME --session SESSION_IDS --pattern PATTERN [--role {user,assistant,tool_call,tool_result,reasoning,system,agent_message,turn_end}] [--ignore-case] [--content {excerpt,full}] [--cursor CURSOR] [--replace]
python3 -I <skill-dir>/scripts/session_events.py next --name NAME [--wait WAIT] [--limit LIMIT] [--peek] [--from FROM_CURSOR]
python3 -I <skill-dir>/scripts/session_events.py unsubscribe --name NAME
python3 -I <skill-dir>/scripts/session_events.py subscriptions
```
<!-- /cli-synopsis -->

### Optional local session bridge (private candidate)

The command profile is disabled by default. It admits one exact runtime-authored
message into an existing, loaded, dedicated idle Codex session. The local
bearer grants message admission and associated reads only; it does not establish
human identity, approval, a mandate, a spending limit, effect authority, or
permission to delegate. The target's existing authority must independently
cover both the native turn and any effects. Keep other native writers out: the
bridge lock does not fence the daemon's TUI or other clients.

This is an offline implementation candidate, not an activation or live trial.
Before use, the operator must qualify the exact daemon version, target policy,
thread cwd and status, full history pagination, retained client message ID,
resume/start policy and dedicated-session ownership. Repeated identical history
reads cannot by themselves detect a consistently stale initial history; daemon
freshness must be qualified separately. The one recorded daemon
0.166.0 observation is not a general qualification; the installed CLI's 0.164.0
version is a separate observation. Deployment, service/tunnel changes, credential
delivery and a live Dot trial require their own authorization and evidence.

A trusted local owner creates private directories and explicitly initializes an
empty store once. The paths below are operator-chosen local paths; remote tool
arguments cannot select a binary, config, socket, policy or store. Use an
owner-only config and store directory, and reject writable or symlinked parents.
For example, with absolute paths and an already-created private lock directory:

```sh
session-bridge --config /private/bridge/config.json store init \
  --database /private/bridge/store.sqlite --lock-dir /private/bridge/locks \
  --native-socket /private/native/app-server.sock
session-bridge --config /private/bridge/config.json grant preview \
  --request-id UUID --session-id UUID --workspace /absolute/workspace --input-file /private/input.txt
session-bridge --config /private/bridge/config.json grant issue \
  --request-id UUID --session-id UUID --workspace /absolute/workspace --input-file /private/input.txt \
  --admit-until 2099-01-01T00:00:00Z --credential-out /private/new-credential \
  --dedicated-session --reviewed-wire-sha256 REVIEWED_DIGEST
```

Preview prints the exact generated wire input locally. Review it and use its
wire digest in the issuance command; issuance rechecks the file and target and
will not overwrite the credential file. Protect that file and the private wire
input files: the store contains sensitive request material. In the original message-only profile, no remote tool can
issue or revoke a grant. The separately authorized optional provisioning profile
can issue a scoped child grant and revoke it through target closure. `grant revoke --grant-id ID` blocks later
credential-authorized reads and submissions; it does not cancel an already
admitted turn. Admission expiry does not end reads of an unrevoked admitted
operation. Local `operation inspect --request-id UUID` shows safe metadata.

The authoritative database, configuration, locks and private wire inputs must
remain continuous. Missing, corrupt or mismatched state fails closed. Clone,
restore, rollback or replacement requires explicit operator state disposition:
retaining a store ID does not detect lost history in an older snapshot. Do not
resume a cloned or restored store as if it were an ordinary restart. There is no
remote retry, unsafe-clear or automatic timeout release for uncertain native
entry; process exit is not proof that the daemon did not receive a turn.

Only trusted server startup may set all four pins: `SESSION_BRIDGE_BINARY`,
`SESSION_BRIDGE_BINARY_SHA256`, `SESSION_BRIDGE_CONFIG`, and
`SESSION_BRIDGE_CONFIG_SHA256`. Use an absolute owner-controlled executable and
owner-only configuration; a missing or mismatched pin prevents the profile
from loading. The server checks helper protocol and pins and advertises
`submit_session_request`, `read_session_operation` and `read_session_result`
only for the enabled profile. The submitted payload is wrapped in the fixed
runtime provenance envelope; neither that envelope nor its digest confers
human or effect authority. Do not send the bearer credential in the payload.

Results are projected from the exact correlated terminal transcript entry,
redacted before paging. Transcript lag or disagreement remains pending or
unavailable. Known-worker lineage and budget/usage observations have separate
sources and coverage; agent-created process closure is unavailable. Worker
observations report source availability and the budget scan time. A partial or
unavailable source, truncated tree or unknown worker state leaves settlement
unknown even when no running worker is observed. Existing
completion delivery and replay retain their own event IDs, and operation reads
do not invent one. Existing session readers and upstream chat or tunnel logs
may retain text independently of bridge revocation.

### Optional target provisioning

Provisioning is a separate operator-issued capability. It is disabled unless
trusted startup sets `SESSION_BRIDGE_PROVISIONING_ENABLED=1` alongside the four
bridge pins. It advertises `create_session_target`, `read_session_target`,
`issue_session_request_grant`, and `close_session_target`; all action-bearing
tools are writes. An operator must separately start the deterministic Rust
owner; an MCP request never starts it. A provisioning credential is a scoped
bearer capability, not proof of human identity or approval.

The permission fixes one task revision, diagnostic profile, workspace and Git
checkpoint, configuration hashes, one target slot, one request ID and initial
payload digest, admission expiry, close-by deadline, and operator-supplied
budget/clock references. The caller supplies no profile, workspace, model,
configuration, native RPC, or shell command to create a target. Grant issuance
accepts the exact initial text and returns a nonsecret handle; the child bearer
remains in the private bridge store. Submit and operation/result reads accept
exactly one authorization form: the original `grant_credential`, or both the
parent `provisioning_credential` and its `grant_handle`. Mixed forms fail.

The owner holds the native connection and services idle notifications and
WebSocket control frames without a concurrent RPC reader. It records allocation
intent before entry,
and never repeats an allocation whose acknowledgement is unknown. A duplicate
create key observes the same slot; a different key conflicts. A restart marks
lost custody rather than silently reacquiring or recreating a target. Reads
remain available after admission expires, while issuance and submission stop;
revocation blocks reads and admission, but the same capability can request
cleanup. Closure revokes the child and archives only the known owned target;
unknown allocation or closure remains uncertain. The owner does not fence
other native clients. A profile verification timestamp is an observation, not
a guarantee that an external actor cannot later change files or connect.

The policy's budget and first-funded-clock references are operator data. They
are not a spend meter or reserve release mechanism; the supervising owner must
monitor spending and report unavailable settlement as unknown. Synthetic
initialization, when needed for initially unreadable history, is one recorded
empty assistant `output_text`, followed by complete stable native TURN reads;
it is not a model response or authority. Known provisioning and child token
formats are redacted from projected text. Never place either bearer in a
request payload, log, or public artifact.

The operator-facing policy schema, explicit owner startup, recovery limits, and
qualification proposal are documented in the session-bridge component README.
Offline fake-native tests do not qualify a profile, native service, Dot, or a
rendered UI. No production activation follows from enabling this source.
