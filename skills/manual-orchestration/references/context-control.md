# Session control observations for manual handoffs

Context telemetry can arrive through a session-specific control subscription
alongside coordination messages. This moves routine observation into a host
process; the worker, parent and root retain the decisions in
[context boundary](context-boundary.md). A pressure observation grants no launch,
handoff, acceptance or root-transfer authority.

## Enrollment and delivery ownership

The parent can prepare a child's listener after verifying its actual identity;
a root owns its own listener. Name the process owner, output collector and
delivery route in the assignment/continuation. Keep one owned listener for the
binding rather than having parent and child unknowingly duplicate it. These
roles can coincide. Enrollment is explicit: no automatic host hook is installed.

**Board topics and context enrollment are separate.** A running `wait` with only
`--topic` selectors does not produce context stats. Before relying on the route,
verify the listener's actual context binding and collector, and retain a valid
initial telemetry observation. An initial normal reading may be quiet, so absence
of an alert is not an enrollment check. Establish delivery from collected output
that reaches the responsible session; a process ID or cursor alone proves less.
If enrollment or timely delivery is unavailable, record that state and use an
owned explicit check before the next context-dependent dispatch/continuation.
After transfer, verify the successor's own binding; the predecessor's listener
and counters do not establish monitoring for the successor.

Use the pinned context-status binary from `provision_context_status.sh`; it must
support `control --help`. Bind the actual session ID and absolute transcript
path. For a Claude child, also bind its agent ID within that session. Do not use
an inherited environment variable as proof of identity. Select planning and
band levels for the observed model/window and applicable campaign; the historical
levels in `context-boundary.md` and `anthropic-host.md` remain hypotheses.

Start the [board wait](../../../docs/codex-chatboard.md#session-control-subscription)
with `--session-id`, the `--context-*` binding options, relevant board topics and
`--follow`. A control-only subscription is possible by selecting just its exact
`control:session:<id>` topic (plus `/agent:<id>` for a Claude child). The host
process observes transcript changes and streams meaningful state changes without
asking the model to poll. Give it a bounded lifetime suited to the assignment.

The collector must actually deliver relevant output. Prefer a compact update
with the next natural model turn when that is timely; an earlier wake needs a
verified host capability. Do not claim delivery because stdout was written or a
process was started. If the host only returns results after process exit, use
the one-shot wait with that host's collection/wakeup mechanism. An uncollected
continuous stream is not a substitute. The CLI does not wake an ended session.

## What to do with an observation

Use the measured input, source timestamp, epoch, observation quality, chosen
levels and optional growth estimate to judge remaining work. `planning`,
`approaching_band` and `in_band` describe pressure; they do not command a reset.
The forecast extrapolates recent requests to an empirical band, not provider
reset timing. Unexpected large outputs can exhaust headroom earlier.

`unavailable` means the counters or their attribution cannot currently support
the decision. A listener error means observation stopped. Use an explicit
context check or repair the binding; do not substitute zero. A return to `normal`
after a new epoch does not establish that a handoff occurred or that lost meaning
was recovered.

Continue keeping the durable continuation current. Preserve the explicit
pre-dispatch and logical-boundary checks until the owning workflow accepts
timely control delivery as equivalent evidence. A new assignment's expected
growth still belongs to the dispatch decision and is not inferred by the monitor.

## Lifetime and recovery

`--follow` retains the cursors and rearms inside one process; its timeout ends
that process, even if messages arrived. Collect its final result and retain
`next_since`, selectors, `next_control_cursor`, and the identity/source/policy
binding. Pass both cursors when resuming the same observation scope. Changing
board filters may require an earlier board cursor. A new session, source or
pressure policy needs a new control baseline; an old cursor is rejected.

The control cursor records what was observed, not what was acted on. After a
delivery interruption, replay the last consumed cursor; duplicate alerts are
possible. The monitor reads current telemetry and cannot promise recovery of
every transient pressure event during a gap.

Idle availability can retain a listener. Explicit transfer or ending that
availability closes or transfers process ownership and makes the delivery gap
visible. Root transfer still requires its existing quiescence and user-mediated
successor procedure; a control channel cannot transfer child ownership.

## Measured campaigns

The user has stated that rollout is unblocked. R2's historical worker-exposure
hold is not a current global rollout gate. Active experiments still own their
exposure rules: adding these instructions or control messages can change a
measured environment and needs that experiment's applicable disposition.
Implementing this capability does not enroll participants, and unchanged worker
files do not prove unchanged exposure. These instructions activate no listeners.
