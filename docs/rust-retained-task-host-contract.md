# Retained repository-analysis task host contract

The permanent `retained-task-workflow` owner stores the meaning and work cursor
of an approved repository-analysis task. `work-engine-task-host serve --startup
<absolute-file>` is its ordinary local entry. The installed lifecycle service
alone owns input FIFO, logical deliveries and attempts, native entry and
settlement, context transitions, custody, and readiness. The host is useful to
an ordinary operator client outside the S8 proof scenario.

This contract specializes the admitted S8 plan at SHA-256
`c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7`
and the accepted admission contract v2 at SHA-256
`151c55e3d81b4dbf094f5d28dad525ad4f33fd8b66fa9d473b05c24ab9d08ab8`.
The v2 revision fixes the cross-store lock order to task then lifecycle. It does
not change S8 outcomes or make the two databases one transaction.
The later narrow native-segment route at SHA-256
`3b5617bb00440ee7ac7ef9b23b51a8885ee0e8eddeb2dd7cbf5546c17ae46388`
keeps the queue attempt as logical custody while lifecycle records each physical
native segment separately. It adds no task-owner scheduler.
The accepted two-cycle authority route v5 at SHA-256
`33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230`
binds ordinary commands to one trusted grant for the current lifecycle context
and operation scope.
The accepted recovery-only route v7 at SHA-256
`cfbe11a0b6cedcc98bf4a2c2e2bd284d921f3b99dbb326622fce1e2d64ab0276`
preserves task-owner readback after a killed native owner without adopting its
session or replaying its turn.

## Trusted startup and local command boundary

The startup file is an owner-only absolute regular file. Its JSON fields are
`profile`, `task_store_path`, `socket_path`, `approved_snapshot_ref`,
`answer_authority_ref`, `enqueue_grant_ref`, `replacement_grant_ref`, and an
embedded `lifecycle` `ServiceConfig`. It also pins `executable_sha256`,
`source_root`, `source_manifest_path`, `source_sha256`, `lock_path`,
`lock_sha256`, `toolchain_path`, `toolchain_sha256`, `features`, and
`provider_identity`. The manifest is a sorted JSON array of relative
`{path,sha256}` entries. Before starting lifecycle, the host checks its own
executable hash, the lock/toolchain/manifest bytes, and every manifest source
entry against the trusted root. The host profile is exactly
`retained-repository-analysis-v1`. Startup pins the approved snapshot, roots,
principal, grants, provider configuration, executable, build and service
bounds. The host's `controlled-proof` Cargo feature forwards both lifecycle
`native-sim-proof` and `controlled-proof`, plus the task owner's controlled
timing hook; a controlled build requires startup
`features:["controlled-proof"]`. A default build requires `features:[]`.
Startup rejects a declaration that differs from the compiled host, and
`host_identity.features` is derived from the binary rather than copied from
startup. Request bytes do not install or change these selections. The task store and socket
parent directories are owner-checked mode 0700, and the host socket is mode
0600. The socket admits the configured local UID; this does not claim isolation
from hostile code running as that same UID.

Each connection carries one strict JSON request and one JSON response, each
terminated by a newline and bounded to 128 KiB. An oversized request receives
`request_too_large` before dispatch; an oversized response is replaced by the
bounded `response_too_large` refusal rather than partial JSON. Commands are:

| Request | Durable owner/lifecycle consequence |
| --- | --- |
| `{ "op":"task.submit.v1", "payload": SubmitTask }` | Publish immutable owner input first; then submit its exact locator through lifecycle v2 enqueue. An ambiguous lifecycle reply leaves `pending_admission`, which requires command/delivery readback. |
| `{ "op":"task.answer.v1", "payload": AnswerQuestion }` | Bind an answer to a pending question with matching recipient, scope, expected owner revision, and trusted human authority reference. Native execution cannot invoke this command as an answer. |
| `{ "op":"task.observe.v2", "task_id": ID, "delivery_id": ID?, "command_id": ID? }` | Return independently qualified owner, lifecycle, native delivery, admission-command and control-command components. No atomic cut or task-command association is inferred. See TO2 below. |
| `{ "op":"task.request_replacement.v1", "payload": ReplacementRequest }` | Check task revision and operator authority, then forward a versioned lifecycle replacement command and return its qualified result. |
| `{ "op":"task.contract_grant.v1", "payload": GrantContractRequest }` | Check operator authority and configured grant identity, then forward an exact lifecycle grant contraction command and return its qualified result. |

