# CE3: production-path custody and exact claim readback

Prepared 2026-10-08 against published `41992a22d86b041d2db357d16345c477804af63a`
(host/domain publication `df9ebb6e` plus S4). This is an implementation-ready
proposal, not implementation or operational authorization. Planning was assigned
`gpt-6-astra/high`; future implementation and ordinary remediation use
`gpt-6-sol/high`. The planner thread is
`01a1193d-9534-7ce3-be38-4ec1893239d0`; effective model identity is not independently
observable here.

CE3 extends the existing `claim-evidence` crate. Its outcome is immutable
production-path observation custody, deterministic establishment, and checked
readback of an exact campaign-selected claim and its evidence. It preserves
`builder_projection` and `campaign_terminalization` as separate claims and
`Established`, `False`, and `Unestablished` as separate outcomes. Establishing
either claim grants neither review acceptance nor terminalization.

The [HP2 contract](rust-host-hp2-contract.md), [claims lane](rust-claims-hp2-plan.md),
[SC2 plan](rust-campaign-sc2-implementation-plan.md), and parent-owned
[integration plan](rust-ce3-sc2-integration-plan.md) retain their ownership.
Work Engine is unused during replacement. New private roots are sufficient;
there is no availability bridge, daemon, dual writer, or temporary Node route.

## Actual baseline and the required delta

| Current source | Actual API/behavior | CE3 addition |
| --- | --- | --- |
| `rust/crates/claim-evidence/src/application.rs:21–113` | `AdmissionBinding`, `FindingAdmissionPort::acquire(&AdmissionBinding) -> Box<dyn FindingLease>`, `ClaimsApplication<P>`; finding binding requires episode revision/result digest | Separate production-path binding and admission trait, because evidence can precede the durable episode result; preserve the finding API |
| Same file, `publish`, `record_reliance`, `project_exact_revisions`, `reconcile`, `reconcile_reliance` | Finding/reliance publication and checked projections/readback only | Observation, establishment and full-claim admission readback methods; these do not already exist |
| `authority.rs:56–205` | Source/config hashes and registered grants checked; only finding profile and three finding permissions accepted | Closed production-path grant profile/permissions and trusted custody configuration; no permission inferred from an existing finding grant |
| `store.rs:122–369` | Schema v1, canonical state plus `operation_admissions`; exact marker/schema fingerprint and directory/database inode binding; `user_version == 1` | Explicit private schema v2 and two domain tables; old roots are not silently upgraded |
| `codec.rs` | `claim-evidence-legacy-json-v1`, code-point ordering, JavaScript scalar spelling, LF, UTF-16-preserving `JsValue` | Reuse unchanged for historical claim/observation/establishment bytes; named new metadata encoding for admission/custody envelopes |
| Legacy `production-path-contract.mjs:38–152`, `production-path-service.mjs:11–52` | Claim schema 1, observation schema 2, establishment schema 1; deterministic evaluator and absent-observation sentinel | Bounded Rust validation/evaluation plus real custody, grant/admission checks and complete readback |
| Legacy `sqlite-store.mjs:286–359` | Observation event replay and establishment operation replay are separate commits | Preserve those two effect identities, add immutable admission/custody binding and exact reconciliation |
| `slice-campaign/src/contract.rs:312–444` | SC1 validates full selected claim documents, exact campaign consumer/candidate/episode and claim-codec identity | SC2 retrieves those same documents for CE3; CE3 never creates a second selection store |

In particular, there is no current `AdmissionRequest`, `ClaimsAction`, or
`with_admission_lease` API. New names below are proposed additions. Existing CE2
projection depth, reliance payload/grant rehash and bootstrap-schema fixes remain
regression requirements ([validation](rust-hp2-sc1-ce2-validation.md)).

## Implementation ownership

The CE3 worker owns `rust/crates/claim-evidence/` only: extend
`src/{lib,application,authority,contract,store,schema}.rs`; add
`src/production_path.rs` and, if needed for clarity, its package-local submodules;
add the v2 SQL definition under `migrations/`; update package-local schema
generation, schemas and `tests/{ce3,ce3_recovery}.rs` with fixtures under
`tests/fixtures/production-path-v1/`. Existing CE1/CE2 tests may change only to
exercise the explicitly selected new-root schema and retain all their invariants.

