# Bounded coding/lifecycle bridge

Status: implemented private candidate; actual SDK/Docker qualification and
independent review require a separate exact runtime admission. Default coding
profiles and lifecycle Pi v1/v2 retain their existing behavior.

`work-engine.pi-coding-lifecycle-fixture.v1` is provider-disabled, with exactly
one allowed `workspace_exec`. `coding-qualification --execute <job.json>` requires
an exact `work-engine.coding-continuity-runtime-admission.v1` authority document
binding the executable SHA-256, job, immutable deadline and these counts:
1 inference container, 1 tool container, 1 validator, 2 SDK sessions,
1 workspace command, 4 fake provider requests and 1 physical reset. Credentials,
live providers, installs, pulls and retries are outside this profile. Public
entry without the exact document fails before Host/runtime creation.

The Host journal remains the sole coding-effect owner. It commits each call
intent before inspection/entry, retains unknown results, and uses its existing
separate network-none tool container and command runner. The installed gate
wraps that actuator in a single-use closure. The exact result artifact is saved
in the Host journal before observation into lifecycle ingress. Reopen restores
neither an execution gate nor replay rights.

Lifecycle owns native parents, grants and replacement. Parent send intent is
durably committed before dispatch. Immediate SQLite transactions check exact
current owner/session/profile, grant revision/revocation/expiry and monotonic
clock while holding the actual effect or IPC write. Host also checks admitted
grant expiry directly before spawn and IPC consumption, plus job deadline and
cancellation. An invalidator ordered after entry fences subsequent entry; it
cannot undo an already-entered effect. Rollback cannot erase the previously
committed Host or parent uncertainty. There is no second coding-tool ledger.

The strict v3 settlement grammar reads immutable raw SDK events/provider input,
Host intent/result/custody and branch history. It joins the exact command, call,
result artifact bytes, stdout/stderr digests, process status, absence of timeout
or leftovers, container identity and SDK toolResult. An event alone never admits
or settles a tool. The checkpoint must contain that exact Host result and the
stable workspace manifest. Text-only continuation uses the owner-derived latest
settled history prefix; recovery requires the owner's exact committed checkpoint.
Generic native settlement rejects v3 profile/protocol identifiers separately,
including mixed bindings, while retaining a valid generic positive control.

The existing receiver's `--lifecycle-bridge` mode keeps the same workspace tool
callback and uses SDK `AgentSessionRuntime.newSession()`. Host removes model tool
writers and confirms absence before freezing the workspace/checkpoint. The fresh
session explicitly reports auto-compaction disabled, empty messages, and a fresh
branch containing only model/thinking metadata. Active conversation and
compaction/summary rows are refused; header or metadata provenance links may
retain predecessor identifiers. The protected boundary is model-visible input:
only the exact admitted checkpoint enters rehydration, and its continued prefix
is owner-derived. Old conversation remains external provenance.
Tools are disabled in that fresh session. Exact rehydration and continued
text work introduce no further tool executions. The original independent
read-only validator checks the same stable workspace and original acceptance
source after the continued turn; its normal proof/export and receipt ACK remain.
An uncertain parent or tool refuses replacement, including after reopen.

`work-engine.coding-continuity-packet.v1` carries an outside budget snapshot and
explicit fixture pressure. Budget schema `work-engine.coding-budget-control.v1`
requires observation-only authority, fixture classification, source/generation,
sequence 1, observation time, finite age up to 300000ms, exact expiry, availability,
4 fake requests and 1 tool call. Lifecycle grants remain the entry authority.
Actual SDK usage/model/window/time are preserved in the initial prompt and raw
provider/history observations, separately from injected pressure and budget.
Fixture pressure is not measured quota exhaustion. Root's aggregate campaign
accounting and review/acceptance remain outside this bounded job.

Offline tests use synthetic SDK/Host records and disposable files. They exercise
admission, exact result reconciliation, unresolved/reopen fences, strict proof
negatives, actual transition/grant logic, exact recovery, continued prefix and
finite controls without entering SDK or Docker. They do not establish actual
runtime compatibility, model comprehension or production/domain acceptance.

## Live continuity profile (separately versioned)

