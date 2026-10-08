# HP2-CE: Rust claims for the native-review host

Prepared 2026-10-07. **Implementation proposal; planning only is authorized.**
This is the claims lane of the [direct host replacement](rust-host-replacement-plan.md),
under the shared [HP2 contract](rust-host-hp2-contract.md) and alongside the
[campaign lane](rust-campaign-hp2-plan.md). Planning uses the requested
`gpt-6-astra`/`xhigh`; implementation and ordinary remediation use the user's
selected `gpt-6-sol`/`high`. The requested identity is recorded separately from
the effective runtime identity, which is unavailable in this planning session.

Work Engine is unused during replacement. The selected design is a permanent
Rust domain library on new private roots, followed by an offline host cutover.
No daemon, Node bridge, provider port, availability layer or dual writer is
needed. This plan neither imports nor disposes of predecessor state. No build,
test, provider call, live-state write, activation or commit ran for this plan.

## Bounded outcome and handoff

HP2-CE provides the exact claims operations HP3 needs: finding creation and
revision, exact-revision reliance and projection, production-path observation,
establishment and readback. The consumer is the permanent Rust
`review-workflow`/`work-engine-review-host`, not a new exercise-only service.
General discovery, semantic-shadow inference, claim maintenance, arbitrary
lineage publication, retraction and RC correction/succession remain outside
this profile. A rejected operation is rejected before domain effects.

The first integrated profile supports initial review, recovery, bounded retry
and finding evaluation as qualified by the campaign/episode owners. Finding
revision is a claims primitive; its availability does not enable retained
remediation. The qualified R3/HP1 host still refuses native remediation and RC
routes. Their later acceptance requires the explicit episode/RC join.

The episode handoff is [HP1 candidate 2](rust-host-hp1-validation.md), in
`/home/bline/code/work-engine-hp1-joined`, on published S2/S3 `b2efdbfa`, frozen
C3/R3 and HP1 candidate 2. Its 45-path aggregate is
`7b1b684d172528b55b57f6e9b739f9945e5bd223cd3459be3082294a3efed27f`;
the source-manifest file SHA-256 is
`cc6b7e8eadbf60dc40f0ae7e55eaedbcbb2a818d7dc2abe10787312171ca8cce`.
HP2 consumes the documented typed episode API and private admission boundary;
it does not reinterpret those tests as claims or host qualification.

## Source-backed starting state

| Existing owner | Observed behavior and consequence |
| --- | --- |
| [`native-review-host.mjs`](../app-server/src/services/slice-campaign/native-review-host.mjs), lines 220–275 | Constructs a finding grant and opens `review-findings.sqlite3`; constructs the finding bridge and production-path service over that store. The runtime claim store is SQLite, not the skill's historical `canonical/store.json` directory. |
| [`sqlite-store.mjs`](../app-server/src/services/claim-evidence/sqlite-store.mjs), lines 38–105, 153–208, 246–359, 546–582 | Schema v5 has a singleton canonical JSON state with integer revision/CAS, plus separate observation, semantic-shadow, establishment and succession tables. Canonical arrays retain immutable records and operation receipts; there is no separate finding-history table. Opening uses explicit first initialization, WAL, `synchronous=FULL` and integrity checks. |
| [`service.mjs`](../app-server/src/services/claim-evidence/service.mjs), lines 24–126 | Publication checks an exact registered grant, profile, permission and scope; operation replay binds request plus admitted grant. Revision publication checks a current predecessor head. New revisions and reliance records append rather than overwrite old evidence. |
| [`identity.mjs`](../app-server/src/services/claim-evidence/identity.mjs), lines 9–71 | Identity JSON sorts object keys by Unicode code point, uses JavaScript scalar JSON spelling and ends with LF. Stable finding IDs hash only namespace/kind/stable subject coordinates; revision IDs hash the full revision without `id`. |
| [`review-finding-bridge.mjs`](../app-server/src/services/claim-evidence/review-finding-bridge.mjs), lines 11–100 | Publishes one operation per finding, carries the original revision forward, and clears current reliance when the revision changes. Finding evidence refers to the exact episode using the claims codec, even though the episode owns a different revision codec. |
| [`reviewer-projection.mjs`](../app-server/src/services/claim-evidence/reviewer-projection.mjs), lines 15–76; [`read-service.mjs`](../app-server/src/services/claim-evidence/read-service.mjs), lines 90–120, 171–193 | Exact selected revisions retain provenance, scope, completeness and limitations. Selections contain 1–100 unique revision IDs. Projection grants no publication, selection, evaluation, acceptance or mutation authority. |
| [`production-path-contract.mjs`](../app-server/src/services/claim-evidence/production-path-contract.mjs), lines 38–152; [`production-path-service.mjs`](../app-server/src/services/claim-evidence/production-path-service.mjs), lines 11–52 | Separates required claim identity, observation schema v2 and establishment schema v1. `mutation_authorized` establishes falsity; missing/mismatched evidence leaves the claim unestablished. An absent observation uses an explicit unavailable sentinel, not fabricated positive evidence. |
| [`native-review-closure.mjs`](../app-server/src/services/slice-campaign/native-review-closure.mjs), lines 104–142, 230–269, 325–345 | Establishes each required claim, records evidence in the episode result, then publishes findings/projection; recovery can finish publication after the episode result. Builder evaluation and reliance are separate from claim publication and review acceptance. |

