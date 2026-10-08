# J1: exact campaign, claims and episode owner join

Planning subject: `035a970443c4b4ca8e663dd0e66f4b8b28746268`, 2026-10-08.
This is a bounded implementation proposal, not implementation authorization or
acceptance of a new storage contract. It refines J1 in
[the episode join plan](rust-episode-owner-join-implementation-plan.md), against
the published ER1/P1 APIs. [ER1/P1](rust-er1-p1-validation.md) and the
[focused P1 follow-up](rust-p1-replay-regression-validation.md) supply accepted
source correctness and bounded replay-performance evidence. Their larger-history
limitations remain. Future implementation uses the user's selected Sol model.

J1 ends when a real campaign has durably registered and recovered exact CE3
observation/establishment/read requests and checked the corresponding historical
native episode result. It does not publish a builder projection, consume claims
for a final outcome, terminalize a campaign, or qualify actual provider custody.
J2 owns those downstream semantics. HP3 owns execution-record production.

## Source facts that determine the design

| Existing owner/API | Material fact |
| --- | --- |
| `slice-campaign/src/application.rs:1216` | `CampaignClaimsAdmission` currently implements only CE2 `FindingAdmissionPort`; `claims_admission()` shares the campaign mutex. |
| `application.rs:1380` | Present finding admission requires the live revision, executing obligation and a matching `Finding`/`Reliance` child hash. It is insufficient for CE3 or post-outcome evaluation. |
| `completion.rs:132` | Existing child intent stores only kind, operation, content digest, root and grant. Initial intent couples later result digest and episode revision; it cannot record pre-episode result identity. |
| `application.rs:2362`, `2741`, `2811` | Private reducers register intent, bind result and map a checked boolean to completed/failed. The boolean is not a joined evidence result. |
| `claim-evidence/src/production_path.rs:116`, `297` | CE3 uses the full `ProductionPathAdmissionBinding` and `ProductionPathRequest`; a lease reads custody while the campaign gate is held. |
| `production_path.rs:1262`, `1380`, `1574`, `1732`, `1893` | Real locator/write/reconcile/read APIs already exist. CE independently checks grants, custody and exact stored request/admission content. |
| `review-episode/src/in_process_admission.rs:225`, `299` | ER1 returns privately constructed `CheckedEpisodeReadback`; transition requests resolve the first introducing historical revision, with observed current revision separate. |
| `review-episode-core/src/state.rs:286`, `reducer.rs:263` | Episode v2 admission records contain claim, establishment, optional observation, consumption references, status, boundary and consumer. Shape validation alone does not establish correspondence to CE. |
| `slice-campaign/src/store.rs:148`, `198`, `697` | New roots are schema 2; marker, user version and snapshots are checked. History validation currently requires result digest and episode revision together. P1's current-read deduplication is already present. |

Paths in this table are relative to `rust/crates/`. Exact source, rather than
sparse/heuristic call edges, establishes the injected owner relationships.

## Public composition and dependency boundary

Use a distinct concrete `CE3CampaignClaimsAdmission`, implementing both existing
CE traits. `CampaignClaimsAdmission` and `claims_admission()` remain the CE2-only
construction route; they acquire no custody capability. The new port's fields,
lease and constructor are private. Its public campaign factory is proposed as:

```rust
pub fn claims_admission_with_custody(
    &self,
    selection: InitialJoinSelection,
    evidence: Arc<dyn ExecutionEvidenceOwner + Send + Sync>,
) -> Result<CE3CampaignClaimsAdmission>;

impl ProductionPathAdmissionPort for CE3CampaignClaimsAdmission {
    fn acquire_production_path<'a>(
        &'a self,
        binding: &ProductionPathAdmissionBinding,
        request: &ProductionPathRequest,
    ) -> ClaimResult<Box<dyn ProductionPathLease + 'a>>;
}
```

The reader is installed by trusted startup composition. A request cannot install
a reader, substitute a root or grant, supply custody as authority, or construct a
checked outcome. The port is bound to the exact campaign `Arc`, root and current
open epoch; a saved port cannot survive a reopen as a new owner. Composition also
installs `InitialJoinSelection`: exact claims `RootIdentity`, production grant
ID/digest, episode anchored root/`NativeRootSelection`, full authority identity
and digest, writer, and generation-1 initial profile. These are startup-selected
data validated against the actual owners, not an additional authority registry.
The factory validates and installs the selection and reader in shared campaign
composition before constructing its port; the selection is immutable for one
campaign open and its digest is recorded in each completion intent. Reinstalling
a different selection/reader in that open refuses. Its caller cannot select a
different owner for an individual stage.

