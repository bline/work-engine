# Direct Rust host and consumer replacement

Prepared 2026-10-07. **Proposed plan; implementation is not authorized by this
document.** The user authorized planning a direct Rust replacement after the
C3/R3 HOST failure and asked to avoid substantial temporary infrastructure
without a specific reason. This plan proposes permanent Rust domain libraries
and a native-review application host. It does not launch implementation, change
a default backend, migrate state, activate a generation, or authorize a provider
call or commit. All implementation and ordinary remediation use the user's
selected `gpt-6-sol`; planning and independently assigned review retain their
separate model and authority contracts.

The parent coordination claim is
`rust-migration:direct-host-plan`,
`418e3ae0-563b-4a75-9f66-15f9778fffbe`. Lifecycle, UI, the shared Cargo workspace,
and existing reviewer episodes remain with their current owners.

**Operating premise:** nobody is using Work Engine during this work. Development
uses the external tools and agents already available; running Work Engine is
not a development prerequisite. An inactive replacement can change its host and
consumers together, followed by one deliberate cutover. Continuous availability,
dual running, rolling migration and zero-downtime drain are not requirements.
Saved state and historical contracts still need explicit disposition; inactivity
does not authorize their deletion or convert unknown effects into settled ones.

## Current planning handoff

HP1 has completed its bounded library-readiness work; see the
[exact validation receipt](rust-host-hp1-validation.md). The user subsequently
authorized parallel HP2 planning. The [claims plan](rust-claims-hp2-plan.md),
[campaign plan](rust-campaign-hp2-plan.md) and
[shared contract](rust-host-hp2-contract.md) refine the HP2 scopes below.
Those documents are implementation proposals, not implementation grants.
The first profile preserves the existing remediation/succession refusal until
the separate RC/episode join is accepted; the roadmap's retained-remediation
exit is conditional on that join. Historical HP1 proposal language below records
the reviewed scope and does not undo its completed status.

## Decision proposed

Link the compiler and review-episode application directly into a Rust
native-review application. Keep the compiler pure. Give the episode application
private access to its store and an explicit trusted admission boundary. Move the
native-review workflow and the claims/campaign operations it consumes into
domain-owned Rust packages; compose them in one host process initially. A second
process for each domain is unnecessary for this first consumer. The reviewer
process remains outside the host's write/admission authority.

This is a substantial application migration, not a transport optimization. The
first bounded implementation proposal is **HP1**, below: finish direct episode
library consumption and successful large-history behavior using the existing
compiler exports. HP1 is preparatory and retires no production caller. HP2–HP4
identify the concrete downstream closure and actual dispatch transition, so
library completion cannot substitute for host replacement.

Use an offline integrated replacement, without building a general asynchronous
Node-to-Rust primitive bridge. The previously reviewed
[generation integration plan](rust-generation-integration-plan.md) remains a
documented alternative; its artifact, configuration, restart and fencing
obligations survive where they protect permanent startup, identity and recovery.
Its two-child packaging and online handoff route are not requirements for the
offline replacement.

These new **HP** identifiers do not redefine C4, RC, R4 or R5. C4 still owns
extension compilation/attachment interoperability; RC still owns conditional
semantic corrections; R4 still owns episode data/failure qualification; R5
still owns operational cutover and predecessor disposition. Their obligations
are carried into the relevant joins below, not declared completed by this plan.

## Evidence that changes the route

The implementation base is committed C2/R2
`bcbc52eb071270dc56d4dc65489b26b269506604`. Isolated C3/R3 evidence is preserved in
`/home/bline/code/work-engine-c3-r3/docs/rust-c3-r3-validation.md` and
`/home/bline/.local/state/work-engine/rust-c3-r3-20261007/`.
The reviewed C3 14-path aggregate is
`ecacc79cdbf0618074532ea6a4edc3b1236f05fa5a9ac0f54d73877cd3a559d1`;
the R3 22-path aggregate is
`63e80543dab0069d21e5d2dea55b13cdcb5a595ec3300d9c0b64d84017c0b1eb`.
The six retained findings were closed. The joined result records 129 Rust tests,
35 fault tests and 91 Node tests passing. Those results establish their bounded
correctness scope, not host readiness or acceptance of a different composition.