No dependency change is anticipated: the package already has SQLite, SHA-256,
JavaScript number formatting, typed errors and schema-test dependencies. A newly
justified dependency or shared Cargo edit is a separate integration-owner handoff.
CE3 does not edit campaign, episode, host/runtime, lifecycle, `terminal-ui`, shared
Cargo/lock, or legacy production code. SC2 owns its concrete campaign port and
joined tests. The parent owns cross-lane contract reconciliation and final join.

## Frozen proposed API seam

Keep `ClaimsApplication<P>` with its current finding API. Add production methods
on an implementation constrained by `P: FindingAdmissionPort +
ProductionPathAdmissionPort`; no second writable application/store is needed.
`ProductionPathAdmissionPort::acquire_production_path` returns an opaque RAII
lease borrowing its trusted owner. No deserialize/DTO constructor creates a lease.
Its signature takes `&self`, `&ProductionPathAdmissionBinding` and the complete
`&ProductionPathRequest`, returning `ClaimResult<Box<dyn ProductionPathLease +
'_>>`. The closed request enum carries the observation command, establishment
command or admission read request corresponding to the binding's stage. The
trusted lease exposes read-only custody readback (verified owner references or
explicit checked unavailability) needed for that request. Custody data returned
by this installed trusted port is distinct from identically shaped request DTOs.

The new `ProductionPathAdmissionBinding` contains campaign root ID, campaign
revision, original campaign operation ID, obligation ID, candidate digest,
selection digest, profile digest, prepared-request digest, exact child-request
digest, review episode ID, attempt ID, exact native-result digest, optional
actual episode revision, optional session ID, operation stage, and access mode.
Stages are `RecordObservation | EstablishClaim | ReadAdmission`; modes are
`Original | RecoverExact`. The native-result digest is the **claims codec digest
of the exact checked native result**, not an episode revision hash. An episode
revision is absent until actually committed; an invented placeholder is invalid.
A present observation requires its exact session ID. Absence is explicit and
must be supported by checked owner evidence, not a caller's missing field.
Campaign candidate/selection/profile/prepared-request digests retain their
campaign-owned encoding contracts. Child-request, full claim, observation and
native-result digests use the claims codec; the optional episode revision uses
the episode owner's codec. Adapters must name these distinctions in typed fields.

SC2's proposed `CampaignClaimsAdmission` implements the existing finding trait
and this new trait. It checks the anchored campaign root, current revision,
durable request/child-operation slot, exact payload, candidate, selection,
profile and live owner epoch. Its lease holds the campaign mutation gate across
the CE transaction and exact readback. Every campaign mutation takes that same
gate. Lease loss cannot clear a durable slot. Recovery admits only registered
original child IDs/digests and grants no provider entry or new payload. CE3 does
not depend on `slice-campaign`; SC2 already depends on CE.

| Proposed application method | Input | Result/effect |
| --- | --- | --- |
| `observation_locator` / `establishment_locator` | Complete command with registered grant ID and admission binding | Validated exact reconciliation identity prepared before any effect |
| `record_observation` | Validated schema-v2 observation, event identity, grant, admission and owner custody readback | `Applied` or `Replayed` observation receipt; one immutable event row |
| `establish_claim` | Full exact selected claim, operation ID, admitted observation ref or explicit absence, grant and admission | Recomputed schema-v1 establishment with `predecessor: null`; independently durable row |
| `read_claim_admission` | Full exact selected claim plus expected establishment ref and optional observation ref, exact admission/read binding | Private-constructible `CheckedClaimEvidence`, only after complete owner readback and deterministic re-evaluation |
| `reconcile_observation` / `reconcile_establishment` | Exact owner locator; establishment read additionally receives the full selected claim | `Absent | Committed(receipt) | Conflicting | Unresolved(reason)`; never a new authority grant |

Separate receipt/locator/result types avoid pretending observation event keys
are finding operation IDs. New write errors follow the existing
`NoEffect | OutcomeUnknown(locator)` distinction. Existing finding locators and
reconciliation signatures remain intact.

`CheckedClaimEvidence` has private fields, no public constructor, no `Deserialize`
and no conversion from the wire DTO. Read-only accessors expose the full checked
claim, optional checked observation, checked establishment, their exact refs,
root/binding, status, sorted reasons, boundary and consumer. The application
constructs it while its lease is held; it records immutable evidence and is not
a continuing write/consumption permit. SC2 must reacquire/recheck its own current
admission when consuming it, and compare the exact full selected claim and
binding again. An old checked value cannot authorize a new campaign operation.

