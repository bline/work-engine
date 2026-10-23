---
name: coordination-channels
description: Get decision-relevant information to the responsible actor before dependent work becomes invalid, using the coordination board's existing capabilities.
---

# Coordination channels

**Load this at root or coordinator scope only.** It is not part of any worker's
read set and must not be referenced from a worker packet, a shared instruction,
or anything a worker inherits. Workers are reached by their parent directly, not
by subscribing. (Reason in `docs/codex-chatboard-channels.md` §Extracting a
skill.)

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
2. **Before you claim a scope, look for who depends on it.** Overlaps matter
   regardless of which declaration came first, so check both directions: work
   that will write what you read, and work that reads what you will write.
3. **Address the party you need, by name.** Path or topic matching discovers
   likely recipients; it does not close the audience. An undeclared reader
   exists and will not be found for you.
4. **Where your work depends on someone acting, require evidence of the
   consequence** — not evidence of receipt. See below.
5. **Absorb what your children do not need.** You are the subscriber; forward
   into a child only through a packet it has not yet received, or at a boundary
   handoff.

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

## Subscribe at startup, re-subscribe after a boundary

Wait with your selectors at startup and after compaction or a handoff. A handoff
must carry **your selector set and the replay position for that set** together;
a position without its scope cannot be replayed safely.

## Not available yet

Prefix matching on `--topic`, a subscription registry, delivery diagnostics,
automatic claim/dependency intersection, and retention are **proposed, not
built** — `docs/codex-chatboard-channels.md` Part 2. Do not write guidance or
packets that assume them.
