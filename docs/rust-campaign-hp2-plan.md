# HP2-SC: Rust campaign review ownership

Prepared 2026-10-07. **Planning only; no implementation is authorized by this
document.** This is the campaign lane of the [shared HP2 contract](rust-host-hp2-contract.md)
and the [direct host replacement](rust-host-replacement-plan.md). The recommended
next work is parallel CE1 and SC0 characterization, then SC1 once its subject
handoff is complete. SC2 completes the initial-review campaign boundary; SC3 is
a separate RC-dependent proposal. Implementers use `gpt-6-sol`
at `high`; planning uses `gpt-6-astra` at `xhigh`. Requested configuration is not
proof of observed model identity.

Work Engine is inactive during development. No availability, dual-runtime,
temporary Node bridge, daemon or generic workflow framework is needed. Existing
state is preserved. This plan does not authorize a provider call, live-root
migration, activation, commit, acceptance, publication or historical deletion.

## Result and ownership

Create `rust/crates/slice-campaign/` as a permanent library containing its
contract, legacy codec, private SQLite store, application, admission, recovery
and bounded subject-verification modules. The application is the only campaign
writer. HP3's `review-workflow` composes it with the qualified episode application,
claim-evidence owner, compiler and provider adapter. The campaign package does
not perform provider execution or reproduce `native-review-closure.mjs`.

The useful first result is a real new-root campaign that has been admitted,
advanced, bound to verified immutable evidence, selected, and durably prepared
for initial review through campaign APIs. It can be closed and reopened and its
exact authoritative request recovered. Table seeding, a caller's `review_ready`
boolean and a Node campaign callback cannot establish that result.

The first complete profile is `campaign-native-initial-v1`: existing immutable
candidate input, schema-v2 selection with the two required claims per selected
obligation, initial review, exact downstream recovery, definite pre-entry retry
and builder finding evaluation. It supports `implementation-review` and its
existing `claude-recon-implementation` alias. Specialist instruction review is a
later profile because its closure validator is separately owned.

The profile rejects legacy review, selection succession, result correction,
retained remediation, replacement candidates, supersession, external bootstrap
adoption, terminalization, completion offers and workspace publication before
their first effect. Unsupported configuration fails at admission, not after a
campaign, episode or provider transition. Read-only inspection may expose
preserved unsupported historical states without making them executable.

## Source basis and the boundary being replaced

| Exact source | Existing consequence retained |
| --- | --- |
| `slice-campaign/contract.mjs:23–53` | Domain-specific revision codec and identity key; identity includes run, slice, attempt and plan version. |
| `contract.mjs:55–110` | Selected obligation, immutable subject, unique obligations/claims, and exact builder/terminal consumers. |
| `service.mjs:63–150,245–319` | Revision CAS; accepted admission; authorized phase consequence; candidate/profile binding; immutable selection. |
| `service.mjs:372–469` | Publish execution admission before calling the workflow; distinguish replay from new provider entry; exact definite-pre-entry retry evidence. |
| `service.mjs:536–622` | Exact finding evaluation and retained remediation state; preserve prior findings and initial episode binding. |
| `sqlite-store.mjs:6–99,133–156` | Transactional state/workspace admission, conditional revision update and private SQLite opening. |
| `native-review-host.mjs:278–500` | Host constructs episode/session/operation identities from the actual campaign, selection and immutable subject. |
| `native-review-closure.mjs:10–39,98–382` | Episode result, claim establishment, finding publication, projection and campaign publication are distinct facts. |
| `review-subject/service.mjs:18–38` and `legacy-backend-adapter.mjs:57–106` | Candidate creation/profile production currently have their own mediated Python owners; they are not campaign reducers. |

Paths above are relative to `app-server/src/services/`. The isolated HP1 handoff
additionally refuses all native Rust remediation at
`native-review-closure.mjs:365` and production-path succession at `:161` and
`native-review-host.mjs:409`. SC2 does not remove those RC fences.

## SC0: freeze the bounded subject handoff