Public reference/claim-admission DTOs are descriptive and schema-versioned. The
historical `owner/reference/revision/sha256/freshness` envelopes retain owner
`claim-evidence` and freshness `exact immutable revision`. Claim refs retain
claim ID/revision; observation and establishment refs use their ID in both
reference/revision. SC2 alone constructs `slice-campaign` / `claim-consumption:`
refs from the historical consumption body and claims codec. CE3 never fabricates
that owner reference or relabels one boundary as the other.

## Custody and trust are separate from hashing

A valid observation ID proves byte binding, not who observed execution. Trusted
startup fixes custody-owner identities, artifact/reference schemes, verifier
identity/version and profile; requests cannot supply their own trusted list.
The production-path admission port must consult the actual evidence owner for
the exact attempt/result/session/transport receipt and artifact identities. A
caller-supplied `observer`, `status: verified`, SHA or `mutationAuthorized: false`
does not satisfy that contract. Persist the checked custody owner/reference/
revision/digest, verifier/profile identity, request digest and grant digest as
separate versioned row metadata; do not add fields to historical observation JSON.
On recovery, verify the saved metadata against the trusted profile and owner
readback. Contradictory bytes are a conflict; inaccessible required custody is
unresolved, not absence and not positive evidence.

The real execution/transport/artifact owner adapter is an **HP3 dependency**.
It is not available merely because SC1 or CE2 compiled. CE3 tests may use a
controlled owner whose immutable records are read by the trusted port; label
that scope explicitly. The CE3+SC2 library join uses real domain applications
and proves their admission/readback composition. Only HP3's actual execution
owner join can qualify runtime attestation. Failure to establish runtime custody
must not be hidden behind a permissive production implementation.

Authority has two checks: a bootstrap-registered CE grant with exact profile,
permission, source and decision scope; and a real campaign lease for this
operation. Extend bootstrap's closed grant validation with profile
`production-path-v1` and permissions `record_observation`, `establish_claim`,
`read_admission`; finding grants retain their existing profile/permissions.
The v2 bootstrap source/config contract names the trusted custody configuration
and its raw SHA. Its public schema and parser must agree, including every new
permission. Reopen checks the same hashes/grant set; it cannot add a grant.
The precise accepted custody-owner/profile values belong to trusted HP3
composition, not a CE default claiming that a legacy observer name is current.

## Deterministic contract and bounded semantic changes

Validate the closed historical claim/observation/establishment shapes and their
derived IDs with the existing claims codec. Preserve proposition, subject,
covered state, acceptance owner/source/route and full evidence profile. Campaign
owns exact candidate/episode/consumer membership; CE independently validates
the supplied claim and binds its digest. Unknown schema/profile/codec refuses
before effects. Known but unsatisfied evidence requirements do not become
unsupported contracts: they retain an `Unestablished` result.

The evaluator retains the existing reason strings:
`observation_unavailable`, `subject_mismatch`, `covered_state_mismatch`,
`mechanism_mismatch`, `observer_not_admissible`, `realization_mismatch`,
`mutation_authorized`, `capability_mismatch`, `continuity_mismatch`, and
`integrity_or_artifact_unavailable`. Sort reasons with the historical ordering.
Any `mutation_authorized` reason wins as `False`; otherwise any reason means
`Unestablished`; no reasons means `Established`. `same_session_resume` maps to
the claim's `retained` continuity for evaluation without enabling retained
remediation. Evaluate only independently custody-checked records.

Absence uses `None`, not a manufactured observation: the establishment retains
`observationId: "observation:unavailable"`, 64 zeroes for `observationDigest`,
`Unestablished` and exactly `observation_unavailable`. No event row is written.
Missing/corrupt storage for a **present** observation is never converted to that
sentinel. Re-read the observation's canonical bytes, ID and digest before
establishment commit and again during checked admission readback.

These differences from permissive legacy code are explicit proposed owner
decisions, not accidental parity failures. Acceptance of this plan accepts the
recommended bounded behavior; a different disposition must be recorded before
changing these outcomes:

| Observed legacy gap | Proposed CE3 disposition |
| --- | --- |
| Schema-v2 validator includes `event_identity` but never validates its nonempty text type; the general observation dispatcher returns directly to that validator | Require nonempty canonical text event identity, with a recorded negative legacy vector; do not preserve SQLite coercion/empty identity behavior |
| Integrity evaluation uses `artifacts.some(...)`, so an empty array passes vacuously | Preserve evaluator bytes/reasons for admissible historical input, but the trusted custody profile must define and verify the required receipt/artifact set; empty input cannot self-attest custody. A new claim-wide “nonempty artifacts” rule would require the claim owner and a versioned semantic decision |
| Direct legacy establishment recording accepts any nonempty evaluator/profile/predecessor and trusts a supplied status | Freeze evaluator `claim-evidence.production-path-v1` and supported profile revision, recompute all status/reasons, require null predecessor in this slice; correction remains unsupported |
| Legacy `evaluate` does not compare selection, obligation, attempt, result or session against the authoritative execution request | Require these through the trusted admission/custody binding before effects; do not reinterpret an unrelated observation as evidence for the current attempt |
| Legacy service/store entry is not itself a registered production-path grant or fenced campaign admission | Add both checks and save their exact metadata; do not treat a fixture or caller DTO as production authority |

Known mismatches in an otherwise admissible observation remain evaluator reasons;
forged authority, corrupt identity or an observation from another request is a
refusal/conflict, not a claim truth assessment. Valid contradictory capability
evidence must reach `False`, not be filtered out as an input error.

## Store version, limits and crash semantics

The selected policy is **new private schema-v2 roots**. Initialize only an empty
root; give its marker/schema history an explicit v2 identity, `user_version=2`
and exact schema fingerprint. Ordinary open refuses v1 before writable access.
No in-place upgrader, grant replacement, predecessor database import, archive or
deletion is included. Preserve old roots/bytes and make incompatibility explicit.
This is an offline development choice, not authority to dispose of old state.
Retain canonical finding state/operation admissions and all existing root,
config/source hash, inode, WAL/FULL durability and readback checks in v2.

Add domain tables `production_path_observations` (unique event key, unique
observation ID, digest, exact canonical JSON, immutable admission/grant/custody
metadata) and `production_path_establishments` (unique operation ID, unique
establishment ID, claim revision, nullable observation-row link, digest,
canonical JSON, immutable admission/grant metadata). The null SQL link represents
the historical unavailable sentinel only. Do not put full selected claims in
`claims[]` or persist another campaign selection. Persist their exact digest and
reference in the row's request binding, and require their document for readback.

Within each transaction, resolve the registered grant and validate its source,
scope and permission, check the original request/admission binding, validate
canonical rows and rehash the saved request+grant+custody metadata. Observation
replay requires identical event key, canonical bytes and original binding;
establishment replay requires identical operation key, exact recomputed bytes
and original binding. Changed grant, request, content or owner custody conflicts.
The second boundary can reuse the one observation only under the same admitted
attempt and custody; its establishment has its own operation identity.

Use exact child-operation identities recorded by SC2 before effects. Recovery
after an owner-epoch change validates the saved original admission binding and
the new recovery lease separately; it must not replace the saved digest with
the new epoch/recovery request. This permits truthful exact replay without
allowing a changed request to masquerade as replay.

Proposed frozen initial limits: 5,000 ms SQLite busy timeout (current value),
1 MiB per production command and canonical observation/establishment row, 4 MiB
complete checked readback/DTO envelope, 16 MiB total production-path canonical
row bytes per root, and the existing 16 MiB canonical finding-state limit.
Count actual encoded envelopes including metadata before commit; inspect SQL
lengths before allocating stored JSON. Preserve codec depth 256 and test maximum
supported nested envelopes, not just inner documents. Capacity refusal before
commit is `NoEffect`; delivery/encoding uncertainty after commit carries the
exact locator. No global performance or unbounded-history claim is made.

Observation and each establishment commit independently. Before commit or after
a confirmed rollback, refusal is `NoEffect`; failed commit acknowledgement,
rollback uncertainty or failed postcommit readback is `OutcomeUnknown` with
anchored root, stage/event-or-operation key, exact payload/record digest,
registered grant digest, original admission digest and custody identity. All
readback rechecks the anchored root/database binding. A successful trustworthy
lookup with no record is `Absent`; malformed/corrupt/inaccessible storage is
`Unresolved`, and a key bound to different content is `Conflicting`.

