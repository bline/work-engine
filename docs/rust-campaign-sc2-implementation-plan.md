# SC2 campaign completion and recovery implementation plan

Prepared 2026-10-08 against shared commit
`41992a22d86b041d2db357d16345c477804af63a`. **Planning only.** The requested planner
is `gpt-6-astra/high`; future implementation and ordinary remediation use
`gpt-6-sol/high`. Planner thread: `01a1193e-07eb-7600-8116-a78eb5402684`;
effective model identity is not independently observable here. This document proposes bounded work under the
[HP2 contract](rust-host-hp2-contract.md), coordinated with
[CE3](rust-claims-ce3-implementation-plan.md) and the
[integration plan](rust-ce3-sc2-integration-plan.md). It authorizes no implementation,
test, provider call, existing-root conversion, commit or activation.

The useful result is a campaign application that durably distinguishes preparation,
possible dispatch, checked result publication, exact downstream recovery and builder
evaluation. It uses real CE and episode owners without acquiring their authority.
Campaign owns selection documents and consumption; CE owns observations,
establishments, findings and reliance; episode owns its transitions and results.

## Current source and the actual gap

| Current source | Observed boundary |
| --- | --- |
| `rust/crates/slice-campaign/src/application.rs:515–684` | `prepare_initial` commits an executing obligation and request slot; `consume_initial_admission` consumes a private handle, checks epoch/root/identity, slot and current revision, and returns request **data**. It neither records dispatch nor holds a lease across another owner. |
| `slice-campaign/src/admission.rs:1–36`, `src/recovery.rs:1–55` | New preparation has a non-cloneable, non-serializable handle; replay has no handle. Receipts and uncertainty locators already distinguish exact operation identities. |
| `slice-campaign/src/store.rs:158–343,354–478` | Exclusive OS writer lock; random owner epoch on open; anchored root/database/marker binding; snapshot and replay integrity checks; one transaction for state, receipt and reservation. Slot currently has only preparation identity. Store/marker/snapshot version is 1. |
| `claim-evidence/src/application.rs:18–110,596–610,643–704` | Actual CE2 seam is `AdmissionBinding` and `FindingAdmissionPort::acquire -> Box<dyn FindingLease>`. CE holds that lease across its write/readback or projection. The full binding requires episode revision/result and cannot be reused unchanged for pre-episode observation. |
| `review-episode/src/lib.rs:256–260,427–536`; `src/admission.rs:14–38` | HP1 provides real `execute_direct`, result validation, exact historical read and transition recovery. `OperationResult` is data. External code cannot construct `AdmittedResult`; its constructor is crate-private. Existing fixture and inherited-descriptor ports do not supply a permanent in-process campaign composition. |
| `app-server/src/services/slice-campaign/service.mjs:372–469,536–547`; `native-review-host.mjs:55–73,392–405` | Existing dispatch/retry and exact pre-entry failure predicates, including owner recovery agreement, are the behavioral source. |
| `native-review-closure.mjs:16–40,104–153,240–276,344–363` | Both claims, episode result, findings/projection and campaign publication are distinct stages; invalid builder dispositions still retain reliance; reported is not acceptance. |

Rust paths without a full prefix above are under `rust/crates/`; the last closure
path is under `app-server/src/services/slice-campaign/`. These are bounded static
observations, not new runtime qualification. [SC1/CE2](rust-hp2-sc1-ce2-validation.md)
and [HP1](rust-host-hp1-validation.md) retain their original test subjects/limits.

## Recommended staging and file ownership

Two buildable stages avoid making CE3 development block all campaign work. This is
a **proposed staging adjustment** to the earlier SC2 exit and needs acceptance of
this plan: SC2 core and the CE3/episode domain join can finish before HP3, but the
positive provider-backed retry path remains unqualified until HP3 supplies its
real custody reader. Do not label that partial result “fully qualified SC2 retry.”

