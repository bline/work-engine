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

`wait_agent` returns as soon as a message arrives. A short timeout therefore
saves nothing and adds a full-context call each time it expires: 1,097 waits
timed out, most at 30–60 seconds, and wait-only calls were about a third of root
supervisors' spend.

- Wait with a long timeout (tens of minutes) instead of repeated short waits.
- Do not interleave `list_agents`, sleeps or status probes while nothing has
  changed.
- When the user is monitoring agents directly, ending the turn after dispatch is
  also valid.

## Messages from sub-agents

Each delivered message wakes the parent at its full context. Ask sub-agents to
message for a final result, a decision or authority request, a blocker, or a
significant evidence change or newly discovered risk that affects a parent
decision, and to report those immediately rather than waiting for a formal
blocker. Routine progress belongs in durable artifacts the parent can read when
it needs it.

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
only the facts that apply; loading everything recreates the context cost. Before freezing a contract or route revision, the builder checks
its premises against them.

## Doctrine and coordination reads

Sub-agents read DESIGN.md as AGENTS.md requires. PHILOSOPHY.md is needed in full
for design, planning and doctrine review, not for bounded implementation. Read
the coordination board as AGENTS.md specifies at startup, after compaction and
before claims or consequential operations, not as a polling loop.

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

## Root context

A root's context grows with every report it receives; roots averaged about 144K
tokens per call. When context is large and no in-flight obligation depends on it,
hand off through Wind Walker to a fresh root with a compact continuation rather
than carrying the whole history.

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
decomposition and discarded tool output; the
[efficiency ledger](../metrics/efficiency-ledger.md) records what each
measurement showed. Track outcomes alongside cost: quota per accepted result, candidate and
remediation attempts, and defects that escape review. A cheaper run that misses
important issues is not an efficiency improvement.
