# Frozen instruction-review subject

Captured 2026-10-28. These are exact excerpts from the named source bytes,
not separately governing instructions. Full-source SHA-256 identities bind the
review scope; line numbers refer to those source bytes.

## proposals/agent-state/same-session-context-continuity/proposal.md

Full-source SHA-256: `42831b3b17a76c9086977cfbf445d352ec7409c30b496f866b898d4771a2770c`.

### Source lines 32-134

## Proposed contract insertion

The following text is proposed for the existing semantic lifecycle contract,
after its Ownership boundary. The transition protocol consumes these outcomes
through its existing admission, custody, lease, effect and recovery contracts.
Acceptance does not require the current app-server paths, a Python controller,
a particular provider API, or a new universal receipt schema.

### Context change, ownership and authority

Context transformation and root ownership transfer are independent operations.
Same-session compaction does not itself transfer a role, establish fresh
independent reasoning, or eliminate the need for durable continuation meaning.
Root transfer retains its owning authorization and adoption boundary.

An active assignment's existing authority may cover same-session lifecycle
administration and automatic continuation when the governing role and host
contracts permit both operations and continuation stays within that assignment.
No new approval is required solely because context changes. A reserved human
decision, explicit route restriction, pause or cancellation remains binding.
Access to a socket, a safe checkpoint, Wind Walker compliance, and unchanged
thread identity do not independently grant actuation or resumption authority.

Because a checkpoint records authority rather than issuing it, the host must
resolve the current governing authority before actuation and again before
productive resumption. Missing or conflicting authority leaves the dependent
operation unestablished. Rehydration permitted for recovery cannot reactivate
the paused assignment or resolve a pending human decision.

### Human input and runtime authorship

The host must establish an authoritative ordering between human input,
context actuation and productive resumption. Before actuation, competing input
invalidates readiness as required by the existing transition lease. After
actuation begins, newly received input remains durably attributable outside
the retiring revision and participates in successor reconciliation. A newer
pause, cancellation or scope change must govern admission before any affected
domain work resumes. Safe custody must not become indefinite invisibility;
receipt, pending disposition and the responsible owner remain observable.

A runtime wakeup retains its actual producer and existing authority reference
through delivery, history, compaction and recovery. Its own text cannot
authenticate that origin or create human approval. If a provider transport uses
a user-message field, the host must preserve the actual origin through an
attributable envelope and a model-visible projection that does not promote it
to human authority. A transport unable to preserve that distinction cannot
establish this automatic-resumption contract.

### Recovery and effect custody

The host owns one durable transition operation and the lifecycle effects it
dispatches. Its existing lease and effect records must bind the exact role,
runtime, predecessor, checkpoint and authorized continuation. Replaying an
event or restarting a process must not create a second compaction or wakeup
for an already committed effect. An effect with uncertain provider entry
requires reconciliation against retained evidence before another attempt;
neither a missing completion record nor an expired process proves non-entry.

Compaction completion and semantic recovery are separate claims. Before
dependent domain work, recovery must establish the applicable role binding,
exact checkpoint, current instructions and authority, unresolved interactions,
collection obligations and valid next scope. Missing, contradicted or
uncertain evidence remains visible under the existing recovery contract.
Opaque provider state may help continuation but cannot be the sole durable
owner of meaning whose loss would invalidate it.

### Operator visibility

The operator must be able to distinguish an intended transition, an observed
context event, accepted recovery, cancellation, failure and unresolved outcome.
The existing lifecycle record and user-facing projection must identify the
initiator, authority source, checkpoint, session and context lineage where
observable, realized mechanism, resumption disposition and remaining owner.
Unknown provider semantics stay unknown. Lifecycle work that ends the visible
turn also needs an owned, evidenced resumption or notification route; a result
file alone is not delivery to the user.

Visibility need not imply a new approval step or a fixed channel. A supported
operation can announce its intent and outcome in the current thread. The host
must not describe it as a root transfer when ownership remains unchanged, or
as a verified fresh context merely because a compaction item appeared.

## Ownership and loading

Domain and role owners retain objectives, claims, decisions and commitments.
The lifecycle host owns ordering, transition custody, attribution and recovery
evidence; it consumes shared fencing rather than redefining that mechanism.
Provider adapters describe and evidence the transformations they can realize.
Wind Walker retains its general continuation-independence contract.

Manual coordinators receive the compact distinction in
[context-boundary.md](../../../skills/manual-orchestration/references/context-boundary.md#same-session-replacement-and-root-transfer).
That clarification restates existing authority and recovery boundaries; it
does not activate this candidate host contract or authorize an improvised
controller. The detailed host contract belongs at host design and admission
boundaries, not in every worker prompt. The proposed host requirements expose
their causal reasons above when they govern an implementing agent.

The present normative review subjects are the proposed contract insertion and
the changed context-boundary paragraphs. They govern lifecycle authority,
input authorship, effect custody and loading reach. Pilot sample size and case
ordering are bounded research choices, not production doctrine.


## skills/manual-orchestration/references/context-boundary.md

Full-source SHA-256: `9a40fc07faaae664e54071b01a479793e7d3b375829ca40b95d81f276daa8da2`.

### Source lines 1-41

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

A saved wakeup is runtime-authored continuation material, not fresh human
authorization, even when a transport presents it as user input. New human
directions still govern. A compaction marker or lower input reading shows a
context event, not that required meaning was recovered. Apply Wind Walker on
recovery and reconcile the durable continuation before dependent work.

Same-session replacement also leaves collection responsibility with this root.
The continuation must retain owners and usable references for outstanding
children and processes. Unavailable notification or collection routes remain
unresolved; an unchanged root ID does not establish their availability.


### Source lines 141-159

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

Record the root's reading at each dispatch and integration boundary, and keep
the assignment record and continuation current continuously. When transfer is
the chosen response, plan it before substantial coordination exhausts the room
needed to settle ownership; on 2026-10-22 a Claude root reached about 580K while
its children stayed under
200K, paying that context on every coordination call.

