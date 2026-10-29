# Same-session continuity for outer development agents

This reference applies to development sessions using this skill and its
chatboard/wait pressure observations. The separate Rust runtime's hard context
limit and `new_context` design are not the controls of these outer sessions.

## Choosing a response

A pressure event is evidence about capacity, not a command to replace context
or permission to transfer the root. Judge the next work and collection still
owed against remaining headroom. Continuing a bounded scope, replacing context
inside the same session, and transferring to a successor have different costs
and capabilities. Same-session compaction preserves root ownership; it does
not remove the need for durable continuation meaning.

Wind Walker owns the prerequisite: correct continuation must depend on no
meaning held solely in the context being replaced. Role and domain owners
retain their objectives, authority, decisions and unresolved obligations.
The continuation can reference those owners rather than copy an entire chat.

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

## Foreground automatic continuation on the Codex app server

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
that same job. A fixture supervisor may instead observe and collect failures;
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
