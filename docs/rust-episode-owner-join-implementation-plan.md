# Scoped episode readback and campaign owner join

Prepared 2026-10-08 against `d4fc06027dc2dc11d91187da11a096c857148c25`.
**Planning proposal only.** This document does not authorize implementation,
tests, provider execution, migration, publication or activation. Future builders
and ordinary remediation use `gpt-6-sol/high`. Planner thread:
`01a11995-a881-7801-bdb4-933c4cc892fa`; effective model identity was not independently
observed. The [core validation](rust-ce3-sc2-core-validation.md) describes the
earlier implementation subject; its historical “uncommitted” wording does not
describe this now-committed planning baseline.

Recommend implementing **ER1, the episode prerequisite**, alongside the separately
owned campaign performance [P1](rust-campaign-history-performance-plan.md). Follow with J1 and J2 below after their exact
handoffs. ER1 has useful independent qualification; it does not qualify the join.
J1 and J2 are proposed bounded follow-ups, not work already accepted by the user.
The [CE3/SC2 integration plan](rust-ce3-sc2-integration-plan.md) and lane plans
retain their semantic owners and excluded operational scope.

## What the committed code actually provides

| Exact source | Consequence for this plan |
| --- | --- |
| `review-episode/src/lib.rs:256,461,499` | Direct typed operations already use the real application. Historical `Read` exists. `Recover` returns the first committed revision **and current state**, so that state cannot stand for the historical result. |
| `review-episode/src/admission.rs:16,26`; `host_admission.rs:223` | `AdmittedResult` construction is crate-private. Native descriptor admission checks exact requests; public request/result DTOs do not mint grants. |
| `review-episode-store/src/sqlite.rs:74,83,356,458` | Native open verifies the marker against the executing binary, then opens SQLite read-write and configures it. A permanent read-only opener is a real prerequisite. |
| `review-episode-core/src/identity.rs:148,220`; `state.rs:144` | Existing `Authority` validates the closed manifest and supplies its exact binding, readers, writer and initial subject. The marker binds a root/executable, not episode authority. |
| `claim-evidence/src/production_path.rs:116,227,297,1732` | CE3 is implemented: full stage request, custody-bearing lease, original admission digest, checked full-claim readback and separate reconciliation. `AdmissionReadRequest` requires the original establishment request/admission hashes as well as record refs. |
| `slice-campaign/src/application.rs:1216,1381,2362,2594,2811` | The concrete port currently implements CE2; its lease holds the campaign mutex. Its finding check requires an executing obligation. Completion/evaluation reducers are private. |
| `slice-campaign/src/completion.rs:148,429`; `contract.rs:180` | Current intent children contain kind/ID/hash/root/grant. Checked outcome carries two claim IDs and `succeeded`; it does not preserve full evidence/consumption/finding outcomes. The private reducer maps that flag to completed/failed. |

Paths in this table are relative to `rust/crates/`. These are bounded source
observations, not new runtime evidence. The join must extend or replace the
private core checked inputs and reducers; exposing their present success flag
would leave a second, weaker completion route.

## ER1: permanent, scoped, read-only episode API

Replace the earlier proposed `TrustedEpisodeReadFiles` plus two new scope/source
documents with **typed startup scope over the existing native root and authority
contracts**. This is a reviewable change from the earlier proposed shape. No
new grant registry, source-document schema, writer grant, broker or service is
needed for this in-process consumer.

Proposed public surface, in `review-episode`:

