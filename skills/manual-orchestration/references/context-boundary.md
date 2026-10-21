# Context boundary: retained or fresh (parents and roots)

A context that reaches the ceiling is emptied without asking. Recovery through
`$wind-walker` and a durable continuation works, but whatever was not written
down is lost, and the session pays to re-derive it. This reference is for the
party that dispatches work or owns a long-lived context; workers have their
own mid-task check in the [worker reference](worker.md#context).

**Who decides.** The party that dispatches the next task decides, because it
knows the task's size and holds the launch authority:

| situation | decider | action |
| --- | --- | --- |
| A retained child is about to receive a substantial new task, including remediation after review | parent (coordinator or supervisor) | pre-dispatch check, then `followup_task` or `spawn_agent` |
| A single assignment is outgrowing its window mid-task | worker | mid-task check, then continue or `handoff_ready` |
| Root context pressure | root | record only; transfer is out of scope |

In the 2026-10-21 case that motivated this, a parent sent `followup_task` to an
idle child reading 190,902 of 258,400 without checking it, and the child
compacted 26 seconds later ([timeline](../../../metrics/probes/auth-worker-dispatch-timeline-20261021.md)).
The child's own boundary check could not have prevented it: it had already
returned its result and did not know the next task.

## Measured basis

Pre-compaction input readings across both 2026-10-21 orchestration trees
(48 threads enumerated by `agents --recursive`, completeness
`partial_or_ambiguous`): **children 171,541-239,002, median 199,918, n = 33;
roots 171,267-224,762, median 208,631, n = 9**, all on 258,400-token windows
([derivation](../../../metrics/probes/compaction-band-20261021.json), restate
with `metrics/scripts/compaction_band.py` as events accumulate). A level
reported against the window overstates headroom: 69% looks comfortable but
already sits inside that band. A context opened after a replacement measured
about 25K on 2026-10-16 and settles near 116K; that a *fresh launch* opens
similarly is `unverified`, though its standing floor - system prompt, tool
schemas, repository instructions - is the same.

The trigger, **150,000 of 258,400 (about 58%)**, is the planning level used for
roots since 2026-10-13. Here it is a **pilot hypothesis (H1), not a product
rule or a calibrated optimum.** Its value is that it can be refuted, where an
undefined "when warranted" produced no handoff decisions in the 2026-10-21
pilot.

## Pre-dispatch check (parent)

Before sending a retained child a substantial new task:

```
python3 -I skills/manual-orchestration/scripts/child_context.py \
  --binary <pinned path> --sha256 <digest> \
  check --session <child id> --rollout <child rollout> \
  [--next-task-tokens <estimate>] --out <assignment dir>/context-check-N.json
```

**H1:** retain only if the child's latest input is below the trigger **and**,
when you can estimate it, input plus the next task's growth stays below the
band floor. Otherwise launch a fresh child with `fork_turns="none"` and a
packet. Record the decision (`retain` or `fresh`), the check's path and a
one-line reason in the assignment record. A recommendation you overrule needs
a stated reason; that is evidence, not a violation. A child that has compacted
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

## Roots

Root transfer is **out of scope** until one verified trial preserves
supervision and collection ownership; a trial needs separate authorization.
Merely spawning a child, writing a packet or compacting does not transfer the
root. At dispatch and integration boundaries the root records its reading and
`continue - transfer unavailable` once, keeps its assignment record and
continuation current continuously, and keeps its own context small by
delegating reads (coordinator reference). Root records are pressure
observations, not handoff decisions.

## Telemetry rules

- Provision the binary once per tool version with
  `scripts/provision_context_status.sh`; it builds outside the shared Cargo
  target, pins by digest under `~/.local/state/work-engine/tools/` and records
  the source commit. Send path and digest to every child in its identity
  binding. Never rebuild in the shared workspace to check context: build locks
  disable observation when work is busiest.
- Identify sessions by verified `session_meta` lineage, never by role name or
  an inherited `CODEX_THREAD_ID`, which can name the parent.
- Unavailable usage is `usage_unavailable` with its class - launch or tool
  failure, identity rejection, absent counters. Never substitute parent
  counters or treat it as zero.
- `epoch` also changes on model, window or availability changes; count
  compactions from lineage, not epoch, and never count an epoch as a handoff.
- The fraction is a proxy for request input, not live allocation. Record
  economics as `not estimated` unless a scenario supplies all five restart-cost
  inputs to the CLI ([guide](../../../docs/context-status-cli.md)).

## Status

H1 changes the instruction the 2026-10-21 pilot registered. Testing it needs a
new cohort, opened by a measurement-role amendment to that
[pre-registration](../../../metrics/probes/handoff-boundary-pilot-prereg-20261021.md)
before the first decision; it cannot be scored against the closed cohort. The
[diagnosis](../../../metrics/probes/handoff-boundary-why-no-handoffs-20261021.md)
records why the earlier block produced no handoffs.
