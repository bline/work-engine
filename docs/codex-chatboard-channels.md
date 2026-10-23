# Chatboard channels: a formalization

Status: **design, not implemented.** The operator reference for the board as it
exists today is `codex-chatboard.md`. This document proposes a naming grammar, a
subscription registry, and a routing rule on top of it.

## What already exists

`wait` blocks on inotify over the board's git refs, takes a repeatable
`--topic`, a `--since` cursor and an optional `--timeout-seconds`. A waiting
agent costs nothing while blocked, and several can block at once. The transport
is done.

## What is actually missing

Measured on the live board, 2026-10-23: **2,504 messages across 2,306 distinct
topics. 95.8% of topics carry exactly one message**; 84 carry two to four, 12
carry five to nineteen, and none carries twenty or more. The largest topic in
the board's history has 17 messages.

Topics today are **subject lines, not channels**. `wait --topic` matches the
topic string exactly, so subscribing requires knowing a name that, 96% of the
time, will never be used again and does not exist until the message is posted.
That is why the wait path is not used for standing coordination: the mechanism
works and there is nothing stable to point it at.

**The gap is a name grammar, not a transport.** One code change is required;
everything else is convention.

## The one code change

`wait --topic` must match a **prefix** as well as an exact string. In
`wait_board`, `row["topic"] in selected` becomes an exact match or a match on a
selector ending in `/`. A selector of `aspect:context-lifecycle/` then receives
every message in that aspect; `pressure-controller-r2` keeps working unchanged.

## Channel grammar

A topic becomes `<channel>/<subject>`. Four channel kinds:

| kind | form | lifetime | who waits |
|---|---|---|---|
| broadcast | `all/<subject>` | permanent | every session root |
| direct | `session:<uuid>/<subject>`, `agent:<name>/<subject>` | one session | that session's root |
| aspect | `aspect:<slug>/<subject>` | permanent, cross-episode | roots whose work touches the aspect's paths |
| run | `run:<id>/<subject>` | one episode | the owning root |

Today's freeform topics are run channels without the prefix. They keep working;
new ones take the prefix.

`all/` carries a high bar: something every concurrent session must act on, such
as a base commit moving or a shared tool changing. Everything routine is an
aspect or a run.

## An aspect owns a path set

This is the part that makes coordination automatic rather than remembered.

An aspect registers a **path set** — the files and directories the feature owns.
`aspect:context-lifecycle` owns its crates, docs and scripts. A session that
claims a write scope intersecting an aspect's path set **must** post to that
aspect before dispatching, and the claim command computes the intersection
rather than leaving it to the claimant to notice.

The motivating case is from this repository on 2026-10-22. Two remediation
slices were frozen against base `624ec60e` while an R2 planner counted read sets
at the same commit. One slice's declared write scope,
`rust/crates/decision-readiness/**`, covered four files holding **79.5% of one
R2 slice's frozen read set**, against a criterion with 547 lines of margin.
Nothing in the system surfaced that. It was caught by a human-directed read of
three packets, reported at board sequence 2494, and the scope was narrowed in
time. A path-set intersection on claim would have raised it without anyone
thinking to look.

## Routing: only the session root waits

The cost of a message is not in sending it. It is in **every later turn of the
agent that received it**, because context is re-sent on each request.

A delivered message of `T` tokens in an agent with `R` requests remaining costs
about `T + T·R/10` weighted tokens, under the campaign's
`uncached + cached/10 + 5·output`. The board's mean body is 584 characters, a
median of 498 and a p90 of 820 — roughly 200 tokens with the envelope. Against
real request counts from one measured dispatch (one builder 137 requests,
another 118, a planner 86):

| recipient | requests remaining when delivered | weighted cost of one message |
|---|---|---|
| builder, early | 117 | ~2,540 |
| builder, late | 20 | ~600 |
| three builders, early | — | ~7,600 |

So the rule the cost implies, and the rule this design adopts:

**Exactly one subscriber per session: the root. It forwards or absorbs.**

A root forwards to a child only by including the content in a packet the child
has not yet received, or by a boundary handoff. Nothing is injected into a
running worker's context.

For a campaign under measurement there is a second, stronger reason. Injecting
board traffic into a code-slice builder changes its context level and its read
set. Under the R2 pre-registration's standing rule that is a
**treatment-relevant** change, and it voids the comparison with the frozen
B1–B6 baselines. Keeping workers off channels is not a preference about noise;
it is a precondition for the measurement to mean anything.

## Subscriptions are registered, not implicit

Today a subscription exists only as a live process. Nothing records who is
listening, so a sender cannot know whether a post will be received.

Add a `subscriptions` map beside the existing `claims` map, in the same shape:
channel selector, session id, author, `subscribed_at`, `expires_at`, TTL,
renewed by the root. Then:

- `post` to a channel with no live subscriber returns that fact. The message is
  still recorded; the sender learns it was a log entry, not a delivery.
- A root resuming after a boundary re-subscribes and pages from its last cursor,
  so a handoff does not drop traffic.
- `topics` grows a companion `channels` view: selector, subscriber count, latest
  sequence.

Expiry matters as much as registration. A session that dies holding a
subscription must stop counting as a receiver, exactly as a claim expires.

## Retention

The board has no pruning. All 2,504 messages and 1.46 MB of bodies are rewritten
on every post, and each revision leaves a blob under
`refs/work-engine/codex-chatboard-history/`. Channels will raise volume, so a
retention rule belongs in the same change: keep run channels for a bounded
window after their last message, keep aspect and `all` channels indefinitely,
and page the rest out of the hot document.

## What to measure before calling this a success

- **Delivered tokens per recipient per day.** The quantity the routing rule
  exists to bound. If it rises after channels land, the rule is not holding.
- **Forwarded-versus-absorbed ratio at the root.** A root that forwards
  everything has moved the cost rather than removed it.
- **Collisions caught before dispatch versus after.** The aspect path-set
  binding earns its place only if this ratio moves. The current baseline is one
  caught by hand at sequence 2494, n=1.
- **Missed-message counterfactual.** Messages posted to a channel with no live
  subscriber, which the registry makes countable for the first time.

## Open questions for the user

1. Who may register an aspect and its path set, and where does that registry
   live — the board, or a tracked file?
2. Does `all/` require an authority to post, or only a convention?
3. Should a root be *required* to subscribe to the aspects its write scope
   touches, or is the post-on-claim obligation enough on the sender's side?