```rust
// Startup data; neither Deserialize nor a writer capability.
pub struct TrustedEpisodeReadScope { /* private fields; validated constructor */ }
impl TrustedEpisodeReadScope {
    pub fn new(authority: Authority, permitted: Vec<EpisodeReadTarget>)
        -> AppResult<Self>;
}
pub enum EpisodeReadTarget {
    Revision(Revision),
    Transition { transition_id: JsString, content_digest: Revision },
}
pub enum EpisodeReadRequest {
    Revision { identity: Identity, revision: Revision },
    Transition { identity: Identity, transition_id: JsString, content_digest: Revision },
}
impl Application<InProcessReadAdmission> {
    pub fn open_read_only(root: &Path, expected: NativeRootSelection,
        scope: TrustedEpisodeReadScope, options: StoreOptions, response_limit: usize)
        -> AppResult<Self>;
    pub fn read_checked(&mut self, request: &EpisodeReadRequest)
        -> AppResult<CheckedEpisodeReadback>;
}
```

`InProcessReadAdmission` fields and constructor remain private to the episode
crate; `open_read_only` is the only new public construction route. It implements
the existing `HostAdmissionPort`, protocol 2, with `AdmittedResult::checked()`
still crate-private. The scope constructor validates full `Authority`, requires
reader `native-review-host`, generation 1/no predecessor for this initial-only
consumer, and a nonempty, duplicate-free bounded exact target set. A transition
target also permits the owner-internal historical resolution of that transition;
it does not grant arbitrary revision requests. Cap encoded scope/request/readback
using explicit existing episode limits and refuse excess rather than truncate.

Trusted host composition supplies the expected selection and the already selected
authority from its startup/selection owner. `Authority::parse`, a source reference,
or matching hashes alone do not authenticate that source. Untrusted requests may
select only a target already in scope; they cannot supply authority, pins, root,
principal or new scope. These are in-process trust boundaries like the existing
native launcher boundary, not protection against arbitrary code running as the
same UID. HP3 must eventually establish its actual startup provenance; ER1 does
not invent that owner.

At open and each read, validate the anchored absolute root, exact selected native
profile/protocol/codec/executable digest, marker and database identity, and safe
file/sidecar paths. Add `EpisodeStore::open_native_read_only(root, options)` using
SQLite read-only access and the existing schema/history validator. It must not
initialize, migrate, checkpoint or run writable journal configuration. Refuse an
unsupported marker/schema without modifying it. Do not use SQLite `immutable`
mode to bypass a live WAL. Test supported WAL/sidecar behavior explicitly; logical
read-only admission is not a claim that SQLite never touches shared-memory state.
Store writes on a read-only handle fail before transaction effects, including if
a caller combines that handle with another admission port.

`read_checked` verifies its exact request before storage access, loads a validated
snapshot through the real store, and privately constructs readback only after:

- Identity, authority binding (including source and manifest digest), writer and
  initial subject match the installed scope. In this initial-only profile,
  unexpected subject/writer/authority changes in the relevant history refuse.
- A revision request resolves that exact state. A transition request checks the
  exact ID/content digest and resolves the **first introducing revision** from
  validated history. It returns that historical state, never the current state
  mislabeled as the committed result. The current observed revision is separate.
- State/result/evidence-admission content uses the episode owner's existing
  parsing and codec. A checked state may have no result; campaign completion must
  require one. A trustworthy missing revision/transition is explicit absence;
  conflict, corrupt history, swapped roots or inaccessible storage are errors,
  never absence. Recovery is a read, never execution or replay of a write.

`CheckedEpisodeReadback` has private fields, no public constructor or deserializer,
and read-only accessors for selection/root, scoped identity/authority digest,
requested target, observed revision, optional resolved revision/state and optional
episode-codec result digest. It conveys checked evidence, not a continuing campaign
permit. `execute_direct` with this port rejects latest-state read, history,
unscoped recover, begin, resume, validation and every transition before effects.

The native marker recomputes `current_exe()` identity today. Preserve that contract:
test setup runs initialization/native descriptor writes and readback in the
**same test executable**, using a child process for fd 3. HP3's chosen binary must
own the corresponding composition. A separate helper executable cannot read a
root created under another executable by substituting its expected digest.
Executable replacement/root migration is not solved by ER1.

