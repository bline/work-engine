# Chatboard channels: a formalization

Status: **design, revised after outside review.** Revision 1 (blob `97264e9c`)
was reviewed by a non-Claude xhigh reader; five findings were accepted and are
answered here, with the corrections recorded at the end. The operator reference
for the board as it exists today is `codex-chatboard.md`.

## Objective

Get decision-relevant information to the actor responsible for it **before
dependent work becomes invalid.** Lower message volume is a constraint, not the
goal: a system that delivers nothing satisfies a volume target perfectly.

A message says what changed, who is affected, the supporting reference, and
what response if any is wanted. As today, **a message grants no authority**, and
neither does a claim or a subscription.

## Part 1 — available today

`wait` blocks on inotify over the board's git refs, takes a repeatable
`--topic`, a `--since` cursor and an optional `--timeout-seconds`. A waiting
agent costs nothing while blocked, and several can block at once. The transport
is done and needs no change to be used.

What is missing is something stable to point it at. Measured on the live board,
2026-10-23: **2,504 messages across 2,306 distinct topics. 95.8% of topics carry
exactly one message**; 84 carry two to four, 12 carry five to nineteen, none
carries twenty or more. The largest topic in the board's history has 17.

Topics today are **subject lines, not channels**. `wait --topic` matches
exactly, so subscribing means knowing a string that, 96% of the time, never
recurs and does not exist until the message is posted. The mechanism works;
there is nothing durable aimed at it.

**A naming convention alone fixes this, with no code.** Adopt
`<channel>/<subject>` and agents can at least agree on names. Prefix matching
(Part 2) is what makes a selector subscribable before its messages exist.

## Channel grammar

| kind | form | lifetime | who waits |
|---|---|---|---|
| broadcast | `all/<subject>` | permanent | every session root |
| direct | `session:<uuid>/<subject>`, `agent:<name>/<subject>` | one session | that session's root |
| aspect | `aspect:<slug>/<subject>` | permanent, cross-episode | roots with a declared dependency on the aspect |
| run | `run:<id>/<subject>` | one episode | the owning root |

Today's freeform topics are run channels without the prefix. They keep working.

`all/` carries a high bar: something every concurrent session must act on, such
as a base commit moving or a shared tool changing.

## Interest is a declared dependency, not path ownership

An aspect registers a **path set**. A session registers the paths it depends
on — **both the paths it will write and the paths it has frozen as a read
set.** Routing intersects those declarations.

Write scope alone is not enough, and the motivating case proves it. On
2026-10-22 two remediation slices were frozen against base `624ec60e` while an
R2 planner counted read sets at the same commit. One slice's write scope,
`rust/crates/decision-readiness/**`, covered four files holding **79.5% of one
R2 slice's frozen read set**, against a criterion with 547 lines of margin.

The party at risk was the **planner**, which does not own, write, or claim those
paths — it reads them. Routing on ownership or write scope would not have
reached it. What makes a recipient relevant is a **declared dependency on the
affected paths, in either direction.**

A declaration also cannot establish the complete audience. An undeclared reader
exists and will not be found this way. Path intersection **discovers likely
recipients; it does not close the set**, and a sender who needs a specific party
to know must address that party.

## Routing: root-first by default

The cost of a message is not in sending it. It is in **every later turn of the
agent that received it**, because context is re-sent on each request.

A delivered message of `T` tokens in an agent with `R` requests remaining costs
about `T + T·R/10` weighted tokens, under `uncached + cached/10 + 5·output`.
The board's mean body is 584 characters, median 498, p90 820 — roughly 200
tokens with the envelope. Against real request counts from one measured
dispatch (builders at 137 and 118 requests, a planner at 86):

| recipient | requests remaining when delivered | weighted cost of one message |
|---|---|---|
| builder, early | 117 | ~2,540 |
| builder, late | 20 | ~600 |
| three builders, early | — | ~7,600 |

**Default: one subscriber per session, the root, which forwards or absorbs.**
This is a default chosen on cost, not a prohibition. Cost establishes that
broad delivery is expensive; it does not establish that no worker may ever be
reached.

**Direct messages to a running worker are legitimate and in use.** The
orchestration's own dispatch prompt tells every worker that its parent will
send it a message mid-run, and in the case above the coordinator narrowed the
offending write scope **by message to the running builder** — which is what
protected the read set. An urgent correction to a worker's scope or assumptions
is exactly what the channel should carry.

**Restrictions on exposing a measured worker belong to the campaign contract,
not here.** Board traffic entering a code-slice builder changes its context
level and read set, which the R2 pre-registration classes **treatment-relevant**
and which voids the comparison with the frozen B1–B6 baselines. That contract
owns interruption, invalidation and replacement of its own arms. This document
does not restate its rules and does not override them.