SC1 is not implementation-ready until this characterization closes. Existing
source establishes candidate receipt schema 1 and physical-profile schema 2 with
`slice_checkpoint_candidate_receipt`; it does not select a historical producer
runtime, gate-receipt contract or qualified oracle corpus. The current source
anchors are `checkpoint.py` SHA-256
`1b814da3ef9e1f20804b934b91f6621ed65159dd9dfab5bfbfdd2df95f82d20b` and
`code_change_profile.py` SHA-256
`47e486ce1539bbfe1ab466d77650d9bb3e0338222478ae445fe6a07ba95e2003` at the paths below.
These are observed source identities, not owner approval or runtime qualification.

SC0 delivers `subject-handoff.json` and a content-addressed `vectors/` directory
under `/home/bline/.local/state/work-engine/rust-hp2-sc0-<run-id>/`. The manifest fixes both schemas, exact
validator sources, historical analyzer/Python/Git producer identities, accepted
boundary and gate-receipt owner/schema/status/subject requirements, captured
artifact hashes, required read-only Git checks and every vector's source and
expected result. Cases cover a valid candidate/profile and wrong ref, commit,
parent, tree, patch, path attribution, gate binding, producer, profile digest and
observation structure, plus each owning codec's identity edge cases. No generic
gate schema, profile producer or checkpoint writer is added.
Checkpoint ownership supplies receipt/Git verification semantics; physical-profile
ownership supplies producer/provenance semantics; campaign ownership selects the
accepted-boundary/gate requirements. The integration owner binds their exact
artifacts in the handoff. Characterization does not approve contract changes.

SC0 exits when those concrete bindings and vectors establish the supported input
profile with no caller-selected trust or unresolved validation dependency. Reuse
existing evidence where it establishes the oracle; any controlled oracle run
needs its own execution authority and exact receipt. Missing evidence remains a
named blocker to SC1. No such run or subject-owner approval occurred in this
planning turn. SC0 changes no product source, shared Cargo file or saved state;
it may proceed alongside CE1 after the corresponding work is authorized.

## SC1: private admission through initial request preparation

SC1 is one retained Sol builder's bounded implementation. Proposed owned paths:

- `rust/crates/slice-campaign/Cargo.toml` and `src/{lib,contract,codec,store,application,admission,recovery,subject_binding}.rs`;
- `rust/crates/slice-campaign/migrations/0001_private_review.sql`;
- package-local contract vectors and focused `tests/{codec,admission,subject_binding,store_recovery}.rs`;
- one bounded SC1 validation receipt/document.

`rust/Cargo.toml`, `rust/Cargo.lock` and any shared dependency registration remain
with the workspace integration owner; lifecycle S4 currently owns that resource.
No lifecycle or terminal UI files are in this lane. Existing episode, claims,
checkpoint and physical-profile owners retain their files and contracts.

The first store is a new private root with a new marker, not a changed executable
digest in R3's `native-host-v1` marker. Marker fields bind root ID, store schema,
campaign profile, trusted configuration and writer identity. Opening anchors the
absolute root before marker/database operations and refuses symlinks, mismatched
identity, unsupported schema and another writer. SQLite uses private files,
foreign keys, `trusted_schema=OFF`, WAL and FULL durability; busy/result limits
are frozen with the package profile. No migration reader opens a predecessor
root for writing.

SC1 exposes `admit`, `advance`, `bind_existing_candidate`, `bind_selection`,
`prepare_initial`, `read` and `reconcile_operation`. Every mutating call names an
operation ID and expected revision, except first admission which expects absence.
There is no `set_state`, generic transition payload or public writable store.

`admit` accepts a host-owned initial authority configuration plus a campaign
request: identity, anchored repository/workspace, accepted-boundary reference and
SHA-256, baseline commit/tree/inter-slice commit, and optional expected-impact
reference. The trusted opener fixes the permitted campaign/workspace/profile and
accepted-boundary bytes; request fields cannot grant permission. Admission and
workspace reservation commit together. Reservation records ownership within this
new root; it does not claim ownership over a live external workspace service.

The API then records `accepted -> implementing -> gate_ready -> review_ready`
with exact authorized consequence records and revision CAS. The existing owner
requires a consequence record, not proof that its phase enum ran tests. The new
profile records the supervisor's accepted boundary and implementation/gate
consequence references; it does not invent test execution or infer gate success
from a digest. If the accepted boundary requires passed gates, the trusted
admission configuration supplies the exact owning gate receipt and its required
status/subject checks. A missing gate owner/schema leaves that profile unsupported.
SC1's controlled gate evidence is produced for the controlled subject and then
read through the admission API; it is not a manually inserted campaign state.

