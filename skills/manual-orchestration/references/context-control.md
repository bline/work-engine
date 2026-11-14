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
an inherited environment variable as proof of identity. For Codex, enroll
explicit token levels matched to the session's actual rollover configuration
and recovery needs. The historical 200,000/235,000 levels in
[context boundary](context-boundary.md#measured-basis) were operating hypotheses
for a 250,000 limit, not defaults for every current session. Window percentages
can put warnings above a configured limit and prevent them from firing. Claude
transcripts record no window and take their own token levels.

`--context-step-percent` controls repeated token-growth alerts. Choose it to
fit the available headroom; neither the reader's default 5% nor a smaller step
establishes timely delivery. A 5% step is 23,750 tokens on a 475,000-token
window. A changed control policy needs a new baseline rather than the old
policy's cursor. This skill does not prescribe a numeric step.

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

## Combine budget and coordination observation

For a supervisor, add the objective's `--budget-record`, independently pinned
`--budget-reader` and `--budget-sha256` to this same wait, alongside the verified
context binding and relevant board topics. The [budget subscription](../../../docs/codex-chatboard.md#budget-observation-subscription)
owns its reader contract and event semantics. Observe budget, context and board
events in every returned batch; `status: messages` can accompany control or
budget events and must not hide them. A board-only wait does not observe either
pressure source. Verify initial context and budget observations separately.

Record the listener's job reference, owner, delivery route, selectors and all
three consumed cursors (`next_since`, `next_control_cursor`,
`next_budget_cursor`) in the existing continuation. Collect available output at
natural decision boundaries while doing independent work. When only collection
remains, use the host's long event wait and keep the supervising turn active;
let `--follow` rearm inside the host rather than waking the model on empty
timeouts. A one-shot wait or an expired listener needs rearming after collection.
On closure, collect its terminal result and settle or transfer its ownership.

Budget or context reader failure leaves that observation unavailable; it does
not mean zero spend or spare context. Reconcile the source before dependent
dispatch. Budget events inform the existing grant and stop rules, context events
inform the next scope's capacity decision, and board messages retain their
sender's authority. None grants a larger budget or resolves pending approval.

## What to do with an observation

Use the measured input, source timestamp, epoch, observation quality, chosen
levels and optional growth estimate to judge remaining work. `planning`,
`approaching_band` and `in_band` describe pressure. For Codex token-budget mode,
use the
[recovery guidance](same-session-context.md#codex-token-budget-refresh-deadline)
and the owning session's configured deadlines. The collector must deliver the
update in time for the responsible agent to preserve meaning before rollover;
no successor handoff is required for same-session refresh.
The [context-control audit](../../../metrics/probes/context-control-usage-20261024.md#cu-02-warnings-need-to-survive-into-the-next-scope-decision)
observed delivered warnings followed by a later task admitted without a renewed
capacity decision. It supports closing that action gap; it did not test the
choice of alert cadence or show that additional generic warnings solve the problem.
The forecast extrapolates recent requests to an empirical band, not provider
reset timing. Unexpected large outputs can exhaust headroom earlier.

`unavailable` means the counters or their attribution cannot currently support
the decision. A listener error means observation stopped. Use an explicit
context check or repair the binding; do not substitute zero. A return to `normal`
after a new epoch does not establish that a handoff occurred or that lost meaning
was recovered.

Continue keeping the durable continuation current.

**A current observation replaces the explicit command at a boundary check.** It
is current when the listener is verified bound to this session, transcript,
epoch and levels; is still running; its output has been collected up to the
decision (reading pending output can share the dispatch call); and nothing since
reported an error, timeout, `unavailable` or epoch change. The reader emits on
every change of level and stays quiet while pressure is unchanged
([board reference](../../../docs/codex-chatboard.md#session-control-subscription)),
so the last observation bounds the present input: at least its measured input,
and below the next level. Use that range in the
[pre-dispatch](context-boundary.md#pre-dispatch-check-parent) or boundary
decision. Run the explicit check when the observation is not current or when
the decision would differ across the range. Record which source the reading
came from. The reading is not the decision: a new assignment's expected growth
still belongs to the dispatcher, and listener silence does not extend an
earlier capacity decision to new scope. In the S10 session of 2026-10-28/29
about 230 tool calls in the tree took explicit context readings over 17.6 hours
while the root ran a listener (n=1).
In band, a new event arrives at each further step of the window
(`--context-step-percent`, default 5). Between steps the listener is quiet, and
that silence does not renew a prior decision to continue for a different task.
The step controls repeated token-growth alerts, not a time interval; context
observation follows transcript changes, while `--budget-interval` controls only
budget reads. In token-budget mode, assess recovery headroom on the first preparation
warning instead of waiting for an in-band repeat.
Delivery and preservation must fit before the next potentially large call;
more frequent alerts cannot prevent one call from crossing a boundary.
Use the scoped capacity decision in
[context boundary](context-boundary.md#capacity-belongs-to-the-next-scope).

## Lifetime and recovery

`--follow` retains the cursors and rearms inside one process; its timeout ends
that process, even if messages arrived. Collect its final result and retain
`next_since`, selectors, `next_control_cursor`, and the identity/source/policy
binding. Pass both cursors when resuming the same observation scope. Changing
board filters may require an earlier board cursor. A new session, source or
pressure policy needs a new control baseline; an old cursor is rejected.

Same-session compaction can change the context epoch while preserving the root
ID. On recovery, verify the actual transcript binding, collection owner and
delivery route before relying on that listener. Retain a cursor only for the
same observation scope; a quiet or lower reading does not establish semantic
recovery. See [same-session continuity](same-session-context.md).

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
