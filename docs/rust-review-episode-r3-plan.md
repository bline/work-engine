# Review-episode R3: native host integration

Status: proposed bounded implementation plan, 2026-10-07. Current authority is
**planning only**. No implementation, test/build execution, provider invocation,
live database access/import, activation, default cutover or commit is authorized
by this document. It refines the [episode lane plan](rust-review-episode-implementation-plan.md)
and preserves the selected [R0 contract](rust-review-episode-r0-contract.md).
The implementation baseline is joined C2/R2 commit
`bcbc52eb071270dc56d4dc65489b26b269506604`, parent S1
`e7c1d88ec0df20ea6213239b6dc2b36abd9f1872`, on
`rescue-detached-20260911`. The historical R2 result's pending-review wording
predates the [completed integration evidence](rust-c2-r2-integration.md).

Recommend one explicitly configured `gpt-6-sol`/`high` builder for implementation,
integration and remediation after acceptance, and a separate initially isolated
`gpt-6-astra`/`xhigh` reviewer. The designer cannot supply an independent review
of this plan by changing labels. Record requested and observed model identity;
unavailable attestation stays unavailable. This recommendation grants no launch
authority and makes no cross-provider independence claim.

## Outcome and acceptance boundary

An explicitly selected native host uses the actual Rust executable, reducer and
SQLite store for every episode operation. JavaScript retains campaign selection,
reviewer/provider execution, implementation-review admission, finding publication,
claim establishment and campaign acceptance. The bridge contains framing,
compatibility and admission composition, never another episode reducer.

R3 proves the host-bound path on new disposable roots through real claims and
campaign services with a controlled reviewer peer. It preserves synchronous
episode calls, typed failure distinctions, exact references, restart/readback and
writer fencing. R2 fixture grants remain an offline test profile; accepting R3
does not promote them to production authority.

There are two distinguishable exits:

* **R3 bounded integration:** all seven episode operations route through the
  host-bound bridge and every transition action remains Rust-owned; normal
  initial review, exact recovery, finding publication and campaign consumption
  are exercised. Production claim correction and changed-result routes that
  lack accepted RC support are explicitly unavailable before provider entry or
  dependent publication. Their rejection is tested. Positive production
  `succeed_evidence` qualification is deferred to RC; offline/action routing tests
  do not supply it. This is a useful integration milestone, not full native-route
  replacement or R4 eligibility.
* **R3 plus accepted RC join:** affected correction/remediation and question
  propagation routes pass the exact three-owner matrix below after claims and
  campaign owners accept the bounded RC delta. Only this exit can support full
  operation/route qualification for R4. R5 still owns operational cutover.

The user/owning coordinator chooses the first bounded profile or explicitly adds
the RC work. Frozen R0 decisions define the target semantics; they did not grant
R2 or this planner authority to change the claims/campaign owners.

## Observed implementation and consequences