At `gate_ready`, `bind_existing_candidate` verifies an already-created candidate
receipt, physical profile and referenced Git objects. It deliberately does not
port checkpoint creation, write Git refs or compute a new physical profile.
The immutable request records raw artifact hashes, historical identities and
captured producer/source/runtime identities, with an exact current campaign CAS.
Only a bound candidate/profile permits transition to `review_ready` and selection.

The narrow verifier takes the existing candidate-receipt construction method:

- `skills/slice-checkpoint/scripts/checkpoint.py:226–262`: candidate kind, private
  ref and exact commit, immutable metadata, parent/tree, path manifest digest and
  binary `diff-tree --no-renames --no-ext-diff` task-patch digest;
- `skills/code-change-profile/scripts/code_change_profile.py:123–193,366–445`:
  candidate subject/schema, tree/patch bindings, profile and subject digest,
  analyzer/checkpoint-validator identity, provenance and observation structure;
- campaign run/slice/plan, baseline, gate receipt and subject bindings checked
  against the admitted scope rather than an independently supplied copy.

This is an explicit proposed historical-artifact input profile. Its fixed,
host-owned configuration enumerates the accepted checkpoint/analyzer source
hashes and captured Python/Git producer identities. Rust verifies those historical
identities; it does not report itself as the Python producer or substitute its
runtime version. Caller-selected producers, arbitrary schema versions and profile
sources outside that set refuse before mutation. Qualification freezes actual
producer/source sets and golden vectors, including observation validation from
`code_change_profile.py:452–606`; no approximate validator is sufficient.

SC1 implements the frozen SC0 handoff. A missing equivalent read-only verification
capability returns to that handoff; it cannot be replaced by skipped checks or a
whole checkpoint-production port. Git inspection has fixed arguments/output
limits; no Node or first-party Python callback enters the qualified route.

`bind_selection` validates the exact current subject and schema-v2 claim pair.
Full required-claim documents remain campaign-selection data; claim-evidence
does not acquire a second selection store. Omitted obligations have no claims;
selected obligations have distinct `builder_projection` and
`campaign_terminalization` claims and campaign-derived consumers. Selection is
immutable for this profile. Preparation resolves the obligation and request from
that state, not from caller-supplied authority, subject or model booleans.

SC1 ends with `prepare_initial` committing an `executing` obligation plus its
exact request receipt. It returns request data and a privately constructed
admission handle. No provider is invoked. Reopening returns the same admission
record; an API replay is not a new entry grant. This is directly consumable by
HP3, while SC2 adds the completion and recovery transitions.

## Types, codecs and store receipts

The [shared contract](rust-host-hp2-contract.md) owns the agreed seam. Proposed
campaign-exported data is deliberately domain-specific:

| Type | Fields and meaning |
| --- | --- |
| `CampaignIdentity` | `runId`, positive safe-integer `sliceNumber`, `attemptId`, `planVersion`; preserve the historical colon-joined key, reject ambiguous collisions rather than normalizing old IDs. |
| `CampaignRef` | Root ID, full identity, exact `CampaignRevision`. |
| `CandidateRef` | Commit, tree, patch identity, candidate-receipt SHA-256 and physical-profile SHA-256. |
| `SelectionRef` | Selection ID, campaign-codec digest and separately named episode-authority digest; these hashes are not interchangeable. |
| `NativeReviewRequestRef` | Root ID, campaign identity, obligation ID, operation ID/kind, prepared revision, request digest, selection, candidate and profile digest. |
| `ConsumptionRef` | Historical owner/reference/revision/SHA/freshness fields; owner `slice-campaign`, reference `claim-consumption:<claimId>`, digest from the claims codec over exact claim revision, establishment, boundary and consumer. |

References are data. A private admission handle additionally binds the live
owner epoch and authoritative request slot. It cannot be deserialized or
constructed from a matching ref. The host's trusted composition creates/reopens
the application; clients and reviewers receive no database, writer or grant.
Validated-data constructors are not authorization constructors.

