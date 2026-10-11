# J2: campaign completion and exact builder evaluation

Proposed on 2026-10-08 against published J1/E1 commit
`3b8d1f4b75375c3328f191748619ca7e515d4a22`. This document authorizes no
implementation, provider entry, migration, activation or publication. It is the
campaign lane of the [next parallel plan](rust-j2-hp3-parallel-coordination.md).
The companion [controlled host plan](rust-hp3-controlled-host-implementation-plan.md)
owns executable composition. Implementation uses the user's selected Sol model.

> **Later annotations.** Blocks headed *Current implementation* were added on
> 2026-10-11 against a working-tree snapshot based on `c72f7a5` (content
> manifest sha256
> `946887f3934c2d5541c41d54ce9fba8d7e1653460d853bac1a788dfa7fa2ac28`). They are
> not part of this plan and do not change its subject. Each points to where a
> symbol the section names is implemented in that later tree. *At subject* means
> a definition of that name existed in the crate at `3b8d1f4` (checked with
> `git grep`); it does not establish the same shape or meaning. *Introduced
> later* means none did.

The reachable exit is initial result publication, truthful claim status,
exact finding publication and builder reliance/evaluation through real owners.
`Reported` means a result has been published. It does not mean an acceptable
verdict, resolved limitations, review acceptance or campaign acceptance. Preserve
the native verdict, episode phase, limitations and unresolved questions even for
a reported result with no findings. Retained remediation and rejected-result
correction remain explicit successors; this is narrower than the broad J2
roadmap's entire outcome vocabulary.

## Published source and placement

The [J1/E1 validation](rust-j1-e1-validation.md) establishes a real campaign,
CE3 and historical ER1 join, including actual controlled-process custody. It
does not establish a campaign outcome or independently enforced read-only
review. A same-UID controlled child truthfully has `mutationAuthorized=true`;
the combined gate preserves unestablished required claims.

| Exact current source | Consequence for this slice |
| --- | --- |
| `slice-campaign/src/owner_readback.rs:3114` | `complete_initial` and recovery return `OwnerReadsJoined`. Preserve that API meaning. |
| `slice-campaign/src/application.rs:979`, `owner_readback.rs:118` | Both existing finding admission routes currently refuse J2. Older proposals describing a still-enabled boolean result reducer or executing-plus-child-hash admission are stale. |
| `slice-campaign/src/completion.rs:304`, `:317`, `:623` | Consumption, four outcome variants and evaluations are reserved, with `Value` payloads. Reservation is not semantic validation. |
| `completion.rs:342`, `store.rs:727`, `:898` | Current readers reject nonempty J2 stages, consumed commitments, outcomes and evaluations. Enabling writes requires corresponding current and historical integrity checks. |
| `owner_readback.rs:1867` | J1 already derives the exact two consumption commitments. Preserve these bodies and refs, including the full establishment reference in the body. Do not substitute the older Node body's establishment ID. |
| `claim-evidence/src/application.rs:599`, `:643`; `projection.rs:231` | Real exact projection, reliance and reconciliation exist. A CE projection requires 1–100 exact revision selections. There is no CE empty projection. |
| `review-episode-core/src/implementation_review_v1.rs:13`, `reducer.rs:263` | Accepted native results include incomplete verdicts and limitations. Episode phase is separate from publication status. |
| `app-server/src/services/slice-campaign/native-review-closure.mjs:17`, `:344` | Existing closure reports when phase is reported **or findings are empty**, and promotes awaiting-builder after all exact findings have reliance plus invalid disposition or valid plus reviewer-verified resolution. Preserve this meaning; do not quietly repair it into a new acceptance rule. |
| `app-server/src/services/claim-evidence/review-finding-bridge.mjs:22` | Existing finding mapping supplies attributed episode evidence, stable claim identity and exact revision lineage. Port its semantics into the campaign composition, not its runtime. |

