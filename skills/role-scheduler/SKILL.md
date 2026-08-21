---
name: role-scheduler
description: Reconcile and monitor durable scheduled items for an active, human-supervised agent role. Use at role startup or when users schedule, defer, inspect, approve, or acknowledge work; do not treat scheduling as authority or activate inactive roles.
---

# Role Scheduler

Use the configured durable scheduler as the owner of scheduled obligations.
Model context, transcripts, terminals, and listener processes are not durable
schedule owners.

The scheduler daemon is also a bounded runtime control plane for role
activation, schedule routing, delivery health, and acknowledgements. Domain
artifacts remain with their existing authoritative owners; retain references
instead of copying their truth into the control plane.

Read [references/scheduler-contract.md](references/scheduler-contract.md) when
creating or changing scheduled items, interpreting authority or catch-up
policy, handling listener recovery, or designing a scheduler adapter.

The current local adapter is
[`scripts/role_scheduler.py`](scripts/role_scheduler.py). Agenda, schedule,
wait, and subscription commands start the detached daemon idempotently and
verify readiness before continuing. By default its SQLite database, socket,
PID, and logs live under the repository's Git common directory at
`work-engine/role-scheduler/`, outside the worktree.

## Preserve authority

The scheduler may retain and surface work, but it must not create authority,
activate a role, or execute a consequence. Operate only while the current
session has an authorized human-supervised activation for the scheduled
recipient.

A due time does not grant permission. Apply the item's recorded consequence,
approval, catch-up, expiry, preconditions, and result policies. Auto-approval
applies only to the exact bound action and subject; invalidate it when that
meaning or its authority conditions have changed.

## Reconcile an active role

When a scheduler capability is configured, reconcile the active logical role's
agenda at startup. Distinguish overdue or missed, due now, upcoming within the
useful configured horizon, expired, superseded, blocked, and previously
presented items. Use explicit current time and timezone for relative claims.

Report the agenda compactly before acting. Ask for authority where the item
requires confirmation. An item marked `execute_and_report` with valid current
auto-approval may run without another prompt; a `request_approval` item may not.
Scheduling examples do not create universal rules for task types.

If the configured scheduler is unavailable, report the unavailable capability
when it affects the requested outcome. Do not fabricate a durable schedule or
store a future obligation only in context.

The skill may ship daemon and control scripts, but a host service manager or
host-managed MCP server owns daemon lifetime. Use an available health or
idempotent start capability only within existing host authority. Installing a
persistent service or changing its auto-start policy requires the corresponding
user authorization.

## Listen while active

After reconciliation, establish the configured blocking listener when the host
can deliver its completion back into the active agent. The daemon retains
ownership; the listener carries only a renewable delivery lease.

Prefer a host-visible streaming subscription when available. A blocking pipe
or socket that emits one identified event and exits is a valid route when it
binds to the reconciled agenda revision and the host surfaces its completion.

If the listener disconnects or its completion cannot be observed, reconcile
again before listening. Do not infer that silence means there are no items, or
that terminal output will wake the model without a host delivery path.

When an item arrives, preserve its identity through presentation, claim,
execution or decision, result delivery, and acknowledgement. Never acknowledge
before the protected consequence has actually occurred.

Use `agenda` for startup reconciliation, `wait` for a one-event blocking
listener, and `subscribe` for a streaming terminal. Supply the authoritative
repository and logical-role identities explicitly. Use `schedule`,
`acknowledge`, `cancel`, `status`, and `stop` only when their corresponding
state transition is intended.