`SubmitTask` includes `command_id`, `task_id`, `subject_id`, `kind`, `objective`,
`engineering_question`, `snapshot_ref`, and optional canonical facts. `kind` is
`repository_analysis` or `design_inquiry`; both use the same lifecycle kernel.
Each fact carries an ID, kind and exact text; optional `current:false` preserves
a superseded decision as historical source material without making it a current
dependency or obligation. Commitment and uncertainty remain distinct
obligation kinds in semantic projection.
The first submit creates the task. Further submits for the same task append
queued immutable input intents while leaving the active cursor unchanged. The
owner stores the exact JSON locator `retained-task-input-v1`; lifecycle's task
port checks the stored bytes, input ID, task/subject, owner revision and cursor
reference before admission. It then owns the input sequence and delivery. The
host cannot infer admission or completion from the owner commit alone.
For lifecycle v2 commands, the host obtains exact subject revision and context
from the supported read-only `subject_revision` operation. The observation
journal cursor and task-owner cursor are distinct revisions and cannot satisfy
that lifecycle command comparison. The host chooses exactly one trusted startup
grant for that context and `enqueue_input` or `request_replacement` scope;
requests cannot supply a grant. Startup validates that its named g0 enqueue and
replacement references match unique initial-context grants. Missing or
ambiguous bindings reject a fresh command before owner publication. A context
change between preflight and command construction is resolved by rereading the
current subject context; the resulting exact command is recorded before send.
An admitted command with a lost reply uses only its saved command ID and digest
for readback, never a new context/grant selection or resend by absence.

Host responses carry `status`, `host_identity`, and the applicable `owner` and
`lifecycle` objects. `host_identity` records PID, Linux process starttime,
incarnation, build ID, profile and the pinned build/source/provider identities. Lifecycle context generation, native session
and attempts come from lifecycle observation. Ordinary context replacement
keeps the host PID/starttime/incarnation; a host restart creates another
incarnation and separate recovery lineage.

Replacement payload fields are `task_id`, `command_id`,
`expected_task_revision`, `authority_ref`, and `reason`. Grant-contraction
payload fields are `task_id`, `command_id`, `authority_ref`, `grant_ref`, and
numeric `expected_revision`. The host records the exact outbound lifecycle
command before dispatch. Repeating either host command checks the original
payload identity and performs only lifecycle readback by the saved command;
an unknown readback is still uncertain and does not cause a resend.
Responses distinguish accepted, explicit rejection and pending/unknown status;
an I/O timeout cannot establish a grant contraction or completed replacement.

Before binding its ordinary socket, the host reads lifecycle's required
`native_delivery_status.recovery_only` boolean and fails closed if that read is
missing or unavailable. The value is fixed for the service incarnation. In
recovery-only mode, `task.observe.v2` and exact previously committed command
readback remain available. A repeated submit or answer returns only its saved
owner receipt when command kind, task ID and payload digest match. A repeated
replacement or grant contraction performs only readback of its saved control
command. An absent/new mutation returns `recovery_only` before any task-owner
write or lifecycle command. Recovery-only observation does not claim a native
turn, logical delivery, or unsafe predecessor close has settled.

## Canonical material and bounded task operations

The task store persists objective, approved snapshot reference, cursor and
revision, source bytes/revisions, questions, answers, completed effects and
attributed result. Its `CanonicalMaterialPort` projects exact `MaterialInput`,
reports relevant dependency revisions, and holds a task transaction across the
short lifecycle commit. A later answer updates the affected question source;
an unrelated task input does not alter that source's dependency revision.
Source trust is assigned by the authenticated host boundary, not by source
text. Missing or uncertain owner readback never becomes current material.

The installed operation registry is exactly `workspace.snapshot.read.v1`,
`task.question.open.v1` and `task.result.publish.v1`. Lifecycle validates native
entry, grant, delivery/attempt and operation identity before calling the task
port. The snapshot operation remains lifecycle-owned. After its child receipt
is committed, lifecycle calls the owner with exact effect ID, delivery ID,
receipt digest and expected cursor revision; the owner records this consequence
idempotently. An ambiguous reply is resolved by effect-ID readback.

