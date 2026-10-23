---
name: coordination-channels
description: Get decision-relevant information to the responsible actor before dependent work becomes invalid, using the coordination board's existing capabilities.
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

Get decision-relevant information to the actor responsible for it **before
dependent work becomes invalid.** Silence is not success: a session that
delivers nothing meets a volume target perfectly and fails this objective.

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
  advances past nonmatching topics.
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

## Recover coordination state at startup and after a boundary

**Recover the state; do not necessarily block for it.** `wait` with no
`--timeout-seconds` blocks indefinitely, so an empty channel can stall work you
could be doing. A bounded `read --since <cursor>` is usually the right move at
startup and after compaction or a handoff. `wait` suits a session whose next
step genuinely depends on someone else — and then give it a timeout. Whether to
block, read once, or leave a wait running beside other work is your judgment.

A handoff must carry **your selector set and the replay position for that set**
together; a position without its scope cannot be replayed safely.

## Not available yet

Prefix matching on `--topic`, a subscription registry, delivery diagnostics,
automatic claim/dependency intersection, and retention are **proposed, not
built** — `docs/codex-chatboard-channels.md` Part 2. Do not write guidance or
packets that assume them.