| Source at the joined baseline | Consequence |
| --- | --- |
| [`createNativeReviewHostOwners`](../app-server/src/services/slice-campaign/native-review-host.mjs), 238–276 | Currently opens JS `review-episodes.sqlite3`, composes the JS reducer and actual claims owner. This is the selection seam; opening both writers is unnecessary. |
| [`createNativeReviewHost`](../app-server/src/services/slice-campaign/native-review-host.mjs), 278–500 | Derives authority from campaign selection, pins generation-1 reviewer session, reconstructs initial subject, and routes provider-result recovery. Selected source revision and stored episode authority must remain distinguishable after selection succession. |
| [`createNativeReviewClosureService`](../app-server/src/services/slice-campaign/native-review-closure.mjs), 98–382 | Uses synchronous reads/transitions across asynchronous reviewer calls. Only initial review currently calls `admitProductionPath`; correction, recovered correction and remediation omit it. Its `status()` can report a zero-finding episode despite retained questions. |
| [`succeedProductionPathEvidence`](../app-server/src/services/slice-campaign/native-review-closure.mjs), 147–194; [`succeedReviewSelection`](../app-server/src/services/slice-campaign/service.mjs), 327–371 | Closure requires the current blocked predecessor before entering claims correction. Episode-only commit recovery cannot simply repeat this guard. Campaign publication follows the other owners and has separate CAS. |
| [`production-path-service`](../app-server/src/services/claim-evidence/production-path-service.mjs), 37–116; [claims store](../app-server/src/services/claim-evidence/sqlite-store.mjs), 363–408 | Claims correction invokes episode succession before its own transaction. `readProductionPathSuccession` already accepts a succession ID or operation ID. Exact predecessor establishment refs are preferable to the current first-page search by claim revision. |
| [`HostAdmissionPort` and `FixtureAdmission`](../rust/crates/review-episode/src/admission.rs); [`main`](../rust/crates/review-episode/src/main.rs), 22–132 | R2 has only an offline registry admission port and rejects production mode. The registry binds exact request/selection/observed revision but is synthetic. A separate host implementation is required. |
| [`Application`](../rust/crates/review-episode/src/lib.rs), 88–354; [protocol](../rust/crates/review-episode/src/protocol.rs), 27–153 | Seven operations exist. Wire `recover` takes transition identity/content digest, whereas legacy `recover(identity)` returns current state. Replies are currently hard-coded `offline_fixture`; production correlation must cover errors too. |
| [`EpisodeStore`](../rust/crates/review-episode-store/src/sqlite.rs), 14–194 | R2 uses `review-episodes.sqlite` beneath an offline-marked private root, validates history and rechecks the admitted observation under `BEGIN IMMEDIATE`. Native integration needs a distinct opening profile, not a renamed offline marker or a live legacy import. |
| [`begin`/`transition`](../rust/crates/review-episode-core/src/reducer.rs), 35–87, 140–214 | Matching replay is persisted in handled transitions, returns latest state, and authorizes the current writer before disclosure. This deliberately differs from the old JS shortcut. |
| [Capability host](../app-server/src/services/slice-campaign/capability-host-runtime.mjs), 30–82; [generation bootstrap](../app-server/src/executable-generation-bootstrap.mjs), 18–83; [snapshot](../app-server/src/executable-generation-snapshot.mjs), 114–210 | Native owners are injected into the real campaign host. Executable snapshots use closed file inventories. New imports and binaries need an exact generation-owner handoff, not discovery via `PATH` or the mutable checkout. |

## Proposed file ownership

Only this plan is written during planning. The following is the concrete future
manifest, subject to acceptance. Concurrent C3, S2/S3, UI and unrelated dirty
source/index changes remain owned by their existing actors.