| Stage / owner | Exact proposed owned files and result |
| --- | --- |
| SC2 core, one Sol/high worker | `rust/crates/slice-campaign/src/{lib,application,admission,contract,recovery,store}.rs`; new `src/{completion,evaluation}.rs`; new `migrations/0002_initial_completion.sql`; package `Cargo.toml`; existing `tests/admission.rs` where changed schema requires it; new `tests/{dispatch,evaluation,completion_recovery}.rs`; focused package fixtures. Compiles against current CE2 and HP1 core; no copied CE3 types, stub success bridge or temporary feature pretending to implement the join. |
| SC2 completion join, same retained worker after handoff | New `src/owner_readback.rs`; finish concrete CE3 port implementation in `admission.rs` and checked completion in `completion.rs`; new `tests/owner_join.rs` and exact compile-fail/API gates. Its Cargo dependency on the real `review-episode` library is explicit. |
| CE3 worker | Only CE3-owned package changes in its manifest: production-path types/store/authority/application and private checked evidence. SC2 consumes those exports; it does not edit the CE package or copy the schema validator. |
| Serialized integration owner acting for episode owner | `rust/crates/review-episode/src/{admission,lib}.rs`, new `src/in_process_admission.rs`, focused `tests/sc2_readback.rs`, and exports/dependency adjustments only if required. Supplies the permanent scoped admission/readback addition described below. This row is a prerequisite assignment, not permission for the SC2 worker to edit episode files. |
| Integration owner | `rust/Cargo.toml`, `rust/Cargo.lock` if package dependency registration changes; exact joined subject/receipts and review. Existing dependency versions remain unless a separately justified dependency change is accepted. |
| HP3 owner, later | Production execution/receipt custody and definite-pre-entry failure readback; actual provider process, host composition and runtime qualification. No HP3 executable or provider adapter is added merely to make this library gate pass. |

The core implements internal checked transition rules and fail-closed public
boundaries; owner-dependent entry points are completed at the join, rather than
exporting public constructors for test tokens. Core tests may test private reducer
inputs but cannot count as owner-readback qualification. Final documentation and
evidence record core, joined-domain and HP3-pending results separately.

No lifecycle, UI U1, compiler, legacy runtime, checkpoint producer, physical-profile
producer or shared framework change is in this manifest. Work Engine is unused.
Use only disposable build/test roots below `/home/bline/code/.work-engine-tmp` when
implementation is later authorized; no test or build runs during planning.

## Campaign operations and durable distinctions

All mutations use an explicit operation ID and expected campaign revision, bind a
closed operation kind and exact content digest, and preserve `Applied`, matching
`Replayed`, definite `NoEffect`, or `OutcomeUnknown`. A replay returns saved data
and receipt, never a new dispatch capability. Keep physical-profile digest,
trusted campaign-configuration digest, selected reviewer configuration and CE
profile/grant identities separately named; current `request.profile_digest` is the
physical profile, while receipt/locator `profile_digest` is trusted config.

| Operation | Checks and durable consequence |
| --- | --- |
| `authorize_dispatch` | Consumes newly issued admission, validates exact active slot/current revision/candidate/selection/config/epoch, fixes episode identity, writer/session/attempt, begin/result transition IDs and reviewer profile, and commits `may_have_entered` plus receipt **before** returning one non-cloneable `DispatchPermit`. A same-content replay or lost commit acknowledgement cannot reconstruct that permit. |
| `recover_request` | Reads original preparation, dispatch and downstream intent at exact root/identity/operation/digest. Returns checked data plus a privately issued `RecoveryHandle` tied to current owner epoch/current revision and original slot; never provider permission. If an explicitly requested first dispatch has never been durably authorized, the owner may re-admit the existing prepared request through a distinct checked preparation recovery method. Recovery itself does not call dispatch. |
| `record_initial_outcome` | Uses checked episode result and both checked claim establishments, or checked failure custody; reconciles every finding/projection before campaign publication. Preserves attempt history and initial episode reference. It cannot consume user-supplied refs/statuses as evidence. |
| `prepare_retry` | Requires the exact active failed attempt and privately checked definite-pre-entry evidence below; appends a new explicit attempt/operation linked to its predecessor, preserving provider/profile/selection/subject and all prior recoveries. Returns new preparation, not automatic provider entry. `authorize_dispatch` is a separate effect. |
| `prepare_evaluation` | Checks trusted builder authority, exact current finding revision and unevaluated finding, campaign consumer, exact consumer tree and decision scope. Durably fixes disposition (`valid` or `invalid`), CE reliance child operation/content and grant identity before CE reliance. Does not call a reviewer. |
| `record_evaluation` | Reconciles real CE reliance/projection against that intent, then commits the exact builder decision and reliance ref with campaign CAS. Neither disposition by itself creates reviewer resolution. Replayed evaluation preserves its original result. |

