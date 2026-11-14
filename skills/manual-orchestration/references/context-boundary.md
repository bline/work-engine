# Context boundary: retained or fresh (parents and roots)

A provider can compact or replace context without asking. Recovery through
`$wind-walker` depends on durable continuation meaning; provider-retained state
alone does not establish that required meaning survived. This reference is for
the party that dispatches work or owns a long-lived context; workers have their
own mid-task check in the [worker reference](worker.md#context).

**Who decides.** The party that dispatches the next task decides, because it
knows the task's size and holds the launch authority:

| situation | decider | action |
| --- | --- | --- |
| A retained child is about to receive a substantial new task, including remediation after review | parent (the supervisor) | pre-dispatch check, then `followup_task` or `spawn_agent` |
| A single assignment is outgrowing its window mid-task | worker | mid-task check, then continue or `handoff_ready` |
| Supervisor (root) context pressure | supervisor | assess capacity for the next scope; if transferring ownership, use the user-authorized procedure under Roots |

Load the relevant decision reference before acting. Reading this index does not require loading historical measurements, root-transfer mechanics or alternative host controls.

## Same-session replacement and root transfer

Context lifetime and root ownership are separate decisions. Same-session
compaction can change the working context without launching a successor or
transferring the root. The Roots procedure below governs ownership transfer;
context pressure alone does not require that route.

Wind Walker requires continuation-critical meaning to have durable owners
before intentional context replacement. That protects recovery; it does not
grant permission to invoke a lifecycle control or automatically resume work.
Those operations remain inside the current role, host and user authority.
Preserving a session ID cannot widen authority or reactivate paused work.
Use the conditional [same-session continuity reference](same-session-context.md)
to assess a concrete control and resumption route. Neither this distinction nor
an available RPC socket establishes that a background controller is supported.
Recovery from provider-initiated compaction still follows Wind Walker.

A runtime- or agent-authored wakeup remains continuation material, not fresh
human authorization, even when a transport presents it as user input. Existing
human authority and new human directions still govern. A compaction marker or lower input reading shows a
context event, not that required meaning was recovered. Apply Wind Walker on
recovery and reconcile the durable continuation before dependent work.

Same-session replacement also leaves collection responsibility with this root.
The continuation must retain owners and usable references for outstanding
children and processes. Unavailable notification or collection routes remain
unresolved; an unchanged root ID does not establish their availability.


## Capacity belongs to the next scope

Admit substantial work into retained context. Read [capacity belongs to the next scope](retained-dispatch.md).

## Measured basis

Interpret H2 or change measured operating margins. Read [measured basis](context-measurements.md).

## Pre-dispatch check (parent)

Continue a child with substantial work. Read [pre-dispatch check (parent)](retained-dispatch.md).

## Transfer of an unfinished assignment (parent)

Move unfinished child ownership. Read [transfer of an unfinished assignment (parent)](retained-dispatch.md).

## Roots

Transfer root ownership with the user authorization this contract requires. Read [roots](root-transfer.md).

## Telemetry rules

Provision or interpret context telemetry. Read [telemetry rules](context-telemetry.md).

## Status

Evaluate the registered pilot or its amendment. Read [status](context-measurements.md).