| Owner | Paths and bounded change |
| --- | --- |
| R3 JS compatibility | New `app-server/src/services/review-episode/rust-adapter.mjs`: synchronous facade, host-scoped admission preparation, framing, response/error validation and reconciliation. |
| R3 native composition | `app-server/src/services/slice-campaign/native-review-host.mjs`, `native-review-closure.mjs`; `app-server/src/index.mjs`: explicit factory selection, private admission-context composition, bounded unavailable-route checks, exports. No provider algorithm or campaign terminal reducer migration. |
| R3 Rust admission/application | `rust/crates/review-episode/src/{admission,lib,main,protocol}.rs`; new `src/host_admission.rs`: native launch profile, complete binding validation and profile-specific replies. Existing offline protocol/profile remains explicitly offline. |
| R3 store profile completion | `rust/crates/review-episode-store/src/{lib,sqlite}.rs`: typed host-root opening and new-root initialization while preserving the two-table schema, integrity checks and transaction semantics. No copy importer changes or live-root discovery. |
| R3 verification | New `app-server/tests/services/review-episode/rust-adapter.test.mjs`, `app-server/tests/review-episode-rust-vertical.test.mjs`, `app-server/tests/fixtures/review-episode-rust/native-host-driver.mjs`; new `rust/crates/review-episode/tests/{native_host,host_recovery}.rs`; extend `tests/support/mod.rs` and `tests/command.rs` there. Rust drives process/fault/timing tests; Node code only exercises the real JS consumer and the transitional adapter. |
| R3 regression/receipt | Bounded additions to `app-server/tests/slice-campaign-native-review-vertical.test.mjs` and `app-server/tests/supervisor-native-review-hosting-integration.test.mjs`; new `docs/rust-review-episode-r3-result.md`. Existing R0 fixtures remain frozen. |
| RC, only after separate owner acceptance | Closure portions above, `app-server/src/services/slice-campaign/service.mjs`, `app-server/src/services/claim-evidence/production-path-service.mjs`, and only necessary read APIs in `claim-evidence/sqlite-store.mjs`; focused cases in existing native vertical tests. Any new durable schema or core semantic change needs an explicit manifest extension. |
| Shared/generation owners | `rust/Cargo.toml`, `rust/Cargo.lock`; `app-server/src/executable-generation-bootstrap.mjs`, its snapshot/worker tests, and selected launch configuration. R3 submits exact requests; these are not concurrent R3 edits. C3 owns `runtime-manifest.mjs`, its Rust compiler modules and proposed manifest facade. |

The proposed implementation uses existing packages and dependencies. It does
not need lifecycle runtime, a generic process framework or new public client
crates. New process/fault test infrastructure is Rust-first. Existing JavaScript
compatibility glue is the already accepted transitional exception. If the
capability-host factory needs one additive configuration parameter, submit that
specific edit to its owner; tests can initially select R3 through the existing
`nativeReviewOwnersFactory` injection without changing its public contract.

## Host admission and access contract

Use a separate `native-host-v1` admission profile, carried over a version-2 host
envelope. Keep R2 version-1/offline behavior unchanged. A trusted factory selects
the binary, native root, launch profile and host principal outside every tool or
reviewer request. It supplies a private admission descriptor through an inherited
read-only anonymous/unlinked descriptor on each invocation; stdin carries the
ordinary request. Descriptor presence is a transport capability within the
accepted host boundary, not cryptographic authentication of any same-UID process.
There is no `--fixture` route in this profile, no caller-selected root/path/port,
no JSON `authorized` shortcut and no default production activation.

The factory constructs the descriptor from actual owners, not from untrusted
references forwarded by a caller. Rust validates the descriptor against the
parsed request and selected root before a reply can disclose state. Its private
permit records the checked binding; neither it nor an admitted result is a
deserializable public DTO. The binding includes:

* protocol/profile, transport request ID and canonical request digest, operation
  and host-principal identity/access scope;
* configured canonical root identity and persistent root-selection digest,
  selected executable digest/protocol and host launch identity;
* six-part episode identity, current campaign selection revision and durable
  authority source/manifest digest, writer actor/provider/generation/session;
* requested expected revision, separately observed current revision or absence,
  exact selected subject, transition ID and content digest, and result digest;
* required claim identities/revisions, establishment/observation/consumption
  references and digests, status, subject, covered state, consumer and boundary,
  including observation attempt/result binding where present.

There are two observation policies, selected by trusted code. Mutation admission
is bound to an exact current revision/absence obtained through an independently
authorized read; the write transaction rechecks it. Reader admission authorizes
one consistent snapshot for the exact requested identity/revision scope, without
requiring that reader to be the current writer. A reader's snapshot revision is
returned as an observation, not supplied as a writer grant. Unauthorized reads
and replays disclose neither state nor existence-sensitive diagnostics.
The native port performs a request/root/principal precheck before the application
loads an episode snapshot, then checks the exact observed-revision binding after
the read. R2's current post-read `admit` call alone is insufficient for that
precheck; add the bounded port method rather than exposing integrity/not-found
details before authorization. Offline tests retain their separate profile.