Coordinate shared export checkpoint **E0** with HP3 before either implementation
depends on it. Recommended permanent placement is the narrow
`review-execution-evidence` crate, owning immutable execution records and their
reader, with dependencies on CE's existing codec and `CustodyEvidence`, and no
campaign, episode or lifecycle dependency. Campaign and the later HP3 executable
are its concrete permanent consumers. A host-local trait would either force
campaign to depend on the host or duplicate its contract; a campaign-owned trait
would make the producer depend on its consumer. CE3's existing port alone cannot
return the native result needed before completion registration. No generic
evidence platform or second custody wrapper is needed.

The coordinated reader surface is:

```rust
trait ExecutionEvidenceOwner {
    fn read_result(&self, reference: &ExecutionEvidenceRef)
        -> Result<CheckedExecutionResult, EvidenceReadError>;
    fn read_custody(&self, reference: &ExecutionEvidenceRef,
        binding: &ProductionPathAdmissionBinding,
        observation: Option<&claim_evidence::codec::JsValue>)
        -> Result<CustodyEvidence, EvidenceReadError>;
}
```

HP3 owns these exports and their exact representation. `ExecutionEvidenceRef` is
an exact owner/root/record/revision/hash locator, not authority.
`CheckedExecutionResult` has private construction and read accessors for native
result value, canonical bytes, claim-codec result digest, raw result-byte digest,
execution provenance and `Present(observation)` versus checked observation
absence with reason. Missing/corrupt/unreadable records are errors, never checked
absence. The custody method returns CE's existing type without another wrapper.
J1's controlled immutable test owner uses E0's fixed in-memory fixture validation
route, with bounded fixture field/digest validation rather than prerequisite
store/runtime implementation or full production record validation. The same
contract helpers are reused by E1. A runtime/type barrier
refuses production/provider profiles even when the default-off test feature is
compiled; feature selection alone is not the boundary. It proves domain
composition only and cannot mint production provenance.

Stable execution identity is campaign root, obligation, candidate, selection,
profile, prepared-request digest, episode and attempt, plus result/session. The
record separately retains dispatch operation and dispatch revision, which J1
compares to the campaign dispatch receipt. It does not pretend that the executor
knew the later completion operation/revision, child hash or episode result
revision. J1 validates those stage facts. For CE's explicit observation-absence
route, admission/custody `session_id` and observation digest are `None`, and
custody artifact references are empty; the execution record still retains its
actual session provenance. `None` is never a timeout or an inaccessible owner.

## Campaign entry points and returned progress

The proposed methods are public on `Campaign`; details of owner selection remain
trusted startup input rather than deserializable command authority:

```rust
pub fn prepare_initial_completion(&mut self,
    identity: &CampaignIdentity, expected: &CampaignRevision,
    obligation_id: &str, operation_id: &str,
    evidence_ref: &ExecutionEvidenceRef)
    -> Result<Effect<InitialCompletionPrepared>>;

pub fn complete_initial(&mut self,
    identity: &CampaignIdentity, expected: &CampaignRevision,
    obligation_id: &str, completion_operation_id: &str,
    owners: &mut InitialCompletionOwners<'_>)
    -> Result<InitialCompletionProgress>;

pub fn recover_initial_completion(&mut self,
    locator: &InitialCompletionLocator,
    owners: &mut InitialCompletionOwners<'_>)
    -> Result<InitialCompletionProgress>;

pub fn register_initial_episode_read(&mut self,
    identity: &CampaignIdentity, expected: &CampaignRevision,
    completion_operation_id: &str, registration_operation_id: &str,
    result_content: &review_episode_core::codec::JsValue)
    -> Result<Effect<InitialEpisodeReadRegistered>>;
```

`InitialCompletionOwners` borrows the real
`ClaimsApplication<CE3CampaignClaimsAdmission>` and an optional
`review_episode::Application<InProcessReadAdmission>`. Its checked constructor
matches installed selections and the port's exact campaign owner. It is not a
generic callback bag or a pair of owner-shaped DTOs. The campaign retains the
installed evidence reader, so preparation can read the exact evidence record
without a caller-supplied result. Preparation reads outside the campaign mutex,
then rechecks the expected snapshot/slot under the gate before committing intent.