The corrected HOST profile is SHA-256
`d389c2d6d485bb961a17524508f59c6967b3bebbf2e0857c623974f45c05ca9b`;
`host-candidate4.json` is
`099b8d7434fa3998acfc81b7026af3d4e39bba53b628e95b05dc52a2d52c0266`.
Normal external control p99 was 7.27 ms legacy, 87.58 ms compiler-only,
69.25 ms episode-only and 143.25 ms combined against the accepted 50 ms target.
All normal work completed. Both episode-selected variants failed all 200
near-ceiling history requests under their 250 ms child deadline. Legacy
completed all 200, but its large-workload control p99 was 583.77 ms. The 31,207,216
byte, four-row history is therefore a distinct successful-result requirement,
not merely an event-loop responsiveness test.

The current compiler bridge performs three synchronous calls in the normal
workload, each including executable capture/checking, staging, launch and reply
validation. The episode bridge adds per-call admission, launch, database opening
and history validation. These observations support removing the repeated process
boundary. They do not apportion the cost of individual stages, prove that Rust
history materialization is cheap, or establish that moving work off an event
loop makes the large request succeed. The injected synchronous 100 ms compiler
timeout and legacy 5,000 ms SQLite busy wait also explain why small hot-path
tuning cannot establish the full control-response gate.

The old failure remains a failure. A direct-host run gets a new exact subject
and profile; it cannot be relabeled as a passing run of the old four-selector
experiment.

## Smallest permanent boundary

| Placement | Proposed responsibility | Why this boundary |
| --- | --- | --- |
| Existing `work-engine-compiler` library | Captured-input skill verification, manifest projection and requirement satisfaction; preserve the historical codecs and distinct path bases. | Pure work already has Rust exports. An IPC service adds no authority or isolation here. Source capture/Git/AEG I/O stays with its existing compiler host owner. |
| Existing `review-episode-core`, `review-episode-store`, `review-episode` | Episode validation, transitions, writer fencing, CAS, exact history/recovery and bounded direct results. | This is one domain owner. Library calls can preserve it without a daemon or per-operation child. |
| Proposed `claim-evidence` package | The exact claim/finding publication, establishment, projection and reliance semantics needed by native review, with its own codec and store. | An episode result does not establish a claim. Claim authority and immutable lineage stay here even when linked into the same executable. |
| Proposed `slice-campaign` package | Campaign admission/revision, selected review obligation, native-review request admission and durable recovery transitions. Broader campaign operations enter only when needed for whole-root dispatch closure. | A caller-provided selection object cannot replace the authoritative campaign state. One process can consult the actual owner before an episode or provider action. |
| Proposed `review-workflow` package and `work-engine-review-host` binary | Compose those domains with subject verification, result validation, reviewer profile/session handling and a bounded operation dispatcher. | This is the permanent production consumer of the libraries. It replaces the native-review application, rather than inventing a CLI solely to exercise a service. |
| Reviewer/provider child | Execute the selected provider realization and return attributable process/session/result observations. | The child must not acquire episode, claims or campaign write handles or host admission capabilities. Provider identity and required observation remain separately verifiable. |

Packages need not each contain a binary, database framework or public protocol.
Start with modules inside the proposed domain packages. A separate service is
justified only by a concrete independently deployed consumer, necessary authority
isolation or separately owned availability requirement. Multiple callers and
serialized database access alone do not force a process boundary. If that need
arises, expose the domain's bounded operations, not raw SQL, generic reducers or
deserializable permits. This plan does not claim protection from arbitrary
malicious code running as the same OS user; R3's inherited-descriptor profile did
not establish that property either.

The Rust host uses bounded worker ownership for blocking database, Git,
compilation and large serialization work. The control dispatcher remains able to
report readiness, queue refusal and unresolved operations while that work runs.
CPU-heavy work on an asynchronous task alone is not isolation. Bound queued work
and bytes, preserve result capacity for admitted operations, and retain a
write's identity when its requester disconnects. These are host consequences;
they do not require a universal actor system or cross-domain scheduler.

### Alternatives and cost

