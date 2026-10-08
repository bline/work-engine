# HP3-E1: permanent execution-evidence custody

Prepared 2026-10-08 against published
`035a970443c4b4ca8e663dd0e66f4b8b28746268`. **Planning only:** this document does
not authorize implementation, builds, tests, providers, publication, activation
or saved-state disposition. Planning was assigned Astra/high; future
implementation and ordinary remediation use Sol/high. The effective model is
not independently observable from the planning tools.

Implement the smallest permanent review execution-evidence owner as
`rust/crates/review-execution-evidence/`. It owns the request-to-process-to-result
custody record and its checked reader. Its first accepted profile runs a bounded
controlled child through the actual producer and reads the resulting immutable
records after restart. It does not implement the complete HP3 host or the real
provider adapter. J1 can implement campaign/CE3/episode composition concurrently
after a small shared interface prerequisite; their final composition gate still
depends on both implementations.

## Scope and the HP3/HP4 distinction

The [host roadmap](rust-host-replacement-plan.md) assigns controlled native host
qualification to HP3 and the real provider adapter, actual supervisor dispatcher
and permanent startup/restart owner to HP4. Later
[CE3](rust-claims-ce3-implementation-plan.md) and
[episode-join](rust-episode-owner-join-implementation-plan.md) documents use
“HP3” for the actual execution-custody dependency and provider-backed retry.
Read that as a dependency on the permanent host/execution lane, not authorization
to move HP4 into this slice. The precise division proposed here is:

| Boundary | Qualification |
| --- | --- |
| HP3-E1 | Actual controlled process, permanent producer/store/reader, exact request/result/session/transport/artifact custody, local pre-entry facts and conservative uncertainty |
| Later HP3 host join | Real campaign dispatch admission, CE3/J1/ER1 composition, selected native host identity and controlled reviewer route |
| HP4 provider/dispatcher join | Actual selected provider, credentials, capabilities and continuity; provider-specific entry/reconciliation evidence; actual supervisor route and startup reconstruction |

E1 neither relabels a controlled process as a provider nor weakens a real
provider claim so that controlled evidence can establish it. A provider-disabled
test proves its named controlled boundary. Provider-backed retry qualification
remains outstanding through HP4. Whole-root cutover, deployment and historical
state handling retain the roadmap's separate authority.

## Source findings and placement

CE3 is already published. `claim-evidence/src/production_path.rs:116–133` owns
the full `ProductionPathAdmissionBinding`; `245–257` owns `CustodyEvidence`;
`297–311` owns the custody-bearing lease/port. `checked_custody` at `1276–1379`
checks the startup custody configuration, reference schemes, exact attempt,
result, session, observation digest and artifact-reference set. Its public DTO
fields are not trusted evidence merely because they can be constructed.

The actual legacy owner is
`app-server/src/services/slice-campaign/native-review-host.mjs`, not the stale
short path `app-server/src/native-review-host.mjs`. Its owner factory at
`239–286` composes the reviewer adapter and domain services.
`reviewer-runtime/native-claude-code-adapter.mjs:315–672` owns the legacy
request/session/process/transport reconstruction. Its current mutable latest
attempt pointer, time-based artifact searches, and returned `verified` fields
are observations of the predecessor, not the permanent Rust custody contract.
In particular, do not copy its authentication-message inference into E1 as a
proof of real provider non-entry.

The new crate is a refinement of the roadmap's `review-workflow` responsibility.
Both campaign J1 and the later workflow need its reader types; putting those in
`review-workflow`, which will depend on campaign, creates a dependency cycle.
Putting execution facts into campaign or claims gives either domain a second
semantic owner. A narrow crate gives this existing responsibility a permanent
acyclic home; it is not a new daemon, general evidence service or storage framework.

Dependencies point from `slice-campaign` and later `review-workflow` to
`review-execution-evidence`, and from that crate to `claim-evidence` for the
existing claim codec, `JsValue`, binding and custody types. CE does not depend on
campaign or the new crate. No episode or lifecycle dependency is needed.
Use existing pinned Tokio, SHA-256, typed errors and SQLite versions; package-local
`rusqlite = 0.37.0` with `bundled` follows the currently published claim crate.
Only add serialization/schema dependencies actually used by the versioned
record contract. Do not introduce a universal encoding or database package.