Avoid ambient mutable "current authority" across reviewer awaits. Native closure
uses an explicit invocation-scoped episode facade made by a private host factory.
The facade has the old seven method shapes. Each call resolves the current
campaign through a once-bound owner callback, verifies its scope, and creates its
own descriptor. A factory not yet bound to its campaign owner refuses commands.
The existing native host constructor can bind that callback; callers cannot
install it through capability input. Historical initial-subject reads use a
separate host reader scope. Selection succession does not silently replace the
stored episode authority source or reset reviewer generation to 1; accepted
succession lineage or an explicit successor writer grant supplies the relation.

For result admission, the trusted host runs implementation-review v1 validation
and reads exact claims-owner establishments and observations. Claim revisions
come from the accepted campaign selection. Recompute each reference digest with
its owning codec; compare claim subject, episode, result, consumption boundary
and consumer. Do not infer applicability from `established` alone. A missing
observation can support an accurately bound unestablished record, never an
established admission. False/unestablished statuses may be recorded and block
dependent consumption. Forged or mismatched refs are rejected. The host asks the
claims owner to establish claims; Rust never evaluates independence or writes
the claims database. `validateResult` remains observational and grants no later
write. Prepared correction successors use the distinct RC path below and cannot
be passed as ordinary committed claims evidence.

The proposed security profile trusts installed host/provider executables and
the host process; it excludes hostile arbitrary native code under the same UID.
The protected untrusted actor is the reviewer/model and its actually granted
tools. The current [native reviewer adapter](../app-server/src/services/reviewer-runtime/native-claude-code-adapter.mjs),
11–22 and 572–578, restricts tools to read/graph capabilities and pins a strict
MCP configuration. Its 152–162 launcher passes only ordinary stdio. Those source
facts are starting evidence, not an OS sandbox claim. R3 qualification records
the actual controlled launch's tool/MCP set, descriptor inheritance, environment,
cwd and filesystem capabilities; the episode descriptor never reaches the
reviewer child or prompts. Negative capability probes must show that this grant
cannot write the DB/sidecars/root selector or invoke the privileged bridge.
Private 0700/0600 paths additionally exclude other Unix users. A broad filesystem
Read grant is not secrecy for on-disk credentials, so no reusable admission secret
is placed in reviewer-readable files or environment. If the owning consumer
requires protection against a compromised same-UID provider executable, this
profile is insufficient: accept a separate OS-principal/sandbox launch contract
before claiming that stronger qualification. No live provider is needed for the
controlled capability/launch tests, and they do not attest a future live grant.

## Synchronous facade, errors and durable recovery

| Existing call | Actual Rust route and compatibility |
| --- | --- |
| `begin({authority, transitionId, unresolvedQuestions = []})` | `begin`; serialize the default explicitly; return frozen `state`. Current-writer matching replay returns latest state without append. |
| `resumeInitial({authority})` | `resumeInitial`; return frozen active initial state only. It does not authorize provider retry. |
| `transition({...})` | `transition` for `record_result`, `record_remediation_subject`, `succeed_evidence`, `replace_writer`, `mark_uncertain`, `retire`; return state. No JS phase calculation. |
| `validateResult({...})` | `validateResult`; return validated result, with later writes independently admitted. |
| `recover(identity)` | `read` with `revision: null`; retain the legacy current-state/null return shape. Do not send this shape to R2's differently shaped wire `recover`. |
| `read({identity, revision = null})` | `read`; exact-revision absence returns null only after successful authorized integrity-checked observation. |
| `history({identity})` | `history`; complete ordered frozen array or explicit size failure, never a partial successful array. |

An internal `reconcileTransition` uses Rust `recover` with exact identity,
transition ID and content digest, returning the first committed revision and
latest state. Its extra recovery metadata does not change legacy returns.
All episode-bearing JSON retains JS UTF-16 semantics, missing/null distinctions,
safe numbers and the domain codec; no Serde round-trip or rehashing old rows.

