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
slice. Track outcomes alongside cost: quota per accepted result, candidate and
remediation attempts, and defects that escape review. A cheaper run that misses
important issues is not an efficiency improvement.