Existing process affordances were examined, not assumed absent:
`lifecycle-runtime/src/process.rs` exposes `ProcessSupervisor`, whose activation
contract requires its controlled GO peer; `native_stdio.rs` exposes `NativeStdio`
with bounded framing and exact reap. The latter discards stderr, frames stdout
as JSON-RPC lines, and its error type does not distinguish initial spawn failure
from failure after a child exists. Neither API exposes the durable byte capture
and exact launch boundary needed here. Importing the lifecycle executor would
also bring its admitted-entry semantics into review. E1 therefore implements
only its narrow process-capture mechanics locally using Tokio child ownership;
it does not copy lifecycle's durable operation/admission/recovery system. A future
shared mechanical extraction needs both owners' accepted API and consumers;
it is not an E1 prerequisite or authority to edit active lifecycle files.

## Interface prerequisite and real parallelism

Before parallel builders edit, the integration owner freezes E0: crate registration,
the reader contract below, named encodings/limits, and a default-off controlled
test-owner feature. E0 supplies no store, producer, launcher or complete record
validator; its fixture validation covers only the fixed contract binding and
result/observation digest invariants needed by J1. This is a small source interface handoff, not a fake working
runtime or evidence of E1 completion. One owner writes the initial crate
`Cargo.toml`, `src/lib.rs`, `src/contract.rs` and test-support surface and shared
workspace/lock edits; E1 takes those crate files after the handoff. J1 imports
the accepted exports and does not redefine them in campaign.

```rust
pub trait ExecutionEvidenceOwner: Send + Sync {
    fn read_result(&self, reference: &ExecutionEvidenceRef)
        -> Result<CheckedExecutionResult, EvidenceReadError>;
    fn read_custody(&self, reference: &ExecutionEvidenceRef,
        binding: &claim_evidence::ProductionPathAdmissionBinding,
        observation: Option<&claim_evidence::codec::JsValue>)
        -> Result<claim_evidence::CustodyEvidence, EvidenceReadError>;
}
```

`ExecutionEvidenceRef` is a versioned descriptive root/attempt/record identity,
including owner, profile, record revision and digest. It confers no access or
dispatch grant. `CheckedExecutionResult` has private fields, no public constructor,
no deserializer and no DTO conversion. Read-only accessors provide the exact
native result `JsValue`, its claim-canonical bytes and claim-codec digest, the
raw returned-byte digest, execution provenance, and checked observation state.
The claim-codec digest is never substituted for physical artifact SHA-256 or
episode result/revision digests.
“Checked” here establishes bounded parsing, exact request/result binding and
custody; the existing result/episode owners still decide semantic result validity.
It does not mean that findings are valid or a review is accepted.

Use `CheckedObservation::Present` with the exact owner-built CE observation
document, or `Absent` with an explicit owner-recorded absence reason. A missing
record, unreadable artifact, unsupported profile, malformed result or lost
transport is an error/unresolved custody, not checked observation absence.
`EvidenceReadError` distinguishes absent exact record, conflicting identity or
bytes, inaccessible/unresolved custody, unsupported contract and capacity refusal.
These errors grant no retry permission. The public trait can be implemented by
trusted Rust composition, like CE's existing admission port; arbitrary installed
Rust code is within that trust boundary. Private checked values alone do not
authenticate a malicious implementation of the trait.

For J1 development, E0's default-off `test-support` feature can create a fixed,
explicitly controlled in-memory owner from independently held fixture records.
It validates bounded fixture fields and exact binding/result/observation digests,
then uses private checked constructors tagged with the closed `ControlledFixture`
evidence class. These small contract checks remain shared with E1; durable root,
record-chain, process and artifact validation arrives with E1 and is not simulated
by E0. `ControlledFixture`, `ControlledProcess` and future qualified provider
evidence classes are distinct closed variants; the fixture factory has no input
that selects another class. Readback carries this class, the startup reader scope
checks it, and a production/provider custody profile rejects fixture and controlled
process classes even in a binary compiled with `test-support` or `--all-features`.
Default-off compilation reduces exposure but does not supply that trust boundary.
This proves domain composition only. E1's producer gate uses neither fixture
record insertion nor this feature. Until E0 lands, parallel reconnaissance and
private edits can proceed, but independently invented lookalike types are not
an executable parallel integration strategy.

