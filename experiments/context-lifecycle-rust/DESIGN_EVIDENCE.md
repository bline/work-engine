# Rust lifecycle requirements and evidence

## Assessment

The Rust experiment establishes a useful ownership and text-turn foundation. It
does not yet replace the current JavaScript lifecycle. The replacement design
retains semantic preservation, authority, custody and recovery requirements while
changing execution ownership, storage, provider boundaries and completion
observation. [DESIGN.md](DESIGN.md) specifies that architecture.

The assessed JavaScript iteration matters: it contains work absent from its published
baseline and remains gate-failed. Its failures are not one undifferentiated problem.
Some are demonstrated production orderings, some are integration omissions, some
are fixture contracts, and some historical schedules remain unexplained. The
distinctions below determine what the Rust design must prevent or test.

The initial assessment and this reconciliation are documentation only. Historical
reports and later accepted component results retain their separate evidence
scopes; this edit ran no runtime checks. The matrix below is not a blanket claim
of passing service or replacement tests. These documents authorize no provider
operation, disposition, migration, publication or activation.

## Offline continuation and accepted progress — 2026-10-07

The current published base is `b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2`.
S0 (`84be6695`), S1 (`e7c1d88e`), S3 (`634161c2`) and S2 (`b2efdbfa`) have
accepted published component exits. S26 below and the source manifest bind their
handoffs; the [implementation plan](IMPLEMENTATION_PLAN.md) records full commits.
These results establish foundations, core/store, executor and public wire/client
boundaries. They do not establish M1's composed service, U1, retained-workflow
integration or cutover. The prior assessment baselines below remain historical
identities, including the failed JavaScript gate and consumed live allowance.

The user selected offline replacement: Work Engine need not remain available
during development and is not required as its own development infrastructure.
This removes availability as a reason for temporary bridges or dual running;
runtime custody, context continuity, uncertain effects and recovery stay required.
S4 still joins the real Rust store/executor/observer. S8 requires an agreed actual
consumer in a permanent Rust host; S9 requires its selected permanent Rust
startup/deployment boundary. HP1 remains independent, and review/lifecycle owners
are not merged. A temporary adapter needs a concrete consumer/need, scope and exit.

Import versus explicitly authorized archive/clean start is unresolved with the
human/state owner. R23/T25, current-custody preservation and rollback evidence
remain required until an exact owning-contract amendment accepts a different
scope. Inactivity neither retires Q3/Q4-like obligations nor proves unknown work
settled. No current legacy-state inventory or operational cutover is claimed.

This bounded reconciliation directly read the governing and cross-session plans,
accepted progress handoffs and prior strategic handoff (S26–S27). Graph coverage
for the selected repository documents at generation `2026-10-07T22:18:38Z`
reported matching metadata and no recorded gaps; this is best-effort metadata,
not source completeness or a runtime audit. No historical evidence hash is
rewritten to make it current. The source manifest records new document hashes
separately from preserved baseline/source identities.

## Initial assessment baselines

| Subject | Identity and observed status |
| --- | --- |
| Initial repository | `/home/bline/code/work-engine`, HEAD `7583e2095725957470e28cc11fc69404386fec63`, with existing uncommitted work preserved. The Rust experiment is not represented by HEAD alone. |
| Current JavaScript effort | `/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-workspace`, detached HEAD `665b15d1d3fb38a9f68536e70191e9b550dd7f13`, plus 17 current paths matching the failed gate's source manifest. |
| JavaScript patch and gate | Patch `f961607389b9edfd98f9673f5ce974500323a3ba2f8acd1a1f7ccfd38edb7127`; stage 7: 241 passed, 1 failed, 8 skipped; stage 8 not run. Failure: `worker_unavailable`. No waiver or pass is inferred. |
| Rust validation | Final parent record reports 30 default tests and three principal actual-CLI controlled probes passed. The 18 source/build inputs still match `parent-validated-sources.sha256`. |
| Rust live proof | `live-text-vertical-2026-10-07`: three authorized native turns entered, three scoped text effects/deliveries succeeded, two input deliveries, one readiness delivery, two native threads, one coordinator. Remaining turn allowance: zero. |

The latest legacy successor-RPC amendment is planning-only. Its source-supported
possible close/RPC race does not establish the saved failing schedule and does not
change the failed gate. Sources S01–S04 below bind these statements.

The live Rust proof's coherent coordinator and adapter database backups are
preserved. The native child disappeared, but its exit status was not directly
observed. That limitation is not concealed by the successful text receipts, and
process exit is not their settlement basis. The live result is neither a general
semantic-quality test nor a tool-enabled proof.

## Evidence method and limits

This was a task-directed Tier 2 inspection, not a line-by-line audit of either
repository. Codebase Memory supplied structural discovery and targeted traces;
coverage was checked for the selected source paths. The initial main graph
generation was `2026-10-07T05:37:04Z`; final path coverage was checked against
`2026-10-07T05:59:58Z`. The legacy graph generation was
`2026-10-04T06:52:20Z`; modified paths were stale and the new observer/runner were
not tracked. Exact source reads controlled those claims. Zero graph callers were
not treated as proof of no callers.
SQLite evidence images are deliberately excluded from the graph; their saved
receipt hashes were checked directly, without a new database integrity audit.

