# Same-session continuity for outer development agents

This reference applies to development sessions using this skill and its
chatboard/wait pressure observations. Codex's optional `token_budget` feature
exposes its own `new_context` tool; distinguish that host capability from the
separate Rust runtime's context controls.

## Choosing a response

A pressure event is evidence about capacity, not permission to transfer the
root. In Codex token-budget mode, apply the refresh deadline below. Otherwise,
judge the next work and collection still
owed against remaining headroom. Continuing a bounded scope, replacing context
inside the same session, and transferring to a successor have different costs
and capabilities. Same-session compaction preserves root ownership; it does
not remove the need for durable continuation meaning.

Wind Walker owns the prerequisite: correct continuation must depend on no
meaning held solely in the context being replaced. Role and domain owners
retain their objectives, authority, decisions and unresolved obligations.
The continuation can reference those owners rather than copy an entire chat.

## Codex token-budget refresh deadline

When the session uses Codex's `token_budget` feature, `new_context` replaces
conversation history without generating a summary. The automatic limit takes
the same route. Missing preparation can leave the continuing agent unable to
recover its assignment or outstanding obligations. Apply this rule to every
role, including native workers and retained reviewers; ordinary compaction and
its summarization are not the recovery fallback in this mode.

Choose preparation and completion levels for the session's actual model,
rollover configuration, expected call growth and preservation work. These
levels are configurable operating margins, not universal token cutoffs or
provider timing guarantees. An explicit deadline in the owning session or
campaign configuration remains binding. A preparation warning calls for a
capacity decision and making recovery safe; it need not mean an immediate
reset unless the owning instructions require one. Account for recovery and
rediscovery cost as well as remaining headroom when choosing when to refresh.