| Route | Cost and consequence | Disposition |
| --- | --- | --- |
| Tune synchronous C3/R3 calls | Small changes; repeats capture/spawn/parse and cannot avoid a synchronous timeout blocking control. Does not solve successful large history. | Retain as evidence, not the replacement route. |
| Broad asynchronous Node bridge | Requires async propagation through synchronous manifest, episode, claims and campaign consumers plus worker/queue/admission/restart rules, then another migration later. | Not proposed absent a named retained consumer whose delivery requires it. |
| One daemon per compiler/episode domain | Additional launch, protocol, authentication, cancellation, reconciliation and deployment seams before a second consumer exists. | Reject for this first composition. |
| Rust libraries inside a permanent native-review host | Reuses the cores; requires honest porting of the claims/campaign/reviewer closure and later dispatch/migration work. Lowest redundant infrastructure, larger application scope. | Recommended. |
| Replace the entire app-server/generation/lifecycle/UI at once | Broadest consumer retirement but joins unrelated authority and operational work. Very high integration risk and no bounded first exit. | Keep as separate domain migrations. |

Relative complexity: HP1 low-to-medium; claim/campaign review closure and the
native host high; full dispatch, state migration and executable replacement very
high. These are scope judgments, not delivery-time estimates. The cost is in
preserving semantics and recovery, not translating syntax.

## Source-backed consumer closure

This is the bounded closure discovered for native review and manifest projection,
not an exhaustive repository-wide retirement inventory.

| Current consumer/source | Consequence for replacement and retirement |
| --- | --- |
| `runtime-manifest.mjs::projectRuntimeManifest` is called by bootstrap `roleEnvironmentSource`, `createExecutableGenerationRoleEnvironment`, `loadRuntimeManifest` and `createNativeReviewHostOwners`. | Moving the native host removes only one caller. Bootstrap and worker reconstruction remain Node consumers until their own dispatch/deployment replacement. Preserve delivery, identity and requirements bases and first hydration as well as reconstruction. |
| `native-review-host.mjs::createNativeReviewHostOwners` constructs the manifest, profile registry, provider adapter, implementation/specialist validators, episode owner, claim store, finding bridge and production-path evidence owner. | Replacing only the episode adapter is not replacing this host. Captured YAML and profile inputs belong to the executable/configuration closure. |
| `native-review-host.mjs::createNativeReviewHost` derives authority from the current campaign, immutable candidate, selected obligation and retained session; the isolated R3 version binds a campaign owner callback. | HP3 gets authority from the actual Rust campaign owner. A request body naming the same fields is not a grant. No new cross-process campaign-grant protocol is invented in HP1. |
| `native-review-closure.mjs::createNativeReviewClosureService` performs initial execution, result admission, required-claim establishment, durable episode result, finding publication and builder projection; it also owns remediation/recovery joins. | The Rust workflow composes real owners and preserves the crash boundary between episode commit and downstream publication. Bounded R3's RC refusals remain until the RC owner accepts their replacement. |
| `slice-campaign/service.mjs::runNativeReview`, retry/remediation/evaluation methods, and `terminalize` publish campaign revisions around review effects. `sqlite-store.mjs` also owns workspace admission and supersession. | A review-only Rust profile can qualify on private roots. Production cutover cannot let Node and Rust update different fields of the same campaign row. All mutators for a migrated root need one owner or an accepted fenced whole-root transfer. |
| `review-finding-bridge.mjs::createReviewFindingBridge` publishes exact finding revisions, reliance and projections; `production-path-service.mjs::createProductionPathEvidenceService` separately records observations and establishment. | Claims need their domain validation/authority/store behavior, not a host-local cache of “established” booleans. Two required claim consumption boundaries and their exact subject/covered state survive. |
| `ImplementationReviewerRuntime`, `NativeClaudeCodeReviewerAdapter`, subject workspace/boundary construction and the loaded profile registry | Rust must preserve exact subject, realized read-only grant, requested/observed provider identity, session continuation and recovery evidence. An S2 process executor is not this provider contract. |
| `capability-host-runtime.mjs::createSupervisorCampaignCapabilityHostRuntime` composes campaign state with workspace, subject, receipt, completion, strategic and coordination capabilities. | Replacing `native_review/*` alone does not retire this runtime. HP4 inventories routes sharing migrated roots and either moves their semantic owner or defers the entire unsupported profile from cutover. |
| Executable bootstrap/role environment/manager and outer proxy host factory, documented in the generation plan | Host lifetime differs from worker/context lifetime. The outer host currently survives worker changes. A new binary needs owned selection and reconstruction; online drain is not a migration requirement under the offline premise. |