The permanent reader has no callbacks into campaign and performs only bounded
local reads under the J1 lease. The proposed J1 `CE3CampaignClaimsAdmission`
holds the campaign gate through CE custody read/transaction/readback, using the
installed `Arc<dyn ExecutionEvidenceOwner>`. No provider/network wait or campaign
SQL transaction occurs inside that custody call. J1 maps unresolved reads
truthfully and rechecks campaign CAS after releasing the gate.

## Three separate trust boundaries

**Startup installation.** A trusted composition entry point opens an anchored
private evidence root with expected schema, owner/verifier/profile identity and
exact source/configuration/executable pins. Reader and producer handles are
distinct; the reader cannot initialize, migrate or append. Requests cannot
select a root path, verifier, executable or trusted profile. Public source/hash
fields describe what startup selected; parsing them does not prove that selection
was authorized. E1's actual-process harness supplies controlled startup authority;
the later HP3/HP4 composition supplies the actual campaign/deployment owner.
There is no claim of defense against arbitrary same-UID code.

**Launch admission.** The producer accepts only an installed, bounded exact
execution scope, validates the complete request against it, and privately mints a
single-use launch ticket after durable reservation. The scope is trusted startup
data, not a wire-deserializable permit. Its first profile is closed to a pinned
controlled child, exact working-directory/subject/input identity, no credentials,
bounded environment and no ambient PATH/command substitution. A caller-supplied
`admitted`, `read_only`, `not_entered` or `verified` value cannot mint the ticket.
The harness's exact scope is explicitly not evidence of campaign admission.

**Recorded facts and CE readback.** Only the producer's owned child pipes and
profile-specific observations create records. There is no public
`record_verified_result(dto)` production API. The reader revalidates immutable
bytes, root binding, producer/profile and required artifact set before returning
custody. CE still checks its independently installed registered grant and custody
profile; J1 still checks current campaign root/epoch/revision/slot/request.
These three checks cannot replace each other.

The stable `ExecutionBinding` includes campaign root, obligation, candidate,
selection/profile/prepared-request digests, review episode and attempt identities.
Provenance separately retains the original dispatch operation and revision.
J1 preparation matches those against its actual DispatchRecord and original
receipt, then registers the exact evidence reference in the completion intent.
Later CE completion operation/revision, child-request hash, stage/access and
optional committed episode revision are J1 stage facts: they did not exist when
the process ran. The reader compares stable execution fields and exact
result/session, receives the full binding, and never claims custody over those
future campaign/episode facts. J1 validates them under its live gate.

CE3's absence branch specifically requires `session_id=None`, no observation
digest and an empty artifact-reference list; its binding must also have
`session_id=None`. E1 retains the actual execution session in result provenance
even when a separately recorded absence justifies that CE envelope. `None` is
never inferred from a caller omitting the observation. A caller requesting
absence against a record containing a present observation is rejected.

## Immutable record and storage contract

Use a new private schema-1 evidence root, with exact marker/schema/profile and
root identity. Preserve old roots; no import, migration or mutable latest pointer
is included. The chosen initial implementation keeps bounded artifacts as BLOBs
in the owner's SQLite database so terminal manifest and artifacts can commit
together; a directory of mutable paths is not the custody store.

The original request record binds the stable identities, dispatch provenance,
raw/canonical input digests, subject/workspace identity, executable and arguments,
nonsecret environment policy, selected controlled profile and expected session.
Never store credential material. Events carry exact attempt identity, predecessor
record digest, kind and version, including prepared, launch-intent, spawn outcome,
captured result/failure and terminal custody. Each is append-only with unique
attempt/stage keys; repeated identical writes replay, changed bytes conflict.
An execution instance and OS PID are separate identities. No timestamps or
“latest file” heuristic choose a result.

The terminal manifest binds result bytes, transport record, session observation,
stdout/stderr and any profile-required artifacts by owner/kind/byte length/raw
SHA-256. It retains claimed versus actually observed provider/model/session facts
separately. E1 has `controlled` transport/realization; real provider/model facts
remain unavailable. Observation bytes are constructed from these owned facts,
not copied from an untrusted result's capability claims. Record identity uses a
new named domain/version codec with fixed golden vectors; its JSON encoding is
not reused for CE or episode identity. A content digest detects substitution;
root/producer installation establishes custody. Hashing alone does not establish
authorship, read-only capability realization or independence.