The SC1 `consume_initial_admission` can remain a request-data helper for source
compatibility, with its documentation clarified; it is not an alternate dispatch
route. New internal state uses exhaustive enums for request kind, dispatch state,
completion and retry evidence rather than allowing arbitrary status strings to
select transitions. Serialized snapshot labels remain explicitly versioned data.

Completion needs campaign-owned durable downstream intent before each external
owner effect. Extend the admitted slot with exact child operation kind/ID/content
digest and expected owning roots, grant/profile bindings, claim revisions,
observation event, establishment operations, episode transitions, result digest,
finding operations and optional evaluation operation. Result-dependent values
are attached under a checked local CAS after result custody/episode readback; do
not invent result hashes at initial preparation. Provide a narrow private
`bind_completion_intent` operation, not a public generic journal or unbounded
`set_state`. The original operation and child IDs remain recoverable independently
of whether the final completion transaction committed.

## Admission lifetime and cross-owner lease

Use one campaign-owned synchronized inner owner containing the real store/config
and live epoch. `Campaign` and its privately created `CampaignClaimsAdmission`
share that owner; no writable store is exported. All campaign mutation and lease
acquisition uses the same gate. This is a package-local implementation of the
existing port, not a new broker. The lease holds the gate for the bounded CE
commit/readback, with no campaign SQLite transaction left open. Release before a
subsequent campaign CAS and revalidate then. Establish a single lock order
campaign gate → CE operation; never hold CE transaction state while invoking a
campaign mutation or recursively acquiring the gate.

Acquisition checks root identity and current filesystem/database binding, active
owner epoch, current campaign revision, selected obligation, original request,
exact active slot and registered child operation/content. Candidate, selection,
config, profile and slot replacement cannot occur while the lease is held. The
port is created by trusted application composition; a serialized binding does
not construct it. Reviewer/client data receives no port, guard or writer.

`AdmissionHandle`, `DispatchPermit`, `RecoveryHandle` and evaluation admission have
private constructors and no `Clone`/`Deserialize`. A fresh open acquires the
exclusive OS writer lock and generates a new epoch; old handles/ports fail. The
durable slot and `may_have_entered` survive losing process/OS locks. A clock,
timeout, poisoned mutex, process exit or lease drop cannot settle a provider
effect. A poisoned/unavailable owner refuses or reports unresolved, never absence.

Exact recovery can finish an originally registered CE child operation after
checked `Absent`, or reuse its exact committed receipt. It cannot change an ID,
payload, grant, consumer or required claim. CE recovery checks the original durable
admission digest while campaign verifies the newly held epoch/current slot; a
new owner epoch does not rewrite the old CE operation digest. No recovery lease
authorizes provider entry. Old request replay after later campaign revisions is
readable history, not current admission.

## Frozen CE3 and episode join

