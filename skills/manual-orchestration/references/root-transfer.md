# Transferring root ownership

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: considering or performing a user-authorized successor-root transfer; same-session recovery does not require this procedure.

## Roots

The root is the top-level supervisor. A root cannot authorize its own replacement: it has no parent. **The user is
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

**A root keeps the compaction limit it started with and passes it to the
children it spawns** ([measured basis](context-measurements.md#measured-basis)). Its own compactions do
not refresh it: both older Sol 6.1 roots compacted twice after the 250,000
limit took effect, each time near 449K. Whether resuming a session picks up the
new value is not established. Until such a root is replaced, each of its calls
between 250K and about 450K carries context the limit would have removed. At its
next quiescent boundary, propose a transfer to the user when the remaining work
justifies one. The user still authorizes it, by the procedure below.

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
     path and sha256, the skill to load as supervisor, and the first actions
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