Graph call edges miss some injected/object-method relationships: for example,
the owner factory's trace reports no callers although the capability runtime
invokes its injected default. Direct source above establishes those material
relationships. No zero-caller result is used as a retirement claim.

## HP1 — Direct-library readiness and successful large history

**Concrete next implementation proposal:** one retained Sol/high builder owns
the episode application/store changes and their focused validation. Freeze an
immutable handoff of the listed C3/R3 candidate bytes first; this reuses accepted
component evidence without declaring the unmet C3/R3 HOST exit complete. Use an
isolated worktree and target directory. There is no shared-root edit until the
existing workspace integration owner hands off any required registration or
dependency change.

The compiler library already exports `project_runtime_manifest`,
`satisfy_runtime_requirements`, `compile_skill_unverified` and
`prepare_skill_verified`. The compiler handoff is their exact input/output and
error contracts, C2 source-capture responsibilities, historical identity vectors
and the future host's required path bases. **No new compiler service or duplicate
API is planned.** A small direct conformance case is justified if existing tests
do not bind the intended host inputs; otherwise this handoff needs no code.

The episode application currently accepts a protocol `Request`, returns a JSON
`String`, and stores a public `EpisodeStore` and admission port. Its `history`
loads a snapshot and clones values into a reply; `read` also clones a selected
state; writes read before admission and transactional write. These observations
identify the bounded work, not a measured attribution of the timeout:

1. Expose a direct application result/operation surface for the future Rust
   workflow, preserving the existing seven operations and typed errors. Keep
   transport framing and final wire emission at executable boundaries.
   Historical identity encoding, stored domain serialization and exact result-byte
   accounting remain application/domain responsibilities. Avoid a
   serialize/parse loop merely to call Rust from Rust. Encapsulate store access
   so a consumer cannot accidentally bypass application admission. Preserve
   private `AdmittedResult` construction and historical identity/result behavior;
   retaining a CLI wire adapter is unnecessary after its consumers are removed.
2. Retain current fixture/native admission profiles; do not make a publicly
   constructible “trusted” permit or introduce a production campaign authority
   mechanism here. HP2/HP3 implement the actual campaign-bound host admission.
   Direct Rust calls still need the selected trusted admission port. Library
   tests using fixture admission remain fixture evidence.
3. Measure one exact near-ceiling read through the direct application, recording
   SQLite fetch, integrity scan/parse, cloning/materialization, serialization,
   response bytes, peak allocation/RSS where observed and deadline outcome.
   Repair only the demonstrated bottleneck within these packages. Reuse owned
   values or bounded serialization where appropriate; do not omit integrity
   validation, silently paginate/truncate the result, lower the corpus limit,
   or substitute a cache without a sound validation/freshness rule.
4. Preserve transaction/replay, writer fencing and possible-commit recovery.
   Read/admit/write ordering changes require actual race evidence that stale
   admission or competing writes cannot cross the commit boundary. A faster
   happy path cannot remove these checks.
5. Include the bounded typed write-result change in `EpisodeStore::write` and
   `WriteDisposition`. Today `write_outcome` builds `reply_json: String`, and
   the store checks its exact length before inserting/committing an applied
   state or returning a replay. Decouple the typed application/store result
   from that wire string without weakening the precommit capacity check.
   The trusted application/store composition accounts for the exact encoded
   result under the selected codec/envelope and limit, not a caller-supplied
   size estimate. An oversized applied result refuses before durable mutation;
   final wire serialization cannot be the first discovery of that condition
   after commit. Replay retains exact result/identity and capacity behavior
   without adding history. Commit/rollback uncertainty still returns
   `OutcomeUnknown`; postcommit delivery failure retains the exact root,
   operation/transition/content identity for possible-commit reconciliation.
   This does not require serializing then parsing between Rust components.