CE2 keeps `AdmissionBinding`/`FindingAdmissionPort`. CE3 adds a separate
`ProductionPathAdmissionBinding` and `ProductionPathAdmissionPort` on the same
claims application, avoiding an invented pre-result episode revision. The new
method is `acquire_production_path(&self, &ProductionPathAdmissionBinding,
&ProductionPathRequest) -> ClaimResult<Box<dyn ProductionPathLease + '_>>`;
the closed request enum carries the full stage-specific command. The lease
exposes checked custody data from its installed trusted owner, never from a
caller DTO. Production methods require `P: FindingAdmissionPort +
ProductionPathAdmissionPort`. The concrete SC2 port implements both traits.
CE3 exports `record_observation`, `establish_claim`, `read_claim_admission`,
both stage locators and both stage reconciliation methods; SC2 calls those
real methods rather than inventing another claims facade. The
production-path binding includes campaign root/revision/operation/obligation,
candidate/selection/profile/prepared-request/exact-child-request digests, review episode
ID, attempt ID, optional session only for explicit observation absence, and exact
native-result **claim-codec** digest and optional actual episode revision. Stage is closed
`RecordObservation | EstablishClaim | ReadAdmission`; mode is
`Original | RecoverExact`. Full claim/observation inputs and expected refs are
covered by the request hash. The campaign port checks these against its saved
intent, not against caller assertions. The native-result digest can exist before
the episode commits; episode revision is only supplied after real owner readback.

CE3 `CheckedClaimEvidence` is privately constructed by real CE checked readback,
has no deserializer and exposes exact root/binding, full selected claim identity,
optional observation, establishment, status/reasons, boundary and consumer.
`record_initial_outcome` obtains the full claim from its own immutable selection
and supplies expected observation/establishment refs. CE verifies the records and
recomputes their status. Observation absence remains `None` with the legacy
unavailable/zero-digest establishment representation; do not persist a fabricated
observation. Rust claim schema validation remains CE-owned; campaign checks
selection placement, subject and exact consumer derivation.

The recommended episode prerequisite is a permanent **read-only scoped in-process
admission owner** in `review-episode`. The proposed minimal API is:

```text
InProcessReadAdmission::open(TrustedEpisodeReadFiles) -> AppResult<Self>
Application<InProcessReadAdmission>::read_checked(EpisodeReadRequest)
    -> AppResult<CheckedEpisodeReadback>
EpisodeReadRequest = Revision { identity, revision }
                   | Transition { identity, transition_id, content_digest }
```

`TrustedEpisodeReadFiles` binds the anchored absolute root, expected
`NativeRootSelection`, config/source paths and their raw SHA-256 pins. Trusted
composition supplies these pins; a read request cannot select files or hashes.
The proposed schema-1 scope document contains profile `native-host-read-v1`,
principal `native-review-host` with read access, anchored root path/selection digest,
full episode identity, selected authority digest, candidate/initial subject,
writer actor/runtime-session/profile, permitted exact revision IDs and/or original
transition ID/content-digest pairs, optional expected episode-codec result digest, and a source
reference. The separate source document identifies the trusted native host owner,
principal, exact root/episode scope, permitted `read`/`recover` operations and
source revision; its actual bytes must equal the startup-selected source hash.
Unknown fields/profile, redirected files, a scope outside that source, changed
root selection, duplicate/empty grants, mismatched identity or an unlisted exact
revision/transition refuse before readback. No request can enlarge the grant.

The opener and checks live inside the episode owner and implement its existing
private admission mechanism. They do not expose `AdmittedResult::new`, a boolean
approval callback or a caller-selected registry. Only exact read/recover are
allowed; begin, resume, result transition, correction and history access receive
no grant from this new owner. The integration owner implements and tests this
bounded proposed contract, without expanding it into provider execution or a
generic grant service. `CheckedEpisodeReadback` is created only after the real
store/admission path checks the scoped request and result integrity. It has no
public constructor/deserializer and exposes only checked refs/content.