Campaign legacy hashing is JavaScript UTF-16 key ordering and JSON primitive
spelling with no trailing LF. Preserve the published revision projection's
explicit `revision: undefined` behavior, initial admission's distinct projection,
numeric edge cases and Unicode vectors; do not silently repair their bytes.
Host episode/selection authority uses the episode codec; claims and consumption
use claim-evidence's code-point ordering/trailing-LF codec. Artifact bytes use
their owning raw-byte digest. Unknown kind/version fails before effects. New
store metadata has an explicitly versioned codec and never rehashes legacy refs.

The campaign SQLite transaction commits state, workspace reservation where
applicable, and an operation receipt together. The receipt binds root, operation
ID/kind, request digest, authority/profile binding, predecessor revision and
result revision/result reference. It is a campaign-owned table, not a generic
cross-domain journal. Same operation plus same content returns the exact saved
result; changed content conflicts. An exact replay can be read after later
revisions without reinstating its old admission capability.

Writes return `Applied` or `Replayed` with that receipt. Refusal with established
absence of effect returns `NoEffect(reason)`. Possible commit returns
`OutcomeUnknown(rootId, anchoredRoot, identity, operationId, kind, requestDigest,
expectedRevision, profileDigest)`. `reconcile_operation` returns
`Absent | Committed(receipt) | Conflicting(actualRef) | Unresolved(reason)` after
checking the exact root and identity. Timeouts and lost replies are never absence.
Exact encoded result capacity is checked before committing a mutation; failure
after commit retains the receipt and recovery identity.

## SC2: initial completion, recovery, retry and evaluation

SC2 extends only package-owned application/admission/recovery/store code and
focused tests. It depends on SC1 and accepted HP2-CE typed operations. It adds
`authorize_dispatch`, `record_initial_outcome`, `prepare_retry`,
`prepare_evaluation`, `record_evaluation` and exact request recovery. These are
campaign transitions consumed by HP3; no reviewer process or closure loop lives
inside this package.

Dispatch authorization rechecks the active request, candidate, selection,
profile, writer epoch and campaign revision, then durably records that the
effect may be entered. Competing requests cannot hold the same obligation slot.
Candidate/selection replacement is unavailable while the slot is unresolved.
An old in-memory handle cannot authorize another effect after reopen, fencing or
revision change. The HP3 adapter binds each provider entry to this exact slot.
For CE mutation/projection, the application revalidates revision, slot and owner
epoch and holds a bounded process-local admission lease through the CE commit/read.
One OS-locked exclusive root writer and the durable request slot prevent concurrent
candidate/selection/profile replacement or supersession. The local lease need
not span provider lifetime: the durable reservation survives it. No SQLite
transaction spans stores. Crash drops OS/local locks, never the admitted slot or
unresolved effects. Reopen obtains exclusive ownership, advances the owner epoch
and reconstructs that exact slot; old handles cannot authorize effects. A
recovery lease reconciles only original operations and grants no provider entry.
No clock expiry or lock timeout establishes settlement.

The host supplies checked owner results to completion; bare deserialized
`status=established`, episode refs or failure strings are not evidence. Episode
readback verifies exact identity, initial/current subject, writer/session,
transition and result. CE readback takes the full exact required claim from the
campaign selection plus expected observation/establishment refs, because CE
establishment storage does not independently retain the full selection claim.
It validates exact status/boundary/consumer. Campaign records its own consumption
reference; CE never grants review acceptance.

The resulting obligation is `evidence_unestablished` when required evidence is
false or unestablished, `awaiting_builder` for open findings, or `reported` for a
qualifying reported/zero-finding binding. Both required claims remain separately
bound even though terminalization is unavailable. `reported` is not acceptance.
Wrong roots/revisions, absent owner readback or contradictory subjects refuse.

Failure retains attempt/session/transport identities and prior attempts.
`providerEntry=unknown` or entered-without-settled-result remains unresolved.
Only checked definite `not_entered` evidence may authorize retry: exact retained
authentication failure or exact pre-spawn authentication/process-start failure,
with matching session and artifact/transport evidence where required. The
caller cannot mint this evidence. Retry has a new explicit operation identity,
links its predecessor and preserves the configured provider/profile. Replaying
the old request, observing process exit or refreshing credentials does not grant
retry by itself. Result-contract rejection is recorded as correction-required;
the correction operation remains refused in this profile.

