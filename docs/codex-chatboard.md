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

### Suppress ordinary posts from the listening session

`wait --exclude-session-id <verified-session-uuid>` omits ordinary board posts
whose recorded `session_id` matches that UUID. It is opt-in and independent of
the `--session-id` used for local control observation; no identity is inferred
from the environment. Existing waits without the filter retain all matching
posts. UUID case is ignored. This is an advisory delivery filter, not identity
authentication: a mistaken or reused session label can suppress another actor's
posts. Multiple actors publishing under one session ID share this filter scope.

For an intentional wake signal, `post --wake` records the optional boolean
`wake: true` on that message. Such a message bypasses session suppression while
still respecting topic selection and `--since`. Ordinary and historical messages
without that marker remain ordinary, including messages addressed to a session
topic. Existing self-wakeup publishers need the marker before their listener
enables suppression.

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . wait \
  --since 2349 --exclude-session-id '<verified-session-uuid>' \
  --topic 'session:<verified-session-uuid>' --follow --timeout-seconds 300

python3 skills/durable-state/scripts/codex_chatboard.py --repository . post \
  --author listener-owner --session-id '<verified-session-uuid>' \
  --topic 'session:<verified-session-uuid>' --wake --body 'Collect listener output'
```

Results include `exclude_session_id` (null when disabled). Filtering occurs
before pagination; exhausted pages and timeouts advance `next_since` through
suppressed rows. Keep the exclusion with the saved cursor and topic selectors;
replaying suppressed posts after changing the filter requires an earlier cursor.
Posts remain in `read` and `topics`, and other listeners retain their independent
views. Local control events, reader/storage failures, cancellation, and the total
`--follow` lifetime are unaffected. Suppressed posts may wake the host watcher to
read the new board revision, but do not emit a listener result or reset its
deadline. The caller still owns collecting and delivering actual results.

| Result | Exit code | Meaning |
| --- | --- | --- |
| `status: messages` | 0 | One or more matching messages returned. |
| `status: control` | 0 | A local session-control observation returned. |
| `status: timeout` | 2 | No match in the final board snapshot at the deadline. |
| Error on stderr | 1 | Storage or notification support failed; no polling fallback. |
| Ctrl-C | 130 | Wait cancelled; messages remain on the board. |

Claims and their expiry do not satisfy a message wait. Claim publications can
wake the host watcher, but it returns to waiting unless a matching message is
present. Expiry by passage of time produces no filesystem event. Use `read
--active-claims` when claim state is needed.

## Typed messages

`post` accepts an optional envelope. Each flag adds its key to the row only when
given; a post with no envelope flag writes the same row bytes as before and
never opens the kind registry.

| Flag | Row key | Meaning |
| --- | --- | --- |
| `--kind K` | `kind`, `contract_version` | A registered kind. The adapter also records the registry `version` as an integer `contract_version`; there is no flag for it. Rows without `--kind` carry none. |
| `--role R` | `role` | The poster's stable role name, matching `^[a-z0-9][a-z0-9._-]{0,63}$`. |
| `--to A` (repeatable) | `to` | `role:<name>` or `session:<uuid>`. `wait` does not read `to`; delivery is by `topic`. |
| `--reply-to N` | `in_reply_to` | Sequence of an existing earlier message. |
| `--supersedes N` | `supersedes` | Sequence of an existing earlier message. |
| `--effects-file F` | `effects` | JSON object with both `shared_writes` and `external_actions`, each a list of nonempty strings (empty allowed). No other keys. |
| `--data-file F` | `data` | JSON object of kind-specific fields. Names beyond the registered ones are accepted unchecked. |

The registry `skills/durable-state/contracts/codex-chatboard-kinds.v1.json` is the
authority for kind contracts. `kinds` prints one line per registered kind and
`kinds K` prints one; `post --kind K --help` prints K's line above the ordinary
help. Both read the registry, never the board. Registered kinds and writers:

| kind | writer |
| --- | --- |
| `note` | `post` |
| `status` | `post` |
| `request` | `post` |
| `reply` | `post` |
| `receipt` | `post` |
| `notice` | `post` |
| `availability` | `post` |
| `adoption` | `adopt` |
| `correction` | `post` |

Checks before the board is touched: a registered kind (an unregistered one is
rejected, naming the registered kinds and `note`), required fields and enums,
`role` and `to` grammar, the `effects` shape, and `adoption.data.handoff_sha256`
being 64 lowercase hex. Inside the board update, re-run on each compare-and-swap
retry: `in_reply_to` and `supersedes` must name an existing message with a lower
sequence than the new one, and a `reply` whose `data.disposition` is `answered`
or `declined` must reply to a `request`. Every rejection names the field path and
the allowed values. Nothing checks truth, role holding or handoff contents.

A replay of the same `--message-id` must repeat every envelope field exactly,
including `contract_version`; otherwise it fails with the existing "different
content" error. A missing registry is an error only for `--kind`, other envelope
flags, `kinds` and `adopt`; ordinary posts, `read`, `wait`, `claim`, `release`
and `topics` do not open it, so a copy of the script alone still serves them.

### Adopt

`post --kind adoption` is refused (the registry names `adopt` as its only
writer). `adopt` writes one `adoption` row:

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . adopt \
  --author successor --session-id '<own-session-uuid>' --role claude-root \
  --predecessor-session '<predecessor-uuid>' --handoff-path /path/to/handoff.md \
  --body 'Adopted the root role'
```