`InitialCompletionPrepared` contains the snapshot, exact completion locator and
registered stage identifiers. It carries no provider/episode writer permit.
The locator contains campaign root/anchored path, identity, obligation, attempt,
completion operation, preparation request digest/config digest and the original
registration `RecoveryLocator`; all fields are checked on recovery.

`InitialCompletionProgress` is version 1, with locator, last checked campaign
revision, exact persisted stage references, newly observed owner receipts, and
one of `Pending(stage, reason)`, `OwnerReadsJoined`, `Conflicting(stage, reason)`
or `Unresolved(stage, owner_locator, reason)`. `OwnerReadsJoined` means the CE
pair and historical episode have been checked and their refs committed; it is
not an attempt outcome. Owner receipts committed after a failed campaign CAS
remain in returned progress and are discoverable through original locators.
`Result::Err` is reserved for refusal before this invocation's downstream
effects, with the saved locator retained; a later error is progress, not a false
global `NoEffect`. A read-only progress response grants no retry or dispatch.

Completion is bounded to one pass through currently reachable registered stages;
it does not poll or wait for an episode writer. Repeated calls/recovery use the
same original identifiers. Recovery reconciles first and may fill a confirmed
absent CE stage under the current checked gate; it never writes an episode or
invokes a provider. A separate observer-only API is unnecessary for J1.

There is one necessary handoff before opening ER1's immutable scope.
`register_initial_episode_read` accepts the native writer's proposed **content
data**, not its authority: the exact `{action, payload}` returned by episode
`TransitionCommand::content()`. It requires `record_result`, exact native result,
the previously checked CE admission pair and consumption commitments, and valid
unresolved-question data. It hashes that content using the episode codec; the
transition ID comes from the saved dispatch, not the caller. It saves the content
and derived target under campaign CAS and returns `InitialEpisodeReadRegistered`
with the `EpisodeReadRequest::Transition` and registration receipt. It grants no
writer capability. Source `review-episode-core/src/command.rs:132` confirms that
transition content is action/payload, not the unknown resulting episode revision.

Before that registration, completion can establish/read the CE pair and return
`Pending(episode_result_read, target_not_registered)`. The trusted descriptor
composition then registers its exact proposed content and opens ER1 with the
returned target plus the already selected authority. `owners.episode=None`
after registration returns pending reader installation. This explicit handoff
avoids fabricating a target in advance, widening ER1's immutable scope, or
assuming the evidence producer owns a later episode write. Tests perform the
same handoff around the real native descriptor transition. An already committed
original transition is also readable after exact registration; correspondence
is still checked before recording joined success.

## Recommended schema decision and frozen J1/J2 vocabulary

Recommend **new schema-3 campaign roots only**, marker
`.campaign-native-initial-v3`, profile `campaign-native-initial-v3`, SQLite
`user_version=3`, snapshot `schemaVersion=3`. Refuse schema 1/2/unknown roots
without upgrade, rewrite or writable SQLite configuration. This is a proposed
owner decision for acceptance with the plan; existing state is not authorized
for archive, migration or disposal. CE remains schema 2 and episode contracts
remain unchanged. The existing SQL tables suffice: registration/progress use
the owned snapshot plus operation receipts. Prefer a clearly named schema-3
new-root SQL definition over adding a third migration that performs no useful
database change. No generic journal/table is introduced.

Retain the existing campaign identity/candidate/selection/dispatch/slot fields.
Replace coarse completion records with the following closed vocabulary in the
schema-3 definition. Serialized names below are camelCase; enums use snake_case;
unknown fields/tags fail. Actual CE/episode types remain authoritative; campaign
serialization adapters encode exact command data and reconstruct through those
owners' validators, rather than inventing parallel public CE DTOs.