**Owned paths:** existing `rust/crates/review-episode/src/{lib,admission,protocol}.rs`,
its CLI entry only as needed to adapt existing wire calls, focused package tests,
and `rust/crates/review-episode-store/` for the measured history/store issue and
the typed write-result/precommit capacity boundary above. The latter includes
`src/sqlite.rs`'s `WriteDisposition` and `EpisodeStore::write`, necessary result
exports and focused tests; it is not a general store redesign.
The exact entry filename and store functions are a bounded implementation
discovery, not permission to expand into campaign, claims or deployment. New
test/support files stay in these packages. A small compiler conformance test is
an agreed compiler-owner handoff. A new result document records exact subjects
and measurements. No Node consumer edits, daemon, generalized process package,
live database, RC implementation or default selection is
part of HP1.

**Proposed exit:** existing component correctness and actual-process fault
semantics remain passing; direct operations agree with the frozen historical
identities/results, including Unicode edge cases; all seven direct operations
exercise the same application owner; wrong authority/revision/scope still refuse;
exact encoded-size boundaries refuse oversized writes without a new state or
history row, preserve replay, and retain reconciliation identity after uncertain
commit or failed delivery;
the exact 31,207,216-byte four-row history succeeds without truncation and its
full elapsed time/stage costs are recorded. Declare a bounded diagnostic watchdog
before the measurement; it bounds the experiment, not the supported operation's
SLA. A timed-out diagnostic is failed evidence. Preserve the old 250 ms child
deadline as a comparison, without treating it as an accepted direct-service
budget. The host consumer selects a supported completion budget from the result
before HP3 performance qualification. HP1's exit establishes API/correctness and
successful-result evidence, **not performance acceptance**. Report memory where
observed; do not invent zero for unavailable counters.

Focused package tests, format, Clippy, locked release/fault builds and affected
historical codec/result conformance checks establish HP1; repeated
whole-workspace testing is not its objective. A direct-library performance test cannot qualify dispatcher
responsiveness. HP1 completion is reported as **library readiness only** and
followed by the HP2/HP3 closure below. If these bounds cannot produce the required
history result, return the exact remaining bottleneck and owner decision instead
of a new framework proposal or a relaxed pass.

## Subsequent slices and composition

These are bounded roadmap scopes, not implementation grants or fully reviewed
file manifests. HP1 is ready for a concrete implementation decision; HP2–HP4
need their exact owner manifests accepted as they are reached.

| Slice | Bounded result and dependency | Exit and actual retirement |
| --- | --- | --- |
| HP2-CE | Rust claim-evidence operations used by review: authority validation, finding create/revise/reliance, exact projection, observation and establishment/readback, on new private roots. Preserve domain codec, schemas and immutable lineage. Exclude general claim discovery/maintenance and RC correction until separately accepted. | Cross-language golden vectors plus real-store authority/CAS/replay/crash evidence for these operations. This is a production-intended domain subset, not full claim-service retirement or a live root migration. |
| HP2-SC | Rust campaign review profile: authoritative campaign identity/candidate/selection read, bounded initial admission into a new private root, native execution/recovery/retry/remediation/evaluation and associated revision CAS. An exact profile rejects unsupported routes before effects. | A controlled admitted campaign reaches review-ready and the supported review states through owned APIs, without direct fixture writes to tables or a Node campaign callback. Terminalization/acceptance and workspace publication remain unavailable unless their owners' complete contracts join. No mixed writer on a shared campaign row. |
| HP3 | Compose HP1 and HP2 in `review-workflow`/`work-engine-review-host`. Link compiler and episode libraries, real claim/campaign stores and result validators; construct exact immutable subject/profile/configuration; use a controlled reviewer through the production-intended adapter port. Own a bounded host command/observation interface for the existing native-review operations. | Actual-process initial review, exact two-claim establishment where required, findings/evaluation, supported retained remediation, no-replay recovery after the episode-result crash cut, successful large history and responsive control under a newly frozen profile. Qualified routes no longer execute Node manifest/episode/claims/campaign review code. This is controlled host qualification; obsolete runtime adapters can be removed from the inactive candidate as their consumers move. |
| HP4 | Replace actual capability dispatch and close runtime dependencies offline. Inventory every route sharing the selected campaign/claim roots, including terminalization, supersession, completion/publication and external bootstrap evidence where configured; implement needed Rust domain operations or explicitly exclude the whole unsupported profile from cutover. Add the real provider adapter and permanent executable startup/restart owner. | The actual supervisor request traverses the Rust dispatcher into the Rust application, reconstructs after process restart, uses bound source/configuration/builds and returns through real consumers. No new Node dispatcher shim or dual-runtime arrangement is needed. Provider-disabled and authorized provider evidence remain distinct. Only here is the selected end-to-end consumer replaced, subject to R4/R5 state and operational acceptance. |
| HP5 with R4/R5 and compiler cutover | Qualify copied-state migration, all enabled RC-dependent routes, rollback/roll-forward and whole-root transfer; then perform separately authorized cutover and source/export cleanup. | One qualified writer for each migrated root; exact downstream consumers use the successor; predecessor code is removed only after a bounded exhaustive inventory and explicit archival/compatibility disposition. Node elsewhere is deferred, not declared retired. |

