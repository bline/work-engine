# Chatboard channels: a formalization

Status: **design, revised twice after outside review.** Revision 1 (blob
`97264e9c`) drew five findings; revision 2 (`fff72101`) corrected three at the
text level and drew three residuals, answered here. The operator reference for
the board as it exists today is `codex-chatboard.md`.

## Objective

Get decision-relevant information to the actor responsible for it **before
dependent work becomes invalid.** Lower message volume is a constraint, not the
goal: a system that delivers nothing satisfies a volume target perfectly.

A message says what changed, who is affected, the supporting reference, and
what response if any is wanted. As today, **a message grants no authority**, and
neither does a claim or a subscription.

## Part 1 — what works today

`wait` blocks on inotify over the board's git refs, takes a repeatable
`--topic`, a `--since` cursor and an optional `--timeout-seconds`. A waiting
agent costs nothing while blocked, and several can block at once. **The
transport is done; resumption is not.**

A returning `wait` does not resume the waiting agent's reasoning — on the
harnesses in use its output lands in a finished tool cell — so an agent wires
the wait to send a wakeup to its own session. That wiring has a trap. `wait`
exits **0** when it returns messages and **2** when the timeout fires, so the
natural guard on exit 0 drops the timeout case, and because nothing else arms a
listener the subscription ends there: no message, no error, no signal. Observed
2026-10-23 in Codex session `01a1653b`, which armed eleven waits at
`--timeout-seconds 3600`; the arms and the exit handling were read from the
rollout and the exit codes run directly (0 with messages, 2 on timeout, 1 on a
bad argument, `130` on interrupt read in `main`). Each wake cost about **34 Kwt**
across three requests — read the notification, record it, rearm — so eleven
messages over 41 minutes cost about **378 Kwt**, with no other work in the
window and six of the eleven on an `aspect:` channel that session was not
working on (n = 1 session).

Two consequences for Part 2. The subscription cost that matters is the
**wakeup**, not the message, so a selector's price is a model turn per matching
message. And a listener needs a way to rearm across a timeout without spending
one.

What is missing is something stable to point it at. Measured on the live board,
2026-10-23: **2,504 messages across 2,306 distinct topics. 95.8% of topics carry
exactly one message**; 84 carry two to four, 12 carry five to nineteen, none
carries twenty or more. The largest topic in the board's history has 17.

Topics today are **subject lines, not channels**. `wait --topic` matches
exactly, so subscribing means knowing a string that, 96% of the time, never
recurs and does not exist until the message is posted.

**The usable convention today is a stable topic with the subject in the body.**
The topic *is* the channel name and nothing more — `aspect:context-lifecycle`,
not `aspect:context-lifecycle/scope-collision`. Under exact matching the
subject-suffixed form produces a fresh unsubscribable topic per message, which
is the present problem restated. Revision 2 claimed the `<channel>/<subject>`
grammar worked today with no code; it does not, and that is corrected. The
suffix form becomes usable only with prefix matching (Part 2, item 1).

## Channel grammar

The selector below is the whole topic today. With prefix matching it gains an
optional `/<subject>`.

| kind | selector | lifetime | who waits |
|---|---|---|---|
| broadcast | `all` | permanent | every session root |
| direct | `session:<uuid>`, `agent:<name>` | one session | that session's root |
| aspect | `aspect:<slug>` | permanent, cross-episode | roots with a declared dependency on the aspect |
| run | `run:<id>` | one episode | the owning root |

Today's freeform topics are run channels without the prefix. They keep working.
`all` carries a high bar: something every concurrent session must act on, such
as a base commit moving or a shared tool changing.

## Declaring a dependency

**Declare only dependencies whose change could invalidate active work.** Not
every file read. A requirement to declare every read would add noise and
conflate exploration with reliance.

A declaration carries four things:

- **the dependent work** — what becomes invalid,
- **the responsible actor** — who must be told,
- **the source identity** — what exactly is depended on,
- **the lifetime** — when the dependency stops mattering.

The source identity decides whether a change invalidates anything at all. **A
read from an immutable commit does not become invalid because HEAD moved.** A
requirement evaluated against current HEAD — an integration gate, a read-set
line count checked against a size criterion — *can* go stale. Fix-a read
`src/js.rs` at base `624ec60e` and nothing about a later HEAD touches that. The
R2 planner's read-set counts are evaluated at HEAD against a 2,000-line
criterion, so they are stale-able. Only the second kind needs routing.

**Where a frozen read-set manifest already exists, reference it.** Do not
maintain a second inventory. `scout2/result-v1.json` and a planner's
`freeze-check.json` already enumerate the paths and counts; a declaration should
point at one, not copy it.