Required production-path claim documents are retained by the authoritative
campaign selection. The claims store retains observations and establishments
that name their revisions; it does not currently insert those documents into
the ordinary `claims` array. Rust preserves this ownership distinction.

## Package, files and dependency direction

Create one `claim-evidence` library under `rust/crates/claim-evidence/`.
There is no binary in HP2-CE. Proposed owned paths are `Cargo.toml`,
`src/{lib,codec,contract,authority,application,findings,projection,production_path,recovery}.rs`,
`src/store/{mod,sqlite}.rs`, `migrations/`, `schemas/`, and `tests/` including
`tests/fixtures/legacy-v1/`. Store modules, admitted grant constructors and
transaction implementation remain private. The public application exposes typed
commands/results and checked reads, not SQL connections or a mutable snapshot.

Use the workspace-pinned SQLite, Serde, SHA-256 and typed-error dependencies
where their exact versions/features are suitable. Common values may reuse
`work-engine-types`; its lifecycle/JCS encoding registry is not the claims
codec. No common reference crate or generic persistence framework is proposed.
Campaign and claims export their own distinct types; HP3 composes them.
Claims does not depend on campaign internals or lifecycle admission/store code.

The CE builder owns only this new package and its bounded fixture/schema files.
The integration owner coordinates workspace membership/dependencies/lockfile
with the existing S4 root owner. Neither the CE builder nor this plan edits the
shared Cargo root/lock, `terminal-ui`, episode/compiler code or legacy runtime.
Any needed shared-path edit has a separately named owner before implementation.

## Exact operation manifest

| Application operation | Inputs and checks | Durable/output consequence |
| --- | --- | --- |
| Initialize/open private root | Trusted startup profile, pinned authority source/configuration, explicit root and bootstrap grant set; initialization only on an empty destination | Persist root/profile binding and validated admitted grants; reopen checks binding and schema without granting new authority |
| Create finding | Admitted request, registered grant ID, operation ID, exact admitted episode/result, finding ID | Legacy `create_claim` identity, initial immutable revision and operation receipt |
| Revise finding | Same plus exact predecessor revision and original finding identity | Legacy `publish_revision`, predecessor CAS, retained initial revision; clear current reliance when revision changes |
| Record finding reliance | Admitted builder-evaluation request, exact finding revision, consumer, consumer revision and scope | Legacy `record_reliance`, immutable active reliance and receipt; no claim applicability or acceptance decision |
| Project exact revisions | Admitted consumer/request binding, 1–100 unique exact revisions, selection reasons, limitations and scope | Read-only exact projection with checked provenance/freshness/completeness; no registry mechanics or authority handles |
| Record production-path observation | Admitted execution binding and typed observation schema v2 | Immutable event-keyed observation with exact bytes/digest and event conflict detection |
| Establish required claim | Exact claim recovered from selected campaign, operation ID, admitted observation or explicit absence | Deterministic establishment status/reasons and immutable operation-keyed record; no correction or new acceptance condition |
| Read/reconcile exact evidence | Root binding, operation/event key and expected content identity; for required claims, exact campaign-owned claim document | Checked finding/receipt/observation/establishment or explicit absent/conflicting/unresolved outcome |