Rust paths above are relative to `rust/crates/`. J2 belongs in the permanent
campaign owner: it owns consumption and result publication, while CE owns
findings/reliance/projections and episode owns review results. A host-local
result cache or universal workflow journal would duplicate these owners. No
Node bridge or temporary full host is needed.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/slice-campaign/src/owner_readback.rs::Campaign::complete_initial`, public as `slice_campaign::Campaign::complete_initial`; at subject.

## Entry, outcome contract and authority

Entry requires a new private schema-3 campaign root, exact admitted selection,
candidate/profile/prepared request, original dispatch and execution reference,
and J1's checked pair plus original native result readback. Reopen reinstalls
the same trusted owner selections. Each operation verifies the current slot,
epoch, configuration and registration receipt; an old reference is data only.
Do not use J2 as an alternate route around a pending or conflicting J1 stage.

| Checked result | Initial campaign publication |
| --- | --- |
| Either CE claim is `false` or `unestablished` | `EvidenceUnestablished`, retaining both exact statuses and reasons separately; no usable finding set or builder projection. A false claim is never relabeled unestablished inside the pair. |
| Both established; native episode phase is `reported`, or exact native finding set is empty | `Reported`, with checked pair, consumption commitments, exact episode/result and finding/projection evidence. An incomplete zero-finding result remains incomplete. |
| Both established; other native phase and nonempty findings | `AwaitingBuilder` with the same complete evidence aggregate. This does not authorize remediation. |
| CE establishment/read, execution record or episode result is missing, conflicting or unreadable | Pending/conflicting/unresolved progress; no invented claim status or outcome. |
| Malformed/rejected result without a checked durable rejection record | Preserve the original failed/pending owner operation. `CorrectionRequired` remains reserved and refused; do not manufacture a rejection reference from an exception string. |

Use a private, non-deserializable `CheckedInitialOutcome` aggregate constructed
only after real owner reads. It holds the exact pair, selected full claim
digests, historical episode/native result, consumption commitments, full
finding receipt set and the applicable projection evidence. It is never a
boolean and cannot be supplied through a command DTO. Expose an authority-free
`InitialOutcomeView` with typed discriminants and exact refs, including native
verdict/phase/limitations/questions. Replaying the view returns no capability.

Proposed mapping to already reserved attempt state: published `Reported` or
`AwaitingBuilder` sets `AttemptOutcome::Completed` and obligation `completed`;
`EvidenceUnestablished` sets `FailedWithResult` and obligation `failed`.
Here `Completed` denotes completion of the initial execution/result publication,
not acceptance. The richer outcome is authoritative for usability. Keep the
original request slot active as an identity/fencing anchor; its dispatch remains
consumed, so no dispatch/retry permit can be reissued. Evaluation eligibility
uses the exact checked outcome, not the obligation status alone. This mapping
is an explicit plan acceptance choice, not existing implemented behavior.

## API checkpoint shared with HP3

Keep J1's public signatures and `OwnerReadsJoined` unchanged. Add:

```rust
pub fn finalize_initial_completion(
    &mut self, identity: &CampaignIdentity, expected: &CampaignRevision,
    joined: &InitialCompletionLocator, outcome_operation_id: &str,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<InitialOutcomeProgress>;

pub fn recover_initial_outcome(
    &mut self, locator: &InitialOutcomeLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<InitialOutcomeProgress>;

pub fn locate_initial_outcome(
    &self, identity: &CampaignIdentity, obligation_id: &str,
    outcome_operation_id: &str,
) -> Result<Option<InitialOutcomeLocator>>;

pub fn prepare_builder_evaluation(
    &mut self, identity: &CampaignIdentity, expected: &CampaignRevision,
    joined: &InitialCompletionLocator, operation_id: &str,
    decision: BuilderEvaluationDecision,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<Effect<BuilderEvaluationPrepared>>;

pub fn complete_builder_evaluation(
    &mut self, identity: &CampaignIdentity, expected: &CampaignRevision,
    locator: &BuilderEvaluationLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<BuilderEvaluationProgress>;

pub fn recover_builder_evaluation(
    &mut self, locator: &BuilderEvaluationLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<BuilderEvaluationProgress>;

pub fn locate_builder_evaluation(
    &self, identity: &CampaignIdentity, obligation_id: &str,
    evaluation_operation_id: &str,
) -> Result<Option<BuilderEvaluationLocator>>;
```

`InitialOutcomeLocator` contains the original J1 locator, outcome operation ID,
registration `RecoveryLocator` and exact registration request digest.
`BuilderEvaluationLocator` additionally binds the exact finding/revision,
consumer tree and decision scope through its registration digest. These are
descriptive locators, not grants. Completion progress contains locator, last
checked campaign revision, saved stages, newly observed owner receipts and one
of `Pending(stage, reason)`, `Committed(view, receipt)`,
`Conflicting(stage, reason)` or `Unresolved(stage, ownerLocator, reason)`.
Evaluation progress uses the same distinctions with a checked evaluation view.
The two read-only locator methods reconstruct exact locators from the original
caller-known operation IDs, immutable registration receipts and retained
campaign intent, including already committed outcomes/evaluations. They verify
root/config/identity/obligation and full registration correspondence. `None`
means that exact campaign registration is confirmed absent, not that an
external effect is absent; inaccessible/corrupt/conflicting state is an error.
Thus a crash after registration but before the returned locator is delivered
does not strand work or require a new operation ID. HP3 persists its chosen
IDs before invoking mutation, then uses lookup followed by recovery on restart.

Finalization registers the initial-outcome stage under CAS before any new CE
effect, then makes one bounded pass through reachable stages. Preparation of
evaluation is effect-free outside the campaign store; it registers exact
downstream commands and returns a recoverable identity. No method polls,
invokes a provider, writes an episode or creates a dispatch permit.
After this invocation observes any downstream commit, later read/CAS/encoding
failures return progress retaining all in-memory receipts and original locators,
not an error that implies global `NoEffect`.
The same rule preserves a newly committed campaign registration if later work
fails. `Result::Err` represents refusal before this invocation's effects, with
the caller-known IDs still available for exact lookup after uncertain delivery.

Keep `InitialJoinSelection` and its digest unchanged. Add one optional
trusted-startup `CompletionSelection`, installed once per campaign open by a
checked campaign factory. It selects the exact CE root, finding grant ID/digest,
builder reliance grant ID/digest, builder consumer identity/tree, decision scope
and authority-source references. Its constructor is a trusted composition API,
not a request deserializer. Owner calls verify selected grant digests through
their real locators and grant validation. Persist the exact selection data in
the registered J2 command payload, and compare it on reopen; do not add a new
top-level schema field or alter the saved J1 selection digest. Refuse a different
installation in one open. No finding/evaluation selection is needed to publish
`EvidenceUnestablished`, so HP3's controlled lane gains no unused authority.

`BuilderEvaluationDecision` contains CE's exact `ClaimId`/`FindingRevisionId`
and `valid: bool`. Bind reserved `EvaluationRecord.finding_id` to that CE claim
ID, with native finding IDs retained losslessly in the finding-set command
mapping; do not force native UTF-16 IDs into Rust `String`. This concrete binding
of the reserved payload is part of J0, not an alternate native-ID codec.
The installed builder consumer, tree, scope and registered grant supply the
authority context. A decision object cannot select its own authority or move
the finding to another consumer. The initial profile binds the original
candidate tree; changed-tree evaluation requires the later remediation join.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/slice-campaign/src/owner_readback.rs::InitialJoinSelection`, public as `slice_campaign::InitialJoinSelection`; at subject.
> - `rust/crates/slice-campaign/src/recovery.rs::RecoveryLocator`, public as `slice_campaign::RecoveryLocator`; at subject.

## Stored representation and explicit schema decisions

Use the frozen schema-3 marker/profile/user version and existing SQL tables.
Enable only J2 variants for which complete write, read, replay and corruption
validation is implemented. Retain the closed stage vocabulary from
[the J1 plan](rust-j1-owner-join-implementation-plan.md#recommended-schema-decision-and-frozen-j1j2-vocabulary).
No schema-1/2 migration, old-root writable configuration, historical-root
disposal or implicit schema-3 expansion is permitted.

The reserved `Value` containers are encoding slots, not permission for arbitrary
JSON. Use private typed adapters with closed fields, exact domain codecs and
owner reconstruction. Store native command/result values as canonical JSON
strings where needed to preserve UTF-16 code units; never route native payloads
through a lossy `serde_json::Value` conversion. Keep raw SHA, claim-codec digest,
episode result digest, transition digest and campaign physical digest distinct.

The J1 singleton stage-ID scheme and six-stage limit must become kind-aware:
J1 kinds remain singleton; `Finding` identifies each exact native finding using
its CE-codec identity digest rather than interpolating arbitrary UTF-16 text;
evaluation kinds include their exact evaluation operation. IDs remain unique.
Validate multiplicity and dependencies, not a blanket unique-kind set. The
initial profile permits 0–100 native findings, matching CE's one exact nonempty
projection. Reject 101+ and oversized encodings before the first J2 downstream
write; do not truncate or silently batch into a different outcome contract.

Each finding stage contains the exact CE `FindingCommand`, original admission,
request digest, selected root/grant, real locator and receipt/progress. Projection
stages contain a real `ProjectionRequest`, its original admission/hash and
selected exact revision set, plus checked owner output/digest. They are read
stages, never fake finding writes. Evaluation `reliance` and `projection` fields
hold these same exact envelopes, with publication state restricted to registered,
owner-committed, checked or committed. The campaign stores attributable owner
results and refs; it does not copy a second canonical claims store.

**Empty finding decision:** CE cannot build a zero-selection projection. Propose
the reserved outcome `projection` field encode a closed sum. Its nonempty arm
is exactly `{kind:"ce_projection", requestId, projectionCanonicalJson,
projectionSha256}`, where the JSON is the checked CE projection's own
`to_js_value()` encoded with CE's codec and the SHA is that value's CE digest.
The registered projection stage binds the selected root, exact request and
admission; this envelope describes that checked read, not a fictional durable
CE projection record. The empty arm is a campaign-owned `no_findings` witness.
Its
body is exactly `{schemaVersion:1, kind:"no_findings", initialEpisodeRef,
nativeResultClaimSha256, findingSetSha256, count:0}`. `findingSetSha256` is the
claim-codec digest of the exact empty array from the checked native result.
Witness identity is `no-findings:<claim-codec digest of body>`, owner
`slice-campaign`; it has no CE projection reference or CE freshness claim.
Validation recomputes the body from checked episode/execution evidence and
requires the complete native finding set be empty. Missing/unreadable findings
are not empty. This follows the existing Node closure's separate zero-finding
builder context, but is a proposed precise Rust representation.

The campaign/schema owner must explicitly accept this binding of an unimplemented
reserved schema-3 payload before enabling it. Old J1 readers continue to refuse
all nonempty J2 states, with bytes preserved. If the owner judges the new tagged
payload to exceed the frozen schema-3 contract, the empty outcome remains refused
and a separately approved version change is required; the implementation cannot
silently add the dimension under schema 3 or claim the zero-finding exit passed.
There is no CE owner change in this proposal.

The two consumption bodies are the existing J1 CE-codec
`{schemaVersion:1, claimRevision, establishment, boundary, consumer}` bodies,
with distinct `builder_projection` and `campaign_terminalization` consumers.
Recompute and compare their digest/ref against both CE readbacks and episode
admissions. Do not cross-substitute the pair. `consumedAtOperationId` records
use at its named consequence: populate the builder-projection commitment in the
same CAS that publishes its usable view. Leave the campaign-terminalization
commitment unconsumed because this slice does not terminalize. Both claims are
checked as required for initial publication, but checking the terminalization
claim is not crossing its terminalization boundary. Its later owner controls
that marker. No marker attests that a human or builder process actually read a
response. This asymmetry is deliberate; the shared record shape does not
justify repurposing a boundary.
Unestablished/false outcomes retain commitments and pair but mark neither as
successfully consumed for a usable projection.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/claim-evidence/src/contract.rs::FindingCommand`, public as `claim_evidence::FindingCommand`; at subject.
> - `rust/crates/claim-evidence/src/projection.rs::ProjectionRequest`, public as `claim_evidence::ProjectionRequest`; at subject.

## Finding and evaluation semantics

Construct finding commands only from the exact checked native result. Preserve
the existing bridge mapping: namespace `implementation-review`, subject kind
`revision-bound-finding`, stable subject episode key plus native finding ID,
evidence baseline the exact initial episode, statement the finding title,
proposition its consequence, attributed-review support, original result
limitations/confidence, source evidence, profile payload finding ID/severity/
episode/outcome, and registered decision scope. Preserve UTF-16 values in this
mapping. Initial publication creates one exact finding revision per native
finding; no revision/remediation route is enabled. Bind every finding ID to its
initial/current revision refs, which are equal in this profile.
The episode key is `review-episode@` plus CE-codec digest of the exact episode
identity; stable subject ID appends `:` and the native finding ID without lossy
conversion. The source reference's integrity is the CE-codec digest of the
exact historical episode value. Keep that digest separate from J1's episode-
codec state/result references, even where ordinary values happen to hash alike.

Register the complete ordered finding command set before publishing the first
finding, so an invalid late member cannot create an avoidable partial batch.
Reconcile every locator before filling confirmed absence; never issue a new ID
for uncertain writes. Publish the exact projection only after the entire set
has checked receipts. For an empty set, check the owned witness instead.
Internal CE publication does not expose a usable builder view: the campaign
projection/publication gate supplies that boundary, and this private profile
does not install unrelated finding readers with bypass authority.

Builder evaluation requires a current `Reported` or `AwaitingBuilder` outcome,
an exact member finding, no previous committed disposition for that revision,
the installed builder authority and selected tree/scope, and current CE revision.
Register reliance and projection intent before any reliance effect. Call real
`record_reliance`/`reconcile_reliance` for **both** valid and invalid decisions.
Only after checked reliance and a fresh exact projection may a campaign CAS
publish the evaluation. Failed CAS retains reliance/projection evidence for
recovery. Duplicate exact operation replays; conflicting decisions or stale
finding revisions refuse. CE itself rejects reliance on a superseded revision.

Retain the original initial outcome receipt and episode reference through later
evaluation publications. Derive the current outcome from that origin and exact
committed evaluation records. Preserve the existing rule: `AwaitingBuilder`
becomes `Reported` only when every finding has exact reliance and is either
invalid or valid with an already reviewer-owned `verified_resolved` outcome.
A builder's valid decision cannot resolve a finding. No later episode result
or reviewer remediation write is introduced. New current episode/result
provenance outside this initial profile causes a conflict/unsupported join,
not silent adoption of a new result.

## Admission, transactions and recovery

Implement the J2 `FindingAdmissionPort` on the existing concrete CE3 campaign
port, retaining its campaign Arc/root/epoch/selection binding. The CE2-only
factory does not acquire custody or evaluation authority. The lease checks the
entire saved J2 request/admission/root/grant correspondence, selected candidate,
profile/request/result/episode, exact registered stage and registration receipt,
both established claims, consumption context, and current attempt/outcome or
evaluation eligibility. No blanket relaxation to all historical matching hashes.

CE2 `AdmissionBinding` has no `RecoverExact` field. Preserve its original
registration-base revision and digest byte-for-byte, and check current recovery
eligibility separately inside the concrete campaign port. Recovery admits only
this exact pending original stage/evaluation under the current owner, not an
arbitrary stale request. The CE3 `RecoverExact` contract remains unchanged.

The order remains campaign registration/CAS, release campaign transaction,
bounded CE call holding the campaign mutex lease, release lease, campaign
progress CAS. Never hold a campaign SQL transaction across CE or ER1. A checked
projection must pass CE freshness verification before campaign publication;
freshness failure is re-read/reconciliation of the same registered projection,
not a new finding/reliance effect. The selected claims writer is exclusive for
this profile; no external writer may race the final owner checks. ER1 reads
occur outside the gate and are revalidated against current campaign context.

Recovery distinguishes absent, committed, conflicting and unresolved at every
owner boundary. Register identifiers before effects; preserve original locator,
admission bytes and exact content through restart. Reconstruct private checked
aggregates through owner APIs, never deserialize them from the campaign's
serialized `Value` payloads. Read-only campaign validation checks every saved
stage against its immutable registration and checked receipt; effectful recovery
also rechecks real owners. Neither path converts missing custody into absence.

Preflight exact encoded campaign state/receipt capacity and every CE command
before new downstream work. Account for the bounded maximum receipts/projection
and outcome representation; if a later checked owner value still exceeds a
bound, retain progress and original locators rather than erase earlier effects.
No oversized public write may commit a state its reader rejects.

## Bounded ownership and parallel handoff

J2 owns `rust/crates/slice-campaign/src/{application,completion,owner_readback,
contract,store,lib}.rs`, a focused new `src/initial_outcome.rs` if useful to keep
the owner composition readable, and package tests `tests/{initial_outcome,
builder_evaluation,initial_outcome_recovery}.rs`. It may extend package-local
support and J1 regression tests without moving their proof into a permissive
fixture. No SQL or new dependency is presently needed. Any shared Cargo/lock
change is an explicit handoff to the current S6/S7 integration owner, not a
parallel edit. CE/episode/evidence exports are consumed unchanged; a discovered
missing export returns to that owner for a bounded interface decision.

Before parallel implementation, freeze the public signatures/progress/locator
types above as checkpoint J0 and resolve the explicit schema/consumption choices.
HP3 can implement its controlled initial/J1 host against published APIs while J2
implements these additions. HP3 owns no campaign file. Its final J2 adapter and
combined qualification wait for J2's accepted API/source handoff; broad HP3 is
not independently complete just because its core compiles. The parent owns all
interface reconciliation and combined qualification. No stub that reports a
successful J2 outcome may stand in for that handoff.

## Qualification and truthful proof limits

Use real campaign, CE and episode stores created through their public APIs,
the native descriptor writer and ER1 historical reader. For established and
false domain paths, use E0's existing fixed controlled-fixture evidence owner,
with its default-off feature and runtime class barrier. This proves domain
composition, including negative evidence, not actual independent execution.
Do not mutate claim profiles or same-UID capability facts to get a green result.
The separate actual-process gate uses E1's permanent producer/reader and
must reach `EvidenceUnestablished` with its original truthful reasons.

Required cases are:

- Exact established pair with zero, one and multiple findings; false/unestablished
  on each boundary and mixed pairs; checked observation absence versus unreadable
  owner; full selected-document substitution and swapped consumers.
- Incomplete zero-findings preserves verdict, limitations/questions and native
  phase while `Reported` remains publication-only. Missing result/finding data
  cannot create a zero-finding witness. Counts 100/101 and exact byte boundaries
  prove complete-set publication or pre-effect refusal.
- Native UTF-16 lone-surrogate finding text/IDs and limitations survive result,
  finding command, CE digest, stored intent and replay without replacement.
- Valid and invalid builder dispositions both produce exact reliance; duplicate,
  stale revision/tree/scope/authority and opposite replay refuse; valid alone
  leaves an open finding unresolved; all-invalid can become reported without
  changing native reviewer attribution or accepting the review.
- Wrong root/epoch/slot/grant/registration/digest, copied owner refs, missing
  receipts and corruption in current and historical results refuse. P1's
  current-read validation remains once, exact receipt checks remain intact,
  and replay-history regressions retain their published evidence limits.
- Compile-fail cases reject construction/deserialization of checked outcomes
  and recovered dispatch permits. Serialized public views grant no authority.

Actual-process fault cuts bracket initial-outcome registration, consumption
stage registration, each finding commit, projection read, final outcome CAS,
evaluation registration, reliance commit, evaluation projection read and
evaluation CAS. Include owner commit followed by campaign read/CAS failure,
and failed delivery immediately after registration or final campaign commit.
At the registration cuts discard every returned/in-memory new locator, then
reconstruct it only from the caller's original IDs through the locator API.
Reopen all real owners;
reconcile absent/committed/conflicting/unresolved without a new ID. The combined
E1 gate preserves its independent output witness and cannot execute the child
again during any J2 recovery. Fixture crash tests attest fixture domain recovery
only, not a live-provider invocation count.

Run focused default/debug and optimized J2/J1 tests, serialized fault tests,
formatting, affected-package Clippy with warnings denied and API compile-fail
gates under the pinned workspace toolchain/lock/features. Retain exact source,
toolchain, executable/configuration hashes, test counts, raw logs, crash ledger
and limitations. Build/disposable roots use `/home/bline/code/.work-engine-tmp`.
P1 integrity/read-count regressions are mandatory; no new broad performance
claim follows from functional tests. No build or test ran during this planning.

J2 acceptance requires reachable positive domain proofs and the truthful E1
unestablished outcome, all corruption/recovery gates, and explicit acceptance of
the representation choices. It does not require a fabricated established live
review. It does not complete rejected-result correction, retained remediation,
provider retry, lifecycle, UI, supervisor dispatch, terminalization, acceptance,
migration/disposition, deployment or Node retirement.

## Planning evidence and acceptance choices

Evidence is retained under
`/home/bline/.local/state/work-engine/rust-j2-hp3-planning-20261008/j2/`:
source hashes, graph/coverage receipt, planning provenance and final plan hash.
All relied-on product source matches the publication; current `AGENTS.md` and
`docs/rust-development.md` are separately hashed instruction context and differ
from that commit.
Planner `CODEX_THREAD_ID` is `01a11cea-1fa7-7a91-a7a2-620172181c39`.
Tier 2 Verify used project `home-bline-code-work-engine`, generation
`2026-10-08T19:06:54Z`. Bounded searches exhausted their pages; bidirectional
traces were sparse/heuristic and did not establish injected owner relationships.
Exact source controls the claims above. Every relied-on path was coverage
checked; all three campaign SQL files were read in full for the reported partial
parse ranges. No recorded gap is not proof of completeness. Binary fixture
vectors were not used as planning evidence.

The acceptance decision covers: the narrow reachable J2 scope and reserved
rejection successor; typed public API J0; exact schema-3 payload binding for the
empty witness and evaluation envelopes; boundary-specific consumption markers; attempt/
obligation mapping; 0–100 complete finding bound; and disjoint ownership with
serialized final host integration. Any missing semantic dimension returns to
its owner/version decision before implementation. This planner changed only
this document and its external evidence, preserving the shared worktree.