The writer anchors absolute root/database identity, refuses unsafe links and
wrong marker/configuration, and holds one exclusive writer fence. Readers use
read-only SQLite access, check the same root/database identity at each read,
validate bounded records and hash every referenced BLOB. Use ordinary supported
WAL semantics, not SQLite immutable mode against a live WAL. SQLite transactions
use the selected full durability policy. A replacement writer cannot reuse an
already launch-intended attempt; root/epoch change is not evidence of non-entry.

Proposed initial limits are 1 MiB request/result/observation each, 4 MiB per
captured stream/artifact, 16 MiB total terminal custody per attempt, 64 MiB total
root payload and 5 seconds maximum database busy wait. Bound reads before
allocation and account for encoded envelopes. Reserve terminal metadata capacity
before launch. Stream overflow stops capture/requests cleanup and records the
exact bounded failure and uncertainty; it cannot create a truncated successful
artifact or make an entered operation retryable. These are a deliberately
bounded controlled profile, not a production workload capacity claim. Freeze
them in E0 and reconcile CE's 1 MiB command/4 MiB readback limits at the interface.

## Producer behavior, entry evidence and crash cuts

The producer owns a real Tokio child handle and bounded stdin/stdout/stderr
capture. It records exact input bytes, launch identity, local exit observation
and the correlation between returned result and installed request/session. The
controlled child is a test peer at the external execution boundary; the producer,
store, reader and recovery path are permanent code. It is not a dummy service
standing in for the future host. The controlled peer's response can prove its
protocol behavior, not remote provider history or an independently realized
read-only grant. Facts that cannot be established remain unavailable.

| Evidence/cut | Permitted conclusion |
| --- | --- |
| Checked rejection before launch-intent; exact durable refusal from the validated startup owner under the original writer | Definite local pre-entry for this attempt; no child launch |
| Platform spawn API positively reports no executable child started; producer retains exact error/outcome and commits that observation | Definite local pre-entry, limited to the qualified platform/spawn route |
| Launch-intent durable, then process death before a trustworthy spawn-failure record | Entry uncertain, even when the harness knows it killed before spawn |
| Child exists, pipe setup or input write fails, timeout/cancel occurs, output is malformed, or process exits | Exact local fact only; no inference that remote entry did not occur |
| Result and all required custody artifacts commit, acknowledgement is lost | Reconcile the exact manifest/reference as committed; never rerun the child |
| Result observed but terminal custody did not commit | Unresolved capture; no reconstructed success from the original request and no replay |
| Exact trustworthy lookup finds no attempt record | Record absent, not a provider non-entry proof or new admission |
| Corrupt record/root/artifact, wrong digest, inaccessible store | Conflicting or unresolved, never absence |

The launch-intent commit is causally required before spawn: otherwise a crash
could leave a real child with no durable operation identity. Persisting it does
not prove entry occurred. Separate the direct `spawn()` failure from every
subsequent setup error; do not classify by a generic error string/code that might
also arise after a child exists. The E1 supported platform must qualify the
specific definite-no-child cases. If that cannot be shown, classify them as
uncertain. No GO protocol or undocumented authentication-message rule creates a
stronger conclusion. The initial controlled peer needs no autonomous retry or
reconnect mechanism.

Append/reconcile operations return exact `Applied`, `Replayed`, `Conflicting`,
`Absent` or `Unresolved`; write acknowledgement uncertainty retains the original
root/attempt/stage/request/record locator. Keep record-commit uncertainty distinct
from process-entry uncertainty. Recovery reads the original attempt and immutable
facts, never launches, resends input, chooses another session or clears a slot.
Kill/reap is cleanup, not settlement. Future HP4 provider reconciliation may add
new attributable records under its accepted profile without rewriting old facts.

Definite local pre-entry evidence is consumable data. Campaign remains the owner
of retry authorization and exact next dispatch admission. E1 issues no campaign
retry permit, and J1 recovery never re-enters the provider. A later real provider
profile must prove all routes by which that provider could have been entered
before upgrading a local statement to provider non-entry.

## Files, tests and exit