| Record | Owned persisted fields |
| --- | --- |
| `CompletionIntent` | `campaignOperationId`, `registrationOperationId`, `registrationBaseRevision`, `obligationId`, `attemptId`, `preparationOperationId`, `preparedRequestSha256`, `dispatchOperationId`, `ownerSelectionDigest`, `executionEvidenceRef`, `nativeResultClaimSha256`, `nativeResultRawSha256`, `executionSessionId`, `observationSelection`, `selectedClaims`, `stages`, `joinedReadbacks`, `outcome` |
| `observationSelection` | Tagged `present { observationId, eventIdentity, observationSha256 }` or `absent { reason, evidenceRef }`; never implicit null due to a read failure. |
| `selectedClaims` | Exactly two entries, ordered by `builder_projection` then `campaign_terminalization`: `claimId`, `revision`, full selected document digest, `boundary`, `consumer`. Full documents come from immutable `Snapshot.review_selection` and are compared, not selected by ID alone. |
| `StageIntent` | `stageId`, `kind`, `registrationOperationId`, `registrationBaseRevision`, `command`, `requestSha256`, `originalAdmission`, `originalAdmissionSha256`, `ownerSelectionDigest`, `ownerLocator`, `evidenceRef`, `progress`. Fields not meaningful to a kind are absent by tagged variant, not empty strings. |
| CE command stage | Exact observation/establishment/read command fields, grant ID/digest and expected full root identity; saved bindings contain all CE3 fields including original access, original campaign revision, session choice and optional actual episode revision. Locators are CE's real `ProductionPathLocator`; receipts are CE's real `ProductionPathReceipt`. |
| Episode stage | Exact identity/attempt, root/native selection, authority/source digest, writer, result transition ID and content digest, expected result claim-codec digest, expected episode-codec digest, expected admission pair. No writer handle or guessed result revision. |
| `StageProgress` | `registered`; `owner_committed { receipt }` for writes; `checked { exactRefs, contentDigests }` for reads; `conflicting { reason }`; `unresolved { reason, locator }`. Confirmed absence is an observed recovery result; it does not erase intent or create a new ID. |
| `JoinedReadbacks` | Optional until complete: both claim revision/establishment/optional observation refs, distinct status/reasons/boundary/consumer, exact custody refs/digests; episode identity/authority/native selection, resolved initial result revision, result transition/digest, episode result digest and native result claim digest. Store last observed revision separately from resolved revision. |
| `ConsumptionIntent` (J2) | `claimRevision`, `establishment`, `boundary`, `consumer`, exact body digest and reference, `registrationOperationId`, `consumedAtOperationId: Option`. Body is CE-codec `{schemaVersion:1, claimRevision, establishment, boundary, consumer}`; owner `slice-campaign`, reference `claim-consumption:<claimId>`. |
| `FindingIntent` (J2) | Exact CE finding command, admission and request digests, root/grant, locator, receipt/progress; one per native result finding. |
| `ProjectionIntent` (J2) | Exact CE projection request/admission/hash, selected revision set, root and checked projection reference/digest/progress. It is a read kind, never a fake finding write. |
| `InitialOutcome` (J2) | Tagged `evidence_unestablished { pair }`, `reported { pair, consumptions, episode, findingSet, projection }`, `awaiting_builder { same fields }`, `correction_required { rejectionRef, evidenceRef }`; references bind complete checked aggregates, not a boolean. |
| `EvaluationRecord` (J2) | `operationId`, `registrationOperationId`, `findingId`, `findingRevision`, `consumerTree`, `decisionScope`, `valid`, exact reliance command/admission/hash/root/grant/locator/receipt, exact projection intent, publication state. Both valid and invalid decisions retain reliance. |

Freeze stage kinds now: `observation`, `establishment_builder`,
`establishment_campaign`, `admission_builder`, `admission_campaign`,
`episode_result_read`, `consumption_builder`, `consumption_campaign`, `finding`,
`finding_projection`, `evaluation_reliance`, `evaluation_projection`,
`initial_outcome`, `evaluation_commit`. The last eight reserve actual J2 variants
with the fields above; J1 validates and serializes their shape but does not expose
their mutation routes. J1's read/open validator refuses any nonempty J2-only
stage, consumed commitment, outcome or evaluation that it cannot semantically
validate; recognizing a serialized tag is not permission to accept an unreachable
future state. J2 must add the already specified semantic checks before enabling
those records. A J1-only binary may refuse a later J2 root without relabeling or
repairing it. `joinedReadbacks` and `outcome` are independent optional
states; J1 always leaves the latter absent. Preserve `DefinitePreEntryFailure`
and the existing retry predicate; HP3's retry connection is later work.

The episode stage may initially be unregistered because its exact transition
content depends on checked CE admissions. Before waiting for/reading that stage,
register the exact episode request and expected admission commitments through
campaign CAS. Derive each consumption reference/body at this boundary and save
it as an **unconsumed commitment**. J1 can check an episode's exact consumption
references without reporting that the builder/campaign consumed the result.
J2 records the actual consequence and `consumedAtOperationId`. Episode admission
references alone neither prove that crossing nor grant terminalization.