HP2-CE and HP2-SC can run in parallel after agreement on exact reference/codec
and result types. Compiler work such as existing C4 can proceed independently
when separately authorized. HP1 does not manufacture compiler work just to keep
two agents busy. One integration owner coordinates HP3/HP4 and serializes shared
Cargo, lock and common-type changes; owners repair their modules through the
join. All these implementation roles use Sol. Design participation does not
establish independent review; any fresh review or retained continuation uses
the configured review authority and preserves its episode lineage.

The fixed next consumption boundary is HP3's native-review application, followed
by HP4's actual dispatcher. Additional library extraction requires a demonstrated
missing capability of those consumers. It is not a replacement completion metric.

## Authority, failure and state invariants carried forward

Campaign selection and operation admission remain campaign-owned. The native
host resolves the exact current campaign/selection/candidate and checks revision
at the relevant mutation/effect boundary. It does not accept a user-provided
`authority` object as proof of the grant. Reviewers cannot select themselves,
evaluate builder reliance, accept a campaign, or mutate the subject through the
host. Provider contracts retain their exact requested/observed identities,
capabilities, continuity, authentication and allowed retry evidence. Local exit
or cancellation cannot establish remote settlement or a definite pre-entry
failure.

Episode completion, claim establishment, builder projection, review acceptance
and campaign terminalization remain different facts. Claims retain exact
subject, covered state, consumer and consumption boundary. An observation may
establish false or unestablished status; a Rust type cannot promote either to
established. In particular, the two required claims for builder projection and
terminalization remain separately bound. Historical episode JavaScript JSON,
claim code-point ordering/newline rules and campaign identity rules remain
domain-selected codecs. No generic serializer rehashes old references.

Cross-store operations are not one transaction merely because they share a
process. Preserve durable admission, exact operation/transition/content identity
and readback for the boundaries between campaign preparation, episode commit,
claim publication and campaign result publication. A disconnect, crash, malformed
reply or uncertain SQLite commit is not evidence of absence. Reconciliation
returns absent/committed/conflicting/unresolved against the exact root and
operation before any authorized retry. Recovery after an episode result can
finish downstream publication without another provider entry. RC remains its
separately owned correction/succession join; no host port silently enables routes
bounded R3 currently refuses.

HP1–HP3 use new private scratch roots and preserve old roots. HP4 cannot adopt
R3's `native-host-v1` marker by changing its executable digest: that marker binds
the old profile/root/build. A successor application profile, identity binding
and migration belong to the root/deployment owners. R4/R5 qualify source fencing,
imported bytes/revisions/history/authorities, unknown pending operations, exact
downstream readback and destination admission. Because the system is inactive,
source fencing can be an observed offline ownership transfer; online coexistence
is unnecessary. The owner may choose an explicitly authorized clean start with
the predecessor state archived instead of import. This plan authorizes neither
discarding state nor silently treating it as migrated. Before new writes, rollback may
be a qualified return to the unchanged predecessor; after new writes, it requires
proven reverse compatibility or an explicit roll-forward/disposition. Restoring
old files or selecting an old binary is not by itself safe rollback.

## Lifecycle and executable-generation boundaries

