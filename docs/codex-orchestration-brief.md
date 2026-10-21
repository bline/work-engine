# Codex orchestration brief

> **Superseded 2026-10-21 by
> [the manual-orchestration skill](../skills/manual-orchestration/SKILL.md).**
> Agents do not need to read this document. It is kept as the history and
> derivation of the measurements the skill cites. Its context check-in block,
> its "not a threshold" wording and its unverified root-transfer procedure are
> replaced by the skill's [context boundary](../skills/manual-orchestration/references/context-boundary.md)
> reference.

Operating defaults for a Codex session that coordinates sub-agents in this
repository. They are default routes with their reasons, not contracts: depart
from one when the situation warrants it, and keep every contract, authority
boundary and evidence obligation intact. Measured basis:
[pure-Codex baseline](../metrics/baselines/pure-codex-20261009/README.md),
2026-10-05 to 10-09.

Every model call re-sends the caller's whole context; about 73% of estimated
quota spend was that re-sending (estimated from token counts at assumed cached
and output price ratios). A call that learns nothing new incurs nearly the same
standing-context charge as one that does.

## Waiting

`wait_agent` blocks until mailbox activity or timeout; child completion can
release it without repeated parent inference while it is pending. Each timeout
that leads to another wait requires a full-context model call. In the baseline,
1,097 waits timed out, most at 30–60 seconds, and wait-only calls were about a
third of root supervisors' spend.

Ending a turn is different from waiting. In the
[2026-10-13 native resumption experiment](../metrics/experiments/parent-resumption-20261013/README.md)
on Codex 0.162.1, a completed child left its ended parent idle for the observed
79 seconds; explicit resumption delivered the queued result. A one-hour
`wait_agent` timeout was accepted and released after 15.2 seconds on completion.
This was one nested-parent trial per treatment, not a guarantee across clients.

- Continue useful independent work while children run. When required results
  are outstanding and no independent work remains, prefer a long blocking wait
  within the active tool and instruction limits.
- Set the timeout against the child's expected remaining execution time. A
  timeout substantially below it produces predictable churn: all six 10-second
  waits in the 2026-10-13 worker sample expired, while the supervisors' long
  waits returned on child completion without the requested timeout elapsing.
- **Do not poll for progress.** After a timeout with no new information,
  re-enter a suitably long `wait_agent`; do not interleave `list_agents`, sleeps,
  or status probes. A short timeout is not a reason to inspect the roster.
- Use `list_agents` only to answer a concrete coordination question that the
  current wait result, messages, and durable assignment record cannot answer:
  e.g., reconciling an unexpected agent identity/state, recovering after context
  handoff, or verifying ownership before reassignment. Record what changed or
  which decision needs that read; never call it merely to check whether work
  is "still running." In the measured 2026-10-12 to 10-13 supervisors,
  `list_agents` made up roughly 34% of remaining wait-only **calls** (not spend).
- Before dispatch, designate who collects each required result and who resumes
  the parent after a wait or ended turn. If integration is still required, keep
  the supervising turn active with a supported blocking wait. End the turn only
  when no required collection or integration remains, or when a **verified**
  external continuation mechanism owns resumption; human visibility is not
  automatic wakeup. If neither waiting nor external resumption is available,
  surface the blocked handoff explicitly rather than assuming completion will
  restart the parent.

## Messages from sub-agents

A message can release an active wait or be consumed during ongoing work;
delivery alone does not guarantee a new parent turn. Processing messages incurs
the parent's context cost, though several messages may be consumed together.
Treat completion notification as an execution-control event: the parent should
consume it while already waiting or working, not wake itself repeatedly to ask
whether it has arrived.
Ask sub-agents to message for a final result, a decision or authority request,
a blocker, or a significant evidence change or newly discovered risk that
affects a parent decision, and to report those immediately rather than waiting
for a formal blocker. Routine progress belongs in durable artifacts the parent
can read when it needs it.

## Task briefs

