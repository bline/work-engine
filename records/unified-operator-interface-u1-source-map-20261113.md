# U1 selected-task source and refresh map

Prepared 2026-11-13 for root review. This is a bounded map, not a new wire
schema, workflow store, owner handoff acceptance or production qualification.
The accepted [UI design](../app-server/docs/control-plane-causal-observability-ui.md)
and [TO2 contract](../docs/rust-retained-task-host-contract.md#to2-independent-task-observation)
continue to own meaning. Root owns separate architectural review and acceptance.

## Selected subject and evidence standing

The explicitly selected task is **`task:ui:sim`**, from the retained installed
controlled-host fixture `/home/bline/.ui-rust-proof/ui-refresh-5K2dI5`.
`components.owner` in its `completed.json` publishes task ID, subject
`subject:s8:task`, owner revision **4**, cursor revision **4**, objective,
input intents, answered question and attributed result. No campaign or run ID is
published; this map does not invent one.

U1 copied the original pending, answered, completed and result records into
[`builder/task-evidence/`](/home/bline/cbm-work/unified-interface-u1-20261113/builder/task-evidence/manifest.json).
The completed response SHA-256 is
`9995e541823f6e8c537d68e8507162dd72ad59c594b72a456c9b210bd246a54f`.
These are **observed historical controlled owner outputs**, not a fresh task
read on November 13. Original file modification times are recorded as archival
metadata; the envelope provides no whole-read wall-clock cutoff. Retrieving or
copying it today does not refresh its source age. The host receipt reports
`native-sim-proof-v1`; no paid provider was used by U1.

The historical host identity is incarnation
`host:91763b832be3df13a1d9dfc0305d47e5`, build `build:s8:external-proof`,
executable SHA-256
`2703037bbcbafb7ec285f9dc9b9688a47efcb59b8dbbac3e23a68a94002c899f`,
source-manifest SHA-256
`4f446a2a8077f4149c3d6c97e6d7a15ce3a000242bdb75f747d5b620f5bf86e5`,
lock `dd0f0d45baf76113e365543ac29da404bf1498c5d1471fca12bcde655527aabc`,
and toolchain `1d76a06616a42a984bad672eeb24ca39a0496e95a8b624e21df0d420d1557734`.
U1 rechecked all **143** retained manifest source entries, lock and toolchain;
no mismatch was found. That verifies retained bytes, not a fresh host execution.
The old socket is not activated by this slice.

## Supported native joins

Each edge below is mechanically derived from exact observed fields, with the
retained response as its evidence. Neither titles nor chronological or pathname
similarity participates.

| Edge | Exact producing owner and binding | Limit |
| --- | --- | --- |
| Selected task → subject | TaskRecord `task_id=task:ui:sim`, `subject_id=subject:s8:task`; matching owner scope | Scoped to this captured owner revision. |
| Task → input | TaskRecord `inputs[].input_id=input:command:ui:sim` | An input intent alone establishes neither admission nor completion. |
| Input → native segment | Subject-wide native collection has the same exact `input_id`; select the row matching the explicitly requested delivery | Subject equality alone would be insufficient for another task sharing that subject. Missing/mismatched input means unlinked. |
| Segment → delivery/attempt/context | `delivery:e11e8c9b06fa717f03a17cf4018152155b07340670758cd4233efde232408e13`, queue attempt `attempt:1`, native attempt `native-attempt:delivery:attempt:1:0`, segment 0, `context:s8:g0` | Native revision is explicitly `not_provided`; do not manufacture it from the lifecycle cursor. |
| Segment → native session/turn | `thread_id=01a142ba-b70b-7eb0-9900-b53bece80f30`, `turn_id=01a142ba-b722-7272-affc-928d3a20aeed` | Native completion is distinct from task result publication and acceptance. |
| Native session → Events read | Exact returned `sessionId` equals the owner-published `thread_id` | This is session-wide activity. It does not attribute every returned entry to this task or turn. |
| Events entry → native turn | **Missing**: `read_session` does not expose a structured turn ID per entry | Never infer it from transcript phrases or time proximity. |
| Task → command result | **Not established** by TO2; selected command target omitted | `command_id` text inside input intent does not override `task_binding:not_established` in command scope. |
| Task → campaign, candidate, review, budget or schedule | **Missing in this selected boundary** | No cross-owner association is invented. |

The [bounded evidence check](/home/bline/cbm-work/unified-interface-u1-20261113/builder/task-join-check.json)
verifies the positive chain and rejects changed task, changed subject and
unlinked input controls. It checks this evidence chain; it is not a new generic
join implementation or client acceptance test.

## Read/access and interpretation boundary

All rows use the UI design's **Observed** class unless marked derived.
Raw task/source/transcript material is private operator data, including when
sanitized. Redaction is not a public-release classification. No real user grant
credential or production provider was used. A future remote transport needs its
own authenticated, confidentiality-preserving qualification.

| Fact owner / route | Subject, source revision and cutoff | Unavailable meaning, access and limits |
| --- | --- | --- |
| Retained task owner via `task.observe.v2`, component `owner` | Task/subject above; owner and cursor revision 4; captured historical response, no published whole-read timestamp | Objective, input intents, question/answer and result are owner facts. Mode-0600 host Unix socket, configured UID, private parents and pinned startup; not isolation from hostile same-UID code. Absent task differs from unavailable/refused owner. Native liveness cannot replace these facts. |
| Lifecycle owner via the same envelope, component `lifecycle` | Subject and selected delivery above; store `store:ef29602dd8350d30440147e493dc4d80`, stream `stream:subject:s8:task`, commit sequence `32` | Snapshot protocol v1; targeted delivery and subject scope. Published event wall time is event provenance, not an atomic envelope cut. Failure remains `unavailable`/`refused`; task absence makes it `not_applicable`. |
| Native delivery owner via component `native_delivery_status` | Subject-wide `native_delivery_segments`, `filter:subject`; revision `not_provided`; captured complete-for-scope limit 64 | Complete 0–64 rows; 65 refuses with capacity_exceeded, never truncates into success. Selected delivery does not narrow native collection scope. Rows preserve opaque native states, nullable identities and history limits. |
| Admission/control readbacks via `command_result` / `control_result` | Configured subject and explicit saved command locator when requested; revision not provided; task binding not established | Both are `not_requested` here. A missing locator is not absent execution. Exact saved command/digest/principal/subject and grant-locator bindings are required for reads. No resend or mutation is implied by unknown readback. |
| Events compositor through `session_query::query` → embedded `owner.py` → actual `Server.call_tool(read_session)` | Current source SHA-256 `586128d82ebd1fa0c8813e5583924cd9ce812e6ffe76de0c2e63d396414e2a8b`, blob `d9617856c74f5792b0daaa13f2a3e816e02557bc` at shared HEAD `df991c81d6871f8c59620f016b564196ce6c8903` | Explicit absolute owner-only profile; local operator's selected homes, bridge/context null for monitor. Selected source bytes are read once, hashed, then executed as that byte snapshot. No source-pin bypass, provider/helper activation or transport fallback. This pin is a review candidate. |
| Current Events read of selected historical native session | Private copy of exact controlled native transcript, SHA-256 `59dd88875fab5ab5d2a6a1924925e1c2ddd3d1ac4c992933beb81cfef7f2f17f`; new U1 query/collector receipt times | Actual current source returned eight entries. Their timestamps remain historical; a fresh parser read does not make the task current. Exact entry IDs are byte-offset/index locators, not causal IDs. No per-turn entry join is exposed. |
| Rust bounded session projection `work-engine.operator-observation.v1` | `source_id=SHA256(exact profile bytes)` and scope hash of source/label/sorted selected sessions; exact output and per-session read completion times in custody | Derived presentation scope, not grant, account identity or owner revision. Ten entries/session, requested 2,000 characters, up to 20 sessions, one MiB snapshot. Per-session errors stay unavailable. New profile/source selection changes scope. Label is navigation text, never task objective. |

TO2 is five independent component reads. Native completeness applies only to its
own query. Task revision, lifecycle cursor, host incarnation and collector time
do not form a cross-store transaction or atomic workflow cut. A current caller
uses one bounded request/deadline and preserves the exact selected endpoint,
request generation and host identity. TO2's current read client uses one
positive deadline of at most 30 seconds and strict bounded decoding, with no
mutation API or version fallback. It currently lives in the TUI; U2 must settle
its shared Rust placement without duplicating its semantics in JavaScript.

For local session reads, `session-query` bounds input at 256 KiB, source at
512 KiB, output at one MiB and helper duration at 35 seconds. It checks profile
permissions, final symlinks, argument types/closed fields and the exact allowed
source digest. The helper redacts/sanitizes errors and preserves owner result
semantics. The collector rejects a non-null bridge. Session lookup can fail for
missing or ambiguous transcripts; malformed/incomplete transcript lines can be
omitted by the owner parser. The read is a bounded tail scan (8 MiB), not complete
history. Entry windows use original characters and can expand over a recognized
credential to redact its whole span; neighboring windows can repeat a marker.
System/reasoning roles are excluded from the collector request. Available empty
means the owner established no matching entries in that bounded scope.

## Per-owner refresh and time reevaluation

“Covered” here distinguishes implemented capabilities from qualified delivery.
**No notification delivery route was activated or runtime-qualified by U1.**
Every uncovered route below remains a condition for U3, even if session activity
is healthy. Reconciliation is read-only and never retries a domain effect.

| Admitted owner | Notification coverage and changes without session activity | Reconciliation and time reevaluation |
| --- | --- | --- |
| Task owner | No task-owner notification contract qualified here. Human answers, input intents, source-currentness changes and published result can change independently of the session stream. | Fresh `task.observe.v2` reads at the explicit endpoint; U2 must preserve original component scopes/cutoffs on failure and fence late replies/host changes. Current terminal implementation documents refresh four seconds after completion; that does not qualify all clients/current host. No task due/expiry policy is published in this response. Age cues must use a named presentation policy, not infer execution. |
| Lifecycle owner | No lifecycle notification-to-read route qualified. Queue admission, settlement, recovery and context changes need not produce a transcript event. | Reread its component independently through TO2. Preserve its own cursor and targeted scope. Owner-published timing may be projected when available; this selection does not publish a scheduler or retry-due policy. Unknown timing remains unknown. |
| Native delivery owner | Session turn events may wake inspection, but do not certify segment persistence, settlement or collection completeness. Native closure/history reconciliation can occur without a new message. | Reread the subject-wide collection. Respect the 64-row refusal and missing revision. Do not renew its freshness from another component or infer settlement from silence/age. No native timeout authority is derived. |
| Admission/control owners | No notification coverage qualified. Saved readback/grant revision may change independently of a transcript. | When explicitly selected, reread original saved locator only. Here omitted targets remain not_requested. Published grant expiry would require clock reevaluation under that owner's policy; none is exposed for this selection. Do not schedule execution or resubmit. |
| Events session owner | Source exposes `session.entry.matched`, `session.turn.ended`, renewable subscriptions and bounded replay. Pattern/role matching covers only selected transcript entries, not all task/source/title/account changes. These capabilities are source evidence, not U1 delivery qualification or remote-connector qualification. | Repeat the qualified bounded read; after a notification, renew/replay/reconcile only within the coverage actually established. File/title changes or gaps outside replay require independent reconciliation. Current `session.alert` deadline kind is retired and never raised. It cannot cover scheduler deadlines or task freshness. |
| Rust projection/file delivery | Collector polling is sequential; source-profile bytes are read at collector startup. File replacement/mailbox delivery is not an owner event. No session event proves that a profile/account/source changed. | Restart/invalidate selection on profile/source/endpoint/account changes. Atomic private-file replacement prevents a partial local snapshot, not an atomic owner cut. Existing consumer 120-second/future-time warning is a presentation cue reevaluated by the consumer clock. Mailbox fetch success must not reset source read time. Browser's existing 60-second polling and additional sync delay do not establish a five-second target. |

Unadmitted operational dimensions remain explicit: campaign and active slice,
candidate/gate/acceptance, independent review findings and obligations,
configured limits/units/cost/context coverage, durable scheduler items and
acknowledgements, repository-wide discovery, production account switching and
remote transport/authentication. Their semantic owners are identified in the
accepted UI design, but this selected read supplies no qualified association or
refresh route. A source that publishes a due/expiry/freshness threshold needs
clock-driven reevaluation even with no record or transcript change. The clock
projects that owner's time/policy; it grants no execution authority. The existing
session stream and retired deadline alert cannot fill any of these gaps.

## Qualification and source limits

The current source passed actual query-to-collector monitor tests, retained-pin
regression, redaction, private output, source update and pin-mismatch controls.
The task-session parse is a new current-parser read of retained controlled
native bytes, kept separate from the historical host observations.

The full four-read granted-operation checkpoint initially stopped before grant
seeding: the bridge secure-path owner refuses group-writable ancestors under
`/home/bline/cbm-work`. Failure and request 5178 remain retained. Root's
`fixture-route-amendment.json` / board 5179 authorized only this original-scope
fixture under `/home/bline/.local/state/work-engine/u1-query-fixture-20261113`.
The worker ACKs that amendment. No security check or shared ancestor was changed.
The actual current owner then passed all four reads through the production Rust
query path and actual bridge using synthetic preexisting records, including
authorization failures and ambiguous native states. A second passing checkpoint
retains exact per-read profile/mode and mutable fixture input bytes as well as
outputs. This is controlled local read qualification; it establishes neither
native execution nor production/remote-connector readiness. The separate
[U1 result](unified-operator-interface-u1-result-20261113.md) binds the custody.

Graph evidence was Tier 2, project `home-bline-code-work-engine`, generation
`2026-11-13T16:42:12Z`, watch active. Exact coverage is archived in
`builder/graph-coverage.json`. Relevant Rust calls have unresolved receiver/type
resolution; current bodies and the actual production call route were read
directly. Generic-name trace edges are not complete caller evidence. Unconsumed
broad search pages are not negative or exhaustive evidence. Historical fixture
files and manifest are outside this graph and were read/hash-checked directly.
Only cited boundary functions were verified; no complete component audit or
absence of all other read routes is claimed.

## Bound prerequisite revisions

| Path | SHA-256 |
| --- | --- |
| `app-server/docs/control-plane-causal-observability-ui.md` | `6b20355f8b04f4faa926e09815ee7864464c953972c7827c18df3b727ac53eba` |
| `docs/rust-retained-task-host-contract.md` | `f2fd68ce9b97307485883b5e8ecc7a67c9b1e89914d944dceaa8e66ebc358576` |
| `planning/unified-operator-interface-next-plan-20261113.md` | `5d01f5b79aa285db7c6851e1640aca9731a01d058faf39243ffaa91005ac1fdd` |
| `planning/unified-operator-interface-u1-budget-20261113.md` | `b229149b0081ebef336e27637222c378b99ea654f2252a178cb1fd26e5808ffe` |
| `rust/crates/session-query/src/lib.rs` | `9975a0bc2455f7accd2dca8e9a5a678b36fcde3462eaaeed4088184bc87a944c` |
| `rust/crates/session-query/owner.py` | `e127460938c087899e91da1a601592770cfdad3fdfaa0dab63a49b96305ffa02` |
| `rust/crates/operator-observation/src/lib.rs` | `9fdf934fee1cead783b8f7dd900da1cf03fc45d0f6fb5f03ac294aabdfe7dfd4` |
| `rust/crates/operator-observation/src/main.rs` | `2fcf5da2249ef14a323aa55dee6988b7fd947fdafa2e0cb886c845acd1d540d3` |
| `rust/crates/terminal-ui/src/task_observation.rs` | `ced562697b68cfd88f008ac322e01fb717411f71e32cbab535e15edd02a8d09c` |
| `rust/crates/work-engine-task-host/src/observation.rs` | `e2cd382fd077484be761faf238f82f0760825cacb31afc498439235eb945876f` |