ER1 owns `review-episode/src/{lib,admission}.rs`, new
`src/in_process_admission.rs`, `review-episode-store/src/{lib,sqlite}.rs`, and new
`review-episode/tests/sc2_readback.rs` plus package-local test support. Existing
core parsing/history code is reused. Any discovered need to change core semantics
is a scope handoff. Package Cargo changes are limited to test/dependency wiring;
shared Cargo/lock edits belong to the integration owner.

ER1 exit: actual native descriptor writes and real-store readback; exact historical
resolution after later transitions; explicit absence/conflict; wrong identity,
root, source/authority, reader, writer, executable, target, result/history bytes
and scope refusal; read-only write rejection and unsupported-schema preservation;
compile-fail construction/deserialization checks. `FixtureAdmission` cannot
qualify this gate. Retain HP1 tests and serialize fault tests because the existing
process-global episode fault environment has a recorded race.

## J1: real CE3 admission, durable stages and exact episode join

J1 consumes ER1's accepted exports and performance P1's final source handoff.
It supplies the concrete production-path implementation on
`CampaignClaimsAdmission`, preserving the actual CE3 signature:

```rust
fn acquire_production_path<'a>(&'a self,
    binding: &ProductionPathAdmissionBinding, request: &ProductionPathRequest)
    -> ClaimResult<Box<dyn ProductionPathLease + 'a>>;
```

Install a narrow trusted execution-evidence owner in startup composition, for
example `Campaign::claims_admission_with_custody(Arc<dyn ExecutionEvidenceOwner
+ Send + Sync>) -> CampaignClaimsAdmission`. Retain the current no-custody CE2
constructor; CE3 requests on it refuse. The owner method receives the complete
binding and optional observation and returns the existing CE3 `CustodyEvidence`
only after reading its immutable record. This is a small adapter contract, not a
new evidence service or a caller-provided `verified` callback. The controlled
implementation lives only in joined test support, with fixed independently held
record bytes and negative substitution tests. No production HP3 implementation
is supplied by this slice.

The lease retains the existing campaign mutex through CE's custody read,
transaction and checked readback. Check current root binding/epoch, active slot,
exact selection/candidate/profile/prepared request, registered child kind/ID/hash,
grant/root and full stage request. CE independently checks its registered grant
and custody profile. Release the gate before campaign CAS; then recheck the exact
expected revision. Never hold a campaign SQL transaction across CE, call campaign
mutation recursively from custody, or interpret lock loss as external settlement.

Extend durable completion intent with exact stage records: original complete
admission bindings and hashes, original request hashes, expected owner roots and
grant identities, event/operation/transition IDs, custody refs, both selected
claim revisions, optional actual episode revision, and obtained owner locators/
receipts. Read admission and projection requests need their own registered exact
bindings; they are not pretend finding writes. Persist a result-dependent intent
under campaign CAS before its dependent owner effect. Native result bytes come
from the trusted evidence owner and have a **claim-codec** digest before episode
commit; episode revision remains absent until actual readback. Episode result
digest and campaign physical/config digests remain separately named.

Proposed campaign API is a narrow owned composition, in new `owner_readback.rs`:
`prepare_initial_completion(expected, obligation, operation, evidence_ref)`,
`complete_initial(expected, obligation, operation, owners)`, and
`recover_initial_completion(exact_locator, owners)`. Each is a method on `Campaign`
with explicit `CampaignIdentity`; `owners` borrows the real CE application and ER1
application plus installed trusted evidence reader, not owner-shaped DTOs.
Preparation registers exact intent and returns the existing campaign `Effect`;
completion/recovery return a versioned progress result carrying exact domain
locators, completed stages and unresolved/conflicting stage. They never return
provider permits. Precise DTO field names follow the stage record above; no
public generic journal or checked-outcome constructor is exposed.