The lifecycle S2 executor, S3 wire/client and S4 service have their own interfaces
and owners. S3 was published locally as
`634161c2f54dc4fcf9dcb53936951ecefe457235`, followed by S2 as
`b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2`, on the shared branch. Board message
1309 records that the published tree matches their accepted joined checkpoint
and all review findings are resolved. This establishes the S2/S3 source baseline,
not integration into this review host or an S4 service. A host handoff uses that
exact accepted revision and records the actual joined source/lock. S4 remains
separately owned; its actual U1 service boundary is not yet established here.

HP1 and the claims/campaign cores do not wait for lifecycle S4. If S2's accepted
code exposes genuinely separable process/framing mechanics, the owning team can
agree a bounded reuse or extraction. The review host does not import lifecycle
admission permits or core/store policy merely to launch a child. Claude review
session/evidence behavior remains a reviewer-adapter responsibility. S3's public
lifecycle client is not a universal review protocol. S4 establishes the actual
lifecycle service and controlled read-only UI handoff; it does not qualify this
host, authorize UI commands or establish production connectivity. UI files and
integration remain UI-owned.

Executable deployment owns exact artifact selection, startup admission,
compatibility, activation and predecessor retirement. The native host owns its
admitted review work and recovery. Context generation, executable generation,
provider session and OS process remain distinct. Routine context replacement
does not force a host/binary change; process exit does not discharge an unsettled
provider action.

HP4 carries the earlier generation plan's durable obligations into the selected
host shape: immutable source/build/lock/toolchain/target/runtime dependencies;
artifact byte/mode identity; captured manifests/profiles and saved path bases;
no ambient target-directory/PATH/backend redirection; early refusal before
candidate effects; same-byte relocation versus changed code/configuration;
actual startup and persisted reconstruction; and explicit exclusive ownership
before a successor opens the selected roots. Reproduce online drain/rollover
machinery only if a future accepted product contract requires it; inactivity
removes that migration requirement here. A content-addressed host binary can link both
libraries, so two separately staged hot-call executables are not required.
The Node generation manager can remain an inactive reference until the permanent
Rust startup/generation owner is qualified. It need not stay operational, and
its replacement need not copy legacy rollout choreography. The future owner's
exact scope is an HP4 dependency, not extra HP1 work.

## Temporary exceptions and removal triggers

| Exception | Specific reason and bound | Removal trigger |
| --- | --- | --- |
| Existing Node source or fixture oracles | Preserve exact characterization and evidence while replacing consumers offline. Runtime compatibility is not required for availability. | Remove obsolete runtime paths from the inactive candidate as their callers move; retain or archive only the fixtures, source subjects and codecs needed for evidence or saved-state interpretation. Final cutover inventories the remaining dependency closure. |
| Existing C2 source-verification/AEG Python behavior may remain when needed by the selected compiler profile | Verified source/semantic behavior is an existing dependency; replacing it is not a prerequisite for direct pure compiler calls. Bind executable/source/configuration identity and report it. | Accepted Rust verification replacement with actual-source parity and consumer cutover; pure-library success alone does not retire it. |
| Provider executable or existing transport dependency | Third-party provider language does not violate Rust-first direction. Any retained first-party Python transport needs a named adapter compatibility reason and pinned evidence, not an unmentioned dependency. | Accepted Rust first-party provider adapter with retained-session, credentials, grant and recovery parity. No automatic switch from Claude to a lifecycle Codex adapter. |

No generic broker, hot-call worker farm, universal service registry or shared
storage framework is proposed. A temporary exception requires an actual
retained consumer and an exit; possibility of future reuse is insufficient.

## Qualification and honest retirement

Preserve the HOST corpus and outcome dimensions: normal work, near-ceiling
compiler input and the exact four-row history, wrong identity/scope, contention,
cold start/restart, and separate four-request stress. Freeze exact source,
release/fault artifact, fixture/profile, observer and machine identities before
sampling; retain incremental raw outcomes and probes. The proposed direct-host
profile keeps external control p99 at most **50 ms** and maximum at most **250
ms** unless the host consumer explicitly accepts another target before the run.
The successful-history completion budget is unresolved pending HP1 measurement
and explicit consumer acceptance before HP3 qualification. Moving a timeout away
from the dispatcher is not success of the operation. This unresolved performance
target does not prevent HP1's bounded API/correctness work.