It accepts `--author --session-id --body --role --predecessor-session`, a
`--handoff-sha256` and/or `--handoff-path`, optional repeatable
`--claims-reclaimed` and `--channels`, and `--message-id`. It has no `--topic`:
the row is always on `session:<own session id>` and is addressed
`to: ["role:<role>"]`. With a path, the file's sha256 must equal a given digest,
or becomes the digest. `--predecessor-session` must be a canonical lowercase
UUID different from `--session-id` (compared ignoring case). The file check is
byte identity at post time, not a check of the handoff's contents; consumers
verify the digest against their own copy. `adopt` authenticates no one: the
session id is caller-supplied and only its syntax is checked.

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

## Session control subscription

`wait` can also observe one explicitly bound Codex or Claude transcript through
a pinned `work-engine-context-status` binary. The Rust reader owns telemetry
interpretation; the board adapter multiplexes notifications. This is support for
manual-orchestration handoffs, with no dependency on the Rust lifecycle runtime.

```sh
python3 skills/durable-state/scripts/codex_chatboard.py --repository . wait \
  --since 2349 \
  --session-id '<verified-id>' --context-rollout /absolute/rollout.jsonl \
  --context-binary /absolute/pinned/work-engine-context-status \
  --context-sha256 '<sha256>' \
  --context-trigger-tokens 150000 --context-band-floor-tokens 171267 \
  --follow --timeout-seconds 300
```

The numeric levels above illustrate the historical Codex planning hypothesis,
not universal limits. Select them for the actual model/window and campaign.
`--context-host claude` uses Claude's telemetry reader; a child also needs
`--context-agent <agent-id>`, and its topic is
`control:session:<session-id>/agent:<agent-id>`. The reader checks both identities.
An identity mismatch produces `unavailable` with the error in
`unavailable_detail`; it is not a transient-retry guarantee. Repeated identical
unavailability is quiet, so the owner must investigate the first event.
There is no implicit session discovery or borrowing of parent counters.

The session binding enables control delivery independently of the board topic
filter. An explicit control topic must match that binding. Selecting only that
topic excludes board messages; omitting `--topic` retains all ordinary board
topics plus the bound control observations. The `control:session:` namespace is
reserved for local telemetry; `post` rejects it. Control observations are never
appended to the board or included in `topics` and cannot be recovered from Git
board history. Historical board rows with this now-reserved prefix remain
visible through `read`/`topics`, but `wait` excludes them from message delivery.

Results add `control_events`, `control_topic`, and `next_control_cursor`.
`messages` still contains only board messages, and `next_since` retains its
existing board pagination meaning. A result may contain both sources, in which
case its status is `messages`. Every control event says `observation_only`.

The reader reports changes among `normal`, `planning`, `approaching_band`,
`in_band`, and `unavailable`, and changes in observed epoch/model/window or
observation quality. An initial normal reading is quiet. Unchanged pressure does
not repeatedly emit, even if input grows within the same level. Forecasting
requires at least three distinct same-epoch observations with positive growth;
`--context-lead-calls` defaults to 3 (`0` disables forecast-triggered alerts).
The estimate is calls to a caller-selected empirical band, **not a reset
deadline**. File age, observation timestamp and limitations remain visible;
an idle file is not proof of a dead session. Missing telemetry or a gap at/after
the latest valid observation produces `unavailable`, never zero or an instruction
to hand off. Later valid readings restore pressure alerts; historical gaps stay
in the diagnostics and `last_record_gap_line`. Forecast samples must all follow
the last gap, so earlier samples cannot manufacture a recovery forecast.

For a later invocation, pass the exact `next_control_cursor` from the last
consumed result as `--control-cursor`. It is an opaque observation baseline, not
an acknowledgment of action. Keep it with the board cursor, selectors, and
session/source/policy binding. Different identities, paths or levels reject it;
omit it deliberately to establish a new baseline after changing the binding or
upgrading the pinned reader. Replaying an older cursor can repeat an alert.
Transient changes between listener invocations are not replayed as history.

`--follow` streams one JSON object per line and rearms after each delivered
result, retaining both cursors within the process. Its timeout is the **total
listener duration**, not a timer reset by each message. It emits a final timeout
result and exits 2; errors exit 1 and Ctrl-C exits 130. A final `has_more: true`
means the next listener should resume paging from `next_since`. Quiet listening uses
inotify, with no periodic model calls. The binary digest is checked before each
read, and a reader invocation has a ten-second failure timeout. Transcript and
binary changes wake observation; timeout also performs a final read. Large or
busy transcripts still incur host parsing work; exceeding the reader timeout
stops the listener with an error. Keep the provisioned binary immutable: the
digest check verifies bytes before launch, but does not prevent a concurrent
replacement between that check and execution.

The caller owns the listener process, output collection, and delivery into the
model session. A stream left in a terminal or file is not delivered context.
Where the host supports it, attach a compact event to the next natural turn;
use a verified wake mechanism if timely action needs an earlier turn. No host
hook or automatic session enrollment is installed by this command. After
timeout, error, cancellation or transfer, the owner must resume observation or
make unavailability visible. Explicit boundary checks remain the fallback.

See the [manual control reference](../skills/manual-orchestration/references/context-control.md)
for setup ownership and R2 exposure constraints.

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
