# Directed coordination in Pi

Use the Pi Chatboard extension for directed coordination between enrolled Pi sessions on the same board and group. Work Engine owns the shared Git-backed board; the extension runs the canonical CLI waiter and owns Pi's local inbox, cursor, and rearming. It does not grant role authority or replace shared board reads, claims, typed workflows, or campaign receipts.

## Establish the route

Use the operator-approved `extensions/chatboard` from `pi-core-plugins`. Loading it is not enrollment. The operator can run `/chatboard setup` to preview and save a private profile. Saving does not apply new routing to a running session. Use `/chatboard config` to compare saved and loaded routing; apply a compatible saved change with an explicit `/reload`. The operator joins an unassigned lane with `/chatboard join MEMBER`; the command does not replace an existing session or lane binding.

Verify the actual session and loaded board/group/lane with `/chatboard config` and `/chatboard status`; look for `listening`. A compatible `/reload` or resume of the same persisted session retains enrollment. `new_context` changes reasoning context, not enrollment or the durable inbox; recheck the existing route rather than starting another waiter. A fresh session or fork does not inherit the lane. A closed Pi process cannot observe messages or start a turn; a registered wake can wait for that exact session to resume.

## Send, inspect, and acknowledge

| Pi action | Delivery |
| --- | --- |
| Coordinator `tell` to named enrolled lanes | One wake per intended recipient after all rows are board-confirmed and its own rows are in its inbox. |
| Lane `report` | Sends only to the exact enrolled coordinator and registers one wake. |
| `note` or `forward` | Directed inbox delivery without a model wake. For a quiet coordinator update, use `note COORDINATOR TEXT`. |
| Ordinary CLI board post | May match the inbox selectors, but does not request a Pi turn. CLI `--wake` bypasses self-suppression, not this boundary. |

Use the `/chatboard` commands or the `chatboard` model tool for the supported actions. A registered wake starts an idle recipient's turn or waits until a busy recipient settles; it does not steer, interrupt, or cancel existing work. Inspect `/chatboard pending` and the sender's original receipt for an unresolved send or wake. Retry by that receipt when appropriate, not by sending the same report again. `submitted` means a Pi transcript marker was observed; it does not prove a successful turn or the requested consequence. A wake left `requested` without a marker may be ambiguous and is not automatically replayed.

A wake points to exact sequence numbers. Inspect them with `show` (all relevant pages), or use `inbox` to find other rows. Reading with `show` and running a wake turn do **not** clear unread entries. After preserving anything still needed, `/chatboard ack SEQUENCE` releases that local payload; it neither deletes the board row nor certifies action. An unread count alone does not diagnose a lost wake. Check the exact send receipt or recipient transcript, and require consequence evidence for dependent work.

## Retire only the delivery this replaces

1. Verify the extension route for the exact sender and recipient. Where a timely model wake matters, establish wake delivery and recipient handling, not just inbox admission. Stop rearming the legacy waiter for those same directed messages once the new route covers them.
2. Reconcile outstanding messages through the old selector set, session exclusion and replay cursor. Check what the new inbox actually contains; use the old scoped waiter or a bounded board read and its original matching scope for gaps. A fresh `start: "now"` begins at the board tail, not at the old cursor. Old quiet reports do not acquire wakes on reload.
3. Keep an explicit route for traffic the role still needs outside the Pi inbox. It selects `role:<group>-<member>` and `session:<uuid>`, not bare `all`, `job:…`, or arbitrary topics. Use an owned listener when timely notification is required; bounded reads can cover notices where the next decision boundary is soon enough. Use the host's background-operation collector or the CLI job route for jobs.
4. Preserve separate [context and budget delivery](../../manual-orchestration/references/context-control.md) and its ownership. The Pi Chatboard inbox carries neither. If the old waiter carried them, keep them covered while removing only the duplicate directed-message selectors.