Opening a question commits its pending recipient/scope identity and does not
answer it. Publishing a result requires current owner cursor/source references
and no required pending question; the task owner commits the attributed result
under its own revision. The task operation ID is an idempotency key. A lost
reply or process death after owner commit is resolved by owner operation-ID
readback, never by retrying because a response is absent.
Lifecycle obtains a fresh owner `current_cursor(task_id)` binding for each new
native task operation after exact delivery/attempt checks. The admission-time
input locator cannot stand in for later owner revisions after a completed read,
question, or authorized answer. The owner repeats that revision check within
its short commit transaction.

For a task mutation, the owner starts a short task `BEGIN IMMEDIATE` transaction,
checks its cursor/source revisions, and invokes the lifecycle grant-fence
callback while still holding that transaction. Lifecycle takes its own `BEGIN
IMMEDIATE` grant/currentness fence, rechecks authority, and invokes the owner
commit callback before releasing the lifecycle lock. This task-then-lifecycle
order also applies to semantic material currentness commits. A contraction
committed before the fence wins; a later contraction waits. Busy, timeout and
ambiguous outcomes remain explicit. No cross-store atomicity is claimed.

Only the controlled-proof build accepts the trusted `proof_barrier_dir` timing
point `task_result_after_owner_begin`. An explicit `.arm` file makes result
publication write `.reached` after task `BEGIN IMMEDIATE` and before its grant
fence, then wait at most 15 seconds for `.release`. This pauses only; it never
supplies grant validity, task currentness, or a commit. The ordinary host grant
contraction still uses the same task-owner mutex before forwarding lifecycle
control, so a contraction requested during that pause can time out as pending
and proceed only after the owner lock is released. A later contraction cannot
retroactively invalidate an earlier result commit. Default builds contain no
task-lock barrier.

The profile allows a fresh native thread per context. It refuses subjects
requiring the same native session unless a separate capability and authority
contract accepts them. Neither a new context nor native terminal text grants
replay, question answers, or result publication. This host exposes no general
shell, repository-write, review, campaign or claim mutation capability.

## TO2 independent task observation

TO2 read-value validation is shared by host, UI and scoped proof receipts through
`lifecycle-wire::validate_command_observation`, `validate_grant_result` and
`validate_native_delivery_status`; lifecycle values use the existing
`parse_snapshot`. These passive validators confer no authority. A present
admission/replacement value is a supported complete command-result v2, and a
grant result is the supported v1 contraction readback. Only the exact unversioned
`{"status":"unknown"}` object represents the current unknown command outcome.
Malformed/future/contradictory values are read failures, not command rejection.

Saved admission/replacement locators are parsed and checked against their command,
subject, principal, operation and digest before read dispatch. A control
`request_sha256` binds the original task payload, not the saved service command;
consumers validate its shape without claiming an unavailable preimage. A present
grant scope also includes saved `grant_ref` and canonical `expected_revision`,
which bind the readback's grant and resulting revision. No task-command edge is
created. Native rows retain the existing required identities, scalar/nullable
fields and opaque state strings; 0 through64 complete rows remain valid and65
must not be truncated into a present result.

The UI retains last-known present components with their original verified scopes
when an owner read is unavailable/refused and dependent subjects become unresolved.
Latest failure scopes remain unresolved and stale; old identities are never copied
into the latest response. Repeated failures retain the same anchor. Established
identity changes, successful absence/inapplicability, endpoint/request/host changes
and conflicting established locator bindings invalidate affected anchors.

`task.observe.v2` is the sole task-observation operation for the changed host.
`task.observe.v1` returns `unsupported_observation_version`; clients must not
silently retry or reinterpret v1. Other mutation versions and lifecycle snapshot
v1 remain unchanged. Optional target keys are omitted, never null; unknown keys
and invalid identifier types are refused.

The envelope has `protocol_version:2`, `status:"observation"`,
`consistency:"independent_components"`, exact validated `request`, existing
`host_identity`, and five `components`: `owner`, `lifecycle`,
`native_delivery_status`, `command_result`, `control_result`. Each component has
its own `scope` and one tagged state:

| State | Payload and meaning |
| --- | --- |
| present | `value`, `revision`, and native-only `completeness`; empty collections are known only here. |
| absent | `record_kind`; a successful lookup found no such record, not proof of no external effect. |
| not_requested | The optional target was omitted. |
| not_applicable | Safe `reason`, such as `task_absent`. |
| unavailable | Safe `reason`; no value was established. |
| refused | Safe `reason`, with `limit:64` only for native capacity refusal. |

Non-present states carry no value or revision. Raw errors, SQL, paths and
credentials are not observation diagnostics. Owner absence makes lifecycle and
native components not-applicable; owner failure instead leaves those reads
unavailable with `task_dependency_unavailable`. Command reads are independently
scoped to the configured subject and selected command, even for an absent task.

Owner revision is `{kind:"task",owner_revision,cursor_revision}` from the same
TaskRecord, scoped to task and subject. Lifecycle revision is
`{kind:"lifecycle",cursor:...}` from the validated snapshot, scoped to subject
and optional delivery target. Known returned delivery identity must match that
target. These independent revisions need not agree and do not establish an
atomic cross-store cut.

Native scope is the subject-wide `native_delivery_segments` collection with
`filter:"subject"`, independent of the requested delivery. Revision is explicitly
`{kind:"not_provided"}`. Successful reads include
`completeness:{kind:"complete_for_scope",limit:64}` for the joined query only.
Zero through 64 matching rows are complete; a 65th yields `refused` with
`reason:"capacity_exceeded",limit:64`, no partial collection or exact total.
The store returns a typed complete/capacity outcome. Only that capacity outcome
adds the reason/limit to the service's `observation_unavailable` envelope;
ordinary storage or transport errors never imply capacity. Startup/recovery
continues to fail closed on unavailable/refused native reads.

Command scope carries subject, command and `task_binding:"not_established"`,
plus the saved admission digest or control kind/request binding when available.
Revision is `not_provided`. Locator absence names `admission_command_locator`
or `control_command_locator`, not absence of execution. Valid domain `unknown`
or rejected results remain present values; read failures/refusals stay separate.
No task-command edge, new durable revision, pagination or shared transaction is
introduced.

The [UI read client](../rust/crates/terminal-ui/src/task_observation.rs) uses one
connection and one caller deadline (positive, at most 30 seconds), including
connect/write/read and bounded strict decoding. It has no owner mutation API,
retry or version fallback. Last-known component values remain scoped and stale
after failed refreshes; endpoint/task/host identity changes invalidate anchors,
and old selection-generation replies cannot overwrite newer selections.
Rendering and fixture projections in the [UI design](../rust/crates/terminal-ui/docs/task-observation.md)
remain a separate integration surface, not automatic qualification of this client.

The TO2 tests separate synthetic 0/64/65 owner/composition/decoder checks from
the installed controlled-host-to-production-UI-client proof. Passing either is
not full S8 acceptance, live-provider qualification or custody/cutover authority.

## Offline schema compatibility and qualification boundary

The [lifecycle store admin owner](../rust/crates/lifecycle-store/src/admin.rs)
exports the actual snapshot schema and reads supported schema 4 or 5 capsules
without migrating them. Manifest and database schema must agree in addition to
the existing digest, artifact and coherence checks. Adoption may migrate a
verified schema 4 image only in its owned destination copy; the source capsule's
schema, digest, store identity and custody revision remain its historical identity.
The destination remains inactive until the existing exact transfer/fence/selection
contract admits it. A schema match alone grants no admission or retry.

[Capsule compatibility tests](../rust/crates/lifecycle-admin/tests/export.rs)
separate genuine schema 4 preservation from current schema 5 export, inactive
adoption and refusal of contradictory or corrupt evidence. The
[native semantic harness](../rust/tests/s6_semantic_preservation.rs) and
[native S5 compatibility harness](../rust/tests/s6_s7_s5_native_compat.rs) bind
their exact executable and fresh run/temporary/build roots before local simulated
native execution. Such provider-disabled qualification is not live inference.

These compatibility requirements do not reopen the accepted TO2 semantics above
or attest the unavailable original full-S8 plan bytes. Scoped TO2 acceptance,
inherited prerequisite qualification, later S9 recovery/cutover and operational
authority remain distinct; see the [S8–S10 roadmap](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md).