Responses and post-parse errors bind profile/source, request ID, request digest,
root-selection identity and operation; the adapter checks these before accepting
a domain result. Preparse/unbound errors cannot masquerade as a matched domain
response. Check one length-prefixed frame and EOF, version, exact response shape,
UTF-8 validity and byte limits. Use direct binary launch with no shell and bounded
stdout/stderr; diagnostics are redacted. Preserve the actual default/fault binary
identities. Public source labels distinguish `native_host` and `offline_fixture`
without changing semantic state hashes.

Map only a verified Rust `ResultContract` error to the existing
`ReviewEpisodeResultError`, preserving the `instanceof` checks in closure/host
result recovery. `Authority`, `RevisionConflict`, `InvalidCommand`,
`StateIntegrity`, `Path`, `Schema`, `Busy`, `ResponseTooLarge`, framing, I/O and
`OutcomeUnknown` remain classified infrastructure/domain errors. A transport
failure is never turned into a reviewer contract correction or a finding.

Known launch failure before a child starts is distinct from a timeout, signal,
overflow, malformed/wrong-correlated/partial reply or child exit after possible
commit. The latter returns unresolved operation identity and invokes/permits a
new authorized reconciliation process. Matching durable content establishes its
first commit revision, not the original reply. Different content fails. Absence
in a successfully validated snapshot permits fresh admission/CAS, not an inference
about provider non-entry. Unavailable/corrupt/unauthorized readback stays unknown.
Retry budgets are bounded; there is no hidden retry loop or JS writer fallback.

Test actual host-process loss as well as child loss. A new host reconstructs the
admission scope from durable campaign/claims/episode records and exact authorized
operation input; it does not need an in-memory grant registry. When the episode
result committed but finding/campaign publication did not, reuse that exact result
and idempotent downstream operation IDs without entering the peer again. When no
durable result exists, the provider adapter's separately owned receipts/session
recovery determine what is knowable; episode absence never authorizes re-entry.

There are three distinct fences. R1 persists reviewer writer generation and
rejects old-writer replay; replacement is a separately granted next generation,
not a transport retry. Store CAS resolves competing admitted Rust commands.
Finally a persistent native-root selection binds this disposable root to Rust,
codec/schema/profile and the selected binary identity. Its host factory refuses
legacy/offline/other-build selection on restart. The marker is an administrative
fence within the trusted launcher, not authorization against arbitrary same-UID
filesystem writes. Old commands can be independently read by an admitted reader
without reviving their writer. R3 does not implement executable deployment
handoff: changing the root's implementation/build binding, adopting a legacy
root, or running old/new generations together belongs to the generation/R5 owner.
An unchanged selected build may restart; no active-root rollback silently opens
the old JS database writer.

## Minimum RC decision and correction sequence

R0 already selects stricter replay, question preservation, exact new-result claim
scope and bounded reconciliation. The following is a proposed concrete owner
delta, not a claim that RC was accepted or implemented.

1. Claims owner exposes exact establishment/observation/succession reads and a
   prepared-correction value constructed from its existing records and authorized
   request. Bind exact predecessor establishment IDs from the blocked episode,
   not an arbitrary matching record in the first 1,000-row page. Preparation is
   explicitly not durable establishment or consumption authority.
2. Bind the complete correction input digest into the new episode succession
   command payload alongside predecessor/successor admissions: operation ID,
   correction authority, campaign predecessor revision, both selection revisions,
   candidate/result, episode predecessor, both claim revisions and establishment
   IDs, observation identity/digest and successor claims. R1 already hashes the
   whole action/payload. This additive payload binding makes changed requests
   conflict at the persisted transition without a new reducer or generic journal.
   Existing historical commands/hashes are untouched. Accept the exact field and
   codec with the episode, claims and campaign owners before implementation.
