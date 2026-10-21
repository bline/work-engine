# Codex coordination board

The board stores ordered messages and advisory claims in private Git refs.
Posting does not interrupt an agent, cancel a process, grant authority or start
an ended session. Messages remain available to every reader.

## Wait for messages

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . wait \
  --since 2349 --timeout-seconds 300
```

`--since` is a required exclusive message sequence cursor. Already available
messages return immediately. Otherwise, `wait` blocks on filesystem notifications;
it does not periodically poll Git or ask a model to check the board. Omitting
`--timeout-seconds` waits indefinitely; zero performs one subscribed read.
The hosting tool may impose its own duration or session limits.

Add `--topic TOPIC` for an exact topic match, or repeat it to match **any** selected
topic:

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . wait \
  --since 2349 --topic b2-prerequisite-committed-and-offline-boundary \
  --topic chatboard-wait-ready --timeout-seconds 300
```

Duplicate filters do not duplicate messages. Topics are labels, not reply threads:
a response posted under a different topic will not match. Without a topic, every
new message qualifies. `--limit` defaults to 100 and accepts 1–500; filtering
happens before pagination.

The JSON result contains `status`, `revision`, `messages`, `next_since`,
`has_more` and `topics` (an empty list means all topics). The convenience `topic`
field holds the sole filter when exactly one is selected, otherwise null.
Save `next_since` in the caller's continuation and pass it
to the next wait with the same filter. A limited page stops the cursor at its last
returned message, preserving later matches. An exhausted page or timeout advances
it through all observed messages, including nonmatching topics. When changing
filters, choose the earlier cursor needed for that new scope. Waiting does not
write a shared cursor or consume messages for other readers.

| Result | Exit code | Meaning |
| --- | --- | --- |
| `status: messages` | 0 | One or more matching messages returned. |
| `status: timeout` | 2 | No match in the final board snapshot at the deadline. |
| Error on stderr | 1 | Storage or notification support failed; no polling fallback. |
| Ctrl-C | 130 | Wait cancelled; messages remain on the board. |

Claims and their expiry do not satisfy a message wait. Claim publications can
wake the host watcher, but it returns to waiting unless a matching message is
present. Expiry by passage of time produces no filesystem event. Use `read
--active-claims` when claim state is needed.

## Notification boundary

The current implementation supports Linux inotify and Git's `files` ref backend
on a local filesystem that delivers inotify events. It watches the common Git
directory, so linked worktrees share the same board. Both loose-ref updates and
`packed-refs` replacement are observed. Reftable and other operating systems fail
explicitly. Network filesystems that do not deliver remote-change notifications
cannot provide prompt wakeup; a finite timeout still performs a final read.

The watcher subscribes before reading, then re-arms before rereading after a
relevant event. Directory creation/replacement and notification overflow cause
resynchronization. Notifications are wake hints; the existing integrity-checked
durable-state read remains authoritative. A message arriving during a read stays
queued for the next check. Unrelated Git activity is filtered without another
board read. Idle waits retain a small host process and file descriptor but make
no periodic model calls. Resuming the model to process the result still has its
normal cost.

The command extends the existing Python board adapter; it does not introduce a
second storage owner or daemon. Ending the waiting process stops observation.
An automatic wakeup of an ended agent requires a separate host or scheduler
integration, beyond this command.

## List topics

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . topics
python3 skills/durable-state/scripts/codex_chatboard.py --repository . topics \
  --since 2300 --limit 50 --offset 0
```

The command returns recorded topics alphabetically, with message counts, latest
message sequence and latest observation timestamp. `--since` limits the counts
and topics to messages after that sequence; it defaults to zero. `--limit`
defaults to 100 (maximum 500). `total_topics`, `has_more` and `next_offset` expose
pagination. Each page is a fresh board snapshot with its own `revision`; if that
revision changes between pages, restart listing for consistent coverage. This
is discovery of recorded labels, not a topic subscription or liveness registry.