Retire only the old bridge-specific mechanics/comparators from the new profile:
four Node backend selectors, Node event-loop/spawn attribution, per-call binary
copy/launch, FD-3 per-command descriptor and 100/250 ms child deadlines. Preserve
their historical measurements. Direct-host evidence records full request cost,
successful/refused/timed-out/unresolved counts, control probes and misses,
queue/backpressure behavior, cold/reconstruction time and memory where observed.
An error workload can refuse promptly without claiming the refused operation
succeeded. Supported single-request large reads require complete results;
concurrent saturation may refuse before admission under the frozen profile and
must keep those outcomes visible.

Host correctness gates include actual-process crash cuts before/after admission,
provider entry, episode commit and downstream publication; same/different
operation-ID replay; competing writer and stale-selection refusal; retained
session/subject checks; exact codec vectors; missing/mutated configuration and
binary refusal; reconstruction without mutable build output; and proof that
reviewer children do not receive writable stores or admission handles. Test
peers substitute only at the external provider boundary. Real stores, native
application and selected dispatcher remain in the route being qualified.

Before claiming a production retirement, HP4/HP5 provide a bounded consumer
inventory with every source/import/dispatch/default/configuration/export path
assigned to migrated, retained compatibility, historical archive or explicitly
unsupported. It covers scripts and non-code configuration as well as graph
symbols. Operational cutover, publication/commit and cleanup each retain their
existing authority. A passing Rust host does not imply that bootstrap, generation
management, workspace publication, non-review claims, full campaigns, UI or
remaining first-party Python/Node code has disappeared.

## Decisions ready for the owner

The recommendation is to accept HP1's narrow direct-library and diagnostic work,
then form HP2-CE/HP2-SC manifests against that handoff and select the supported
history budget before performance qualification.
No separate compiler daemon, episode daemon or broad Node asynchronous bridge is
needed. The current task authorizes only this plan; no implementation is launched.

Later owner decisions are explicit: exact HP2 domain-operation manifests and
private-root profile; RC scope before affected routes open; actual provider and
required independence/evidence profile; whole-root campaign/claim route closure;
generation integration/dispatcher shape; and R4/R5 operational migration and
rollback. These are separate acceptance boundaries, not missing reasons to do
HP1 or permission to enlarge it silently.

## Evidence scope and provenance

This planning used Tier 2 Verify: bounded structural discovery and bidirectional
depth-one traces, exact source reads for material injected relationships and
isolated changes, and direct reads of governing/planning documents. Main project
`home-bline-code-work-engine` was ready, generation `2026-10-07T20:28:51Z`, coverage
recorded `2026-10-07T21:56:30Z`. Exact-path coverage for the governing documents,
plans, lifecycle plan and named main consumer paths reported `metadata_match`
and `no_recorded_issue`. This is a best-effort signal, not complete runtime
closure. Relevant symbol searches returned `has_more: false`; bounded traces
were untruncated. The first unqualified owner-factory trace was ambiguous with a
test function, so the exact production qualified name was traced instead.

The isolated `work-engine-c3-r3` project was ready in fast mode at
`2026-10-07T20:20:01Z`. Coverage reports its two result documents excluded under
`docs/`, and its native-review host/closure, episode `lib.rs` and
`host_admission.rs` changed since indexing. Those claims use direct current
source/document reads. The bounded review amendment also checked episode-store
`src/sqlite.rs`; its metadata was changed, so direct current source established
the `reply_json`/precommit-size/commit-uncertainty boundary. Its graph trace had
ambiguous cross-domain method matches and is not relied on for callers.
Compiler `lib.rs` and episode `admission.rs` had matching
metadata. Main graph results are not evidence for the isolated adapter changes.
The retained diagnosis supplied measurement qualifications; no new benchmark,
test, build, provider operation or database read was performed for this plan.

This plan's closure is enough to reject a primitive-only bridge and bound HP1;
it is not the exhaustive deletion proof required at HP4/HP5. Only this new
planning document is owned by this task. Existing dirty files, other domains,
the branch and index are preserved.
