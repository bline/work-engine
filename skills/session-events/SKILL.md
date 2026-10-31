---
name: session-events
description: Deliver new Codex or Claude Code transcript entries that match a regular expression to an MCP Events subscriber such as OpenAI Dot, as signed webhooks, and let it list, search and read local sessions. An operator-run service; it grants the subscriber no authority over the sessions.
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
  machine.
- **`sessions`, `match`, `status`, `revoke`** are local CLI views: search by
  title, dry-run a pattern (prints event data, sends nothing), inspect stored
  subscriptions without secrets, and refuse a subscription.

State lives in `~/.local/state/work-engine/session-events/` (`--state-dir`,
`$SESSION_EVENTS_STATE`): `subscriptions/` (written by `serve`; holds signing
secrets, mode 0600), `delivery/` (written by the daemon), `verified/`,
`revoked/` and `title-cache.json`. Transcripts are found under `$CODEX_HOME`
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

One event each time a watched session stops and waits for input. The only
argument is `session_ids`. The payload carries `event_id` (the same as the
delivery's `eventId`), `session_id`, `session_title`, `host`, `entry_id`,
`turn_id`, `status`, `reason`, `ended_at`, `duration_ms` (`-1` when the host
records none), `final_message` and `final_message_truncated`. The final message
is redacted and cut to 4,000 characters, never through a secret. If the turn had
no final message, `final_message` is empty and the status is still sent.

- **Codex:** a `task_complete` record gives `completed`. A `turn_aborted`
  record gives `interrupted` when its reason is `interrupted`, otherwise
  `failed`. Across 669 rollouts on this machine (2026-10-30) there were 2,027
  `task_complete` records (66 without a final message) and 24 `turn_aborted`
  records, all `interrupted`, and no failure record of any kind. `failed`
  has therefore never been observed.
- **Claude Code:** an assistant text record with `stop_reason: "end_turn"`
  gives `completed`, with no duration. An interrupted Claude turn produces no
  event.

A turn end reports only that session. A coordinating root can stop while its
workers, which are other sessions, are still running. Overspend or a stuck turn
happens before any turn ends, so alerts for those need a separate subscription.

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
- Each event gets up to 5 attempts over about 13 minutes (after 5 s, 30 s,
  2 min and 10 min), with the same `eventId` and a fresh signature each time.
  `410` is not retried. After `413` or the fifth failure the event is abandoned
  and the watermark passes it; it can still be replayed from an earlier cursor.
  Three consecutive abandoned events suspend delivery until the next refresh.
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
   with `Restart=on-failure` is one way to do this.
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
- Claude Code subagent transcripts (`<session>/subagents/`) are not watched.
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
# global options, given before the subcommand: [--codex-home CODEX_HOME] [--claude-home CLAUDE_HOME] [--state-dir STATE_DIR]
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
