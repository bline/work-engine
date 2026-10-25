---
name: coordination-channels
description: Keep responsible actors reachable for relevant coordination, including while idle, and deliver information before dependent work becomes invalid using the coordination board's existing capabilities.
---

# Coordination channels

The user has confirmed rollout is unblocked. R2's historical root/coordinator-
only restriction is not a current global loading rule. Active experiments still
own their exposure conditions: adding this skill or its messages can change a
measured environment, and that experiment's disposition governs the change.
A subscription grants communication reach, not authority over another actor.

This capability owns **communication responsibility only.** A message grants no
authority, and neither does a claim or a subscription. The receiver's own
contract decides what it may do.

## The objective

Keep responsible actors reachable for relevant coordination, and get
decision-relevant information to them **before dependent work becomes invalid.**
Availability includes receiving requests, questions, information and handoffs
while idle. Low message volume alone does not establish success; neither does
an uneventful listening period establish failure.

## Your responsibilities

1. **Say what changed, who is affected, the reference, and what response you
   want.** A message without a named consequence is a log entry.
2. **Make a relevant overlap visible regardless of which declaration came
   first.** Check when you claim a scope **and** when you bind a dependency on
   something already claimed. A check done only at claim time misses a
   dependency bound afterwards — which is the order the one known collision
   actually had, the writers' claims landing 21 seconds before the reader's
   dependency. Both directions each time: work that will write what you read,
   and work that reads what you will write.
3. **Address the party you need, by name.** Path or topic matching discovers
   likely recipients; it does not close the audience. An undeclared reader
   exists and will not be found for you.
4. **Where your work depends on someone acting, require evidence of the
   consequence** — not evidence of receipt. See below.
5. **You are the subscriber; forward selectively — not late.** Absorb what
   your children do not need, because a message in a worker's context costs
   tokens on every one of its later turns. But **when a worker's scope or
   assumptions have changed, correct it while it runs.** That is what this
   channel is for, and the one known collision was contained exactly that way:
   a scope narrowed by direct message to a running builder. Choose the cheapest
   route that **arrives in time** — a packet not yet read, a boundary handoff,
   or a direct message mid-run. Cost chooses among routes that work; it does
   not justify a correction that arrives too late.

## Four states, kept separate

| state | established by |
|---|---|
| declared interest | a posted or registered intent to listen |
| liveness | nothing you can read; a process may die a second after any check |
| receipt | the receiver saying so |
| **consequence** | evidence that the thing you needed has happened |

**Receipt does not establish consequence.** "Message received" does not
establish "scope narrowed". An acknowledgment is sufficient only when its
content establishes the consequence; otherwise require an observable result, and
state what that result does and does not cover.

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

Suggested stable selectors: `all` (something every concurrent session must act
on — a base commit moving, a shared tool changing), `session:<uuid>`,
`role:<name>` (both also valid `to` addresses) and `agent:<name>` for direct address, `aspect:<slug>` for a durable feature area,
and a run name for one episode.

**A selector is not free, and its cost is not the message.** The wakeup a
message forces costs a model turn. One observed coordinator spent about
**34 Kwt per wake** — read the notification, record it, rearm — so eleven
messages over 41 minutes cost roughly **378 Kwt** and ran no other work in that
window (n = 1 session, Codex-side, 11 wakes). Six of those eleven were an
`aspect:` channel the session was not working on. Choose subscriptions for your
ongoing role, current dependencies and availability to other sessions. A direct
channel or a channel relevant to that role can matter even without a current
assignment. Use bounded reads for broader awareness where prompt delivery is
unnecessary. Limit irrelevant wakeups without excluding messages that could
reveal new work or dependencies.

## Message kinds and typed views

`post` accepts an optional typed envelope; a row without `kind` reads as `note`.
The registry `skills/durable-state/contracts/codex-chatboard-kinds.v1.json` is
authoritative and this table is a projection of it. Field rules, enums and the
checks run before a row is written are in `docs/codex-chatboard.md`
("Typed messages"); this skill does not restate them.

| kind | consumer | required (registry paths) | writer |
|---|---|---|---|
| `note` | anyone | none | `post` |
| `status` | successors and readers reconstructing a run | `data.subject` | `post` |
| `request` | the addressee; an open-requests view | `to`, `data.ask` | `post` |
| `reply` | the requester | `in_reply_to`, `data.disposition` | `post` |
| `receipt` | the sender, tracking delivery | `in_reply_to` | `post` |
| `notice` | every session the change can invalidate | `data.changed`, `data.action` | `post` |
| `availability` | the successor; the rotation gate; role resolution | `role`, `data.state` | `post` |
| `adoption` | the predecessor's gate; role resolution; peers re-addressing | `role`, `data.predecessor_session`, `data.handoff_sha256` | `adopt` only |
| `correction` | readers of the superseded message | `supersedes`, `data.wrong_claim`, `data.error_class`, `data.consequence`, `data.check` | `post` |

Post flags that build the envelope: `--kind`, `--role`, `--to` (repeatable),
`--reply-to`, `--supersedes`, `--effects-file`, `--data-file`. `post` refuses
`adoption`; `adopt` is its only writer and grants no authority to commit a
rotation. A contract fixes a message's shape and meaning, not whether a
situation calls for one.

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

### Read views

Read-only projections of unauthenticated reports, not committed state:

- **`kinds [K]`** prints the registered kinds (or one); reads no board.
- **`read --kind K --role R`** (each repeatable; the two combine with AND) filters
  before `--limit`. `--kind note` includes rows without a kind.
- **`read --format compact`** prints one tab-separated line per message; a reading
  aid, the JSON row is the record. It is refused with `--active-claims`.
- **`roles [--role R]`** answers what sessions *reported* about each role's
  adoptions and endings, with a state of no_adoption, single, overlap, fork,
  conflict or ended. Where one is reported it names a *reported* holder, never a
  holder or owner (otherwise `not established`); every succession is
  `committed: not established`.
- **`requests [--to ADDRESS]`** lists `request` rows by addressee with their
  replies and receipts and whether each is still open.

Output fields for `roles` and `requests` are documented in `docs/codex-chatboard.md`
("Roles", "Requests").

## Monitor channels and recover coordination state

**Listen according to your ongoing role and availability, including while
idle.** Another session's message may reveal a request, question, new information,
handoff or dependency; none requires you to have runnable work already. An idle
session that remains available to other sessions continues monitoring `all` and
its relevant channels. Idleness alone is not a reason to stop listening. When
that availability ends or transfers, make the change visible to sessions relying
on it rather than silently dropping the listener.

Use reads or timed waits alongside other work, or host-side waiting while idle.
Choose observation timing so relevant messages reach the responsible actor in
time. Declaring selectors or leaving a wait's output uncollected does not
establish that messages were observed. Receiving a message lets you assess its
relevance and the action permitted by your contract; it does not itself grant
authority or require other sessions to block.

**Blocking work is a separate decision.** Suspend a dependent step when an
unresolved dependency requires it; independent work can continue while you
listen. Give `wait` a timeout: without `--timeout-seconds` it blocks indefinitely.
Choose that lifetime for the expected availability or decision boundary, not as
a short heartbeat. Quiet availability should not require recurring model calls:
a 55-second timeout that wakes the model to rearm can resend its full context
about once a minute, even when the input is cached. A host tool's short
collection/yield limit is separate from the listener lifetime; collect or rearm
inside an owned host process/cell when that capability is available.

At startup and after compaction or a handoff, recover the missed messages before
relying on prior coordination state. A bounded `read --since <cursor>` usually
does this without stalling runnable work. Continue monitoring after recovery;
a startup read alone cannot reveal later changes.

A handoff must carry **your selector set and the replay position for that set**
together; a position without its scope cannot be replayed safely.
Record the session exclusion with that scope so the cursor's skipped rows remain
interpretable. A successor selects its exclusion using its own verified identity
and required publishers. If the new delivery scope needs previously skipped
posts, replay from an earlier cursor; the cursor alone cannot recover them.

## Arm a listener so it cannot lapse silently

`wait --follow` owns rearming within a bounded listener lifetime. The caller
still owns collecting its stream and delivering relevant messages into the
session; a terminal or file receiving output is not model delivery. Use it when
the host can consume a running stream. Otherwise keep the one-shot collection
route below. Timeout, cancellation and error end either form of listener.

**Finishing the listener's collection cell does not resume your reasoning.**
On a harness that does not re-enter the turn, arrange for that cell's completion
to send a message to **your own** session. An inner quiet-timeout retry need not
finish the cell or send a message. Mark a delivered completion as a wakeup in the
message itself — it arrives through the same path a user message does, and a
wakeup mistaken for user authority invents work nobody asked for.

**Handle timeout without turning silence into an inference loop.** A guard
that wakes only on exit `0` and abandons exit `2` silently drops the subscription.
Prefer re-running a timed-out wait inside the same owned host process/cell,
carrying its returned cursors, until the chosen availability bound. Wake the
model for returned messages/control events, errors, or that bound's disposition;
quiet host-side retries need no model turn. If host-side rearming is unavailable,
use a suitably long bounded wait and make its end visible. A short timeout is
appropriate when a real near-term decision requires it; repeated empty timeouts
alone do not justify a recurring model wakeup.

Either shape meets the same test: **after any outcome of `wait`, name what is
listening.** If the answer is nothing, your subscription has lapsed, and the
channel cannot tell you so.

## Session control subscription

The wait interface can also deliver local context-pressure observations on
`control:session:<verified-id>` (with `/agent:<id>` for a Claude child).
Enrollment binds the session, transcript, pinned telemetry binary and explicit
planning levels. See [manual control setup](../manual-orchestration/references/context-control.md)
for exact options, ownership, recovery and campaign exposure constraints.

Control events are typed observations in `control_events`, separate from board
messages. They are not posted to the shared board and grant no authority.
Listening to board topics does not enroll context monitoring. At startup or
recovery, a root/coordinator establishes its actual context binding and output
collector, or an owned explicit-check fallback, under the manual context
contract. A quiet listener is not proof of normal pressure or active enrollment.
The listener suppresses unchanged pressure and carries an opaque control cursor
across invocations. A forecast concerns approaching an empirical band, not an
exact reset time. Use the event to inform the owning handoff decision; preserve
explicit checks when observation or delivery is unavailable. Do not infer that
support for control subscriptions has automatically enrolled any agent.

## Not available yet

Prefix matching on `--topic`, a subscription registry, delivery diagnostics,
automatic claim/dependency intersection, and retention are **proposed, not
built** — `docs/codex-chatboard-channels.md` Part 2. Do not write guidance or
packets that assume them.
