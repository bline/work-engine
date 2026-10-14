# Codex orchestration brief

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

High and extra-high effort are worth their cost for design, contract formation
and independent review. For mechanical implementation, tests and validation runs
against an explicit contract, consider medium effort and compare candidate
attempts and findings with the baseline.

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
chosen boundary. The next call opens at about 80K — system prompt, `AGENTS.md`,
tool schemas, re-sent messages — costing four times a normal call because
nothing is cached yet, and the session then spends about thirteen calls climbing
to 116K re-reading doctrine. One cycle costs roughly 2 Plus-units and leaves a
116K floor. What does not survive the emptying is whatever was not written down:
retention across a replacement was 117 bytes for twenty consecutive windows of
one root.

Two consequences. First, maintain the continuation packet **continuously**, not
only when a handoff is planned, because compaction will take the window without
asking and the packet is what the new window reads instead of re-deriving.
Second, a lower handoff trigger is worth nothing by itself: with a 116K floor,
the 39K of headroom under a 155K trigger buys about 34 calls, so replacing three
times as often costs as much as the lower context saves. Lower the floor
first — a sized packet and a sized doctrine reload — and the trigger becomes
worth taking. Measured values and the projection are in the
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
