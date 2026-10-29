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
| A retained child is about to receive a substantial new task, including remediation after review | parent (coordinator or supervisor) | pre-dispatch check, then `followup_task` or `spawn_agent` |
| A single assignment is outgrowing its window mid-task | worker | mid-task check, then continue or `handoff_ready` |
| Root context pressure | root | assess capacity for the next scope; if transferring ownership, use the user-authorized procedure under Roots |

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

A decision to finish bounded work covers that work, not a later substantial
assignment. Before admitting substantial new work into a retained context,
the decider assesses usable current telemetry against the expected work,
result collection, and the room needed to preserve unfinished obligations.
Record the observation, the scope being admitted and the continuation choice
with its reason in the existing assignment or continuation record. Unknown
growth or unavailable telemetry remains uncertainty; it is not spare capacity.
For a parent, the existing pre-dispatch record carries this scope and decision;
no second record is needed.
The choice of retention, decomposition or transfer remains judgment under the
owning workflow, including the root's user-mediated transfer boundary.

This applies to a root accepting a new implementation task as well as a parent
dispatching remediation. Finishing an urgent bounded operation above a planning
level can be reasonable without making the same context suitable for the next
task. An unchanged pressure level may produce no new control event; listener
silence therefore cannot extend an earlier capacity decision to new scope.

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