For joined test setup, use existing actual episode writes through
`NativeHostAdmission` with an actual inherited descriptor (a controlled subprocess
may deliver it), then use the new scoped library readback. `FixtureAdmission` is
not a substitute for the qualified readback. This setup qualifies owner APIs and
local custody only; it does not exercise a reviewer provider or establish HP3
runtime evidence. Existing native descriptor and offline APIs retain their own
qualification scopes.

Readback checks absolute episode root and selected root identity, full episode
identity, initial/current candidate commit/tree/patch, selected authority digest,
writer actor/runtime-session/profile, exact transition ID/content digest and
committed revision, immutable result bytes/digest and evidence-admission pair.
`Recover` returning a committed revision plus **current** state is not the exact
historical result: read that committed revision explicitly and validate it.
The campaign stores both initial and current episode refs; this initial-only
profile rejects unexpected subject/writer/selection changes. The checked object
is evidence, not a campaign write permit; consumption rechecks the slot and CAS.

CE2 finding receipts/projections and reliance refs also need real exact owner
readback. SC2 `owner_readback.rs` calls the actual CE application under its port,
checks exact child operation/claim/revision/result/consumer/grant bindings and
creates private campaign completion evidence. It never accepts a deserialized
`ClaimProjection` or a callback returning `status=established` as final proof.

Trusted execution custody and failure readback are a different missing owner.
HP3 must supply real result/receipt/session/artifact/transport checks and the
observer/profile provenance consumed by CE3. SC2 does not place provider custody
in the episode store. Controlled observations in joined library tests establish
domain behavior only; positive runtime attestation and retry await HP3.

## Claims consumption and status mapping

For each selected obligation, preserve both `builder_projection` for
`slice-builder:<identity>` and `campaign_terminalization` for
`slice-campaign:<identity>`. Match full claim revision, candidate/episode, covered
state, acceptance source/profile, boundary and consumer. Build campaign-owned
`ConsumptionRef` using the claim codec over
`{schemaVersion:1, claimRevision, establishment, boundary, consumer}`, retaining
owner `slice-campaign` and reference `claim-consumption:<claimId>`. Bind both
records to the episode evidence admissions and final campaign receipt. This does
not execute terminalization or imply that a terminal acceptance boundary crossed.

| Checked state | Campaign consequence |
| --- | --- |
| Either required claim is `False` or `Unestablished` | `evidence_unestablished`; preserve each distinct status/reasons and exact refs. Do not publish builder findings/projection as usable evidence. Missing establishment records are unresolved, not synthetic unestablished records. |
| Both established, admitted episode reported or exact zero-finding result | `reported`, with both bindings; no acceptance or terminalization authority. |
| Both established, admitted result has open findings | `awaiting_builder`; exact current/initial finding revisions and real bounded projection retained. |
| Every finding has exact reliance and builder `invalid`, or builder `valid` with reviewer-owned `verified_resolved` | May move `awaiting_builder` to `reported`; preserve decisions/reliance. Valid disposition alone remains awaiting builder. |
| Result contract rejected | `correction_required`, retained rejected result/attempt/custody evidence; result correction remains unavailable. |
| Provider entered/unknown without settled checked result | Unresolved executing slot; no retry permission. |

No obligation status string bypasses these checks. `False` is not collapsed into
missing evidence. Terminalization, acceptance, claim/selection succession,
retained remediation, result correction, replacement subject and RC stay refused
before effects, including when a caller names an otherwise valid historical ref.

## Definite pre-entry retry contract

The source predicates are `service.mjs:418–449` and
`native-review-host.mjs:55–73,392–405`. SC2 preserves their semantic distinctions
but requires owner-checked evidence instead of accepting recovery JSON:

- Retained `authentication_required`: schema 1, `not_entered`, available session
  exactly equal to continuation session, and matching actual transport receipt
  and session-artifact digests. Recorded and current adapter recovery must both
  qualify for that session. A refreshed checked authentication observation can
  replace current recovery only with prior recovery retained and a new explicit
  retry operation.
