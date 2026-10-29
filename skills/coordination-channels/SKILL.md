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


## References

- Read [references/board-commands.md](references/board-commands.md) when you run a `codex_chatboard.py` command (`wait`, `post`, `read`, `topics`, `claim`, `release`), choose a topic string, or address a message with `to`.
- Read [references/typed-messages.md](references/typed-messages.md) when you post a message with a `kind`, or use `kinds`, `roles`, `requests`, `adopt` or the typed `read` filters.
- Read [references/session-control.md](references/session-control.md) when you subscribe to, receive or act on `control:session:` context-pressure events.

## Selectors and their cost

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

### Skill release notices

Changed installed skills are announced on the exact `all` topic; `all` is not
a wildcard. A listener using `skill-release` for release notices must also
select `all` to receive changed-skill notices. Keep the selector and replay
position together as described below when changing the filter. Publication,
selector match, collected observation and actual refresh are separate events.

An installed-link switch does not change instructions already loaded into a
running session. Assess the affected skills against guidance you have loaded
and your next dependent work. Before that work, re-read relevant still-installed
guidance, including relevant references, through the current installed links;
retain useful task reasoning. If relevant guidance is no longer installed,
preserve the missing dependency and its owner rather than assuming a re-read.
Irrelevant or unchanged content needs no refresh. Forward relevant corrections
in time under the recipient's contract and any experiment exposure conditions;
the notice grants no authority.

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

## Not available yet

Prefix matching on `--topic`, a subscription registry, delivery diagnostics,
automatic claim/dependency intersection, and retention are **proposed, not
built** — `docs/codex-chatboard-channels.md` Part 2. Do not write guidance or
packets that assume them.
