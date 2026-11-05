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