3. Claims correction and closure reconciliation read the historical predecessor
   and the committed transition by operation ID, rather than requiring that the
   latest episode remain in the blocked predecessor phase. Read claims succession
   by operation ID using the existing API. Compare all lineage, including the
   episode's first successor revision; latest-state replay alone is insufficient.
4. Campaign publishes the exact successor selection/binding only if its original
   expected revision and selected claims still match, or returns its already
   committed exact operation. Builder projection and terminal consumption require
   readback of the exact episode/claims/campaign join and current result binding.
   Episode `reported` alone does not release the gate.

The minimum route uses explicit authorized retry carrying the same correction
input, plus existing durable records. Missing retry input remains unresolved.
If autonomous recovery without those inputs is required, the claims/campaign
owner must separately accept a durable prepared-operation record; R3 does not
invent one implicitly. One database cannot certify that all three committed.

| Durable position after interruption | Authorized continuation and protected boundary |
| --- | --- |
| No episode successor | Revalidate exact preparation and current CAS; submit one successor. Claims preparation alone enables no consumption. |
| Episode successor only | Match the handled digest and historical predecessor/result to exact prepared inputs; finish claims correction. Keep builder/campaign consumption closed. |
| Episode plus claims succession | Verify both exact successor establishments/observation and episode link; publish campaign successor only at matching campaign CAS/selection. |
| All three | Return the existing exact operation and builder projection after full binding readback; no extra history or establishment and no provider entry. |
| Conflicting operation reuse, changed selection/CAS, mismatched result/observation, replaced unauthorized writer or missing owner data | Preserve committed truth and report unresolved/conflict to the owning route. No rollback, automatic successor selection, new provider review or narrowed claim. |

For ordinary result correction and remediation, RC must propagate required claims
and selection through `executeCorrection`, `recoverCorrection` and
`executeRemediation`, obtain new claims-owned establishment for the exact new
result, and verify current candidate/episode applicability before projection or
terminalization. Changed candidate claims need their owner's accepted successor
selection; the bridge cannot rewrite claim subjects. Retain old admissions as
history and reject their use for result B. Update closure status/projection and
campaign consumption so retained unresolved questions do not become reported
merely because there are zero findings. Rust's R0 target alone cannot fix those
JS consumer decisions.

Until RC is accepted, the bounded R3 profile refuses required-claim result
correction/remediation and production-path succession at the host entry point
before new provider entry. Direct episode operation support is still tested
under separately authorized controlled grants, including blocked/false evidence,
replacement, uncertainty and retirement. Successful isolated `succeed_evidence`
is never labeled successful production correction. Ordinary v1 routes may be
qualified only for an explicitly selected v1 campaign; no v2 selection is
downgraded or routed through a fixture grant.

## First real vertical and test matrix

The first implementation increment opens a new host-profile root and composes
`createNativeReviewHostOwners` → actual native host/campaign method → synchronous
Rust adapter → `work-engine-review-episode` → real SQLite → JS finding/claims and
campaign owners. Use the real implementation-review validator and host-derived
authority. Substitute only the external reviewer peer and explicit offline
test inputs/credentials/subject fixtures. The peer produces controlled receipts
and counts entries; all artifacts label that evidence synthetic. Do not replace
the episode factory with an in-memory reducer or fake the claims/campaign store.

Drive begin/result, publish an exact finding or zero-finding result, close every
owner, start a new host OS process, and recover exact state/history/downstream
references. Include an initial v2 case using actual claim establishment from
controlled observation, and false/unestablished variants. This proves real owner
composition under controlled evidence, not live reviewer independence.