No public generic `publish(action: String, Value)` is needed for this subset.
Closed Rust enums exclude unsupported actions. Finding publication remains
per-finding, matching the legacy bridge; an entire review result is not one
atomic claims transaction. Each finding's stable child operation ID is retained
in the workflow intent so partial publication can be reconciled exactly.

## Identity, codecs and schemas

Name the domain codec `claim-evidence-legacy-json-v1`. Its name selects the
existing bytes; it adds no envelope to those bytes. Preserve compact JSON,
code-point ordering at every object depth, array order, string escaping,
JavaScript number spelling, finite/safe-integer checks and one final LF.
Direct Rust input and imported JSON follow the same validation. Unknown schema,
profile and codec combinations fail explicitly; request data cannot pick a
different codec to make its own digest pass.

Golden vectors bind UTF-16/code-point ordering differences, non-BMP keys,
escaped/unpaired surrogate behavior, control characters, negative zero,
fraction/exponent spelling, safe-integer boundaries, nested objects, arrays,
null versus missing fields, final LF and every ID prefix. The implementation
must preserve legacy-accepted identity bytes; a `serde_json::Value` conversion
that loses historical string/number distinctions is insufficient. There is no
authorization to silently narrow accepted historical data or rehash it as JCS.

Keep separate claim ID, finding revision ID, reliance ID, observation event ID,
observation ID, establishment ID and operation ID types. A claims digest of an
episode or campaign reference is not that owner's native revision digest.
Authority-reference data uses the legacy `integrity_sha256`/status shape;
consumer references use their `sha256` shape. Do not conflate them because both
contain hashes. Legacy JSON field spelling and schema versions remain explicit.

Generate versioned public DTO schemas and checked-in compatibility vectors.
New Rust projection build provenance identifies Rust truthfully; it must not
advertise `claim-evidence-javascript-v1`. Preserve historical canonical-input
digest and interpretation, including the legacy logical `canonical/store.json`
label, without pretending that label is the physical SQLite path.

## Authority and private construction

The permanent claims bootstrap owner validates the startup-selected profile,
exact bootstrap configuration bytes/digest, authority-source bytes and their
references before admitting grants to an empty root. Bootstrap checks profile,
actor, grant identity, scope and the closed allowed permission set. A request
field saying `verified`, a caller-supplied authority object, or knowledge of the
database path is not that admission. Reopening cannot re-bootstrap or replace
the admitted grant set. Unknown/mismatched roots refuse before mutation.

Public commands name an admitted grant ID. The application resolves its private
registered grant and checks action, profile, scope and exact grant digest within
the claims transaction, including replay. `AdmittedGrant`/`AdmittedClaimsWrite`
have private constructors and no deserializer. Canonical records still retain
the full legacy authority reference and producer attribution. The bootstrap
source is an explicitly bound host-owner configuration, not a fixture-only
grant or a new general authorization service.

That root grant is not campaign operation authority. A narrow trusted admission
port, bound when the application is composed, checks the durable campaign
request, selection, candidate, obligation, configuration and allowed action.
Its production implementation belongs to campaign/HP3; a serialized reference
cannot implement it. The campaign owner preserves its admitted-request fence
through the dependent write, then resolves its own revision with exact readback.
A read-current check followed by an unfenced write is insufficient. No database
transaction is held open across domain stores to manufacture atomicity.
The campaign implementation uses its exclusive root writer, durable admitted
operation slot and process-local lease: conflicting campaign mutation stays
blocked through the CE commit/read, and crash recovery reacquires a new owner
epoch against the surviving slot. Lock expiry is not evidence of settlement.

Recovery of an already admitted operation is distinct from admission of a new
effect. The trusted recovery path can read/reconcile the exact recorded request
after a crash; it does not select a reviewer, create a replacement operation ID,
enable provider entry, or weaken an unmet required claim. Reviewer-facing code
receives immutable projections only, never owner construction or write handles.
These package boundaries constrain trusted composition and accidental misuse;
they do not claim isolation from malicious code with the same OS authority.