Use actual `observation_locator`, `record_observation`, `establishment_locator`,
`establish_claim`, both reconciliation methods and `read_claim_admission`.
Supply full selected documents from `Snapshot.review_selection`, not just IDs.
Read requests include original establishment request/admission hashes, obtained
record hashes and optional observation pair. Derive these from saved commands and
owner receipts; preserve `None` for checked observation absence.

Recovery verifies the live slot/current owner under the gate while retaining the
**original** campaign revision and admission digest in saved CE commands. CE3's
`original_digest()` normalizes access only; replacing the old revision with the
new current revision would change identity. `RecoverExact` must match registered
original content and separately checked current recovery context. Reconcile
before filling a confirmed absent stage; never retry uncertainty under a new ID.

Native episode result writes retain the existing descriptor owner. This read-only
join does not create another writer. Tests supply those real transitions at the
registered boundary, then ER1 resolves the original result transition/revision.
Absent episode commit remains an explicit pending owner stage; J1 recovery can
observe a later original commit but cannot replay the provider or manufacture
the missing result. Check full episode identity/attempt, candidate subject and
result commit/tree/patch, selected authority/writer/profile, transition content,
exact historical state and both evidence admissions against the CE readbacks.

J1 exit is the real-store checked pair and exact historical episode readback,
including partial observation/first establishment/second establishment/episode
states, stale epoch/revision/slot, changed full claim and missing custody refusal.
It does not publish a usable builder projection or claim final joined completion.

## J2: findings, consumption, outcome and builder evaluation

Replace the private boolean `CheckedInitialOutcome` with a private checked aggregate
constructed only by `owner_readback.rs`: exact episode refs, both full checked
claim bindings/statuses/reasons, distinct consumption refs, exact finding receipt
set/projection and applicable rejected-result disposition. Preserve the initial
and current episode refs separately. The campaign stores refs and binding digests,
not a second claims or episode store.

Build each consumption digest with the claims codec over the existing
`{schemaVersion:1, claimRevision, establishment, boundary, consumer}` body, keeping
owner `slice-campaign`, reference `claim-consumption:<claimId>`, and distinct
builder/campaign consumers. Cross-check it against the episode admission pair.
Neither consumption record is terminalization or acceptance.

Use real CE `publish`, `reconcile`, `project_exact_revisions`, `record_reliance`
and `reconcile_reliance`. Register all exact child requests before effects;
reconcile each finding in a partial batch before reading the exact projection.
Extend CE2 campaign admission to explicit current finding/projection/evaluation
intents, including post-outcome `awaiting_builder` state. Its present “executing
plus Finding|Reliance hash” check cannot simply be relaxed to every historical
request. Revalidate original admission identity and current consumption context
separately, including exact root, grant, consumer tree and decision scope.

Proposed wrappers are `prepare_builder_evaluation(identity, expected, operation,
decision, owners)` and `complete_builder_evaluation(identity, expected, operation,
owners)`, returning campaign `Effect`/exact downstream progress. Trusted builder
authority is installed by composition and matched to CE's registered reliance
grant; a decision DTO does not prove authority. Validate exact current finding
revision before durable intent, then reconcile real reliance/projection before
publication. Both valid and invalid decisions retain reliance. A valid decision
alone does not resolve a reviewer finding.

| Checked owner result | Durable campaign consequence |
| --- | --- |
| Either claim False or Unestablished | `evidence_unestablished`; preserve both distinct statuses/reasons; no usable findings/projection. A missing establishment is unresolved instead. |
| Both established, valid result with zero/open findings | `reported` / `awaiting_builder`, with exact refs; neither means accepted. |
| Result rejected by actual owner contract | `correction_required` with rejection/custody identity; no correction capability added. |
| Exact reliance plus all invalid dispositions, or valid plus reviewer-owned verified resolution | Recompute whether reporting is possible. Reviewer resolution/remediation writes remain outside this initial-only slice. |
| Provider effect unknown or episode/result stage missing | Preserve unresolved original stage; no retry permission. |