Evaluation is a separately authorized builder decision on the exact current
finding revision, consumer tree and decision scope. The workflow records CE
reliance through CE's authority-checked API, then the campaign publishes its
evaluation under CAS. A reviewer result cannot select itself or evaluate its
findings. An invalid disposition still preserves the exact reliance and decision;
a valid disposition does not imply resolution until reviewer evidence does.

## Crash boundaries and qualification

| Cut or conflict | Required observable outcome |
| --- | --- |
| Before/after campaign admission or CAS commit | Exact absent/committed/conflicting/unresolved readback; one workspace holder; no state without its operation receipt. |
| Prepared request before dispatch authorization | Durable request remains; effect entry requires a fresh exact authorization. |
| Dispatch authorization before/after provider entry | May-have-entered persists; no automatic provider replay. HP3 owns actual provider settlement evidence. |
| Observation/establishment committed before episode result | Immutable CE records may exist without episode consumption; reconcile exact IDs before reuse. |
| Episode result committed before findings/projection/campaign result | Recover episode and exact CE operations, finish downstream publication under CAS, with zero additional provider entries. |
| Reliance committed before campaign evaluation | Reuse the exact reliance receipt for the same evaluation; changed finding/decision conflicts. |
| Stale selection, competing writer, changed root/config or uncertain commit | Fence/refuse or return unresolved; never reinterpret as a new request. |

SC1 qualification uses a real temporary repository with immutable candidate
objects and a real private SQLite root. API calls establish every state. It
checks codec/subject vectors, invalid provenance, exact replay/content conflict,
stale CAS, two-open writer contention, root relocation policy, symlink/root
substitution, crash/reopen and result capacity. Test fixture generation may
create external Git evidence; it may not insert campaign rows.

SC2 joins real CE and HP1 episode applications/stores through their owned APIs
for result/evaluation readback and injected local crash cuts. Controlled owner
observations are labeled as domain qualification, not live provider evidence.
HP3 adds actual host processes and substitutes only at the external provider
port; its exit proves initial review, two-claim establishment, exact result
recovery without replay and responsive large-history/control behavior. Passing
SC1/SC2 tests alone does not establish the complete host or retire dispatch.

All future builds/tests use scratch below `/home/bline/code/.work-engine-tmp`,
with exact source/config/lock/toolchain and gate outputs retained. A joined
integration owner serializes shared root/lock changes and freezes the qualified
source. No tests, builds or provider calls were run for this planning document.

## Later scope and state disposition

SC3 requires the RC/episode and claims owners to join before retained remediation
or claim/selection succession can become enabled. It preserves the initial
episode reference, exact replacement subject, retained session, prior finding
lineage/evaluations, subject/result transition IDs and no-replay recovery. The
campaign remediation state machine may be specified without claiming that HP1
can execute it. Specialist review and result correction also need their exact
validator/provider recovery contracts. These are outside the first SC1/SC2
implementation recommendation.

HP4/HP5 inventory all writers sharing any proposed destination root. Acceptance,
receipt finalization, workspace transfer/publication, completion and supersession
stay unavailable until their complete owners join. There is no mixed Node/Rust
writer for different fields of the same campaign row. One-process composition
does not make campaign, episode and CE commits atomic.

Predecessor state remains untouched. The later state owner chooses qualified
copy/import or explicitly authorized archive/clean start. Saved unresolved
provider effects remain unresolved under either choice. Before new writes, an
unchanged predecessor may support qualified rollback; after new writes, rollback
needs reverse compatibility or explicit disposition/roll-forward. This profile
does not claim live-state migration or whole campaign-service retirement.

## Evidence receipt

Tier 2 Verify used project `home-bline-code-work-engine`, generation
`2026-10-07T22:52:47Z`, coverage recorded `22:52:48Z`. Relevant searches exhausted
their pages; bounded bidirectional native-review traces were untruncated.
Injected relationships missing from graph edges were verified in exact source.
All cited main paths reported `metadata_match/no_recorded_issue`; this is a
best-effort signal, not completeness. HP1 checkout paths were outside that
project, so their RC fences use direct source reads and hashes, not main-index
claims. The [HP1 validation](rust-host-hp1-validation.md) remains the authority
for the qualified joined handoff and test results.

The compact query/coverage/source-hash receipt is
`/home/bline/.local/state/work-engine/rust-hp2-planning-20261007/campaign-evidence.json`.
This is bounded planning evidence, not a deletion inventory or adversarial review.