J2 may fill these records and implement their transitions. It may not add tags,
change serialized meaning, reinterpret existing records or silently broaden
admission under version 3. Any missing semantic dimension found during J2 is an
explicit owner decision and compatibility/version review before effects.

## Identity, ordering and stage registration

Avoid a circular hash: an admission binding uses the revision **before** the
registration CAS. The snapshot cannot contain its own resulting revision inside
the command whose bytes determine that revision. Resolve the registration's
resulting revision through its exact operation receipt. Original access requires
the registered command plus that live registration revision; after progress or
restart, `RecoverExact` retains the original command and separately verifies the
current campaign context. It changes only CE3 `access`; CE's `original_digest()`
normalizes that field and nothing else. Never replace the saved admission's old
campaign revision with the current one.

At each lease acquisition, validate root binding/open epoch, installed evidence
reader and owner selection; campaign identity; exact active slot/attempt,
dispatch/may-have-entered state; current selected full claims, candidate receipt,
profile/selection/prepared request; registration receipt and exact stage command;
grant/root/stage/key/hash; stable execution/result/session; and stage eligibility.
On recovery, unrelated historical commands do not become admissible just because
their hashes occur somewhere in history. The original registration must belong
to this still-current attempt and the exact stage, without final J2 outcome or
conflicting replacement. CE2 J2 admission later uses explicit intent kinds and
consumption context; it does not simply remove today's `executing` check.

Use the existing process-level writer lock plus campaign mutex. No acquisition
of a campaign SQL transaction spans another owner. The order is:

1. Validate/read campaign, prepare exact commands, commit registration under the
   campaign gate, then end the campaign transaction and release the gate.
2. Call a real CE method. Its port acquires the campaign mutex, checks exact
   registration/current context, and retains the lease through custody read, CE
   transaction and CE checked reconciliation. Custody performs only bounded
   immutable reads and cannot call campaign, CE writes, provider or episode.
3. Return from CE and drop its lease before campaign receipt/progress CAS. Recheck
   current expected revision and registration identity. A failed CAS leaves the
   downstream commit explicit and recoverable; it does not roll it back.
4. ER1 historical reads occur without the campaign gate or a campaign transaction;
   their checked immutable result is committed only after a fresh campaign CAS.

During a lease the nested order is campaign gate, evidence read, CE transaction;
the evidence read releases any internal lock before CE begins its write. There
is no reverse owner callback. Reconciliation that CE exposes without a campaign
lease remains observational; J1 revalidates current campaign context before a
subsequent effect or publication. Root loss or poisoned lock is unresolved/refused,
never evidence of external settlement.

Register observation and both establishment commands before their effects.
Use `production_path_request_sha256`, `observation_locator` and
`establishment_locator`; custody-populated locators from receipts supplement the
pre-effect locator, never replace its identity. Register each `AdmissionReadRequest`
after the establishment receipts are known, including original request/admission
hashes and obtained record hashes. Preserve exact optional observation pairs.
Reads are real stages with their own binding, not reused write bindings.

Before episode comparison, load native result through the evidence owner and
verify its claim-codec digest. Parse the exact same value with episode's codec;
keep raw SHA, claim digest, episode result digest, transition content digest,
campaign digest and actual episode revision as distinct named values. Check the
full episode identity/attempt, candidate subject and result commit/tree/patch,
writer/authority/profile, result transition, exact historical state and the two
complete admissions. `observed_revision` is diagnostic; it cannot replace
`resolved_revision`. A valid episode shape with a swapped full claim or missing
admission is not joined. The native descriptor writer is unchanged.

## Failure and recovery consequences