Selected documents and receipts were read, material functions and ranges were
inspected, and the two saved source manifests were checked against current bytes.
Hashing a file establishes its identity, not that every line was reviewed. The
machine-readable [source manifest](design-sources.json) distinguishes source
inspection, report inspection and identity-only baseline coverage.

Reported tests were not rerun. Historical reports retain their original evidence
limitations; a source-inferred finding is not promoted to a reproduced failure.
No independent reviewer or Claude inference was used for this assessment. Current
proxy state, Q3/Q4 disposition and running production schema were not refreshed.
The design therefore assumes no new operational facts about them.

## Capability comparison

“Present” here means code or a documented contract exists, not that the unfinished
legacy composition passed its full gate.

| Capability | Current JavaScript effort | Current Rust experiment | Replacement decision |
| --- | --- | --- | --- |
| Admission and transition ownership | Reservations, leases, durable protocol and canonical tail | Typed owner/epoch plus durable coordinator | One per-subject reducer and transactional owner; external executor cannot grant admission. |
| Autonomous completion | Production tail and shared completion observer exist after earlier omissions | Coordinator executor progresses independently of observers | Every applied fact atomically plans the next justified effect; no consumer advancement API. |
| Semantic preservation | Compiler/verifier, projection and continuation contracts | Supplied package and bounded carried-fact demonstration | Preserve rich semantic contract; real inference effects, source reload and adequacy challenges. |
| Governing authority | Exact grants, source references and signed disposition validation | Narrow experiment authority | Pinned trust resolver and semantic reference validation, distinct from wire schemas. |
| Provider facts | Exact provider/turn identities, registries, multiple route classes | Pinned Codex native text receipts and history checks | Capability-specific adapters with explicit evidence domains; no universal terminal seal. |
| Tools and active work | Real retained-host integration is the intended target | Text-only supported settlement | Consume bounded owned-operation contracts; execution may remain with an admitted harness or domain service. Require explicit authority and settlement guarantees. |
| Custody and FIFO | Ready delivery, durable claim, failure/yield/disposition and handoff | Ordered durable inputs, conservative unknown entry | Keep stable delivery lineage across context generations; no implicit retry or retirement. |
| Restart and cancellation | Many patched callback, replacement and teardown paths | Conservative restart and detached observation | Owned tasks plus transactional recovery; distinguish local cancellation from remote settlement. |
| Readiness and proof | Same-run capture, identity-bound observer and proof runner | Byte/readiness binding and text vertical | One installed service/scenario driver; semantic readiness plus exact run/receipt binding. |
| Offline handoff | Schema-14 reconciliation, custody capsule, launch/manifest fences | Not production-integrated | Stable store for ordinary upgrades; explicit cross-store protocol for initial import and exceptional moves. |
| UI and diagnostics | Completion projections and detailed gate diagnostics | Experimental observer and saved replay evidence | Freeze versioned read-only DTOs first; commands remain a separate authority contract. |
| Efficiency | Existing deterministic cache-aware proposal | No accepted optimization policy | Measured usage and conditional payback first; forecasting/ML later and optional. |

Sources: selected legacy production paths in `design-sources.json`, S01–S05,
S16–S21 and the governing repository documents. This table does not assign a
misleading completion percentage to capabilities of very different difficulty.

## Related operation contract ideas

Three existing ideas refine the tool boundary without making their proposed
mechanisms production dependencies:

- **S19, Service Plane:** lifecycle consumes common identity, authority, effect,
  evidence and recovery information while domain payloads keep their proper owner.
  Its candidate operation grammar is not an accepted universal schema.
- **S20, Authority Commit Gate:** admission before dispatch does not establish
  current authority at a later commit. Each operation needs an explicit owner-defined
  entry-bound or commit-bound contract. The broad gate's upstream invalidation and
  dependency/writer registry prerequisites, plus its open review findings, remain
  unresolved. The lifecycle design does not adopt it or relax its invariant.
- **S21, Runtime ports:** a harness can own provider turns and native tools without
  also owning Work Engine authority. Exactly one executor/tool-loop owner is
  selected; its required guarantees must be demonstrated, not inferred from a
  wrapper or a provider name.

These are direct readings of selected idea sections, with matching graph metadata
at `2026-10-07T06:06:18Z`. No implementation audit or new runtime test establishes
that these proposed guarantees are already available. The design adds a bounded
operation contract and two acceptance scenarios, not a general service-plane or
cross-owner gate implementation project.

## Rust-first libraries and implementation planning

The initial design pass recorded the user's direction toward Rust and request for shared
library choices plus a multi-slice plan. [Rust development](../../docs/rust-development.md)
records that direction without retiring existing components. The proposed
[implementation plan](IMPLEMENTATION_PLAN.md) applies it to lifecycle; neither
document authorizes implementation or operational migration.

The existing lifecycle manifest already declares rusqlite, Serde/serde_json and
SHA-256 support, with Schemars/jsonschema as development dependencies. The UI
experiment independently declares Serde/serde_json. Its documented boundary uses
public schemas/fixtures rather than direct engine-database access. These facts
support shared public values, `lifecycle-wire` and `lifecycle-client`; they do not
establish a production workspace or existing stable shared API. The declared
Rust minima differ (lifecycle 1.92, UI 1.88), so S0 must select a compatible supported
toolchain rather than assuming the experiments already share one.