Give each sub-agent a compact brief rather than relying on it to rediscover the
situation: objective, authority and boundaries, exact artifacts and revisions,
acceptance evidence, and the **applicable settled facts**.

Settled facts are constraints already established by accepted slices, accepted
contracts or committed code that bear on this task: for example a schema's
version gates and uniqueness constraints, an actor's concurrency model, or a
published wire or client contract. An independent classification of S4–S8,
counting each independent finding or revision once and excluding later
candidates, bundled and dependent revisions, found about 42% directly reopened
such a fact and a further ~36% were arguably preventable by better contract
formation; ~22% were genuine discovery. For example, S8 v3 contradicted S1's
committed schema and v6 assumed a race S4's single-actor loop rules out. Include
only the facts that apply; loading everything recreates the context cost.
Before freezing a contract or route revision, the builder checks its premises
against them.

### Context check-in block for task briefs

Include this block in manual coordinator, supervisor, worker and reviewer briefs
when adopting the boundary-check pilot below. The parent supplies the verified
identity after launch; a role name or inherited environment variable may identify
another session. This is an operating experiment, not a product retirement rule.
Workers and reviewers are not assumed to read this document: copy the block into
their scoped launch brief or post-launch identity message, with concrete values.
For retained agents, deliver it explicitly and carry it in their continuation
record; confirm receipt at the next task boundary. Do not load this entire brief
merely to convey the block.

> **Session:** `<verified session_meta.id>`; **rollout:** `<exact path>`;
> **parent/collection owner:** `<owner>`; **continuation:** `<path>`;
> **context-status binary:** `<prebuilt absolute path>`; **SHA-256:** `<digest>`.
>
> This is the adopted manual pilot's operating check, not a product retirement
> rule or an override of owning campaign contracts. It tests telemetry-assisted
> judgment, not a threshold or the CLI's isolated causal effect.
>
> At each logical task boundary, before starting another substantial task, run
> the supplied binary with `--session <id> --json`; do not build in the shared
> workspace for each check: build locks and transient edits can disable the
> observer just when concurrent work is busiest. Retain the JSON and record observation time,
> input/window proxy, `latest_request.epoch`, lineage/reset observations and
> **transfer availability** (`available`, `unavailable`, or `unestablished`),
> naming the actual transfer mechanism and owner. When transfer is available,
> record **continue** or **prepare handoff**, with a short reason considering
> next-task size, retained context value and reconstruction cost. Compare with
> the previous boundary: low usage just after compaction does not erase earlier
> pressure. Epoch changes can have causes other than compaction; do not count
> them as successor handoffs. Record economics as `not estimated` unless an
> explicit scenario supplies its assumptions. Put this in the existing
> result or continuation; send a separate message only when parent action is
> needed. Boundary checks expose the choice before compaction makes it for us;
> they are not a per-call monitoring loop or an automatic percentage cutoff.
>
> If transfer is unavailable or unestablished, record **continue—transfer
> unavailable/unestablished**, preserve the reason and observed pressure, and
> exclude it from the pilot's primary handoff-choice assessment (class E).
> This is a constrained continuation, not evidence that retaining context won
> an economic judgment. Missing telemetry and missing transfer are separate.
>
> If identity or usage cannot be established, record **usage unavailable** and
> distinguish launch/tool failure, identity rejection and absent counters;
> do not substitute parent counters or treat missing usage as zero.
> Maintain a recoverable continuation regardless. When replacement is warranted,
> apply Wind Walker and report `handoff_ready` with the artifact path/revision
> and SHA-256, unfinished obligations and live ownership. This requests transfer;
> it does not claim task completion or permission to abandon children/processes.
>
> For a child, the parent verifies the handoff, launches the successor, verifies its actual
> session ID and rollout, and publishes an updated binding before resuming work.
> Preserve predecessor identity and accounting; a fresh session starts from zero.
> Supply the successor its own ID and exact handoff reference, and obtain its
> ownership acknowledgement. Same-session compaction retains the ID; a new
> session does not. Preserve truthful reviewer continuity through replacement.
> A root has no parent: transfer is available only through a separately verified
> mechanism that preserves supervision and collection ownership. Merely spawning
> a child, writing a packet or undergoing compaction does not transfer the root.
> Without that mechanism, retain ownership and report the unavailable route.