## Overlap must be visible regardless of arrival order

Checking an intersection only when a scope is **claimed** catches a writer
arriving after a reader's declaration and misses a reader binding a dependency
after the writer's claim.

The motivating case is the second kind, which revision 2 did not see. On
2026-10-22 the two remediation packets were frozen at **19:10:12** and the R2
planner's packet at **19:10:33** — the writers' claims preceded the reader's
dependency binding by 21 seconds, and all three agents launched within 19
milliseconds of each other. The read-set manifest they collided over had existed
since **14:15:48**, nearly five hours earlier, but nothing bound it to active
work until the planner was dispatched. **A claim-time-only check would have
missed this.** What surfaced it was a hand-directed read of all three frozen
packets, after freeze and before any builder wrote.

**The required invariant:** a relevant overlap becomes visible to both parties
regardless of which declaration arrives first. Evaluating on every declaration —
claim *and* dependency binding, in both directions — satisfies it. The mechanism
is an implementation choice; the invariant is not.

A declaration also cannot establish the complete audience. Path intersection
**discovers likely recipients; it does not close the set**, and a sender who
needs a specific party to know must address that party.

Write scope alone is not sufficient interest. In the case above the party at
risk was the **planner**, which does not own, write or claim those paths — it
reads them. Routing on ownership or write scope would never have reached it.

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
A default chosen on cost, not a prohibition. Cost establishes that broad
delivery is expensive; it does not establish that no worker may be reached.

**Direct messages to a running worker are legitimate and in use.** Every
worker's dispatch prompt tells it that its parent will send a message mid-run,
and in the case above the coordinator narrowed the offending write scope **by
message to the running builder** — which is what protected the read set. An
urgent correction to a worker's scope or assumptions is what the channel is for.

**Restrictions on exposing a measured worker belong to the campaign contract.**
Board traffic entering a code-slice builder changes its context level and read
set, which the R2 pre-registration classes **treatment-relevant** and which
voids the comparison with the frozen B1–B6 baselines. That contract owns
interruption, invalidation and replacement of its own arms. This document
neither restates nor overrides its rules.

## Evidence of consequence, not of receipt

Four distinct things, which must not collapse into one another:

- **Declared interest** — a registered subscription. Durable, inspectable.
- **Liveness** — whether the process still exists. A lease renewed a second ago
  proves nothing.
- **Receipt** — that an agent read the message.
- **Consequence** — that the thing the sender needed has happened.

Receipt does not establish consequence. *"Message received"* does not establish
*"scope narrowed."*

**Where further work depends on a consequence, require evidence of that
consequence.** An acknowledgment suffices only when its content establishes it;
otherwise the evidence is an observable result — and the result must be stated
for what it covers.

The case above shows the gap and the care it needs. The coordinator's post
*claimed* the consequence: the four read-set files excluded. An endpoint
`git diff --name-only` over those paths coming back empty does **not** establish
that claim. It establishes only **no net change to those files between the two
compared revisions**. It says nothing about a transient edit later reverted, it
does not show the scope instruction caused the outcome, and it cannot speak to
future writes at all.

What the evidence does support, with its basis named: the branch holds exactly
two commits, and neither commit's diff includes any of the four files; the
worktree is clean at head; and across fix-b's whole transcript, 8 tool calls
mention any of the four filenames, of which 2 are write-capable and both target
`check.rs` and the result file. So: **no committed or recorded write to those
four files during the run, and none outstanding at head.** Future writes remain
outside the evidence, and the attribution to the narrowing message is
correlation with its timing, not a demonstrated cause.

A matched subscription is therefore not evidence of delivery, and `post`
reporting *no subscription matched* is informative but not an absence of
readers: an unsubscribed agent may read the message later.

## Part 2 — proposed tooling

Not available today. Each item is **separately implementable**, though not
independent — item 5 depends on item 2's state.

1. **Prefix matching on `--topic`.** In `wait_board`,
   `row["topic"] in selected` also matches a selector ending in `/`. The one
   change that makes `<channel>/<subject>` subscribable before its first message.
2. **A `subscriptions` map** beside `claims`: selector, session id, author,
   `subscribed_at`, `expires_at`, TTL.
3. **Dependency declarations**, carrying the four fields above and referencing an
   existing frozen manifest where one exists.
4. **Order-independent overlap evaluation** satisfying the invariant above.
5. **Delivery diagnostics** on `post`: whether any subscription matched.
   Requires item 2.
6. **A `channels` view** beside `topics`: selector, declared subscribers,
   latest sequence.