Status: source and offline tests only. No Docker, SDK or provider operation has
run for it; actual runtime compatibility and real model behaviour are `not
established`. The fixture profile above is unchanged.

`work-engine.pi-coding-lifecycle-live.v1` composes the existing direct live Host
provider route (`gpt-6-luna`, pinned catalog, existing native credential
projection, fixed endpoint, existing exact provider admission) with this lifecycle
bridge. It admits exactly 1 workspace command, 1 reset, 2 SDK sessions and 4
provider requests (`max_requests == 4`, `max_context_bytes <= 24000` so each
`coding_provider_request` frame fits the receiver's 64 KiB bound). The pinned
packet is `work-engine.coding-live-continuity-packet.v1` and carries the exact
bounds plus a budget classified `live_provider_observation`; the fixture label is
refused here and the live label is refused by the fixture. The Host plan's
provider admission document is not widened.

Entry fencing per provider request: the receiver emits `coding_provider_request`;
the Host writes its durable unknown Reservation (pinned model, finite budget,
unknown never regranted) before the lifecycle gate records the request and checks
the current parent/grant; the grant is written through the current-turn send fence
with a consumption deadline. Usage settles only from raw completed provider
evidence, as in the direct live route. A bare `provider_request` is refused. Tool
entry and reset use the same lifecycle fences as the fixture. The stream wrapper is
re-bound for each SDK session after `newSession()` with one counter.

Real model text is provenance, not the owner checkpoint. Lifecycle profile/protocol
v4 settlement commits the actual assistant final text and its digest in the existing
`native_turns` final fields for every profile, so `native_final_material` version 1
still reports the model's own bytes next to its raw history reference. The owner
checkpoint is a separate material and is not stored in those fields. No second copy
is persisted: it is recomputed by the same validator from the exact Host result
artifact, workspace manifest and settled history prefix in the immutable ingress rows
that the settled invocation references (the model text appears only as a sha256 whose
digest must match the committed final text). `settle_pi_coding` checks, in the settling
transaction, that this derivation equals what it validated. Reset, transition text,
rehydration prompt equality and the receipt consume the derived checkpoint for v4
turns (the committed final text for every other turn/profile), a caller-supplied
checkpoint that differs from it is refused, and the Host reads it back through
`pi_coding_live_checkpoint`. If the committed final text or the referenced evidence
is altered, the derivation is refused rather than re-served. Generic native settlement and v3 reject v4 identifiers, including mixed profile/protocol
pairs. v4 additionally allows text blocks before the single `toolCall`; a second
tool call, thinking blocks, a missing tool, a non-`stop` final, an extra provider
request or any unmatched native event fail closed (no retry, no cap increase).
The sole recovery input is the checkpoint; tools are disabled after reset.

Not claimed: that a real model performs the happy path, that the SDK re-binds the
wrapper as expected, or any production/domain acceptance.

The live profile's sole tool-bearing turn uses the initial ingress prefix from
sequence 1 through its committed history coordinate. Derivation reads only that
slice. Later duplicate observations preserve checkpoint availability; genuinely
conflicting observations still fence the parent through normal owner handling.


## Producing-binary job facade

`coding-qualification` retains `--execute PINNED_JOB` and also serves the fixed
server interface: `pi-docker run --request PINNED_JOB` and
`pi-docker inspect|reconcile|cancel --state ORIGINAL_STATE`.
Run invokes the same bounded lifecycle composition and successful delivery ACK,
then emits the real Host `{receipt_id,receipt}` wrapper. The legacy `--execute`
summary remains unchanged. Neither output grants semantic acceptance.

Inspect reads the original journal under its existing Host lock. Reconcile opens
that same producing-binary state and collects through Host recovery, without
installing a new lifecycle gate or starting a replacement provider turn. Cancel
verifies the producing binary and original job identity and writes the existing
Host cancellation marker while a run may hold the lock. SIGTERM/SIGINT use the
Host's existing stop handling. An old producer's journals cannot be reinterpreted
by a rebuilt executable.

`tests/coding_driver.rs` exercises the actual executable with synthetic collected
journals and missing-input failures. Server live-class tests use shell fixtures
for timeout, interruption, replay and Events HTTP composition. These are offline
evidence; actual live qualification remains an independently admitted run.