The [CLI guide](context-status-cli.md) owns telemetry semantics and metadata-only
child discovery. Its latest-input fraction is a proxy, not live allocation or a
demonstration of savings. The root uses its verified metadata and retains
supervision until a successor or explicit continuation owner can collect results.
The parent provisions a versioned prebuilt binary outside the shared Cargo target,
verifies its digest at delivery, and supplies its path/digest to successors.
Any tool upgrade gets a new binding; agents do not silently rebuild or replace it.
This avoids shared build locks and transient workspace edits disabling observation.

This pilot measures **telemetry-assisted boundary judgment**, not a calibrated
threshold or the CLI's isolated causal effect. It cannot separate the effect of
telemetry from asking an agent to reconsider continuation. Economics is optional:
record `not estimated` when absent, or supply all five CLI restart-cost inputs
and explicit cache/remaining-call assumptions with their basis. Those are
conditional estimates, not measured reconstruction cost or proof of savings.

## Doctrine and coordination reads

Sub-agents read DESIGN.md as AGENTS.md requires. PHILOSOPHY.md is needed in full
for design, planning and doctrine review, not for bounded implementation. Read
the coordination board as AGENTS.md specifies at startup, after compaction and
before claims or consequential operations, not as a polling loop.

## Codebase Memory across worktrees

This installation enables `auto_index`, `auto_watch`, and `watcher_enabled`.
Auto-indexing can bootstrap a project, but a lasting watch belongs to a live MCP
connection rooted at **that worktree**. Querying or manually indexing another
worktree does not register its watch; a one-shot CLI call cannot retain one
after it exits. The main repository and an allocated slice worktree are distinct
projects. A normal MCP client connection rooted at the worktree is sufficient;
a temporary dedicated connection can serve the same purpose when needed, with
an owner responsible for closing only that connection at retirement. Workspace
cleanup does not authorize stopping a shared daemon or another agent's connection.