Before sending a retained child a substantial new task (`<skill-dir>` is the
manual-orchestration skill's directory):

```
python3 -I <skill-dir>/scripts/child_context.py \
  --binary <pinned path> --sha256 <digest> \
  check --session <child id> --rollout <child rollout> \
  [--next-task-tokens <estimate>] --out <assignment dir>/context-check-N.json
```

**H1:** retain only if the child's latest input is below the trigger **and**,
when you can estimate it, input plus the next task's growth stays below the
band floor. Otherwise launch a fresh child with `fork_turns="none"` and a
packet. Record the decision (`retain` or `fresh`), the check's path and a
one-line reason in the assignment record. A recommendation you overrule needs
a stated reason; that is evidence, not a violation. Parent estimates of a
task's growth are not yet calibrated: on 2026-10-22 a remediation estimated at
40K grew about 61K (n=1). Until more data exists, use the largest growth
observed for comparable work, not a guess. A child that has compacted
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

A root cannot authorize its own replacement: it has no parent. **The user is
the root's parent for transfer.** The root prepares everything and the user
authorizes the transfer. The launch itself can now be mechanical: `claude --bg`
starts a top-level session, a Codex root starts as a thread on the shared
app-server daemon (never `codex exec`, whose process locks the thread against
the user's terminal for the whole turn), and
[`scripts/root_rotation.py`](../scripts/root_rotation.py) launches the
successor only after an authorized commit. Merely spawning a child, writing a
packet or compacting does not transfer the root.

Record the root's reading at each dispatch and integration boundary (a current
[listener observation](context-control.md#what-to-do-with-an-observation) counts), and keep
the assignment record and continuation current continuously. When transfer is
the chosen response, plan it before substantial coordination exhausts the room
needed to settle ownership; on 2026-10-22 a Claude root reached about 580K while
its children stayed under
200K, paying that context on every coordination call.

**Transfer only at a quiescent boundary.** A successor root cannot reach the
predecessor's children: their IDs, completion notifications and `SendMessage`
routes belong to the predecessor's session. So before transferring:

1. Stop dispatching. For each live child, collect its result or have it end
   with `handoff_ready`; record each in the assignment record. No child may be
   running when the root hands off.
2. Settle or record every decision in flight, and refresh the assignment
   record (owners, results, open obligations, pinned binaries, frozen and
   errata digests, skill commit).
3. Apply `$wind-walker` and write a root handoff: objective and authority,
   accepted decisions and open questions for the user, the assignment record
   path and digest, every child handoff the successor must dispatch from,
   claims held, the destination ref and HEAD, and the next actions. Record its
   sha256. For Codex, preserve this root's observed model, effort and service
   tier with their source and explicit successor launch settings under
   [the Codex host guidance](codex-host.md#preserve-root-settings-across-continuation).
   Shared defaults or another role's tier are not evidence of this root's
   configuration; verify what the successor actually receives.
4. Launch the successor by one of two routes:
   - **Gate.** Write a quiescence assertion and run
     `root_rotation.py prepare <handoff> --title 'Readable task name'`. It validates the handoff, checks
     quiescence independently (board claims, children launched through
     [`scripts/role_host.py`](../scripts/role_host.py), session processes
     beneath the root), publishes its continuation lineage on the board, and
     stops at `awaiting_authority` with no successor. Commit rechecks lineage;
     a superseded or consumed handoff cannot launch another successor.
     After the user authorizes, run `root_rotation.py commit --detach`. It waits
     briefly for launch identity, without waiting for predecessor retirement.
     Give the user the returned full successor ID and resume command before
     ending the turn, identifying whether adoption is still pending or verified.
     The gate launches the successor, posts the handoff to its own
     board topic, waits for its typed `adopt`, posts the predecessor's
     `availability: ending`, and retires the predecessor
     with `--retire stop`, archives a collected Codex predecessor with
     `--retire archive`, or keeps it with the default `keep`. `watch`
     accepts the authorization as a board message, but only from a named
     approver and only when it cites the handoff's sha256.
     For a Codex successor, supply the saved settings on the invocation that launches:
     `commit` or `watch`. Flags supplied only to `prepare` are not carried to a
     later command. For example, add `--host-arg=-m --host-arg=<saved-model>`,
     `--host-arg=-c --host-arg='model_reasoning_effort="<saved-effort>"'`, and
     `--host-arg=-c --host-arg='service_tier="<saved-cli-tier>"'` to that command.
   - **Manual.** Publish the validated handoff with the board's
     [`handoff-publish`](../../../docs/codex-chatboard.md#prepared-continuations-and-obsolete-handoffs)
     under this role and session. Give the user the exact successor instruction: start a new
     session in the repository. For Codex, give the saved model, effort and
     service tier as exact launch settings or matching UI controls, including
     the intended Fast state. For another host, carry settings with their known
     source and mark unobserved values unestablished. Then give a prompt naming the root handoff
     path and sha256, the skill to load as coordinator, and the first actions
     (verify the digest and run `handoff-status` for this role and digest,
     for Codex verify the successor's own applied settings, read the assignment record,
     adopt on the board, then re-claim board
     resources under the new session id).
5. The successor verifies the digest and recorded continuation status before
   taking ownership. An obsolete handoff redirects recovery to the returned
   references; conflicting or unregistered lineage needs reconciliation with
   the durable assignment and user decisions. A hash alone establishes bytes.
   The successor acknowledges ownership with the board's
   `adopt` command and in the assignment record, and only then dispatches.
   The predecessor takes no further coordination action; the user stops using
   it except to answer questions about its own record.

Use the same readable task title across continuations, with a short session or
handoff identifier. After adoption the gate labels Codex sessions `current` and
`superseded by <id>`. Names aid discovery; the board and exact session IDs bind
recovery. Display-update failures are recorded without undoing adoption.

The user needs the destination to follow a transfer; an ID in a log or a board
message alone does not deliver it. The predecessor owns reporting the launch
destination. If the detached response is `launch_pending`, report that truthfully
with its exact `result_file` and name a collector/delivery route for the eventual
ID; do not claim the transfer complete or leave collection implicit. The
file is evidence, not a user notification. Without a verified delivery route,
the predecessor continues collecting the launch result before ending its turn.
The per-invocation file distinguishes launch, verified adoption, and terminal failure,
and cannot reuse a previous transfer's ID. A successor also identifies itself
in its first user-facing update after adoption. Waiting for predecessor retirement
before revealing the ID would deadlock a predecessor that must end its turn first.

Codex `role_host.py rename codex ID --name NAME` updates display metadata without
a model turn. `archive codex ID --descendants-collected` and `unarchive codex ID`
manage recoverable history. Collect descendants/results first: server archival
also attempts spawned descendants, and may move rollout files. Resolve or
preserve evidence referenced by their old paths before choosing archival.
Unarchiving restores history without resuming work or restoring role authority.
Do not infer completed work or cleanup permission from a cryptic title or age.

Launch children the successor must be able to see through `role_host.py
launch --parent <root session>`. Its ledger is what makes them parent-scoped,
since neither host CLI records a parent. Deliver to a retained Claude session
only through `role_host.py deliver`: `claude --bg --resume` on a session that is
still loaded starts a copy under the same name instead of delivering, which
leaves two live instances of one role.

Release the predecessor's board claims at step 3 and have the successor
re-claim them at step 4, so a claim never names a session that no longer
coordinates. If a child cannot reach a safe stop, defer the transfer and
record why.

## Telemetry rules

The [session control subscription](context-control.md) can supply pressure
observations through the board wait interface. The coordinator establishes an
observation route at startup; this subscription and explicit checks are
alternative mechanisms, not evidence that either is already active. It preserves these
decision owners and the explicit checks as fallback; enrollment, output
collection and model delivery have named owners. It does not automate handoffs.

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
