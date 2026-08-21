# Scheduler Contract

Read this reference for schedule mutation, authority interpretation, recovery,
or adapter design. The canonical architectural proposal is
[`ideas/role-aware-agent-scheduler.md`](../../../ideas/role-aware-agent-scheduler.md).

## Ownership boundary

The always-alive scheduler daemon owns schedule identity, timestamps, recipient
binding, delivery state, and acknowledgement state. It persists due items while
no role is active. It does not launch agents or execute scheduled work.

The active role owns judgment and actions within current authority. A listener
owns only delivery connectivity. The host owns routing a completed wait back
into model execution.

The daemon consequently forms a bounded runtime control plane. It may own role
activation leases, schedule routing, listener cursors, claims, delivery health,
and acknowledgements. It references rather than duplicates authoritative Git,
campaign, test, review, and other domain artifacts.

## Logical and runtime identity

Address items to stable logical identity:

```yaml
recipient:
  repository_id: work-engine
  logical_role: slice-supervisor
  logical_agent_id: optional-stable-specialization
```

Bind delivery to ephemeral activation identity:

```yaml
activation:
  session_instance_id: runtime-specific
  lease_epoch: 1
  authorized_by: human-session
  expires_at: timestamp
```

Never use a model name, provider session, process ID, terminal ID, or context
window as the durable recipient.

## Scheduled item

A useful item shape is:

```yaml
id: stable-item-id
recipient: {}
intent: Human-readable semantic consequence
due_at: timestamp-with-timezone
on_due: execute_and_report
approval:
  policy: auto_approve
  granted_by: human
  granted_at: timestamp
  expires_at: timestamp-or-null
  scope: {}
catch_up:
  policy: run_when_next_active
  max_lateness: duration-or-null
result_policy: summary_and_failures
preconditions: []
evidence_refs: []
status: scheduled
```

Supported values belong to the implementing contract. Useful semantic modes
include:

- consequence: `execute_and_report`, `request_approval`, `notify_only`, or
  `prepare_then_request_approval`;
- approval: `require_confirmation` or `auto_approve`; and
- catch-up: `ask_if_missed`, `run_when_next_active`,
  `run_if_within_window`, or `expire_if_missed`.

Do not infer unspecified policy from the item label or task category.

## Reconciliation result

The scheduler should return enough state for the active role to distinguish:

- overdue and never presented;
- overdue and previously presented but unresolved;
- due now;
- upcoming inside the requested horizon;
- blocked or awaiting a precondition;
- expired or superseded; and
- claimed work whose prior delivery lease was lost.

The role reports semantic summaries rather than raw daemon records. It must not
describe an item as missed merely because it was due while inactive when the
item's contract defines another consequence.

## Delivery operations

The adapter may expose different names, but it should make these consequences
reachable:

```text
reconcile(recipient, activation, now, upcoming_horizon)
wait_next(recipient, activation)
subscribe(recipient, activation, after_revision)
claim(item_id, activation, expected_revision)
acknowledge(item_id, claim, result_ref)
decline(item_id, authority_ref, reason)
defer(item_id, authority_ref, new_due_at)
```

`wait_next` may block while the activation remains valid. The daemon must retain
the item across tool disconnects. A completed wait has value only if the host
delivers that completion into agent execution.

`subscribe` is the preferred streaming affordance when the host can surface
events asynchronously. A one-event blocking pipe or socket is compatible when
it binds atomically to `after_revision`, returns a durable event identity, and
cannot lose an event between reconciliation and waiting.

## Packaged daemon

The current prototype is distributed as:

```text
scripts/role_scheduler.py
```

It combines the daemon and control client, auto-starts on schedule access,
verifies readiness, and uses a singleton lock. The daemon and its durable store
own schedule state. Its detached-session launch is the current route, not a
process-lifetime invariant; a service manager or host-managed MCP configuration
may later replace it without changing scheduler semantics.

Do not silently install persistent services, alter host startup configuration,
or infer permission from ordinary schedule use. Installation is a distinct host
mutation; runtime execution authority remains a distinct scheduled-item policy.

SQLite may be the first local durable-store adapter. Keep storage behind the
control-plane interface: SQLite is not an invariant and must not leak into the
meaning of role identity, authority, delivery, or acknowledgement.

## Execution checks

Before acting on an item, establish:

- the current activation matches the logical recipient;
- approval is present when required and has not expired;
- the intended action and subject still match the approved scope;
- catch-up policy permits the current lateness;
- material preconditions remain true; and
- duplicate execution is excluded or safely reconciled.

When any check fails, preserve the item and present the actual decision or
blocker. Do not broaden approval to make the item executable.

## Result and acknowledgement

Result policy describes what the human needs after the consequence. A test may
return a summary plus failures; a scheduled slice may return an approval prompt
without executing. Store large outputs in their appropriate artifact owner and
reference them from the scheduler record.

Acknowledgement means the scheduled consequence actually reached its protected
state. Presentation, claim, process launch, terminal output, and inferred
success are not substitutes.