At worktree allocation or context recovery, distinguish configuration from
observed state: check the exact project root, `index_status` watch status and
reason/strategy, and the current store generation. A watch marked active is
not proof that the latest edit is indexed. After edits, check candidate-file
coverage and content verification; use a bounded workspace reindex if needed
and direct source for changed, unverified, partial, or missed ranges. A manual
`index_repository` with `persistence=false` avoids repository writes but does
not itself retain a watch. Tool availability can change mid-session, so
rediscover the available MCP tools before assuming only a CLI or source fallback
is possible. The [repository Codebase Memory instructions](../AGENTS.md#codebase-memory)
and [repository-search evidence contract](../skills/repo-search/SKILL.md) own
query, coverage, and fallback details; clean coverage remains best-effort, and
text returned by `search_code` remains unverified source text.

## What a call costs

A model call is charged for the whole context it carries, not for what it
returns. Measured across 2,313 calls on 2026-10-13, cost per call rises with
context almost linearly: 0.087 Plus-units at 65K, 0.134 at 137K, 0.185 at 187K,
0.272 at 230K. Calls above 150K were 47% of the round and 58% of its spend. Two
things follow, and they are the whole cost model: **spend is calls times
context**, so the levers that matter either remove a call or lower what every
call carries.

Removing calls is the one available immediately. In the most expensive session of
that round — 150 of the round's 338 units over 1,180 calls — 93% of tool calls
carried a single tool invocation and 60% asked for 2,000 output tokens or fewer,
against a mean context of 136K. It paid a 136K charge to receive 1,800 tokens,
over a thousand times. 339 of its calls were hash or verify bookkeeping: 138
separate `sha256sum` invocations, 42 `jq` reads, 21 `git` reads, each its own
round trip.

Batch independent operations into one cell. `Promise.allSettled` over several
`tools.exec_command` calls is the native form, and
[`skills/evidence-battery/`](../skills/evidence-battery/SKILL.md) is the
declared form for a verification round: one plan of named checks — path digests,
read-only `git`, JSON selections, bounded regex sweeps — one process, one result
with one `battery_sha256`. Genuinely sequential steps stay sequential; a write
must land before its verification. Most verification batteries are not
sequential, they are a list.

Then stop re-establishing what has not changed. 17% of those hash calls
recomputed a path set an earlier call had already established, one seven-path set
fourteen times. `evidence_battery.py --compare` answers an unchanged battery in
about 34 tokens, and otherwise names only the ids that moved. Asking the
environment what it already told you is the same mistake as asking the scheduler
what the wait result would have said.

## Tool output the model never receives

The exec harness truncates a cell's output above its budget and reports only the
count of discarded tokens. Across ten sessions from 2026-10-11 to 10-13 it
recorded 123 truncations across 110 tool results, discarding about 1.40 million
tokens, and 44 of those 110 results were Codebase Memory output. One
implementation agent lost every coverage result it took, in single payloads of
13.9K and 23.0K tokens, so it held no verified freshness for any path despite
fourteen coverage calls.

The lever is asking for less per call, not making more calls. Paging is the
recovery, not the goal: a truncated batch costs its own full call and then the
re-requests to recover what was lost, so a request that fits under the budget is
the cheapest form, and batching up to that budget is right. Every Codebase
Memory tool takes the parameters that make a request fit: `max_output_tokens`
(default 3200, 2500 for snippets), `limit`, `fields` or `detail: ids` to drop
columns, `source_mode: outline` before `full`, `diagnostics: none` on coverage,
and `paths` holding the files actually cited rather than every file touched.
Printing a tool's own schema or echoing a whole response consumes the same
budget as evidence and establishes nothing.

**Setting a limit is not sizing a request.** In the five hours after this
guidance was recorded, 75 of 76 truncated calls had already set an explicit
`max_output_tokens`, and truncation per model call did not fall. A limit is the
point at which output is cut, so `cat DESIGN.md` under a 15,000 token limit
loses the tail exactly as it would with no limit. Size the request to the
budget — a line range, an outline, a path list, `detail: ids` — and set the
limit as the guard against being wrong about the size, not as the plan. Checking
the size first with `wc -l` is worth doing and is not sufficient on its own; one
session did that and truncated anyway.

The reload after a context reset is where this bites hardest. 32% of truncated
calls touch DESIGN.md, PHILOSOPHY.md, this brief or a skill reference, and 38%
are issued within two minutes of a reset, because the mandated reload is the
largest single read a session makes and it recurs once per context window. Load
it in sized pieces that fit, and treat a truncated reload as a reload that did
not happen: a session that proceeds on a silently cut DESIGN.md is working
without the doctrine it just paid to read.

Keep traversals narrow for the same reason. `detect_changes` already defaults
`edge_types` to `CALLS`; widening it to `MENTIONS` to pick up documentation adds
2,544 edges in this repository, 80% of which land on `File` nodes rather than
symbols, against 2,702 doc references that do not resolve at all (1,569 of them
ambiguous). That is volume without behavioral precision: a `MENTIONS` edge
records that a document names code, not that the code governs or affects it.
Widen the edge set only when documentation reference is the actual question.

Truncation is silent in its effect. The agent sees a warning, not the missing
content, so a conclusion drawn afterwards can rest on evidence that was never
read. When a result reports truncation, re-request the specific part needed
rather than proceeding, and do not cite a path whose coverage result was
truncated.

## Effort and model

Use [model-selection guidance](model-selection.md) for role-specific choices.
The user's 2026-10-20 selection is **`gpt-6-sol` at medium effort for builders**,
including implementation, integration, tests and ordinary remediation. Move
existing builders at a completed handoff; record the requested and observed
configuration of the successor. This selection does not change retained reviewer
or architecture-role settings, or a live trial's provider configuration.

Recent measurements show similar weighted tokens per call across Sol effort
levels; they do not establish equal quality or total task cost. Compare calls
to acceptance, repair rounds and escaped findings. Higher effort for design or
review remains role guidance, not a measured guarantee of a better result.

## Worker-owned handoff pilot

For the current manually supervised Codex builders, try earlier context
replacement without a supervisor monitoring loop. This is an operating pilot,
not a Work Engine context-retirement policy or a measured optimum. It does not
change the slice-builder skill's same-identity workflow or its terminal receipt
schemas; campaign adoption needs a separate continuation-protocol change.

The first worker did not hand off before context pressure: the soft observed-token
trigger was unavailable to it. The current [manual decomposition
experiment](manual-decomposition-pilot.md) therefore assigns a coherent bounded
outcome to each fresh builder. At that outcome boundary, return the result;
the supervisor evaluates it and refreshes evidence before dispatching the next
packet. An unfinished worker can still return a safe handoff when needed.

The collaboration interface alone did not expose worker usage in that initial
trial. The [context-status CLI](context-status-cli.md) now reads explicitly bound
rollout counters. For the current manual pilot, include the task-brief check-in
block above and make the continue/handoff decision at logical task boundaries.
An unavailable or rejected observation remains unavailable; it does not justify
guessing an identity. Keep monitoring loops out of the workflow. Neither the
CLI nor this experiment establishes a context ceiling or optimal reset threshold.

Keep a compact continuation file current as decisions and validation state
change. Before yielding, apply Wind Walker: the next worker must be able to
continue without meaning available only in the departing context. Carry the
objective and authority; source/worktree identity and uncommitted changes;
settled decisions and rejected approaches worth preserving; exact evidence and
test-result pointers; unfinished work, failures and pending review; and the next
useful action. Aim for about 500–1,000 tokens plus links, without omitting a
required obligation to meet that target. Preserve ownership of any running
process or child; defer replacement if that ownership cannot transfer safely.

The handoff uses the existing completion notification:

1. The worker writes the durable continuation and ends with `handoff_ready`, its
   path/revision, and a short account of remaining work. This is an unfinished
   assignment, not successful slice completion or an audit receipt.
2. The supervisor collects that event while doing independent work or using a
   long `wait_agent`. It reads the compact handoff, checks that ownership and
   pending obligations are covered, then launches a fresh builder with
   `fork_turns="none"`, `gpt-6-sol`, and medium effort. It passes the continuation
   reference and scoped assignment rather than copying the former conversation.
3. The supervisor verifies the successor's actual session ID and rollout and
   updates the versioned handoff binding, retaining the predecessor identity.
   It passes the successor its own ID for telemetry and the exact continuation.
   The successor acknowledges the assignment, binding and continuation at entry.
   The supervisor records the new owner on that event; the old builder receives no
   further work. The same retained reviewer remains responsible for its review
   episode unless its own contract requires a change.

Keep the supervisor turn active until required results are collected. Do not use
an idle-agent message as a launch: use `followup_task` for an intended retained
continuation and `spawn_agent` for a fresh context. Routine progress messages,
roster probes and 60-second status loops add no handoff evidence. The parent
still pays for collecting and dispatching at an actual handoff; this removes
monitoring calls, not all supervision cost.

Evaluate the pilot from existing receipts and session logs after meaningful
work: handoff/opening context, reconstruction calls, calls and weighted cost to
accepted work, repair rounds and escaped findings. Record unavailable values
as unavailable. Compare the extra handoff/reconstruction cost with avoided
context re-send before claiming savings. Worker completion and context handoff
remain different outcomes throughout.

## Root context and supervision ownership

A root's context grows with every report it receives; roots averaged about 144K
tokens per call in the baseline, and recent supervisors reached 166K–180K.
Context re-send compounds the cost of every coordination call, even when much
of it is cached. **Check context size at each dispatch/integration boundary**;
for a 258K window, plan a handoff around 150K (roughly 60%) and execute at the
next safe boundary if the remaining work does not justify carrying that history.
These are operational triggers, not hard limits; record why a handoff is deferred.

Use Wind Walker to pass a compact, durable continuation to a fresh root. Carry
only the current objective, settled authority/constraints, assignment-to-owner
map, required results and acceptance gates, outstanding decisions/obligations,
verified evidence/artifact pointers, and the next actions. Re-read source
artifacts as necessary rather than copying entire transcripts or tool outputs.
Verify that the new root has accepted supervision before the old root relinquishes
it. Do **not** strand in-flight children or assume a new root automatically
inherits their completion notifications. If ownership transfer cannot be
verified, retain the existing supervisor and schedule another safe boundary.

**The same packet serves the ceiling, which is a handoff nobody planned.** These
sessions run with auto-compaction effectively disabled, so a window that reaches
the limit is emptied rather than summarized. Every ~50–80 minutes a long-running
supervisor reaches it: 76 observed replacements all fired at 208–244K, none at a
chosen boundary. The next call opened at about 80K when this was measured — system prompt,
`AGENTS.md`, tool schemas, re-sent messages — costing four times a normal call
because nothing is cached yet, and the session then spent about thirteen calls
climbing to 116K re-reading doctrine. That opening is now 25.1K and the climb to
116K takes about eighty calls. One cycle costs roughly 2 Plus-units. The 116K is the level the window
settles at, which still measures 115.6K; the level it *opens* at is now 25.1K,
not the 80K below (see the 2026-10-16 addendum). What does not survive the emptying is whatever was not written down:
retention across a replacement was 117 bytes for twenty consecutive windows of
one root.

Two consequences. First, maintain the continuation packet **continuously**, not
only when a handoff is planned, because compaction will take the window without
asking and the packet is what the new window reads instead of re-deriving.
Second, a lower handoff trigger was worth nothing by itself *while the floor
was 116K*: the 39K of headroom under a 155K trigger bought about 34 calls, so
replacing three times as often cost as much as the lower context saved. Lower
the floor first — a sized packet and a sized doctrine reload — and the trigger
becomes worth taking. **This arithmetic is wrong, and the
2026-10-16 addendum supersedes it: 116K is the level a window settles at, not
the level it opens at. The opening is 25.1K, so a cycle is about 145 calls
rather than 34 and the extra replacements cost about 2 units, not 17.** Measured values and
the projection are in the
[efficiency ledger](../metrics/efficiency-ledger.md) under Per-call cost.

Use a durable, compact assignment record as the deterministic source for known
owners, expected deliverables, and received results. Update it on actual
assignment, message, completion, and handoff events; don't repeatedly call a
model or `list_agents` to rediscover unchanged execution state. Judgment about
results remains with the supervisor; roster maintenance does not need inference.

## Validation scope

Before declaring a candidate ready, run its required gates and validate affected
downstream consumers, including those reached through shared contracts and
schemas. Shared-boundary changes require the applicable joined-workspace
qualification. Report any unavailable or incomplete gates explicitly. A schema
bump in S8 broke 14 S7 admin tests that lifecycle-service checks alone did not
exercise.

## Measuring the effect

After a run, compare against the baseline with the scripts in
`metrics/baselines/pure-codex-20261009/scripts/`: wait-only calls and timeouts,
root share of spend, re-read share, and settled-fact contradictions per accepted
slice. `metrics/scripts/` adds per-session profiles, coverage outcomes, cost
decomposition and discarded tool output; use
`metrics/scripts/supervision_profile.py` for dispatch-normalized supervision
measurements when available. The
[efficiency ledger](../metrics/efficiency-ledger.md) records what each
measurement showed. Report **units, waits, messages, and status probes per
dispatch**, timeout rates, context tokens per call, and whether a parent resumed
after completion. Report two per-call numbers with them: **tool invocations per
call** and **output budget requested per call**, whose starting values are 93%
single-invocation and a 1,800 token median. They are the adoption check for
batching, and they are countable from the rollout without interpretation. Keep same-definition comparisons separate from broad
coordination categories and account for task-mix differences: the 2026-10-12 to
10-13 same-supervisor observation reduced units/dispatch from 1.82 to 1.21,
but it was not a controlled same-workload experiment. Track outcomes alongside
cost: quota per accepted result, candidate and remediation attempts, and defects
that escape review. A cheaper run that misses important issues is not an
efficiency improvement.

## Addendum, 2026-10-16: the opening fell, so the trigger arithmetic changes

Measured on three live supervision trees — 3 roots, 10 workers, ~5,400 calls,
~650 Plus-units over the 20 hours from 2026-10-15T18:04Z — compared against two
earlier trees on identical definitions. The sessions were still running, so
call counts drift slightly between readings.

### What 116K actually was

The section above reports a 116K floor. That number is the level a window
**settles at**, and it is still correct: the mean context within a window
measures 115.6K. What is not correct is using it as the level a window
**opens at**. The first call after a replacement is **25.1K** (p25 23.6K, p75
27.6K, max 34.2K), and the climb from there is concave — 6,214 tokens per call
over the first step, 1,358 by call 13, 545 by call 120, reaching 116K at about
call 80.

That distinction is the whole trigger argument. The arithmetic above took 39K of
headroom under a 155K trigger and got about 34 calls per cycle. With a 25.1K
opening the headroom is 130K and a cycle is **145 calls** (measured median),
so replacing under a 155K trigger costs about **1.4x as many replacements, not
3x**. At a measured 0.163 units per replacement call that overhead is **close to
2 units** against a bill of 650 — it is not the obstacle the earlier paragraph
assumed.

### What the trigger is worth

Calls above 150K were 28% of calls and 40% of spend. Two bounds, because the
answer depends on an assumption the data does not settle:

| trigger | saving, no growth model | saving, linear growth |
|---|---|---|
| 170K | −2.8% | −13.1% |
| **155K** | **−4.7%** | **−18.1%** |
| 120K | −13.0% | −29.6% |

The left column recharges every call that exceeded the trigger at exactly the
trigger, which is the least favourable outcome that still respects it, and adds
the extra replacement cost. It is a floor, not an estimate. The right column
assumes context grows linearly between opening and trigger; that model
reproduces the bill as actually run to 0.6%, but growth is concave, so it
overstates the saving for any *lower* trigger. A pooled median trajectory would
settle this and does not: beyond about call 50 only slow-growing windows
survive, and the median turns down from survivorship rather than from
behaviour.

**Take the trigger, and expect single digits rather than the optimistic
figure.** The direction is certain — no call can cost more than the trigger
allows — and the replacement overhead is small enough that trying it is cheap.
Do not push it arbitrarily low: the opening itself creeps within a session, one
worker going 23.6K to 26.4K over seven windows and another 27.5K to 32.4K over
three, so re-read the opening rather than assuming 25K.

### Progress across three eras

Identical measurement over one pre-brief tree, one post-brief tree, and last
night:

| | 10-07/08 | 10-11/13 | 10-15/16 |
|---|---|---|---|
| spend | 1,654 u | 992 u | 650 u |
| calls | 12,953 | 7,054 | 5,404 |
| mean context | 129K | 135K | 118K |
| real tool work | 69% | 81% | **84%** |
| wait-only share of calls | 13.2% | **5.0%** | 9.6% |
| messaging | 13% | 4% | 4% |
| opening after replacement | 27.9K | 43.7K | **25.1K** |
| trigger | 210K | 213K | 209K |

Two things are working. Coordination overhead has been squeezed out steadily —
real tool work rose from 69% to 84% of spend, and messaging fell from 13% to 4%.
And the opening came down, after regressing badly in the 10-11/13 tree, where
it reached a median 43.7K and a worst case of 81.5K.

One thing regressed. **Polling nearly doubled last night against the post-brief
tree**, from 5.0% to 9.6% of calls, concentrated in two workers that issued 88
`sleep` and 232 `wait` calls. The brief's guidance was adopted and then partly
lost. A poll pays the full standing-context charge to learn nothing; ask the
scheduler once, with a real timeout.

The trigger has not moved in any era: 210K, 213K, 209K. Nothing is replacing at
a chosen boundary anywhere, which is why this addendum exists.

### Did the provider change what it charges

No, not detectably. Within the Pro plan, where comparing points per weighted
token needs no assumption about plan multipliers, the three weekly windows that
were neither clipped at 100% nor short of a denominator give **6.600, 8.064 and
6.764** Plus-units per million weighted tokens. Last night sits between the two
earlier windows. The spread between windows is wider than any change, so a
charging change on the order of 10% would not be visible here, and none larger
is present.

A clipped window understates the price, because spend continues past 100% while
the counter cannot, so only unclipped windows carry this comparison.

### Calibration, corrected

The ledger price of 7.826 Plus-units per million weighted tokens **overpredicts
by about 16%**; the measured Pro windows give a median 6.764. Read absolute unit
totals and percentages about a sixth lower until the ledger is refitted. Ratio
results such as the trigger table are unaffected, because the error is
multiplicative.

An earlier reading of this put the overprediction at 25%. That came from a
20-hour sub-window whose first sample already stood at 13%, so the tokens that
produced those points sat outside the denominator. A weekly window measured from
its own reset removes that error, and is the form to use.

The cached price itself remains unidentified. Three attempts — hourly buckets
within one window, 59 clean days, and the weekly series — were all degenerate,
returning negative weights, because cache share sits at 0.97 to 0.99 nearly
everywhere and leaves cached and uncached collinear. The product of price and
cache weight is pinned; the split is not.

### Free instruments, and one that does not exist

`token_count.rate_limits` carries **`used_percent` and
`rate_limit_reached_type` on every call**, with a stable `resets_at` identifying
the weekly window. This is the account's own charge and costs nothing to read.
Report `used_percent` at session start and end alongside the per-dispatch
numbers, and prefer a whole window measured from its reset over any shorter
delta. Note that `resets_at` jitters by a second or two between requests, so
group by the minute.

`rate_limit_reached_type` was null in all 4,975 samples: no throttle was
reached. Do not read that as provider reliability. **Rollouts record no error
payload type at all** — no `error`, no `stream_error`, in any of 257 sessions —
and no terminal log is retained, so provider-side stream and capacity failures
leave no durable trace. A question about provider reliability needs an
instrument created in advance; it cannot be answered from rollouts after the
fact.

### Work against cost

The brief asks for quota per accepted result. The 10-15/16 window produced **10
commits and 30,772 lines of Rust across 86 files** — 21,583 implementation,
9,189 tests — for a measured **48 points of a weekly Pro quota**, a full weekly
quota every 1.74 days at that pace. Per-tree attribution is not separable from
the rollouts, because roots and workers discuss each other's commits, so this is
a tree-level aggregate.

Spend follows the work, not the supervision: the three roots took 19% of the
bill and the ten workers 81%, with two implementation workers alone taking 56%.

### What a child agent should carry from this

- Replace context at a chosen boundary rather than riding to the ceiling. Every
  era measured replaced at 209–213K and none at a chosen boundary.
- Keep the continuation packet current, not boundary-triggered. A richer packet
  is affordable: retention content does not inflate the opening.
- Do not poll. This is the one place where last night was worse than a week
  earlier.
- Report `used_percent` at start and end, per weekly window. It is free and it
  is the only ground truth for what the work actually cost.