## Store, CAS and crash semantics

Use a new `native-review-claims-v1` private-root profile with a root binding and
its own versioned migration history. Retain the canonical JSON state/revision
model and the observation/establishment tables needed by this consumer.
Do not call this schema legacy v5 compatibility: HP2 does not implement legacy
semantic-shadow/succession storage or import arbitrary legacy databases.
Existing roots remain unchanged; a later explicit importer or archive/clean
start decision handles their full contents and unresolved obligations.

Anchor the actual absolute root before marker/database access and retain that
binding through reconciliation. Check root/profile/schema identity, canonical
bytes, digests and cross-record references on reads. Use real SQLite
transactions, a bounded busy policy, WAL and full durability, with owner-private
database/sidecar access. A filesystem path change cannot redirect readback to an
unrelated database. Root initialization and restart have crash tests too.

Finding mutation and its existing `operations[]` receipt commit in one claims
transaction with canonical-state revision CAS; no new generic journal is
needed. Replay uses the same operation ID and legacy digest of request plus
admitted grant. Different content/grant/action under that ID conflicts.
Observation replay keys on event identity and exact canonical bytes;
establishment replay keys on operation ID and exact establishment bytes.
These are distinct implemented contracts, not a universal exactly-once claim.

Return `Applied` or `Replayed` with exact result identity and readback locator.
Return `NoEffect` only for refusal before mutation or confirmed rollback.
Uncertain commit/rollback or loss of delivery after possible commit yields
`OutcomeUnknown` retaining root, operation kind/ID and exact content/grant
identity. Reconciliation returns `Absent`, `Committed`, `Conflicting` or
`Unresolved`. Corruption, inaccessible stores or an uncertain lookup are never
`Absent`. A different operation ID is not an authorized retry of uncertainty.

Observation custody and establishment are independently durable stages, as in
the existing service. A crash can leave an observation without establishment;
readback finishes the admitted establishment without inventing another event.
Two required establishments may similarly be partially present. Only the
complete exact pair can satisfy the consuming workflow. No cross-store or
two-claim atomicity is inferred from sharing a process.

## Required claims and HP3 consumption

Keep `builder_projection` consumed by `slice-builder:<campaign identity>` and
`campaign_terminalization` consumed by `slice-campaign:<campaign identity>` as
two separate required claims. Preserve proposition, candidate commit/tree/patch,
episode ID, covered state, consumer, boundary, acceptance owner/source and
evidence-profile revision. Changing any identity dimension changes the claim;
missing evidence does not authorize a narrower substitute.

CE exports distinct `ExactFindingRevisionRef`, `ExactRelianceRef`,
`ExactProductionClaimRef`, `ObservationRef` and `EstablishmentRef` data types.
Checked `ClaimAdmission` carries the exact refs, optional observation, boundary,
consumer and `Established | False | Unestablished`. The campaign owner exports
selection/candidate/request references and owns the consumption reference.
None of these serializable values grants authority.

Checked establishment readback receives the full required claim recovered from
the selected campaign revision. It recomputes that claim's binding, checks the
stored establishment and observation digests and status, and rejects mismatched
selection/obligation/attempt/result/session bindings through the trusted
admission contract. It does not pretend to recover an unstored required-claim
document from CE alone. Deterministic evaluation preserves the legacy reasons:
mutation-authorized evidence is false; unavailable, inadmissible or mismatched
evidence is unestablished. Constraint or a successful Rust return is not evidence
that the claim held.

HP3 persists campaign intent before effects, reads exact episode/claims results
after uncertainty, and publishes campaign completion under its own CAS.
Recovery after a durable episode result finishes finding/claim publication
without another provider entry. Exact projection does not evaluate findings;
reliance does not accept review; established terminal evidence does not authorize
terminalization. The latter remains unavailable in the first host profile.

## Bounded implementation and gates