| Interruption/refusal point | Required result and permitted recovery |
| --- | --- |
| Evidence record missing/unreadable/changed; root, identity, grant or full selected claim mismatch before intent | Refuse preparation with no new campaign/CE effects; no checked absence or provider retry. |
| Preparation campaign commit uncertain | Return existing campaign `OutcomeUnknown` locator. Reconcile it before any CE action; absent may repeat the exact original registration CAS, committed reuses it, conflict/unresolved stops that join. |
| Registered stage, CE confirmed absent | If current slot/intent/custody still match, invoke only that saved command. Observation checked-absence route skips observation publication and establishes using `None`. |
| CE observation committed, campaign receipt absent | Reconcile original event locator; adopt exact receipt under campaign CAS, then continue. Do not regenerate observation/event ID. |
| One establishment committed | Reconcile both saved establishment locators with the full original claims; preserve first receipt and fill only confirmed absence. |
| CE `NoEffect` after earlier stage commits | Report pending/refused stage plus retained committed stages, never global no effect. Fixing authority/identity by editing the command is not recovery. |
| CE `OutcomeUnknown`, conflicting or unresolved reconcile | Retain exact locator and stage; no new operation ID, no next dependent effect and no inference from process exit. |
| CE commit/read succeeds, campaign CAS loses | Return downstream progress; subsequent recovery independently reconciles it and rechecks current ownership. It cannot force-adopt into a replaced attempt. |
| Both claims checked; episode target unregistered | Return `Pending(episode_result_read, target_not_registered)`; no absence claim or ER1 request is made. Register exact content before constructing the scope. |
| Exact episode target registered; reader not installed | Return `Pending(episode_result_read, reader_not_installed)`; trusted composition opens ER1 from the selected Authority/root and registered target. |
| Exact episode target read and transition absent | Persist pending episode read with exact target. Only the original descriptor owner may later supply the transition; J1 does not write or replay provider work. |
| Episode transition present after later episode updates | ER1 returns first introducing state; compare that state and persist its revision, keeping observed current revision separate. |
| Same episode transition ID, different content; swapped authority/root/writer/result/admission; corrupt history | Conflict/refusal, never missing. Do not fall back to latest state or another transition. |
| Reopen/new epoch/stale handle, changed slot/revision/selection or already consumed J2 outcome | Old live handles refuse. Recovery data can read/reconcile exact history, but no new stage effect without current exact admission. |
| Campaign capacity refusal after a downstream commit | Preserve owner locators and return partial progress; reclaiming or dropping the receipt is not permitted. Preflight bounded encoding before every owner effect reduces this risk without claiming cross-store atomicity. |
| Repeated joined completion | Exact data replay/checked progress only. It creates no provider, dispatch, episode writer or retry capability. |

## Implementation substeps and ownership

The parent owns shared-interface reconciliation and workspace Cargo/lock changes.
The J1 worker owns `slice-campaign/src/{application,completion,contract,store,lib}.rs`,
new `src/owner_readback.rs`, a schema-3 new-root SQL definition if needed,
package Cargo dependencies, and `tests/{owner_join,owner_join_recovery}.rs` plus
focused package test support. It does not own CE, ER1 or HP3 product changes.
Source locations above establish why this belongs in campaign's owner, rather
than the host or a generic coordinator. Avoid rewriting P1's read mechanics.

1. Parent freezes E0's shared evidence exports, checked-absence behavior, fixture
   construction route and minimal package/dependency skeleton. Freeze schema-3
   owner decision and full vocabulary above. This is the prerequisite checkpoint;
   independent implementation from zero is not claimed.
2. Implement new-root schema/profile and exact record validation, preserving
   original dispatch/slot/history/receipt checks and P1's single current decode.
   Remove the obsolete private success-boolean completion route; J2 outcome
   constructors remain unavailable. Existing private tests move to meaningful
   schema-3 intent assertions, not a publicly forgeable substitute.
3. Implement concrete CE3 port and gate/registration checks with real CE ownership;
   compile-check both CE traits and the no-custody path's lack of CE3 admission.
4. Implement preparation, exact CE stage orchestration and partial-commit recovery.
   Stage-dependent command registration uses campaign CAS after predecessor
   receipt observation; capacity checks precede each effect.
5. Implement ER1 exact historical join and persist checked refs, with no episode
   writer changes or J2 outcome transition. Qualify joined native descriptor tests.
6. Run the bounded acceptance gates below, retain exact evidence, review the final
   joined subject and reconcile shared Cargo changes through the parent.

After step 1, J1 domain code and HP3's execution producer/store implementation have
disjoint ownership and can proceed in parallel. J1 tests can use the controlled
immutable owner, while HP3 proves actual record production separately. Combining
those into production qualification is a later integration gate, not implied by
either lane passing. Lifecycle S6/S7, terminal UI, host launch and availability
infrastructure are independent owners and excluded from J1.

## Meaningful acceptance and retained evidence