- Pre-spawn `authentication_unavailable`: schema 1, `not_entered`, unavailable
  session exactly equal to the allocated retry session, no continuation session,
  and exact recorded/adapter recovery digest agreement.
- Pre-spawn `process_start_failed`: the same predicates plus a nonempty owner
  error code and explicit `preSpawnRetry`; preserve profile/provider. Process
  failure alone says nothing about already-entered effects.

The custody checker also binds campaign slot, instance/attempt, candidate, profile
and underlying artifact content, rather than accepting merely well-shaped hashes.
Unknown result, timeout, missing local episode receipt, exit, credential refresh,
or a request replay never satisfies this predicate. Retry preparation makes no
automatic call and every new dispatch requires a new exact permit. Core tests
cover the closed predicates and refusal; a positive real-owner retry gate is an
explicit HP3 dependency, not satisfied by a fake failure-reader implementation.

## Root schema, integrity, capacity and uncertainty

Use new private schema-2 campaign roots for SC2 qualification. Preserve
`0001_private_review.sql` and apply the added schema-2 definition only during
empty-root initialization, with marker, `user_version` and snapshot versions
checked consistently. Profile identity remains explicitly versioned; freeze
the exact marker/config encoding in tests. Ordinary `open` accepts the supported
schema-2 identity and rejects schema-1 roots unchanged. Existing SC1 test roots
are not silently upgraded: recreate disposable roots through SC1 APIs using the
same frozen SC0 input artifacts. Live-root import/upgrade is outside this plan.
Probe unsupported marker/database schema through read-only access before writable
SQLite configuration; refusal must not create sidecars or rewrite version metadata.
The current `TrustedConfig::controlled_sc0` input restriction remains; SC2 does
not quietly turn it into general campaign admission.

Schema 2 adds dispatch/attempt and downstream/evaluation intent to the owned
slot/state, with checked foreign keys to operation receipts and explicit binding
digests. Operation receipts retain the exact saved result. Persist immutable
prior attempts, initial episode, evaluations and consumed evidence references.
An unresolved slot cannot be erased, replaced or cleared on restart. A retry
atomically records its predecessor and updates the slot only after checked failure.

Every read/replay/recovery validates canonical state/revision, SQL revision versus
snapshot revision, receipt identity/kind/content and prior/result revisions,
request/prepared revision, active slot/attempt, selected candidate/profile/claims,
child-operation digests and foreign owner refs. Validate semantic consistency as
well as recomputed hashes; a coherently rehashed invalid state must fail. Never
rehash historical claims/episode data with the campaign codec. Preserve SC1's
explicit prepared-revision exclusion from request/state hashing and add vectors
for appended attempts rather than introducing recursive revision identities.

Retain root/database inode and marker/config checks on writes and readback, one
OS writer, private files, WAL/FULL durability and bounded busy policy. Schema
init, reopen, marker substitution, database substitution, stale epoch and
cross-process contention are actual-store gates. A root lock is not protection
against malicious same-UID code; this is the trusted composition boundary.

Preflight exact encoded response/state/receipt/intent capacities before any
campaign mutation, retaining current 1 MiB snapshot and 64 KiB receipt bounds
unless an accepted profile change is necessary. Final evidence refs keep large
results in their owner; never truncate history or silently drop findings to fit.
If CE/episode has committed but campaign completion exceeds its capacity, preserve
intent and downstream receipts and expose incomplete downstream publication;
do not claim globally no effect. Every domain keeps its own locator. Campaign
commit ambiguity returns `OutcomeUnknown` with exact root, identity, kind,
operation/content, expected revision and config binding. Only confirmed rollback
or pre-effect refusal is `NoEffect`; read failure/corruption is `Unresolved`.

## Qualification and completion receipts

