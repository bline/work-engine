# Board commands, topics and addresses

Typed flags and the `kinds`, `roles`, `requests` and `adopt` commands named below are in [typed-messages.md](typed-messages.md); "Suggested stable selectors" and "A selector is not free" are in `SKILL.md`.

## What the board gives you today

`python3 skills/durable-state/scripts/codex_chatboard.py --repository . <cmd>`

- **`wait --since <n> [--topic T]… [--timeout-seconds S]`** blocks on filesystem
  notification. It costs nothing while blocked and several waits can coexist.
  Save `next_since` in your continuation and pass it to the next wait **with the
  same filter**; a changed filter needs an earlier cursor, because the cursor
  advances past nonmatching rows. `--topic X` also matches rows addressed `to` X
  (see "Role and session addresses"). It exits **0** when it returns board or
  control events and
  **2** when the timeout fires (`1` on error, `130` on interrupt). Both 0 and 2
  mean a one-shot listener has ended and nothing is listening until you arm
  another. `--follow` instead streams JSON lines and rearms within the process
  until its total timeout, cancellation or error.
- **`wait --exclude-session-id <verified-own-id>`** suppresses that session's
  ordinary board posts from delivery while advancing past them. This avoids
  a redundant wake when the publisher is the listening actor. Other actors can
  publish under the same session ID (including Claude subagents); their ordinary
  posts are suppressed too. Enable the filter only when those posts are not
  needed by this listener or their publishers mark required signals with
  `--wake`. `post --wake` marks an
  intentional signal that bypasses this suppression; topic and cursor filters
  still apply. `read` retains all posts, and local control events remain
  deliverable. The session string is advisory attribution, not authentication.
- **`post --author --session-id --topic --body [--ref]…`** appends a message.
- **`read [--since] [--limit] [--active-claims]`** pages the board. Typed flags and the
  `kinds`, `roles`, `requests` and `adopt` commands are under "Message kinds and typed views".
- **`topics`** lists topics with counts and latest sequence.
- **`claim` / `release`** record a resource claim with a TTL. A claim announces;
  it does not grant.

**`--topic` matches a row's topic exactly,** and it also returns a row
whose `to` list contains the selected string (see "Role and session addresses").
So a durable channel must be a **stable topic
string with the subject in the body** — `aspect:context-lifecycle`, not
`aspect:context-lifecycle/some-subject`, which would create a fresh
unsubscribable topic per message. Of 2,306 topics on the board, 95.8% carry
exactly one message; that is what the stable-topic rule exists to avoid.

### Role and session addresses

`--to` takes `role:<name>` or `session:<lowercase uuid>`. To wait for rows addressed to
you, subscribe with the address as a topic:
`wait --since <n> --topic role:<name> --topic session:<uuid>`. `wait` returns a
row whose `topic` matches or whose `to` contains a selected address, once per row.
How it matched is reported beside the row, not in it: a top-level `matches` key
lists `matched_by` (`topic`, `to` or both) and appears only when some returned row
matched by `to`. A consumer that must rely on the topic test ignores rows whose
`matched_by` lacks `topic`. **Being addressed to a role does not establish that
the recipient holds it, and delivery is not authorization;** `to` routes a wake
and nothing else. A waiter on only its `control:` topic gets no `to` matches.
