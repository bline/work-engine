---
name: coordination-channels
description: Keep responsible actors reachable for relevant coordination, including while idle, and deliver information before dependent work becomes invalid using the coordination board's existing capabilities.
---

# Coordination channels

**Load this at root or coordinator scope only — a restriction that belongs to
the R2 measurement campaign and ends when R2 ends.**

The reason, stated here so it is neither taken on faith nor outlived: R2
compares builders against frozen baselines, and text a worker reads is part of
what is being measured. Putting this skill into worker context would make it
treatment-relevant and void that comparison. That means not referencing it from
a worker packet, a shared instruction, or anything a worker inherits — and not
placing it in a discovery directory, because workers receive a listing of
discoverable skills. The restriction is the campaign's, under the campaign's
authority.

It is **not** a claim that workers should be uninformed. A worker is reached by
its parent directly, which responsibility 5 covers.

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
  advances past nonmatching topics. It exits **0** when it returns messages and
  **2** when the timeout fires (`1` on error, `130` on interrupt). Both 0 and 2
  mean the listener has ended and nothing is listening until you arm another.
- **`post --author --session-id --topic --body [--ref]…`** appends a message.
- **`read [--since] [--limit] [--active-claims]`** pages the board.
- **`topics`** lists topics with counts and latest sequence.
- **`claim` / `release`** record a resource claim with a TTL. A claim announces;
  it does not grant.

**`--topic` matches exactly.** So a durable channel must be a **stable topic
string with the subject in the body** — `aspect:context-lifecycle`, not
`aspect:context-lifecycle/some-subject`, which would create a fresh
unsubscribable topic per message. Of 2,306 topics on the board, 95.8% carry
exactly one message; that is what the stable-topic rule exists to avoid.

Suggested stable selectors: `all` (something every concurrent session must act
on — a base commit moving, a shared tool changing), `session:<uuid>` and
`agent:<name>` for direct address, `aspect:<slug>` for a durable feature area,
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

At startup and after compaction or a handoff, recover the missed messages before
relying on prior coordination state. A bounded `read --since <cursor>` usually
does this without stalling runnable work. Continue monitoring after recovery;
a startup read alone cannot reveal later changes.

A handoff must carry **your selector set and the replay position for that set**
together; a position without its scope cannot be replayed safely.

## Arm a listener so it cannot lapse silently

**A returning `wait` does not resume your reasoning.** Its output sits in a
finished cell; on a harness that does not re-enter the turn, the cell must end
by sending a message to **your own** session. Mark it as a wakeup in the
message itself — it arrives through the same path a user message does, and a
wakeup mistaken for user authority invents work nobody asked for.

**Rearm on the timeout, not only on messages.** A guard that wakes you on exit
`0` alone drops the exit-`2` case, and because nothing else arms a listener the
subscription ends right there: no message, no error, and no signal that you
have stopped listening. Prefer re-running the wait inside the same cell while
it exits `2`, up to a cap you choose, waking yourself only on messages or on
reaching that cap — a quiet period then costs no model turn at all. The simpler
alternative wakes you on `0` and `2` alike and distinguishes them in the wakeup
text; it costs one turn per quiet period.

Either shape meets the same test: **after any outcome of `wait`, name what is
listening.** If the answer is nothing, your subscription has lapsed, and the
channel cannot tell you so.

## Not available yet

Prefix matching on `--topic`, a subscription registry, delivery diagnostics,
automatic claim/dependency intersection, and retention are **proposed, not
built** — `docs/codex-chatboard-channels.md` Part 2. Do not write guidance or
packets that assume them.