A configured deadline is not a cost-optimal reset interval. The
[M22 replay](../../../metrics/efficiency-ledger.md#what-an-earlier-handoff-is-worth-replayed-on-44-builder-threads)
found that earlier replacement's modeled benefit depends on total restart and
rediscovery cost; its older models and host do not calibrate this route.

- Before intentional replacement, invoke Wind Walker, preserve required meaning
  through its existing owners, and publish a concise chatboard recovery note
  with the verified session ID, assignment and continuation reference. Ensure
  the note can be found after reset; the latest 20 messages may not contain it.
- Leave enough headroom for preservation and invoke your own `new_context`
  before a configured completion deadline. If the deadline is near or ordinary
  work could consume preservation headroom, suspend ordinary work and make
  recovery safe. A supervisor may request this action, but cannot execute the
  model-only tool for its child. Do not use a compaction helper for this route.
- In the new window, follow AGENTS.md, invoke Wind Walker and recover through
  the board note and its existing owners before resuming the assignment. An
  unchanged session ID or lower reading alone does not establish recovery.
  Missing continuation-critical meaning is a recovery failure: report it to
  the current collection owner and stop dependent work instead of guessing.

Keep the continuation usable during ordinary work so automatic rollover also
has a recovery path. This uses the existing continuation, board and pressure
listener; it requires no successor handoff or transfer of ownership. Preserve
review episodes and process/child collection responsibilities. If the tool or
its recovery route is unavailable, stop ordinary work and report the limitation
while preservation headroom remains. These instructions do not enable the
feature or set its numeric thresholds. The [native child probe](../../../metrics/probes/codex-native-new-context-20261114.md)
established one supervised child recovery, not autonomous pressure handling or
an optimal reset interval.

## Authority and control capability

An existing assignment may cover ordinary context management. Apply its actual
boundaries, including current human restrictions; do not invent a new approval
step merely because the session ID stays the same or changes. Root transfer
still uses the authorization and adoption procedure under
[Roots](context-boundary.md#roots).

A durable checkpoint establishes recovery readiness, not authority to invoke
a provider control or resume work automatically. Establish the available
control, its target, what it interrupts or queues, and who will collect its
outcome. An operator using an available native compaction control is a manual
route; it is not evidence of automatic self-resumption.

For an automated route, establish how intervening human input, cancellation,
uncertain provider entry and resumption are handled for that actual host.
Reading input and then sending an RPC does not make those actions atomic.
An uncertain request must not be replayed merely because its process exited or
a timeout expired: first reconcile its observed entry and outcome. If the
required control or delivery behavior is unestablished, use a supported route
or keep the dependent operation unresolved. Do not fill the gap with an
improvised background controller.

## Preparation and operator visibility

Before intentional replacement, invoke Wind Walker and preserve usable owners
or references for the next objective, authority boundaries, accepted and
rejected decisions, uncertain facts, pending human interaction, outstanding
children/processes and their collectors, and the next authorized work.
Include the skill and role references needed for recovery. For Codex, record
the session's applied model, effort and tier observation before replacement, with
its source under [the host guidance](codex-host.md#preserve-root-settings-across-continuation).
Preserve source revisions where continuing against a different revision could
change a decision. This is a semantic obligation, not a required packet schema.

Announce the intended same-session replacement, its continuation location and
how continuation or failure will reach the operator. If the turn will end,
name the actual resumption/notification owner and an observable terminal
outcome location. A file on disk or an uncollected wait process is not delivery;
silence at that location means unresolved rather than successful progress.

A runtime- or agent-authored wakeup is continuation material. Preserve its author and delivering
actor even when the transport renders it as a user message. A wakeup cannot
grant a pending approval, widen the task or reactivate paused work. Current
human direction, including the existing assignment, governs the next admission
to productive work; no fresh approval is implied by ordinary continuation.

## Recovery and continued observation

In the new context, invoke Wind Walker and reconcile the continuation with
current human direction and the actual source state before dependent work.
Recover unresolved questions as unresolved. A compaction item, lower input
count or unchanged session ID establishes no semantic recovery by itself.

Role ownership and child/process collection responsibilities remain with this
session. Verify usable collection routes; report unavailable routes as unresolved
instead of treating their work as complete. Preserve the meaning and scope of
retained review evidence rather than claiming a fresh independent review.

Re-establish the actual session/transcript binding and pressure delivery under
[session control](context-control.md). Existing cursors are reusable only for
their observation scope. A new binding or policy needs a fresh baseline; an
unchanged binding still needs an owned collector and timely delivery. Do not
silently duplicate or orphan the prior listener.

For Codex, compare a model, effort and service tier observation tied to recovery
with the saved observation under [the host guidance](codex-host.md#preserve-root-settings-across-continuation).
An event at or before replacement is last-known configuration, not proof of
post-recovery preservation. Preserve an authorized change explicitly; detect
and surface a mismatch before configuration-dependent work.

Report whether replacement occurred, which session continues, whether required
meaning was recovered, and any unresolved operation or owner. Provider
completion and semantic recovery are separate observations.

## Qualification and benefit

Qualification is specific to the observed host, control, input paths and
resumption route. A supervised idle-fixture compaction does not qualify an
autonomous interactive-root controller or concurrent-input ordering. Preserve
unsupported and failed results alongside successful ones.

Compare preparation, compaction or successor launch, recovery, first accepted
work, operator effort and visible failure handling. Lower post-compaction input
alone is not net savings. A useful result can justify a bounded route without
making it a universal pressure policy or eliminating the handoff alternative.

Plan a requested compaction below the session's automatic compaction point. For
a session started under the 250,000 limit, that point is just under 250K; a
compaction planned for 300K or more is never reached, because the provider compacts
first, mid-turn, without the controller's preparation or attributed recovery
([measured basis](context-boundary.md#measured-basis)).

## Foreground automatic continuation on the Codex app server

For an authorized Codex supervisor that needs a smaller context while children
remain operational, prefer same-session refresh over transferring the root when
the current host supports the control and recovery routes below. Preserve the
children's exact identities, assignments, review episodes, pending messages,
process/job references and collection routes in the continuation. The root
keeps ownership; context refresh is no reason to retire or redispatch them.
Establish that the actual host preserves the helper and child collection routes
across source-turn completion and compaction. An idle-fixture result alone does
not establish this case with active children.

A bounded helper agent can own the compaction controller and collect its
foreground job while the root ends its source turn. Give the helper the
verified root SID/source turn, prepared operation and continuation references,
control scope, outcome location and failure-delivery route. It drives that one
refresh, collects its own job and reports the terminal outcome; it is not a
successor supervisor and cannot dispatch engineering or alter authority. The
root records the helper and its own recovery/collection obligations, and verifies
the helper's controller/job binding and failure-delivery route before yielding.
The saved job reference must name the helper and its collection route as well
as its actor-scoped exec handle; the root cannot assume it can poll that handle.
After recovery, reconcile both the helper outcome and every outstanding
child before dependent dispatch, without replaying work already in flight.

Use the existing `auto_continue.py` CLI adapter for native compaction and
attributed recovery rather than inventing a second controller. A literal
`/compact ...` is a native UI control only on a route that implements it;
queuing that text as a message does not establish compaction. A helper using a
different CLI route must establish its target, native control, completion and
recovery delivery on that host. Do not assume this Codex route works on Claude
Code or Pi. If control or collection is unavailable, preserve the continuation
and use an available supported route; do not claim autonomous refresh succeeded.

For an authorized standing assignment, `scripts/auto_continue.py` provides a
bounded same-SID route. It does not install a service. Prepare with the verified
SID, absolute rollout, exact active source turn, durable continuation, pinned
Wind Walker `SKILL.md`, pinned context-status binary and SHA-256, controller
author and an actual failure collector and
delivery route. Bind a frozen continuation snapshot distinct from the role’s working continuation;
it points to the operation directory, which contains the journal and terminal
outcome. A changed snapshot or pinned Wind Walker digest is a supersession
signal and refuses admission, so keep both pins unchanged while the operation
remains eligible for admission, including during retryable refusals and pauses;
record working notes elsewhere. Start `run` as a foreground `exec_command` that yields, save its exec
job reference with `bind-job` in the operation journal, announce the collector and result location,
and end the source turn. The controller waits for source completion, collects
an asynchronous native compaction and starts the attributed recovery turn;
it exits without waiting for recovery completion. The recovered actor collects
its own job, or collects the helper's terminal receipt through the bound route
when the helper owns that job. A fixture supervisor may instead observe and
collect failures;
it does not wake or repair the actor. A named route is necessary for failure
visibility; a result file alone is not delivery. If no recovery wake occurs,
the actor cannot collect the failed job in a recovery turn. Exec job handles
may be actor-scoped, so a different collector needs its own verified observation
route rather than assuming it can collect that handle.

The controller records intent before native entry, reconciles uncertain entry
from native history, and never blindly retries an uncertain compact or wakeup.
One namespace-local SID reservation and one irreversible recovery claim fence competing
operations and replay. The operation records the resolved reservation path and
namespace derived from `XDG_STATE_HOME` or its default; different namespaces do
not share this fence. Only `complete` releases the reservation; pause, cancel,
failure and uncertainty preserve it. The named owner can inspect the operation,
reservation, native history, effects and process custody and resolve the exact
reservation and compare its operation ID with the journal. Once all entries,
effects and processes are settled, the owner may remove that exact reservation
file as technical reconciliation. Routine authorized technical reconciliation
does not require fresh human permission;
there is no automatic release command. Recovery invokes the pinned Wind Walker skill, verifies
its own SID, rollout and wakeup turn, reads the continuation and current human
direction, and reconciles pending questions, effects and collection ownership.
It uses recovery-only work until the pinned context-status binary reports a
usable latest post-replacement request and positive window. `admit` records the
actor's direction reconciliation, checks those observations and post-boundary
applied settings, and claims operation custody before the declared effect;
`complete` records the result afterward. The claim grants no authority and
cannot resolve a human approval. Preserve a genuine user settings change with
its evidence; unknown or unexplained settings mismatches remain unresolved.
If a wakeup response was lost, run the same operation to reconcile its existing
entry before admission, never to resend it. An admitted but unfinished effect
is uncertain and must not be replayed automatically.

`pause` and `cancel` serialize with the operation; native source interruption,
visible intervening turns and changed continuation prevent stale entry. Native
reads and `turn/start` are not an atomic fence against arbitrary other clients;
a newly started turn can also be temporarily invisible to native reads, extending
the observation gap beyond the RPC interval.
Simultaneous direct TUI input, cancellation after admission, hidden mutations,
missing evidence after a process or machine crash, daemon restart ordering and
failure without a working collector are unsupported. The journal and exit
status disclose uncertainty; they do not prove universal exactly-once effects.