Primary documentation was consulted for Tokio, rusqlite, Schemars/jsonschema,
thiserror, tracing, Proptest, Loom, Cargo workspaces and an RFC 8785 encoder
candidate. Library capabilities inform the design; they are not a dependency
security audit or proof that particular versions/features compose correctly.
The source manifest records the references separately from local hashed inputs.

The design now distinguishes a U0 schema/fixture handoff from U1 read-only
observation against the actual controlled service. It adds package-boundary and
dependency/profile checks. Existing process/crash/semantic scenarios remain
required; parallel lanes cannot satisfy them by separate component passes.
Model assignments were proposed under S22; two OpenAI models do not establish
cross-provider review independence. That design pass launched no implementation
workers. Later S0–S3 completion and bounded S4 authority are recorded above;
the historical experiment comparison does not override those newer results.

### Cross-domain migration refinements

The user supplied another session's static Rust control-plane assessment and
authorized incorporating four refinements. Its overall 116-file inventory and
value/complexity ratings are attributed input, not independently re-audited here.
Direct reads of S24/S25 verify the narrow compatibility hazard: claim-evidence
sorts keys by Unicode code point and appends a newline; workspace coordination
uses JavaScript's default key sort and no newline. One replacement serializer
would therefore change historical hash inputs. Neither a schema-valid payload
nor a common SHA-256 primitive makes those identity contracts interchangeable.

The proposed changes retain the S0–S10 sequence and M1 integrated-service exit:

- **S0:** new versioned wire encoding, trusted codec selection and golden vectors;
  demonstrate legacy differences without building every legacy codec. S7/S9
  implement and qualify only the codecs needed by the accepted import.
- **S2:** process/task mechanics remain separable from lifecycle admission and
  provider-specific evidence. Future consumers can reuse mechanics without
  inheriting the lifecycle reducer or a second tool-loop owner.
- **S1:** reusable SQLite mechanics do not own schemas, durability requirements,
  transition/CAS semantics or external-effect settlement. No generic persistence
  framework is required for the first service.
- **S9-D/S9-H:** prove selected fresh Rust startup and current-state recovery
  before productive development; separately prove the historical
  deployment/handoff boundary if saved-state transfer is later authorized.
  Preserve distinct context, executable, provider-session and process identities.
  A full executable-generation-manager rewrite remains separate work.

T03/T24/T25/T28/T36 carry the strengthened checks below; no new runtime result or
acceptance milestone is claimed. U0 at S3 and U1 at S4 remain available before the
whole replacement finishes. The graph reported matching metadata and no recorded
coverage gaps for the selected paths at generation `2026-10-07T06:37:19Z`; exact
source/document reads control these bounded claims, not graph completeness.

## Requirements and acceptance mapping