| Slice | Owned implementation | Required exit evidence |
| --- | --- | --- |
| CE1 — first implementation proposal | Library skeleton, historical codec/contracts, permanent bootstrap/registered-grant owner, private real SQLite store, finding create/revise and exact operation readback | Golden bytes/identities; initialized/reopened root; wrong authority/profile/scope and bootstrap reuse refused; predecessor CAS and competing writers; exact replay/conflict; actual-process cuts around commit and readback; no unsupported operation effect |
| CE2 — exact consumption | Finding reliance and bounded exact projection over CE1, with authority-free consumer DTOs | Reliance binds current exact revision and consumer; revision change clears projected reliance without deleting history; unknown/duplicate/mismatched revisions refuse; provenance/completeness/limitations retained; restart and stale projection checks |
| CE3 — production-path evidence | Observation schema v2 custody, deterministic establishment, exact claim/admission readback and partial-stage recovery | Both boundaries and all three statuses; event/operation replay conflict; lost/corrupt observation refusal; crash between observation and each establishment; recovered claim from real campaign selection at HP3 join |

CE1 is a complete bounded domain slice, not a mock adapter or entire application
rewrite. Tests may exercise a controlled trusted admission port, but that proves
only the claims port contract. CE1 cannot claim authoritative campaign admission
until the actual HP2-SC owner joins in HP3. Its own bootstrap and persisted grant
checks are real production-intended code and run before every finding write.

Use real temporary SQLite roots under `/home/bline/code/.work-engine-tmp/`,
including reopened and competing-process tests; no `/tmp` build/test roots.
Capture exact source/toolchain/lock/profile/fixture identities and raw outcomes.
Use legacy source and checked-in vectors as bounded compatibility oracles, not
runtime dependencies of the Rust host. Fault injection belongs to a nondefault
test feature and does not substitute an in-memory store for the qualified route.

Negative gates cover malformed/unknown schema and codec, integer/string edge
cases, unregistered or drifted grant, stale predecessor, reused IDs with changed
content, corrupt canonical bytes/digests/references, root mismatch, busy timeout,
reopen after kill and ambiguous commit/delivery. Compile-fail/API checks establish
that consumers cannot deserialize permits or obtain writable store internals.
Run focused package tests, formatting/Clippy and the joined checks required by
the integration owner; no repeated full qualification without a changed subject.

HP3's composed gate uses actual campaign, episode and claims stores and checks
the exact readback chain, partial finding batches, both required claims,
no-provider-replay recovery and stale admission refusal. Its controlled external
reviewer peer substitutes only at the provider boundary. Library completion
retires no Node dispatcher, changes no provider and establishes no host latency
target. HP1's history result and HP3 responsiveness remain separately qualified.

## Decisions, blockers and state disposition

Ready decisions: one claims library/private store; domain-owned codec and DTOs;
per-operation receipts; first CE1 scope; no common crate, daemon, Node bridge or
live-state compatibility requirement. Implementation needs the owner's acceptance
of this manifest and a workspace-registration handoff. HP3 additionally needs
the real campaign admission implementation and frozen host profile. These are
specific joins, not reasons to implement general claims infrastructure first.

Historical import versus explicit archive/clean start, full root dispatch closure,
RC/remediation, terminal acceptance and operational ownership transfer remain
HP4/HP5/R4/R5 decisions. Inactivity does not delete records or settle unknown
effects. Rollback before successor writes may return to a preserved predecessor;
after writes, reversal needs qualified compatibility or explicit disposition.

## Evidence scope

Tier 2 Verify used graph project `home-bline-code-work-engine`, full generation
`2026-10-07T22:52:47Z`, metadata recorded `2026-10-07T22:52:48Z`. Bounded searches
for claims symbols and native-review owners had `has_more: false`; depth-one
bidirectional traces for the finding/production-path factories identify the
native owner factory. Injected method relationships required exact source reads;
zero graph callees were not treated as absence. Exact-path coverage for the
relied-on main sources/documents/tests reported `metadata_match` and
`no_recorded_issue`, a best-effort signal rather than completeness proof.

The HP1 handoff is taken from its exact validation/manifest references, not from
the main graph or rerun tests. The planning receipt at
`/home/bline/.local/state/work-engine/rust-hp2-planning-20261007/claims-evidence.json`
retains source hashes/ranges, graph coverage, model-identity limits and scope.
Only this new plan and that external receipt belong to this lane; shared Cargo,
S4/UI files, other dirty work, branch and index remain with their owners.