E1 owns only the new crate: `src/{lib,contract,producer,process,store,reader}.rs`,
`migrations/0001_evidence.sql`, package-local schema/encoding fixtures,
`tests/{controlled_process,custody_readback,crash_recovery,api_boundary}.rs` and
its bounded Rust test peer/support. Module factoring may change without moving
semantic ownership. The test peer is not an installed service or replacement
runtime. No campaign application/store edits, CE3 behavior changes, episode
writer changes, lifecycle changes or UI changes belong to E1. The integration
owner alone edits `rust/Cargo.toml`, `rust/Cargo.lock` and coordinates J1's package
dependency. The accepted interface is an immutable handoff before those parallel
lanes begin; later interface changes return to that owner.

The independent E1 exit requires:

1. Actual process proof through the real producer: exact controlled request,
   result/session/transport/artifact custody, reader reopen in a fresh process,
   wrong request/session/attempt/root/profile rejection, and no caller-status
   bypass. Direct fixture inserts cannot qualify this gate.
2. Actual-process fault cuts before/after launch-intent, spawn outcome, result
   capture, terminal transaction and postcommit delivery. Restart distinguishes
   durable refusal, uncertainty and committed custody; an independently counted
   controlled invocation never increases during recovery.
3. Tampered BLOB/manifest/hash/marker/database, duplicate changed attempt, reader
   write attempt, competing writer, unsupported version, bounded output and
   failed cleanup cases. The reader returns no checked success for partial or
   inaccessible custody. Record absence is tested separately from observed absence.
4. Compile-fail boundaries for launch ticket/checked result construction and
   deserialization and read-only handle mutation; runtime refusal of production-
   profile use of controlled test-support records. Focused debug/release regressions, formatting
   and Clippy; retain exact source/toolchain/lock/features/executable/configuration
   identities and raw logs. No repository-wide benchmark is needed for this slice.

Builds and disposable roots belong under `/home/bline/code/.work-engine-tmp` when
implementation is separately authorized. These are planned gates, not tests run
by this planning task.

After E1 and J1 land, the integration owner runs the real-store composition with
the permanent reader and controlled actual-process records, exact CE observation
and both distinct claim readbacks, actual ER1 episode history, and downstream
recovery without another invocation. This does not claim J2 finding/consumption
completion. The full native host and then HP4 must additionally establish actual
dispatch/startup provenance, provider identity and transport/session continuity,
read-only realization evidence, provider failure/reconciliation and accepted
retry behavior. Neither an E1 unit pass nor J1 fixture pass substitutes for that
later joined gate.

## Decisions and retained dependencies

Accepting an implementation proposal must settle the narrow crate placement and
E0 ownership, the closed controlled profile, new private root/record codec and
limits, exact platform spawn-failure proof, and the startup installation boundary.
The recommended choices above make E1 independently implementable without a
campaign/store edit. If the user instead requires real provider attestation as
the first exit, it is not this independent slice: actual campaign dispatch,
selected adapter and permanent startup join become prerequisites, and parallel
completion with unfinished J1 cannot honestly be claimed.

Remaining runtime work includes the full workflow/binary, real provider adapter,
credentials and isolation/capability observation, retained sessions/remediation,
correction/succession, actual dispatch, executable generation management, whole-root
migration, performance qualification and operational cutover. Lifecycle owns
context/admitted effect accounting and never receives a false settlement from
local child exit. Review custody owns these review attempt facts; it does not
replace lifecycle's operation ledger or reuse its permits as review authority.

## Evidence and limitations

Tier 2 Verify used graph project `home-bline-code-work-engine`, generation
`2026-10-08T15:01:27Z`. Exact-path coverage at `2026-10-08T15:05:29Z` reported
metadata match/no recorded gap for every relied-on path; this is best-effort,
not a completeness proof. Bounded owner/contract symbol searches were exhausted.
An initial broad lifecycle search returned 30 of 55 rows and was narrowed to
the exact discovered mechanical files; no exhaustive lifecycle/retirement claim
uses that unfinished broad search. The native owner-factory bidirectional trace
was untruncated (16 callees, no reported callers), but injected callers are
underrepresented; exact source, not that absence, grounds placement.

Evidence hashes, graph receipts, baseline comparisons and limitations are retained
under `/home/bline/.local/state/work-engine/rust-j1-hp3-planning-20261008/hp3/`.
All inspected implementation sources match the stated baseline. Required working-
tree `AGENTS.md` and `docs/rust-development.md` differ and are separately hashed
as current instruction context, not misrepresented as baseline bytes.
This lane wrote only this plan and its evidence files. No product edits, builds,
tests, provider execution, commits or operational changes were performed.