| Matrix group | Observable acceptance evidence |
| --- | --- |
| Complete facade | All seven calls and six action routes target the selected real binary; positively qualify admitted native operations and explicitly reject RC-dependent native admission until accepted. Preserve R2 offline success coverage separately. Verify exact current/revision/history semantics, UTF-16 IDs, null/default handling and frozen returns; unsupported/oversize history is explicit. |
| Admission/access | Mutate every binding dimension; forge status/ref/grant, wrong root/profile/principal/selection/build, offline grant in native mode, unauthorized read/replay, malformed extra frames. No unauthorized disclosure or write; no capabilities inherited by the reviewer. |
| Error projection | Invalid reviewer result becomes the existing result-contract correction path. Busy/integrity/path/framing/timeout/unknown failures do not become reviewer judgments. Swapped/uncorrelated errors are transport failures. |
| Replay/fencing | Current-writer exact replay after later transitions appends nothing; conflicting ID fails; replaced writer replay fails; independent reader can reconcile. Two processes race same/different commands and replacement with exact durable outcomes. |
| Actual process cuts | Barrier-observed child kill before commit, after commit before reply, partial reply; host kill after durable result before findings/campaign publication. New host and new child recover exactly one committed transition and preserve entry count. No sleeps as proof of a cut. |
| Downstream failures | Real finding publication failure, stale campaign CAS, blocked/false/missing evidence and immutable reference mismatch. Retry uses persisted result; no provider replay or inferred acceptance. |
| RC conditional matrix | Each position above after actual host restart; same/different correction request IDs; latest episode beyond first successor; pending questions; result B with A's admissions; changed candidate; exact claim/campaign readback. Run only under accepted RC behavior, otherwise verify explicit unavailable-route fencing. |
| Build/root selection | Missing/drifted binary, unsupported protocol, default fault controls, wrong native/offline marker, attempted legacy factory on Rust root, restart with another selected build. Fail explicitly without opening a fallback writer. |
| Combined HOST | Actual Node host in an OS child with an external monotonic probe, C3/R3 separately and together, cold/warm success, sustained bursts, maximum supported payload/history, SQLite contention, errors, timeout and restart. Record timer/control-route latency and whole-request cost. |

Reuse R2's feature-gated barriers and process driver where appropriate. A fault
build is a separate artifact from the default release executable. No provider,
live DB copy, default cutover or fault control in the production launch profile
is part of these tests. Process-kill results do not establish arbitrary-media
power-loss guarantees.

## Shared HOST responsiveness and executable handoff

C3 retains synchronous manifest projection/requirements and asynchronous
hydration. R3 retains synchronous episode APIs. Their blocking time composes,
including claims lookups, initial subject reads and error reconciliation; two
separate green microbenchmarks cannot qualify the joined Node host.

Use one parent/host-consumer-owned frozen HOST profile, referenced by both plans.
Before measurements, bind hardware/OS/Node/toolchain, exact two binaries and
launch profiles, realistic and maximum supported workloads, concurrency, sample
count/warmup, timer/control route and acceptance thresholds. Proposed engineering
targets are external control-route delay **p99 ≤ 50 ms, maximum ≤ 250 ms**;
these are neither measured nor accepted product requirements yet. Proposed R3
per-child ceiling is 100 ms, SQLite busy ceiling 50 ms, 8 MiB input/32 MiB output
maximums inherited as compatibility ceilings and 64 KiB diagnostic ceiling.
The host profile may tighten payload/history support with explicit refusal.
Two calls for snapshot admission plus write, and later reconciliation, count
toward the host measurement. A timeout is a resource ceiling, not a latency proof.

Measure legacy/legacy, C3-only, R3-only and both bridges on the same frozen
workload. Report command p50/p95/p99/max, cold launch, claims lookup, history scan,
full host control-route delay, timeouts and memory/output bounds. If baseline or
either/combined Rust path misses the accepted target, report it; do not silently
raise the target or omit worst cases. Joined synchronous acceptance remains unmet.
The host consumer then chooses a revised supported workload/budget or separately
accepted process/async/addon interface. An async change includes claims'
synchronous callback and campaign callers; returning a Promise from this facade
is not a compatible repair.