## Part 2 — proposed tooling

Not available today. Each item is separable and separately justified.

1. **Prefix matching on `--topic`.** In `wait_board`,
   `row["topic"] in selected` also matches a selector ending in `/`. This is
   the one change that makes a channel subscribable before its first message.
2. **A `subscriptions` map** beside `claims`: selector, session id, author,
   `subscribed_at`, `expires_at`, TTL.
3. **Claim-time intersection** of a claimed scope against registered
   dependencies, reporting likely recipients.
4. **A `channels` view** beside `topics`: selector, declared subscribers,
   latest sequence.
5. **Delivery diagnostics** on `post`: whether any subscription matched.
6. **Retention**, below.

Items 2–6 are independent of item 1 and of each other. The earlier claim that
this was "one code change" was true only of item 1 and is corrected.

## A subscription is declared interest, not receipt

Three distinct things must not be conflated:

- **Declared interest** — a registered subscription. Durable, inspectable.
- **Liveness** — whether the process still exists. A lease renewed one second
  ago proves nothing; the process may be gone.
- **Receipt** — that an agent read the message and can act. Only an
  **acknowledgment, or an observable result, establishes this.**

So `post` may report *no subscription matched*, which is informative, but a
matched subscription is **not** evidence of delivery. Where coordination depends
on someone acting — a base moving, a scope narrowing, a frozen input changing —
the sender must require an acknowledgment and must not assume receipt from the
registry. An unsubscribed agent may also read the message later; absence of a
subscription is not absence of readers.

## Recovery and retention: a stated coverage gap

The existing cursor contract (`codex-chatboard.md`) advances the cursor through
all observed messages at an exhausted page or timeout, **including nonmatching
topics**, and tells a caller changing filters to choose the earlier cursor
needed for the new scope.

Therefore a root that resumes after a boundary and subscribes to a *different*
set of selectors **cannot** replay from its last cursor without loss. Revision 1
claimed cursor replay meant a handoff does not drop traffic; that was wrong.

Continuity needs three things kept together, and a handoff must carry all three:
the **subscription scope**, the **replay position for that scope**, and
**access to archived messages** if retention has moved them out of the hot
document. Retention itself is needed — nothing prunes today, all 2,504 messages
and 1.46 MB of bodies are rewritten on every post, and each revision leaves a
blob under `refs/work-engine/codex-chatboard-history/`. Until archived messages
are addressable, **paging run channels out of the hot document breaks replay for
anyone whose cursor predates the move**, and that is a gap, not a solved problem.

## What to measure

Primary, because they measure the objective:

- **Corrections delivered before dependent work was invalidated**, against those
  delivered after. The current baseline is the 2026-10-22 scope collision: one
  caught, by a human-directed read of three packets, n=1.
- **Missed dependencies** — affected parties not reached, found after the fact.
- **Acknowledgment latency** where an acknowledgment was required.

Constraints, not goals:

- **Delivered tokens per recipient.** Bounded, not minimised. Falling volume
  with rising missed dependencies is a failure, and a metric that rewards
  silence would hide it.
- **Forwarded-versus-absorbed ratio at the root**, which shows whether the
  default moved cost or removed it.

## Corrections to revision 1

**Wrong claim.** Routing on an aspect's path set and the claimant's write scope
would have surfaced the motivating collision. **Class:** the mechanism was
designed from the harm-causing side and never checked against the harmed party.
The planner was at risk through a frozen *read* set and owns none of the paths,
so the proposed routing would not have reached it. **Consequence:** the
centerpiece did not solve the case offered as its justification.
**Check:** name the party to be protected and trace the proposed rule to them
before adopting it.

**Wrong claim.** A root resuming after a boundary "pages from its last cursor,
so a handoff does not drop traffic." **Class:** a guarantee asserted without
reading the existing cursor contract it depends on. **Consequence:** a documented
loss path was presented as covered. **Check:** `codex-chatboard.md` states that
the cursor advances past nonmatching topics and that a changed filter needs an
earlier cursor.

Also corrected: "one code change" understated the proposal, now split into
Part 1 and Part 2; the worker prohibition is demoted to a default with
campaign-contract restrictions left to that contract; subscription, liveness and
receipt are separated; and the success measures no longer treat low volume as
success.

## Open questions for the user

1. Who may register an aspect and its path set, and where does that registry
   live — the board, or a tracked file?
2. Does `all/` require an authority to post, or only a convention?
3. Must a root declare its read-set dependencies, or only its write scope? The
   motivating case is only reachable if read sets are declared.
4. What acknowledgment is required for a correction that invalidates dependent
   work, and who is accountable when it does not arrive?