The expanded semantic snapshot requires a version choice. Recommend **new private
campaign schema/profile 3 roots**, with an explicit new-root schema definition,
marker/user_version/snapshot agreement and unchanged refusal of historical roots.
Do not silently reinterpret core schema-2 snapshots or migrate them. CE remains
schema 2 and episode storage/marker contracts stay unchanged. This policy is a
proposed owner decision requiring acceptance with J1/J2, not already approved.
Freeze the full J1/J2 record vocabulary at J1 entry so J2 does not quietly change
the chosen version. Preserve snapshot/receipt capacity bounds and preflight exact
encoded bytes; later campaign refusal cannot erase already committed CE effects.

J1/J2 ownership: `slice-campaign/src/{application,completion,contract,store,lib}.rs`,
new `src/owner_readback.rs`, `migrations/0003_owner_join.sql` if the version policy
is accepted, package Cargo dependency on real `review-episode`, and new
`tests/{owner_join,owner_join_recovery}.rs` plus focused support/regressions.
Changes to CE3 exports or episode semantics require their owner handoff, not copied
types or a convenience bypass. Shared Cargo/lock remains integration-owned.
Performance P1 owns campaign read mechanics; serialize overlapping `store.rs`
edits and qualify its final handoff before the joined candidate. Performance
changes preserve receipt/revision/admission validation; no cached trust is assumed.

## Gates and explicit remaining dependency

J2 final qualification uses real campaign/CE/episode applications and new private
roots established through their APIs, real native descriptor admission and ER1
readback. Exercise all three claim outcomes, both boundaries, full selected claim
substitution, historical/current mismatch, grants/consumers, partial findings,
stale evaluation and invalid-disposition reliance. No direct row insertion,
`FixtureAdmission`, permissive port or deserialized projection substitutes for
positive admission. Corruption tests may mutate copies only to demonstrate refusal.

Actual-process cuts bracket completion intent, each observation/establishment,
native episode result, each finding, outcome, evaluation intent, reliance and
evaluation commit. Restart distinguishes absent/committed/conflicting/unresolved,
reuses original IDs and locators, and never increases provider invocation count.
Confirmed pre-effect refusal is local NoEffect; cross-owner partial publication
remains explicit downstream progress. Replayed completion never yields a dispatch
permit. Run debug/optimized focused regressions, formatting, Clippy with warnings
denied and API compile-fail gates; serialize fault environments. Bind exact source,
toolchain/lock/features/executables, configuration and raw logs in completion
receipts. Disposable roots/build output stay under `/home/bline/code/.work-engine-tmp`.

Controlled immutable execution evidence qualifies **domain composition only**.
HP3 still owns actual execution/transport/session/artifact custody and provider-
backed positive retry, including checked definite-pre-entry evidence. No provider
adapter, host launch, Node bridge, lifecycle or UI work enters these slices.
Acceptance/terminalization, retained remediation, correction/succession, saved-
state disposition, deployment and Node retirement remain separate authority.
Do not call J2 a fully qualified host or fully qualified runtime retry.

## Planning evidence

Tier 2 Verify used `home-bline-code-work-engine`, coverage generation
`2026-10-08T03:35:49Z`. Relevant symbol searches exhausted their bounded pages;
bidirectional traces were untruncated but sparse/heuristic, so exact source
establishes the material injected-call relationships. All relied-on paths were
coverage-checked. Campaign SQL line 16 has partial parse coverage and was read
directly; other paths reported metadata match/no recorded issue. That is a
best-effort signal, not proof of completeness.

Source hashes, exact observations, graph receipts and limitations are retained in
`/home/bline/.local/state/work-engine/rust-episode-join-performance-planning-20261008/episode-join-evidence.json`.
All relied-on Rust source matches the stated commit. Required working-tree
`AGENTS.md` and `docs/rust-development.md` are separately hashed instruction
context; they are not represented as bytes from that commit.
Only this document and that evidence file were written by this planning lane.
No tests, builds, providers, staging or commits were run.