C3 supplies immutable compiler digest, protocol version and launch-profile
identity outside its semantic manifest hashes. R3 supplies the analogous review
binary, host-envelope/profile, codec and root-selection contract. The generation
owner receives the complete transitive JS module inventory plus both executable
artifacts/runtime dependencies, verifies byte/mode identities and seals the
snapshot. Test startup from that closed inventory without the mutable source
checkout or `PATH` fallback. Missing admission/adapter modules or binary drift
fail before work. Generation owner serializes bootstrap/snapshot edits; R3 owns
the native host edit and applies any C3 host call-site request after agreed
handoff. Neither plan edits the other's facade or shared root concurrently.

## Validation, rollback and remaining decisions

After implementation authorization, the retained builder runs focused Rust
application/store/default and fault tests, the Rust-driven host process matrix,
the two new Node compatibility/vertical tests, and existing native-host/vertical
regressions. The integration owner freezes the joined C3/R3 candidate before
workspace tests, formatting, all-target/all-feature Clippy, locked release,
dependency/security/license checks and the combined HOST profile. Source/BOM,
binary hashes, feature flags, cut counts and unavailable measurements belong in
the R3 result. Independent review targets admission authenticity, persisted replay,
claim applicability, correction cuts and actual consumer failures.

The starting joined C2/R2 evidence is 122 Rust tests, 32 fault tests plus one
intentional helper omission, and 40 clean-joined Node tests. The dirty shared
checkout's 11 legacy Node failures were separately reproduced as pre-existing
`skills/slice-builder` source/pin drift. Preserve that source and its pins; run
clean exact-candidate validation and report any dirty-main result separately.
Do not "fix" unrelated files to obtain an R3 green gate. S2/S3 started as isolated
S1 lanes; board 1246 subsequently advances S2 onto the joined baseline and reserves
its private pinned-Tokio integration. Coordinate any future root/lock registration
with that owner and its handoff. R3 requests no Tokio dependency. S2/S3 are
coordination peers, not R3 dependencies; lifecycle S4–S10 and UI replacement are
not prerequisites.

Rollback during R3 disables the opt-in factory and preserves disposable Rust
roots for readback; it does not reopen them with JS. Existing unselected roots
retain their existing owner. On possible commit, stop dependent consumption and
reconcile rather than restore an earlier file or erase history. R4 must qualify
selected real copies and cross-reader/writer compatibility; R5 must authorize
root inventory, drain/fence, import/cutover and rollback after new writes. No live
root is opened or imported by this plan or its bounded implementation profile.

Acceptance still needs three concrete owner decisions: (1) native admission and
launcher threat scope, including whether the current tool-grant boundary is
sufficient; (2) the one frozen HOST budget/profile; (3) bounded R3 exit with
affected routes closed, or an explicitly owned RC extension implementing the
minimum correction and changed-result consumption behavior above. Exact
generation inventory registration is a serialized integration handoff. None
requires waiting for the complete lifecycle migration.

## Planning evidence limits

Read the required DESIGN, PHILOSOPHY, Rust direction, model guidance, reviewed
episode lane plan, R0, R2 plan/result and integration record. Wind Walker startup
and the shared board were reconciled; board 1245 and the parent handoff establish
the joined commit and parallel planning authority. No operational role schedule
or active campaign is assumed from this planning assignment.

Retrieval tier was Verify, project `home-bline-code-work-engine`, ready index;
the exact-path coverage receipt used generation `2026-10-07T19:33:11Z`. Scoped
symbol searches for native/episode/claims/generation owners and relevant depth-1
traces in both directions were consumed without remaining relevant pages. Broad
discovery searches were narrowed, not used for exhaustive claims. Graph traces
omit injected callbacks and some Rust method edges, so the material factory,
callback, admission, store, replay and recovery claims were verified directly in
source. Every source/document relied on above received exact-path coverage with
no recorded issue and metadata match; this is best-effort, not completeness.
No test, build, benchmark, provider, database or import was executed for planning.