Core qualification uses real API-established campaign state and real SQLite,
immutable Git candidate/profile inputs and explicit startup configuration. No
direct row insertion creates admitted state. Verify exact replay/content conflict,
CAS, private-construction/serialization refusal, stale handle/epoch, lease blocking,
two writers, response capacity and unsupported-operation refusal.

The later joined gate uses the real campaign, CE3 and HP1-plus-scoped-admission
applications/stores. It checks both claims and every status, exact episode
historical readback, both consumption records, real findings/projection/reliance,
evaluation authority and stale finding/consumer rejection. It includes mutation
of every identity dimension, absent/corrupt owner records, coherently inconsistent
state, conflicting child IDs, partial finding batches and lease/CAS races. A
loose fake admission or readback implementation is not the final qualification.

| Crash cut | Required recovery observation |
| --- | --- |
| Before/after preparation and dispatch campaign commit | Exact absent/committed/conflicting/unresolved; one slot; replay supplies no dispatch permit. |
| Dispatch committed before possible provider entry | Remains may-have-entered; provider count never increases through library recovery. |
| Before/after completion-intent campaign commit | No unregistered child effect; saved IDs/content reused exactly. |
| Before/after observation, first establishment, second establishment CE commits | Recover each original event/operation; no fabricated missing claim; pair completion is not atomic. |
| Before/after episode result commit | Exact transition recovery then historical read; finish downstream against original result, never another provider entry. |
| Each finding CE commit and projection read, before/after campaign outcome commit | Preserve partial publication; exact child receipt reuse and capacity failure remain visible. |
| Before/after evaluation preparation, CE reliance and campaign evaluation commits | Same finding/decision/grant/consumer, no duplicated reliance or discarded invalid disposition. |

Run these as actual-process fault cuts on disposable roots, alongside focused
locked debug/release tests, Clippy with warnings denied, formatting and compile-fail
checks. Source/fixture/config manifests bind exact path bytes/modes, baseline,
Rust 1.92.0 and actual `rustc -Vv`, Cargo.lock, enabled features, build profile,
executable hashes, scratch paths and raw gate outputs; rehash after gates. No
measured runtime/build identity is inferred from requested planner identity.

Shared S4 integration reported a process-global `REVIEW_EPISODE_FAULT_CUT` test
race (chatboard 1364–1365). Treat that as inherited evidence, not a reproduced
product defect. The joined fault gate runs serially (`--test-threads=1`), or the
episode owner first makes a separately bounded harness repair and requalifies it.
Do not use an unqualified parallel all-feature run as a passing gate.

Completion receipt reports the exact core and joined subject hashes, real owner
APIs used, test totals and failures, retained review findings/remediation, scope
and the pending HP3 custody/retry/runtime gates. Passing these library gates does
not establish provider settlement, a complete host, responsiveness, terminal
acceptance, live-state disposition or Node retirement. No tests/builds ran for
this planning document.

## Planning evidence

Tier 2 Verify, graph project `home-bline-code-work-engine`; initial parent
generation `2026-10-08T01:58:57Z`, final path coverage generation
`2026-10-08T02:02:44Z`, recorded `02:03:27Z`. Exact symbol searches exhausted their
relevant pages; a broad BM25 discovery page was narrowed rather than treated as
exhaustive. Bounded bidirectional traces were untruncated but missed injected
calls and produced a false `String.clone`→lifecycle edge; direct source, not that
edge, supports the ownership findings. All relied-on paths have recorded
coverage; SQL lines 8, 13, 17, 25 and 33 are partial parse coverage and the whole
file was read directly. A later cross-lane check used generation
`2026-10-08T02:05:59Z`; the changing CE3 draft had `metadata_changed` and was
read directly. Clean metadata is not proof of completeness.

Source hashes, ranges, queries, limitations and planner thread identity are in
`/home/bline/.local/state/work-engine/rust-ce3-sc2-planning-20261008/sc2-evidence.json`.
This document and that receipt are the only SC2 planning outputs. Parent owns the
integration plan and advisory resource claim; other actors' work is preserved.
