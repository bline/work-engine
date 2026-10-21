# Call cost: every role

A model call is charged for the whole context it carries, not for what it
returns. Across 2,313 calls on 2026-10-13, cost per call rose almost linearly
with context: 0.087 Plus-units at 65K, 0.134 at 137K, 0.185 at 187K, 0.272 at
230K. **Spend is calls times context.** Each rule here removes a call or lowers
what every call carries.

## Do not poll

`wait_agent` blocks until mailbox activity or timeout, and a child's completion
releases it without parent inference. Each timeout that leads to another wait
is a full-context call; in the 2026-10-05 to 10-09 baseline, wait-only calls
were about a third of root supervisors' spend, and on 2026-10-15 two workers
issued 88 `sleep` and 232 `wait` calls.

- Continue independent work while children run. When required results are
  outstanding and nothing independent remains, take one long blocking wait.
- Set the timeout against the child's expected remaining time; a timeout far
  below it produces predictable churn.
- After a timeout with no new information, wait again. Do not interleave
  `list_agents`, sleeps or status probes.
- Use `list_agents` only for a concrete coordination question that the wait
  result, messages and assignment record cannot answer, and record which.
- Ending a turn is not waiting: a completed child does not reliably wake an
  ended parent. Keep the supervising turn active while collection remains.

## Batch independent operations

In the most expensive session of 2026-10-13, 93% of tool calls carried one
invocation at a mean context of 136K, and 339 calls were hash or verify
bookkeeping. Put independent operations in one cell with
`Promise.allSettled` over several `tools.exec_command` calls. For a
verification round, declare a battery with
[`skills/evidence-battery`](../../evidence-battery/SKILL.md): one plan, one
process, one `battery_sha256`. Keep genuinely sequential steps sequential.

## Do not re-establish what has not changed

17% of those hash calls recomputed an already established path set.
`evidence_battery.py --compare` answers an unchanged battery in about 34 tokens
and names only the ids that moved.

## Size requests; a limit is not a size

The exec harness truncates output above its budget and reports only a count.
Ten sessions from 2026-10-11 to 10-13 discarded about 1.40M tokens in 123
truncations; 75 of 76 later truncated calls had already set
`max_output_tokens`. A limit is where output is cut. Size the request itself -
a line range, an outline, a path list, `detail: ids` - and keep the limit as a
guard. For Codebase Memory use `limit`, `fields` or `detail: ids`,
`source_mode: outline` before `full`, `diagnostics: none` on coverage, and
`paths` holding only cited files. Keep `detect_changes` on its default `CALLS`
edges unless documentation reference is the question.

Treat a truncated result as unread: re-request the part needed, and do not cite
a path whose coverage result was truncated.

## Reload in pieces

After a context reset the mandated doctrine reload is the largest read a
session makes; 32% of truncated calls touched doctrine or skill references and
38% came within two minutes of a reset. Load it in sized pieces. A silently cut
DESIGN.md is a reload that did not happen.

## Report what reaches the parent

Message the parent for a final result, a decision or authority request, a
blocker, or an evidence change or risk that affects a parent decision, as soon
as it arises. Routine progress belongs in durable artifacts the parent reads
when it needs them; every message costs the parent its context.
