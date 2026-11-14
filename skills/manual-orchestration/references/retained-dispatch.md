# Capacity for retained and unfinished assignments

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: admitting substantial new work into retained context, continuing a child, or transferring unfinished child work.

When using the historical H2 recommendation, read [its measured basis and status](context-measurements.md) for the trigger, band floor and qualification limits. The current assignment owns configured deadlines; historical levels do not set them.

## Capacity belongs to the next scope

For Codex sessions with `token_budget` enabled, apply the
[recovery guidance](same-session-context.md#codex-token-budget-refresh-deadline)
and any deadline set by the owning session or campaign configuration. The H2
fresh-child recommendation below does not require replacing an owner that can
refresh and recover in place. The dispatcher still owns the next assignment
and its capacity assessment; an ordinary retain decision cannot waive an
explicit configured deadline.

A decision to finish bounded work covers that work, not a later substantial
assignment. Before admitting substantial new work into a retained context,
the decider assesses usable current telemetry against the expected work,
result collection, and the room needed to preserve unfinished obligations.
Record the observation, the scope being admitted and the continuation choice
with its reason in the existing assignment or continuation record. Unknown
growth or unavailable telemetry remains uncertainty; it is not spare capacity.
For a parent, the existing pre-dispatch record carries this scope and decision;
no second record is needed.
The choice of retention, decomposition or transfer remains judgment under the
owning workflow, including the root's user-mediated transfer boundary.

This applies to a root accepting a new implementation task as well as a parent
dispatching remediation. Finishing an urgent bounded operation above a planning
level can be reasonable without making the same context suitable for the next
task. An unchanged pressure level may produce no new control event; listener
silence therefore cannot extend an earlier capacity decision to new scope.

In the 2026-10-21 case that motivated this, a parent sent `followup_task` to an
idle child reading 190,902 of 258,400 without checking it, and the child
compacted 26 seconds later ([timeline](../../../metrics/probes/auth-worker-dispatch-timeline-20261021.md)).
The child's own boundary check could not have prevented it: it had already
returned its result and did not know the next task.


## Pre-dispatch check (parent)

Before sending a retained child a substantial new task (`<skill-dir>` is the
manual-orchestration skill's directory):

```
python3 -I <skill-dir>/scripts/child_context.py \
  --binary <pinned path> --sha256 <digest> \
  check --session <child id> --rollout <child rollout> \
  [--next-task-tokens <estimate>] --out <assignment dir>/context-check-N.json
```

**H2:** retain only if the child's latest input is below the trigger **and**,
when you can estimate it, input plus the next task's growth stays below the
band floor. Otherwise launch a fresh child with `fork_turns="none"` and a
packet. Record the decision (`retain` or `fresh`), the check's path and a
one-line reason in the assignment record. A recommendation you overrule needs
a stated reason; that is evidence, not a violation. Parent estimates of a
task's growth are not yet calibrated: on 2026-10-22 a remediation estimated at
40K grew about 61K (n=1). Until more data exists, use the largest growth
observed for comparable work, not a guess. A child that has compacted
reads low, but holds only what its continuation kept; weigh that, not just the
number.

**Remediation** follows the same rule. For a builder over the trigger, the
dispatching parent writes the remediation packet from the builder's result, its
continuation and the review findings, and gives it to a fresh worker; the
finished builder is not asked to write a handoff. Inside a configured Slice
Builder campaign its same-identity contract governs instead. A retained
reviewer's replacement follows its own review contract and truthful continuity
(supervisor reference).

`tree --parent-thread <id>` checks every child at once (the CLI's own
`agents --with-usage` gives the raw readings without the assessment). Use it to
answer a dispatch question, not as a status loop.


## Transfer of an unfinished assignment (parent)

When a worker reports `handoff_ready`:

1. Read the handoff; check ownership and pending obligations are covered.
2. Launch a fresh worker, `fork_turns="none"`, with the active model selection,
   the continuation reference and the scoped assignment, not the former
   conversation.
3. Verify the successor's actual session ID and rollout, update the versioned
   binding with predecessor identity retained, send the successor its identity
   binding and the exact continuation, and obtain its acknowledgement.
4. Record the new owner. The old worker gets no further work. Its accounting
   stays with the task.

Defer the transfer if ownership of a running process or child cannot move
safely, and record why.
