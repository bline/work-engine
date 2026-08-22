# Role-Aware Agent Scheduler

## Status

**Idea / architecture proposal with local prototype**

This document preserves a proposed Work Engine capability for durable,
human-supervised scheduling. The role-scheduler skill now includes a local
SQLite and Unix-socket prototype. An MCP adapter and asynchronous host wake-up
path are not yet implemented.

## Motivation

Scheduled obligations can outlive a model context, an agent process, or a
human-supervised session. Keeping a future obligation only in model context is
unsafe because compaction or replacement can erase or distort it. A detached
terminal is also an insufficient owner: its output may survive without being
delivered back into model execution.

Work Engine needs a durable owner that can remember when work becomes due while
preserving the human's authority over whether an active agent may perform it.

## Core model

The scheduler is an always-alive daemon. It owns schedule identity, timing,
delivery state, and acknowledgement state outside model context. It does not
activate agents or create execution authority.

An agent role participates only while a human-supervised role activation is
live. At startup, the active role reconciles its agenda and reports:

- overdue or missed items;
- items due now;
- items coming up within a useful horizon;
- items that expired, were superseded, or remain blocked; and
- the authority and catch-up consequence attached to each item.

If no matching role is active, due items remain durably pending. The daemon
does not launch an agent, mutate a repository, invoke a provider, or perform an
external action merely because time passed.

The governing invariant is:

> **The scheduler may preserve and surface due work, but it must never create
> authority, activate a role, or execute a consequence.**

## Control-plane consequence

Once the daemon knows stable role identities, active delivery leases, durable
schedules, pending events, and acknowledgements, it forms a small runtime
control plane. Scheduling is its first capability rather than necessarily its
final product boundary.

The control plane may make these facts observable and controllable:

- which logical roles currently hold supervised activation leases;
- which obligations are scheduled, due, claimed, awaiting authority, or
  acknowledged;
- which delivery listeners are connected and at what agenda revision;
- whether delivery is healthy, stale, or unavailable; and
- human-authorized cancellation, deferral, lease revocation, or resumption.

It should not become the owner of every workflow artifact. Campaign receipts,
test reports, review evidence, Git state, and other domain truth remain with
their existing authoritative owners. The control plane stores the coordination
state and references needed to route those consequences.

The scheduler is one component of this control plane, not its final owner or
complete boundary. A client-facing control protocol may compose scheduler
projections with activation, runtime-binding, agent-state, and workflow-owned
projections while preserving each owner's semantics. The bidirectional client
and environment-affordance direction is recorded in
[Work Engine Control Protocol and Environment Affordances](work-engine-control-protocol-and-environment-affordances.md).

## Identity and activation

Scheduled responsibility belongs to a stable logical recipient, not a model,
thread, terminal, or context window.

```yaml
recipient:
  repository_id: work-engine
  logical_role: slice-supervisor
  logical_agent_id: review-remediation-supervisor # optional specificity

activation:
  session_instance_id: ephemeral-runtime-id
  lease_epoch: 7
  authorized_by: human-session
  expires_at: 2026-08-21T17:00:00-06:00
```

A replacement session may acquire a new activation lease for the same logical
recipient and reconcile pending work. Lease expiry ends delivery authority; it
does not erase scheduled obligations.

## Scheduled consequences

A scheduled item records the consequence the human intended, not only a label
and timestamp. Useful consequence modes include:

- `execute_and_report`;
- `request_approval`;
- `notify_only`; and
- `prepare_then_request_approval`.

Approval and catch-up are separate decisions:

```yaml
approval:
  policy: auto_approve # or require_confirmation
  granted_by: human
  granted_at: 2026-08-21T16:10:00-06:00
  expires_at: 2026-08-21T18:00:00-06:00

catch_up:
  policy: ask_if_missed
```

Other plausible catch-up policies include `run_when_next_active`,
`run_if_within_window`, and `expire_if_missed`. These are capability choices,
not a closed taxonomy; the durable item must preserve whatever policy the
current scheduler contract actually supports.

Scheduling alone never grants authority. Auto-approval is valid only for the
bound action, subject, recipient, preconditions, time window, and authority
source. A semantic change to the requested consequence invalidates the prior
approval rather than silently broadening it.

## Examples

A scheduled test may need execution plus its output:

```yaml
intent: Run the supervisor test suite
on_due: execute_and_report
approval_policy: auto_approve
result_policy: summary_and_failures
catch_up_policy: run_when_next_active
```

A scheduled slice may need a human decision rather than execution:

```yaml
intent: Begin persistent-state Slice 1
on_due: request_approval
approval_policy: require_confirmation
result_policy: present_proposed_action
catch_up_policy: ask_if_missed
```

These examples illustrate consequences, not universal task-type rules. A test
may be expensive or externally mutating, and a previously reviewed slice may
carry explicit execution authority.

## Delivery and listening

The active role can establish a blocking listener after startup reconciliation.
From the agent's perspective this is event-driven: the daemon completes the
wait when an item becomes deliverable.

The listener owns no durable truth. If it disconnects, the daemon retains the
item. Reconnection starts with reconciliation before another wait is
established.

A background terminal is useful only when the host delivers its completion
back into agent execution. A detached process that merely accumulates output
does not by itself wake a model. The implementation therefore needs either an
outstanding tool call, an asynchronous host bridge, or another explicit
delivery mechanism whose completion becomes model input.

The preferred delivery capability is a stream-backed subscription that the
host can surface while other work continues. A one-event blocking pipe or
socket is a simpler compatible route: it waits at an agenda revision, emits one
durably identified event, and exits. The revision handshake prevents the race
where an item becomes due after reconciliation but before the listener blocks.

## Packaging and process lifetime

The scheduler distribution may live inside the role-scheduler skill. A useful
package could include deterministic scripts for daemon execution, installation,
health inspection, service control, schema migration, and a thin MCP adapter.
Keeping those related assets together makes the capability portable and gives
the skill one maintained interface to invoke.

Embedding the code does not make the skill invocation the process owner. Skill
instructions are loaded for model work; they are not a service supervisor. An
always-alive daemon needs a host-owned lifetime such as a user service or a
host-managed MCP server. The skill may verify that service and invoke an
idempotent, already-authorized start operation, but it must not claim durable
availability merely because it launched a detached child process.

Installation and persistent auto-start are host mutations. They require the
authority appropriate to the target environment. Once installed, daemon
restart policy can be owned by the service manager while execution of scheduled
work remains bound to active human-supervised role leases.

SQLite is a practical first storage adapter because it supplies durable local
transactions and restart recovery without adding an external service. It is an
implementation choice, not the product contract. The control-plane interface
must preserve the ownership and lifecycle consequences if another store later
replaces it.

## Lifecycle

One possible semantic lifecycle is:

```text
scheduled
  -> due_pending
  -> presented / claimed
  -> awaiting_human / executing
  -> acknowledged / declined / deferred / expired / superseded
```

The implementation route may vary, but it must preserve these consequences:

- a due item is not mistaken for execution authority;
- presenting an item is not mistaken for completing it;
- a lost activation or listener cannot lose the item;
- repeated delivery cannot duplicate an acknowledged consequence;
- a replacement role can distinguish unseen, presented, claimed, completed,
  declined, deferred, expired, and superseded work when those distinctions
  matter downstream; and
- current time and timezone are explicit when describing lateness or proximity.

## Startup projection

When a durable scheduler capability is configured, an active role should
reconcile its agenda before ordinary work whose ordering could make a due item
stale or invisible. A useful report might be:

> Scheduler reconciliation:
>
> - Overdue: Claude review was scheduled for 4:30 PM. It is now 4:35 PM. It
>   has not run. Its catch-up policy requires confirmation. Should I run it?
> - Upcoming: Repository checkpoint review is scheduled in 10 minutes.
> - Later: Campaign reconciliation is scheduled tomorrow at 9:00 AM.
>
> No scheduled work was executed while this role was inactive.

After reconciliation, the active role may establish a listener if the host can
truthfully deliver its completion. If no durable scheduler is available, the
agent must not emulate durable scheduling by storing obligations only in model
context.

## Open implementation questions

- Which host streaming bridge or MCP transport should expose the first
  subscription implementation?
- What establishes and revokes a human-supervised role activation lease?
- Which authority references are strong enough for auto-approved consequences?
- What result artifacts and acknowledgement receipts need durable schemas?
- How should upcoming horizons be configured without turning a useful default
  into universal policy?
- Which additional runtime-control capabilities have real consumers, rather
  than merely being convenient to centralize?