Positive tests create campaign, CE and episode roots through their actual APIs,
install real CE grants, produce native descriptor episode transitions in the
same test executable identity, then reopen with ER1. No positive direct SQL
insertion, `FixtureAdmission`, permissive custody callback, serialized checked
token or copied owner DTO can replace those owners. Corruption tests alter
disposable copies only to demonstrate refusal. Test evidence labels controlled
execution custody as domain composition, not provider-backed independence.

Required cases cover both distinct boundaries and full claim substitution;
Established/False/Unestablished kept separate; checked observation absence versus
missing evidence; wrong grant/root/epoch/slot/attempt/session/result; exact stage
hash and original admission on recovery; current/historical episode mismatch;
the valid schema with wrong admission pair; later episode revisions; and a CE
lease blocking campaign mutation without deadlock. Progress adoption is monotonic:
reconciliation cannot lose an earlier committed stage. Include command registration
revision self-reference prevention, swapped registration receipt, and recovery
after later progress with the original binding unchanged. No J1 outcome or usable
builder projection is emitted in any case. Test target-not-registered separately
from checked exact-target absence and unavailable reader; tampered writer content
must refuse before scope construction, and a fresh ER1 reader must admit only
the registered exact target under the selected authority. Test J1 read/open refusal of injected
nonempty J2-only stages/outcomes/evaluations, even when their shape is valid.

Actual-process cuts bracket preparation commit; observation commit; each of two
establishment commits; dependent read-stage registration; native descriptor result
commit; and final joined-readback campaign commit. Cut after an owner commit but
before campaign progress as a distinct case. Restart checks absent/committed/
conflicting/unresolved, original IDs/locators, exactly one immutable owner effect
per command, and unchanged provider invocation count (zero for this domain-only
suite). The same executable initializes/writes/reads native episode roots; a
different helper executable cannot fake its selected digest. Serialize existing
process-global fault environments.

Run locked focused campaign/CE/ER1 tests and compile-fail/API boundaries in debug
and release, formatting and all-target/all-feature Clippy with warnings denied.
Preserve existing CE3 checks, ER1 read-only refusals and P1 integrity/performance
regressions. Verify P1 decode/hash attribution on its existing workload after
store changes; rerun its retained focused replay comparison only if measurement
or changed read work makes regression unresolved. Do not claim a new broad SLO
or silently drop checks to recover timing. Preflight maximum encoded snapshot,
receipt, command/readback sizes and prove over-cap refusal before dependent
effects. Keep raw logs, toolchain/features/lock/executable/config/source hashes,
generated roots and cut ledger under the accepted disposable evidence root.

Acceptance is the real-owner pair and exact historical result joined with all
partial stages recoverable and no capabilities reissued. It excludes J2 findings,
reliance/evaluation/consumption publication, provider-backed positive retry,
correction/succession, retained remediation, saved-state disposition, deployment,
live-root migration, Node retirement and temporary Node interoperability.

## Planning evidence and owner decisions

Tier 2 Verify used graph project `home-bline-code-work-engine`, generation
`2026-10-08T15:01:27Z`; final coverage recheck used refreshed generation
`2026-10-08T15:32:30Z`. All relied-on Rust source still matches the baseline.
Relevant narrowed searches had no further pages. Both-way
depth-1 traces were untruncated; CE/ER1 traces were sparse and campaign edges
included heuristic resolutions, so they do not establish absence or exhaustive
reachability. Direct source supplies the material method/control-flow evidence.
Every relied-on path was coverage-checked. Campaign SQL migrations had reported
partial parse lines (`0001`: 8,13,17,25,33; `0002`: 16), read directly in full.
Other paths reported metadata match/no recorded issue; this is best-effort
coverage, not completeness. No repository-wide negative claim is made.

Evidence, source hashes and limitations are retained at
`/home/bline/.local/state/work-engine/rust-j1-hp3-planning-20261008/j1/`.
Required instruction documents and baseline source are distinguished there.
Only this plan and that lane's evidence are owned by this planner; no tests,
builds, provider calls, staging, commit or runtime changes were performed.

Before implementation acceptance, the owner decisions are the proposed schema-3
new-root/refusal policy and the tiny shared evidence contract checkpoint. No
additional generic contract mechanism is required. If the shared implementation
cannot supply the exact native result plus explicit observation selection,
or J2 requires a field outside the frozen vocabulary, return that concrete
boundary to its owner before effects rather than quietly weakening the join.