Invariant IDs refer to [DESIGN.md](DESIGN.md#invariants). Test IDs below name
required future tests, not existing pass claims. Together they define replacement
coverage without preserving the legacy module layout.

| Requirement | Owner and required consequence | Invariants | Acceptance |
| --- | --- | --- | --- |
| R01 Exclusive admission | Store/reducer selects one owner and excludes competing old-engine admission; concurrent old/new availability is not required. | I01, I13 | T01, T02 |
| R02 Distinct identities | Core separates subject, context, process, build, native turn, input, delivery and proof run. | I07 | T03 |
| R03 Bounded authority | Trusted resolver binds scope, signer, expiry and revisions; operation owners specify entry-bound or commit-bound validity and required enforcement. | I07, I10 | T04, T18, T35 |
| R04 Command idempotency | Store preserves accepted and rejected command identities; changed replay conflicts. | I03, I07 | T05 |
| R05 Crash-safe entry | Executor records entry first; missing evidence never grants resend permission. | I03, I04, I11 | T06 |
| R06 Immediate result ownership | Runtime owns success/error delivery before external completion can occur. | I05, I14 | T07 |
| R07 Observation provenance | Adapter preserves ingress identity and distinguishes duplicate routing from distinct emissions. | I07, I14 | T08 |
| R08 Truthful classification | Reducer exposes failure, interruption, unknown settlement and historical completion separately. | I04, I08 | T09 |
| R09 Autonomous tail | Applied durable facts drive all required stages without caller or proof advancement. | I05, I12 | T10 |
| R10 FIFO and custody | Store preserves payload, acceptance and delivery identity; newer input cannot overtake. | I06, I07 | T11 |
| R11 Yield continuation | Scheduler carries one logical delivery through another transition without replaying effects. | I06, I08 | T12 |
| R12 Semantic sufficiency | Semantic service binds materials, commitments, uncertainty and governing authority, then verifies adequacy. | I02, I07, I14 | T13, T14 |
| R13 Native continuity | Provider selection respects subjects requiring the same native session. | I02, I07 | T15 |
| R14 Owned operation accounting | Typed operation contracts bind semantic owner, execution owner, authority guarantee, outcome, settlement and recovery; unsupported routes remain fenced. | I04, I07, I14 | T16, T34, T35 |
| R15 Exact switching | Reducer refuses unleased reset, unexpected lineage changes and unsupported replacement routes. | I01, I07 | T17 |
| R16 Honest clocks | Runtime distinguishes wall expiry, monotonic wait budget and completed-but-late facts. | I09, I10 | T18 |
| R17 Read-only observation | Projection/wait cannot mutate lifecycle; dropped consumers cannot abandon accepted execution. | I05, I08 | T19 |
| R18 Same execution proof | Proof harness uses actual run, consumption, build, profile and incarnation evidence. | I07, I12 | T20 |
| R19 Cleanup ownership | Runtime owns setup through shutdown and records exact tasks, children and settlement. | I05, I08, I09 | T21 |
| R20 Replacement races | Supervisor closes the launch gate and drains a stable owned set; normal recovery is not same-proof continuity. | I09, I13 | T22 |
| R21 Durable artifacts | Store/artifact port uses complete publication and coherent backups; partial evidence never becomes success. | I07, I14 | T23 |
| R22 Upgrade compatibility | Startup rejects incompatible stores before mutation and keeps one writable owner. | I11, I13 | T24 |
| R23 Cross-store continuity | Admin protocol preserves current custody and delivery state through all handoff crash cuts. | I06, I11, I13 | T25 |
| R24 Claim-specific retirement | Admin/reducer requires exact evidence and fresh authority; no retry or fake provider receipt. | I04, I07, I13 | T26 |
| R25 Diagnostic isolation | Test controls cannot change production authority; evidence collection has bounded truthful outcomes. | I10, I12 | T27 |
| R26 Wire contracts | Wire module separates shape from semantic validity, specifies domain/version-bound codecs without rewriting historical hashes, and supports explicit deployed-client compatibility. | I07, I14 | T25, T28, T36 |
| R27 Route and package closure | Typed production operation registry and packaged startup replace source-span inference. | I01, I12, I14 | T29 |
| R28 Efficiency without prediction | Policy uses disjoint cache counters, explicit prices and conditional future-work assumptions. | I02, I10 | T30 |
| R29 Horizontal integration | Role host is an external subject binding, not a lifecycle implementation class. | I01, I05 | T31 |
| R30 State-machine coverage | Core, runtime and store are tested at their real boundaries, including races and failures. | I01–I14 | T32, T33 |
| R31 Shared code without shared authority | Common public values, wire/client and separable process/SQLite mechanics preserve domain-owned policy and transactions; clients cannot import private lifecycle/store/provider capabilities. | I01, I05, I07 | T24, T36 |
| R32 Reproducible dependency and capability boundary | Pinned toolchain, resolved dependencies, schema generator and production feature graph preserve the tested contract and exclude fixture/admin domain entry. | I07, I12, I14 | T24, T37 |

## Lessons from the current effort

These are properties to preserve, not instructions to port each old fix.

| Lesson | Evidence strength and boundary | Rust design consequence |
| --- | --- | --- |
| F01 Lease promotion was not the full lifecycle | S05 records the stopped live proof; its no-tail-caller diagnosis is source-supported. | Applying a stage result transactionally plans the next stage. T10 must finish after the initiating caller disconnects. |
| F02 Progress is not completed proof | S06 records reset/successor/rehydration but capture-file `ENOENT`; admission reopening and final proof were not established. | Wait on committed state, validate same-run evidence, then tear down; offline preparation follows. T19–T21. |
| F03 Controlled sequencing differed from live | S07 findings are source-inferred: controlled evidence was prepared before the shared observer, and actual-worker failure coverage missed the runner. | Substitute provider behavior, not the service or orchestration sequence. T10, T20. |
| F04 Echoed identities looked validated | S07 identifies placeholder/echoed run and generation values. | Bind observed build/incarnation and consumed proof run separately; mutate each independently. T03, T20. |
| F05 Wrapper safety left a raw escape hatch | S08 identifies options overrides, an unbound runner export and bootstrap before consumption/cleanup ownership. | Private bound constructors, immutable scope and pre-launch setup ownership; no alternate public bypass. T04, T20, T21. |
| F06 A timer race is not a deadline contract | S08 identifies missing post-resolution/final checks. | Check absolute authority after awaits; retain late completed-close evidence without granting proof success. T18. |
| F07 Replacement escaped resource ownership | S09 identifies a changed worker outside the runner's initial cleanup subject. | Explicit supervisor-owned launch registry, stable shutdown scope and separate normal-restart proof. T21, T22. |
| F08 Failure could remain projected as pending | S09 identifies durable terminal failure not promptly recognized by the observer. | Outcome and settlement are independent dimensions in one committed projection. T09. |
| F09 Early rejection lost its handler | S10 controlled ordering reproduced worker exit 1 and absent failed receipt before association; after association, receipt persisted and worker exited normally. | Register completion ownership before starting work, retain errors as data and bind early observations later. T07. This does not prove every historical exit had that schedule. |
| F10 Duplicate notification dispatch inflated observations | S11 records identical notification bytes under two observation IDs, one claim and one receipt, with source fallthrough. Duplicate execution was not established. | Preserve one ingress coordinate through fan-out; count provider starts, claims, outcomes and deliveries separately. T08. |
| F11 Error observed did not mean effects drained | S12 shows an allowed ordering where error emission precedes bookkeeping cleanup; unsafe-close history can survive later zero counts and normal child exit. The saved failure's exact schedule is unproven. | Explicit drain acknowledgments and latched unsafe history, tested on both sides of the boundary. T21, T22. |
| F12 Diagnostics changed fixture eligibility | S13 describes an overlong inherited test argument rejected by a new diagnostic cap; the later shorter-selector pass supported that explanation, not a general production fault. | Test diagnostics with full-gate payloads, disabled compatibility and clear underlying errors. No arbitrary legacy byte limit becomes a Rust invariant. T27. |
| F13 Yield was mistaken for ordinary success | S14 identifies a branch whose first delivery was interrupted while a later lifecycle/continuation progressed. | Classify first-attempt outcome separately; preserve all later-input assertions. T09, T12. |
| F14 File existence preceded complete JSON | S14 identifies both acknowledgment producers and release publication. Incomplete JSON was observed, but the exact failed write interleaving was not retained. | Prefer typed barrier IPC; where artifacts remain, atomically publish complete immutable bytes and validate identity. T23, T27. |
| F15 A fixture can command a successor already owned by teardown | S03 gives a source-supported possibility for S02's `worker_unavailable`, not a reproduction or completed repair. | Setup/runner handoff revokes fixture control; success-recovery and replacement-rejection are separate scenarios. T22. |
| F16 Untyped time units rejected the Rust setup | S04/S15 record a seconds-versus-milliseconds error rejected before any provider entry. | Distinct validated wall timestamps and durations, preflight bounds and checked conversion. T18. |

The earlier regex/template source-scanner failures, executable-inventory omission,
and initial cross-generation custody gap were also reported in the conversation.
The current source confirms those responsibilities exist, but this assessment
does not re-establish every historical finding against its original candidate.
Their requirements are explicitly covered by T24–T29; they are not counted as new
reproductions or proof that the final JavaScript patch is correct.

## Proposed discriminating acceptance tests

Every actual-process case must retain exact process/incarnation IDs, command and
entry counts, durable observations/results, teardown outcomes and coherent state
readback where a database absence claim matters. A timeout or missing file alone
cannot establish non-entry or receipt absence. Barriers acknowledge the exact
boundary before fault injection; passive sleeps are not ordering proof.

| Test | Scenario and required result | First milestone |
| --- | --- | --- |
| T01 One writer and owner | Two actual services race to open one store through canonical and symlink paths. Only one owns admission; the loser cannot mutate or dispatch. Hard-linked database aliases are refused before entry. | M1 |
| T02 Old and new engine selection | Same subject is offered to both engines. Durable selection admits exactly one; fallback cannot silently enter the other after timeout. Test unavailable selected owner too. | M4/M5 |
| T03 Typed and durable identity rejection | Compile-fail tests prevent local context/build/turn mix-ups. Runtime negatives swap valid-looking IDs, digests and generations across durable records and fail before entry/application. Context, executable, provider-session and process mappings are explicit, not assumed one-to-one. | M1/M2 |
| T04 Grant trust and scope | Wrong issuer, request-supplied key, expired/revoked grant, stale revision, altered payload and caller override all fail at the public boundary. Valid wire shape supplies no authority. | M1 |
| T05 Command replay | Lose the response after commit; exact replay returns the same accepted/rejected result. Changed bytes conflict. Restart does not allocate a second input or effect. | M1 |
| T06 Entry crash cuts | Kill before entry commit, after commit/before send, after send/before observation and after result commit/before reply. Prepared may proceed under original authority; uncertain entered attempts never automatically resend. | M1/M2 |
| T07 Completion ownership | Actual child succeeds or fails before start response, before custody association and after association. Every result has an owner; durable failure persists without process crash, duplicate claim or lost error. Include non-custody calls. | M1/M2 |
| T08 Ingress versus execution | Route one terminal emission through all eligible consumers once; replay its coordinate idempotently. Then emit identical bytes under two legitimate source coordinates. No blanket byte-dedup hides the second event. Preserve independent start/claim/receipt counts. | M2 |
| T09 Outcome dimensions | Completed, failed, interrupted, unknown and contradictory facts produce distinct projections. Durable failure is visible even if settlement remains unresolved. A later transition does not erase earlier completion. | M1/M2 |
| T10 Autonomous complete composition | Enter through the real host, disconnect its waiter, run the full transition through the production reducer/executor, then observe successor readiness and custody without any test-side advancement. Run the same scenario driver with controlled and separately authorized live adapters. | M4 |
| T11 Custody and ordering | Enqueue two inputs, select but do not yet accept a successor, then accept custody without delivery. Hold the first entered claim; prove the second cannot start. Preserve payload, lineage and stable delivery identity across restart. | M1/M4 |
| T12 Yield through a second cycle | Active input yields into transition A, resumes under explicit continuation authority, then requires transition B. One logical delivery retains its receipts/work cursor; no effect replay and no next-input overtaking. | M4 |
| T13 Semantic preservation corpus | Preserve unresolved human approval, superseded decision, commitment, governing instruction, canonical reference and uncertainty. Successor uses their current meaning without answers being repeated in the next input. Verify missed/contradictory obligations fail. | M3 |
| T14 Semantic freshness and provenance | Invent a material reference; swap compiler/verifier invocation; change canonical or grant revision during verification; introduce late human answer. Reject or re-evaluate exact dependencies without accepting a stale package. Unrelated subject changes do not invalidate it. | M3 |
| T15 Native session constraints | Fresh thread succeeds only for a subject whose contract permits it. A same-native-session reviewer is refused unless the selected reset capability or explicit replacement authority satisfies its contract. | M2/M3 |
| T16 Effect-domain adequacy | Test text-only success, denied-before-entry tool attempt, accepted async question, running tool, child/remote effect and missing descendant evidence. Only the supported evidence domain settles; native terminal alone does not. | M2/M4 |
| T17 Switch attribution | Unleased reset, wrong old/new thread, stale switch identity, duplicate switch and unexpected compaction cannot be adopted by a later valid transition. | M2/M4 |
| T18 Time and deadlines | Use injected clocks for before/at/after deadline; test seconds/milliseconds confusion, rollback, restart, resolution before timer scheduling, final-return expiry and late completed close. Expired authority blocks new entry, not accounting. | M1/M4 |
| T19 Read-only wait | Drop UI/socket/waiters, lose event notifications and reconnect from a cursor. Accepted work continues; waits never advance state; gaps resync. Timeout and unavailable observation are not terminal operation failure. | M1/M2 |
| T20 Bound proof identity | Mutate each run/consumption/build/profile/root/incarnation independently, including same-build replacement. Reject public unbound entry, overrides and cross-run capture. Delayed capture occurs while the runner already owns cleanup. | M4 |
| T21 Setup and teardown | Fail discovery, startup, observation, receipt validation and close. Preserve original error plus cleanup report. Distinguish close not attempted, rejected, timed out, resolved and resolved late. Retain exact child exits and parent effects. | M1/M4 |
| T22 Launch and replacement races | Pause a new launch before registration and after spawn boundary; close the launch gate and prove no child escapes ownership. Separately prove valid restart recovery and invalid same-incarnation proof continuation. Hold a real effect across close, then release it; unsafe history remains. | M1/M4/M5 |
| T23 Publication and snapshots | Pause a blob before publication; readers see no committed partial artifact. Conflicting existing bytes fail without overwrite. Simulate commit/WAL boundaries and inspect a coherent image, not a main-file-only hash. | M1/M3 |
| T24 Schema and startup closure | Boot the packaged executable against supported versions and an incompatible future version. The actual old executable refuses before mutation. SQLite helpers preserve selected durability/migration requirements and do not expose a second writer. Pure grant/contract code is present; fixture and offline orchestration entry points are not live capabilities. | M1/M5 |
| T25 Current-state handoff and rollback | Import into actually absent roots through inactive destination, source fence, manifest CAS and destination ownership. Kill/retry at each step and reject stale/conflicting capsules. Rollback includes candidate-current completed and pending claims, never the initial snapshot alone. Real deployment/lifecycle composition preserves distinct identities, source fencing and old reference/hash readback under the original codecs. | M5 |
| T26 Claim retirement and late evidence | Attempt retirement with unresolved remote work, stale observation, wrong claim, alternate path, conflicting grant and concurrent paused launch. Each fails. A valid separate authority retires no-retry; late contradictory evidence fences dependents and preserves history. | M5 |
| T27 Diagnostics and barriers | Run supported maximum envelopes with diagnostics off/on. Wrong identity, malformed data, timeout, partial publication and absent acknowledgment produce bounded honest failures. Proof deadlines do not expand to collect cleanup evidence. | M1/M4 |
| T28 JSON schema and wire compatibility | Generate schemas from DTOs; validate strict commands, unsupported observation variants, schema-valid semantic invalidity, duplicate keys, Unicode and large counters. Rust/JavaScript vectors agree for each supported domain/version/codec; distinguish historical newline, key ordering and numeric rules. Wrong/unknown codec contracts fail; old identity bytes are preserved, not rehashed under the new envelope. S0 pins the counterexamples; S7/S9 qualify each codec reached by import. Old/new deployed client-service combinations negotiate compatibility or reject explicitly; reconnect/read-only clients cannot mint permits. | M2; foundations in S0/S3, import in S7/S9 |
| T29 Route coverage | Exercise every registered production operation through real admission and adapter entry. Map all eight legacy proof responsibilities to an explicit supported path or justified replacement; no missing class or direct-provider bypass. Packaged startup proves dependency closure. | M4 |
| T30 Cache-aware economics | Reconcile raw versus normalized counters, distinguish unavailable from zero, avoid double-counted cache/output charges, pin price revision and calculate conditional payback. No claimed optimum relies on an invented future workload. | M2, policy later |
| T31 Generic lifecycle | Run the same kernel for two differently named subject kinds with equivalent contracts. Change a role binding without importing a role-specific transition class. Context replacement keeps its process alive when supported. | M4 |
| T32 Model and fault coverage | Property tests permute queue arrival, completion, contradiction, timeout, restart and grant changes; assert invariants after every commit. Loom covers selected local synchronization only. Disk/busy/full, saturated result queues and observation-gap faults fence rather than deadlock or fabricate success. | M1 onward |
| T33 Trust and operating bounds | Validate socket principal isolation, denied commands, log redaction, storage permissions, task/frame limits, evidence retention and owned-child cleanup on the selected deployment. No same-user hostile-code isolation is inferred from Rust. | M2/M6 |
| T34 Operation and execution ownership | Exercise selected domain-service and harness-owned operations through the same lifecycle accounting boundary. Swap contract/implementation revisions, forge safe-looking metadata, expose an ungated tool path or assign two tool-loop owners: reject before unauthorized entry. Equivalent supported routes preserve exact evidence without requiring a second executor. | M2/M4 |
| T35 Revocation across effect boundaries | Deterministically race grant contraction with entry and with mutation commit. Entry-bound completion is allowed only under an explicit owning contract; commit-bound mutations must lose to prior invalidation. Kill/restart between entry, commit and receipt: preserve exact results or uncertainty, never infer safe retry. Unspecified validity or incomplete invalidation coverage prevents admission; no local-transaction claim spans unchecked external owners. | M1/M2/M4 |
| T36 Shared package and client isolation | Compile a consumer using only public wire/client packages; dependency checks exclude core/store/provider from that client and Tokio/SQLite/provider from shared values/wire. Compile-fail cases reject fabricated internal permits and mutation through read-only clients. Any extracted generic process/SQLite crate cannot import lifecycle core/store policy; otherwise inspect the module boundary and test that its facts/helpers do not grant admission or bypass the store. Run the same fixture/DTO contract through the actual service projection; UI fixture rendering alone is not U1. | S0/S1/S2/S3 foundations; S4 integration |
| T37 Dependency/profile reproducibility | Rebuild supported production and controlled-test profiles from the pinned toolchain/lock/features; compare schemas under pinned dialect/generator settings. Verify packaged startup and public capability enumeration exclude fixture providers and offline-admin entry on the domain endpoint. A changed dependency/schema output is explicit input change, not an inherited gate pass. | S0 foundations; S4/S10 integration |

M0–M6 are defined in the design. A later milestone may strengthen an earlier test;
the first-milestone label does not claim full production qualification there.
T02 tests exclusive selection and refusal, not an obligation to run both engines
online. The first S9-D selected `read_only` startup slice exercises packaged
binary/config/root binding and same-store restart only. It leaves retained
task-host/runtime composition and current Rust pending/completed/uncertain effect
recovery and rollback open. T25 historical import and cross-store
candidate-current rollback are deferred, not passed; the 108 historical
unclassified rows remain unresolved. S10-D may report only its tested
provider-disabled development composition, while full M5/M6 and replacement
acceptance remain separate. The S9 deployment composition targets a selected permanent Rust owner.
No numeric load, latency or memory guarantee is selected before measuring the
actual composition. Performance is a separate acceptance dimension, not inferred
from the language.

## Mapping the eight legacy proof responsibilities

The current clean-cutover source registers eight route classes. The Rust design
must account for their purpose without reproducing eight arbitrary adapter loops.

| Legacy class | Rust responsibility and required evidence |
| --- | --- |
| `context_attestation` | Bound source projection and predecessor/native identity, attributable to the real host. |
| `semantic_compiler` | Entered compiler effect with exact input/material and output provenance. |
| `semantic_verifier` | Distinct verifier effect and adequacy judgment; independence policy is explicit. |
| `retirement` | Authorized predecessor quiescence/switch and effect accounting; fresh-thread selection does not imply a reset occurred. |
| `rehydration` | Successor receives exact checkpoint and governing materials under control-only authority. |
| `readiness` | Identity/freshness checks plus demonstrated semantic continuation, not digest echo alone. |
| `reconciliation` | Atomic successor selection and exact custody projection; not automatic domain delivery. |
| `blocked_probe` | Real domain admission attempt remains fenced during transition, with observed zero unauthorized provider entry. |

The production operation enum is closed only for the implemented protocol.
Extensible semantic content remains versioned data, not a growing enum of every
kind of role reasoning. This preserves the repository distinction between hard
invariants and model judgment.

## Source catalogue

All local source/report bytes used for this assessment are pinned in
`design-sources.json`; relative source-code paths there are rooted explicitly.
These short IDs identify the material references used above.

- **S01** — [legacy source manifest](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/c2-midcapture-targeted-Y2CxTA/source-sha256.txt).
- **S02** — [last failed gate](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/c2-f961-gate-fHlZr4/failed-gate-receipt.json), SHA-256 `1b1b54478f6727d7aa0d2982602c3c904acbebd409dbf2cd79cce7f41753e291`.
- **S03** — [unimplemented successor-RPC amendment](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-successor-rpc-test-amendment-plan.md), SHA-256 `d799a648408eb7666cd8de2929dffdc8df0d1ba414f0342feef38ae7238b52fd`.
- **S04** — [Rust live receipt](evidence/live-text-vertical-2026-10-07/receipt.json), SHA-256 `feb5b5317dced64d452c75fb30393e843442ab3b50c6c3e3f65ef418e077f673`; [parent validation](PARENT_VALIDATION.md), [native contract](CODEX_CONTRACT.md), [native vertical](NATIVE_VERTICAL.md), and [source manifest](parent-validated-sources.sha256).
- **S05** — [lease-tail live proof stop](/home/bline/.local/state/work-engine/context-lifecycle-v28-activation-20260930/proof-stop-v29-proof4-lease-tail-20261002.json).
- **S06** — [capture-manifest live proof failure](/home/bline/.local/state/work-engine/context-lifecycle-published-candidate5-real-proof-20261004/terminal-failure-receipt.json).
- **S07** — [first shared-runner review adjudication](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-v4-candidate-1-review-adjudication.json), C1-F1 through C1-F6.
- **S08** — [frozen runner boundary adjudication](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-v2-frozen-patch-readonly-adjudication.json), V2-R1 through V2-R3.
- **S09** — [replacement and terminal-failure adjudication](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-v3-candidate-2-review-adjudication.json), C2-F1 and C2-F2.
- **S10** — [two-ordering actual-worker diagnosis](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-two-ordering-implementation-diagnostic-receipt.json).
- **S11** — [duplicate-terminal observation diagnosis](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-duplicate-terminal-observation-diagnosis.json).
- **S12** — [original-shutdown causal assessment](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-original-shutdown-causal-assessment.md).
- **S13** — [diagnostic envelope diagnosis](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-063cc980-candidate-invalid-diagnosis-v2.json) and [equivalent short-selector check](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/c2-short-selector-stage4-XVX7ie/discriminating-result-receipt.json).
- **S14** — [yield and handshake repair plan](/home/bline/.local/state/work-engine/context-lifecycle-schema14-consumer-wait-20261004-artifacts/completion-boundary-c2-yield-handshake-repair-plan-v2.md).
- **S15** — [Rust pre-entry time-unit diagnosis](evidence/live-admission-rejection-2026-10-07/diagnosis.json).
- **S16** — [semantic lifecycle contract](../../app-server/docs/semantic-context-lifecycle-manager.md) and [transition protocol](../../app-server/docs/context-lifecycle-transition-protocol.md).
- **S17** — [efficiency proposal](../../ideas/01-semantic-durability-and-context-lifetime-judgment.md), especially the deterministic cost and later-prediction phases.
- **S18** — [Work Engine design](../../DESIGN.md) and [philosophy](../../PHILOSOPHY.md): authority, semantic ownership, evidence establishment and context lifetime remain governing constraints.
- **S19** — [Service Plane and the Kernel/Domain Boundary](../../app-server/ideas/pending/service-plane-and-kernel-domain-boundary.md), especially sections 3–4 on operation contracts, owners and capability grants; exploratory, not an accepted common schema.
- **S20** — [Authority Commit Gate and Effect Admission](../../app-server/ideas/pending/authority-commit-gate-and-effect-admission.md), sections 1–3, 7–12 on status, mutation-boundary guarantees and unresolved prerequisites; not an adopted lifecycle dependency.
- **S21** — [Provider Turn, Harness Runtime, and Operator Projection](../../app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md), especially section 5 on explicit execution ownership and invalid compositions.
- **S22** — [Engineering model selection](../../docs/model-selection.md): revisable Astra planning/review and Sol implementation defaults, with effective-identity and independence limits.
- **S23** — [UI separation](../../app-server/docs/opentui-ui-exploration.md), selected client/host boundary, and [UI Cargo manifest](../../app-server/experiments/terminal-ui/Cargo.toml): independently owned experiment, declared dependencies and minimum Rust version, not live protocol integration evidence.
- **S24** — [Claim-evidence identity codec](../../app-server/src/services/claim-evidence/identity.mjs), `compareUnicode`, `serialize`, `canonicalJson` and `digest`: direct-source verification of code-point ordering and newline-terminated hash input, not a claim-service migration audit.
- **S25** — [Workspace coordination codec](../../app-server/src/services/workspace-coordination/contract.mjs), `canonicalJson` and `digest`: direct-source verification of default JavaScript key sorting and no appended newline, not a workspace-service migration audit.
- **S26** — Accepted S0–S3 progress: [S0 integration receipt](/home/bline/.local/state/work-engine/rust-context-lifecycle-implementation-20261007/s0-shared-branch-integration-receipt.json), [S1 accepted handoff](/home/bline/.local/state/work-engine/rust-context-lifecycle-s1-20261007/s1-accepted-handoff.json), [S2 corrected terminal handoff](/home/bline/.local/state/work-engine/rust-context-lifecycle-s2-20261007/s2-terminal-handoff-receipt-corrected.json), [S3 handoff](/home/bline/.local/state/work-engine/rust-context-lifecycle-s3-20261007/s3-handoff-receipt.json) and [S2/S3 publication](/home/bline/.local/state/work-engine/rust-context-lifecycle-s23-20261007/local-publication-complete.json). These are inspected result records, not rerun qualification.
- **S27** — [Offline strategic handoff](/home/bline/.local/state/work-engine/rust-context-lifecycle-s23-20261007/offline-rust-cutover-strategic-handoff.json), [current bounded authority](/home/bline/.local/state/work-engine/rust-context-lifecycle-s4-20261007/authority.json), [migration direction](../../records/rust-migration-plan.md#direct-rust-replacement-planning-after-host-qualification), [host operating premise](../../records/rust-host-replacement-plan.md) and [generation-plan disposition](../../records/rust-generation-integration-plan.md#current-disposition). Later host/startup and saved-state decisions remain with their owners.

## What review and implementation should settle next

Review the proposed ownership and state model as a whole, especially the separation
of semantic adequacy, delivery outcome, remote settlement and local shutdown.
Confirm the new-thread/native-session compatibility policy and the initial tool
profile's operation owners, authorization semantics and evidence contract before
promising replacement of a real retained workflow. Select the
semantic acceptance corpus and independence requirement explicitly.

The S0–S10 plan retains M1 as one coherent service boundary. S0–S3 are complete;
S4 is the next actual-service integration exit. Preserve the
existing text experiment as a comparison and avoid replacing its saved evidence.
The first full workflow milestone is M4, not
“the code compiles” or “the old example still passes.” Production substitution
additionally requires M5 recovery/migration and M6 acceptance.

The UI can continue with historical read-only views now. U0 at S3 supplies accepted
wire fixtures; U1 at S4 supplies the versioned read-only client and controlled live
service, before the rest of M2's provider/profile work is complete. Command UI
awaits separately accepted authority contracts. This assessment does not change
the UI session's files or claim its private replay format is already that runtime API.