Recovery finishes only original registered stages: an observation may exist
alone; one boundary establishment may exist while the other is absent. Retain
the existing rows and fill only confirmed missing stages under an exact recovery
lease. A committed unavailable establishment is immutable: later evidence needs
a separately authorized correction, unavailable in CE3. Never change operation
ID to retry uncertainty, recreate an event, or re-enter a provider. Only the full
exact pair can satisfy SC2's completion consumer.

## Fixtures, gates and dependency exit

Freeze source-hashed legacy vectors for valid claim/observation/establishment
bytes, all three statuses, ordered multi-reason results, the two boundaries,
unavailable sentinel, reference envelopes, event and operation conflict, Unicode,
scalar/date identity and integer/depth edges. Keep malformed legacy acceptance
cases in a separate expected-delta set. Capture the old evaluator as a bounded
oracle for valid supported input, not as a runtime dependency or an instruction
to port every defect. Include canonical ISO-8601 positive/negative timestamp
vectors; no wall-clock freshness inference is added.

Required implementation gates:

1. Focused default and optimized CE package tests with real SQLite roots;
   CE1/CE2 regressions, schema-generator byte comparison and schema conformance.
2. Nondefault `test-faults` actual-process cuts before/after observation commit,
   before/after each establishment commit and after commit before delivery;
   restart/readback yields the exact absent/partial/complete state. Keep scratch
   and build output under `/home/bline/code/.work-engine-tmp/`.
3. Tampered canonical JSON, IDs, column keys, request/grant/custody hashes,
   missing referenced rows, wrong database/root, changed bootstrap, stale epoch,
   unsupported v1 root, busy contention, capacity and depth refusal. Changed
   status plus consistently rehashed record must still fail re-evaluation.
4. API/compile-fail checks: no writable store, no constructible/deserializable
   checked evidence or lease, no DTO-to-authority conversion. Controlled custody
   tests prove wrong owner and caller `verified` flags cannot bypass readback.
5. Package formatting and Clippy with warnings denied; source/lock/toolchain and
   fixture identities recorded with raw outcomes. No broad tests are run merely
   because a documentation plan was written.
6. Parent-owned CE3+SC2 gate: real private roots; SC1-valid selected claim pair;
   concrete SC2 lease held across CE commit/read; root/revision/request/profile
   drift refusal; immutable owner evidence; full exact readback of both claims;
   genuine `False`/`Unestablished` disposition; crash at every partial stage;
   recovery rejects provider re-entry and never manufactures a consumption ref.
   Use the permanent bounded episode-owner admission/readback prerequisite for
   real episode-store reads/recovery. Execution observations come from an
   explicitly controlled immutable owner until HP3; this gate does not qualify
   runtime custody.

CE3 can implement contracts, codec vectors, schema-v2 storage, registered grants,
deterministic evaluator, checked readback and local crash recovery independently
against its declared port. The **SC2-dependent final join** is concrete campaign
admission/recovery leasing and consumption using the full documents from real
selection state and the bounded episode-owner prerequisite's real-store result
readback/recovery. HP3 additionally supplies actual runtime result production,
execution/transport evidence custody and the provider-boundary proof. Domain-only success
does not retire that dependency or qualify a functioning host.

Non-goals are CE-owned claim selection, generic discovery/maintenance, RC
correction/succession, retained remediation, acceptance/terminalization, host
runtime or provider work, daemon/API service design, a new common framework,
saved-state disposition, publication and operational cutover. UI U1 and S4
resources remain with their existing owners.

## Planning evidence

Tier 2 Verify used graph project `home-bline-code-work-engine`; exact-path
coverage generation `2026-10-08T02:02:44Z` reported metadata match and no recorded
issue for every relied-on path, which is a best-effort signal. Function/method
queries were exhausted. The admission-trait trace returned low-confidence
heuristic links into lifecycle code; direct source showed these do not establish
a dependency, so they were rejected. Exact source reads supplement the graph
for trait calls and SQL/JSON semantics. No repository-wide absence claim is made.

The receipt at
`/home/bline/.local/state/work-engine/rust-ce3-sc2-planning-20261008/ce3-evidence.json`
binds source hashes, queries/pages/trace limitations, coverage, decisions and this
document. Planning wrote only this document and that receipt; it ran no tests,
builds, providers, live-store operations, implementation, staging or commits.