7. **Retention**, below.
8. **A rearm-safe wait.** A `--follow` mode that keeps listening across a
   timeout, so a subscription cannot end on a timeout the caller forgot to
   handle. Until then the caller must loop the wait itself; see Part 1.

Revision 1's claim that this was "one code change" was true only of item 1.

## Recovery and retention: a stated coverage gap

The existing cursor contract (`codex-chatboard.md`) advances the cursor through
all observed messages at an exhausted page or timeout, **including nonmatching
topics**, and tells a caller changing filters to choose the earlier cursor
needed for the new scope.

So a root that resumes after a boundary and subscribes to a *different* set of
selectors **cannot** replay from its last cursor without loss. Revision 1
claimed cursor replay meant a handoff drops nothing; that was wrong.

Continuity needs three things kept together, and a handoff must carry all three:
the **subscription scope**, the **replay position for that scope**, and
**access to archived messages** if retention has moved them. Retention is
needed — nothing prunes today, all 2,504 messages and 1.46 MB of bodies are
rewritten on every post, and each revision leaves a blob under
`refs/work-engine/codex-chatboard-history/`. Until archived messages are
addressable, **paging run channels out of the hot document breaks replay for
anyone whose cursor predates the move.** That is a gap, not a solved problem.

## What to measure

Primary, because they measure the objective:

- **Corrections delivered before dependent work was invalidated**, against those
  delivered after. Baseline: the 2026-10-22 scope collision, one caught by a
  hand-directed read of three packets, n=1.
- **Missed dependencies** — affected parties not reached, found after the fact.
- **Consequence latency** where evidence of a consequence was required.

Constraints, not goals:

- **Delivered tokens per recipient.** Bounded, not minimised. Falling volume
  with rising missed dependencies is a failure, and a metric that rewards
  silence would hide it.
- **Forwarded-versus-absorbed ratio at the root**, showing whether the default
  moved cost or removed it.

## Extracting a skill: a timing constraint

The reviewed objective is suitable for a compact agent-facing skill, with the
corrections, the campaign example and Part 2 left in supporting references.

**Worker-facing extraction must wait for R2 to finish.** Under the R2
pre-registration's standing rule, text a worker reads is
**treatment-relevant**, and adding a coordination skill to the worker read set
pauses R2 or voids the comparison with B1–B6. A skill scoped to **roots and
coordinators only** is a **covariate** under the same rule and may land during
R2, recorded with its commit and bias direction. Classify before extracting.

## Corrections to earlier revisions

**Wrong claim (rev 1).** Routing on an aspect's path set and the claimant's
write scope would have surfaced the motivating collision. **Class:** the
mechanism was designed from the harm-causing side and never traced to the harmed
party, which held a frozen *read* set and owns none of the paths.
**Consequence:** the centerpiece did not solve its own justifying case.
**Check:** name the party to be protected and trace the rule to them.

**Wrong claim (rev 1).** A resuming root "pages from its last cursor, so a
handoff does not drop traffic." **Class:** a guarantee asserted without reading
the cursor contract it depends on. **Consequence:** a documented loss path
presented as covered. **Check:** `codex-chatboard.md` states the cursor advances
past nonmatching topics and a changed filter needs an earlier cursor.

**Wrong claim (rev 2).** The `<channel>/<subject>` convention "fixes this, with
no code." **Class:** a capability attributed to today's tooling without
re-checking the matcher it runs on; exact matching makes every subject a new
topic. **Consequence:** readers would have adopted a convention that delivers no
subscription. **Check:** `wait_board` compares `row["topic"] in selected`.

**Wrong claim (rev 3).** An empty endpoint diff established that the four
read-set files "was touched" by nothing. **Class:** an endpoint comparison read
as a statement about the whole interval and about intent, when it bounds only
net change between two revisions. **Consequence:** a narrow negative was
reported as scope compliance, in this document and in the cohort score.
**Check:** per-commit diffs, worktree state, and a transcript search for
write-capable calls — each with what it does not cover stated.

**Incomplete (rev 2).** Overlap detection was specified only at claim time, which
is order-dependent; the motivating case is the order it misses. Replaced with an
invariant. Also corrected: receipt and consequence separated, with evidence of
consequence required; "independent" narrowed to "separately implementable".

## Open questions for the user

1. Who may register an aspect and its path set, and where does that registry
   live — the board, or a tracked file?
2. Does `all` require an authority to post, or only a convention?
3. What evidence of consequence is required for a correction that invalidates
   dependent work, and who is accountable when it does not arrive?
4. Is a root-and-coordinator-only skill wanted during R2, or is the whole
   extraction held until R2 ends?
