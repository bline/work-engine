# RP2: permanent Rust initial-review host join

Status: proposed implementation plan, 2026-10-16. This is a bounded successor to
[the provider execution plan](rust-review-provider-execution-plan.md), RP0/SC3-D
at `af4546c0`, and published RP1 at
`596d2c9f3993587efe2241a8e653caec63badaeb`. It proposes code and offline
qualification; it does not authorize an installed Claude call, production
credential access, spending, publication, activation, or a retained review
entry. The builder's plan and a later review of it are not acceptance by a
campaign or credential owner. RP3 still selects the exact production
model/profile/cost envelope and authorizes one live initial call; SC3-H alone
joins a later, separately Applied retained call.

## Outcome and present seams

RP2 joins one fresh schema-4 campaign, original episode, accepted initial claim
pair, installed qualified execution profile, checked E1 reader, and J1/J2
completion through the existing permanent `work-engine-review-host` binary. A
durable result may be completed after restart from its original capture without
another provider entry. Intent or incomplete capture remains unresolved; a
caller disconnect, status request, process restart, successful owner read, or
serialized locator never creates an entry capability. The host returns the
actual CE/episode/J2 state, including false or unestablished evidence, rather
than labeling every structured provider response a completed review.

Source observations at the published RP1 baseline:

| Owner seam | Consequence for RP2 |
| --- | --- |
| `review-workflow/src/startup.rs::TrustedStartup::load` accepts only `review-host-controlled-initial-v1`, controlled E1 custody, SC0 identity/claim config, and `manifest-only-unverified`; `campaign_config` calls `TrustedConfig::controlled_sc0`. | Add a separately tagged, pinned qualified startup profile. Preserve the existing schema-1 controlled route and its refusals exactly. A request or test fixture must not switch that route into production. |
| `review-workflow/src/initial.rs::execute_initial` consumes only an `Applied` campaign dispatch and checks episode begin before its private `InitialLaunch`; `launch` uses controlled `EvidenceWriter`; `join` opens controlled `EvidenceReader`. | Keep this single ordering and private launch branch. Select a checked provider adapter and reader after the original Applied dispatch; replay and recovery cannot reconstruct that branch. |
| `review-workflow/src/recovery.rs::recover_initial` is observation-only for controlled E1. RP1 `QualifiedWriter::reconcile` finalizes `capture_complete`, replays a checked terminal, or reports unresolved/definite pre-entry failure. | Use the original qualified scope, attempt, dispatch receipt and evidence root on recovery. Never call `execute_initial` from recovery. |
| `review-execution-evidence::claude` has installed `TrustedClaudeProfile`, `QualifiedScope::bind_installed`, checked writer/reader and retained session custody, but only a test-gated offline entry-permit constructor. `CandidateBoundary::prepare` requires a fresh private root and has no reopen API. | Add only the production Applied-to-entry bridge and immutable boundary reopen/readback needed by the host. Do not add a retained-entry or session-evolution production permit. |
| `slice-campaign::TrustedConfig::remediation_v4` already creates a fresh schema-4 root with dynamic gate verification; `InitialJoinSelection` accepts `QualifiedProvider`; `CompletionSelection::new/install_completion_selection` exists. The permanent host never installs the J2 selection. | Add a distinct qualified-initial schema-4 config and Applied capability issuer; install exact CE/J2 selection. No campaign reducer or schema rewrite is proposed. |
| `work-engine-review-host` has one worker and one responsive control loop. Its existing 700-probe test uses a short controlled peer. | Keep the same bounded command/status protocol and one worker; measure the new held-provider workload rather than inferring its latency from the old peer. |

The graph trace of `Workflow::execute_initial` reports one relevant caller,
`Workflow::run`, and exact campaign prepare/authorize/consume calls; exact source
settles the semantics. Generic graph `join`, `clone` and `drop` edges are not
owner evidence. The review support packet records the graph generation,
coverage, source hashes and source-read fallback.

## Accepted authority to bind at installation

The qualified startup is a **new** installed profile, not a mode flag on H1's
wire DTO. Its read-only, bounded startup document pins the permanent host
binary/source/toolchain/lock/features/runtime dependencies; fresh campaign,
episode, claims, E1 and CBM private roots; one campaign identity and obligation;
the immutable candidate tree, baseline diff, gate capture/manifest/receipt,
physical profile and selection; the exact accepted initial `fresh_initial`
builder/campaign claim pair; canonical episode authority; role document and
compiler projection; E1 executable/guard/CBM/launcher/catalog identities;
session UUID, attempt horizon, credential-source *metadata*, and the selected
storage/runtime policy. The accepted profile and model are externally selected
installation inputs at RP3, not values derived from this builder's model or a
request. RP2 can qualify the installer against test-owned pinned identities
without installing or reading a production credential.

The trusted loader checks exact file bytes, type, mode, owner and canonical
paths; the running host's own executable; disjoint owner roots; selected
candidate/claim/episode identities; closed tool and routing catalog; prompt
and request size; the compiler's actual projection and its verification status;
and the applicable accepted startup/profile source. It rejects runtime
requirements whose real compiler owner cannot verify them. It may truthfully
use `manifest-only-unverified` only when the selected role/profile requires no
unavailable source-verification or AEG claim; no string relabels it verified.
No broad arbitrary campaign or provider admission is inferred from a read-only
startup file. RP2 adds a distinct `TrustedConfig::qualified_initial_v4` and
schema-4 profile/marker, leaving `remediation_v4` and schema-3 H1 unchanged.
Its trusted config digest includes the owner-accepted installation reference
and digest as well as the boundary, gate, candidate, selection, claim pair and
writer identity. The campaign store verifies that profile and digest on fresh
open and cold reopen; it cannot adopt a remediation, schema-3, evidence or SC0
fixture root. The profile is still initial-only: `prepare_remediation`,
`authorize_remediation_dispatch`, `SessionEvolutionPermit` and provider
`--resume` are unreachable from RP2's host protocol.

### Applied-to-E1 capability contract

`slice-campaign` owns the production `InstalledProfileAuthority` and the
fresh dispatch issuer. The production acceptance input is an independently
owner-selected **installed acceptance pin**: an exact source reference and
SHA-256 from the protected host deployment registry, separate from startup
JSON, request data, campaign readback and the E1 profile loader. The campaign
API takes a private-field `InstalledAcceptancePin`, obtained only through
`InstalledAcceptancePin::open_production(&HostDeploymentPin)` from that
independently selected registry. `HostDeploymentPin` is another private-field,
non-Serde campaign type opened by `HostDeploymentPin::open_installed_host()`
from the trusted permanent-host deployment bootstrap, outside the qualified
startup document and protocol; the selected deployment location/digest is an
RP3 owner input. Neither type has a test-support constructor. The caller
cannot pass a bare path/digest or fabricate either value by decoding JSON.
RP3's campaign, provider and credential owners must supply that selection
before any installed call; RP2 has no real pin. The trusted host opens the
pinned source as a bounded regular file, verifies canonical path, owner/mode, exact bytes,
accepted decision/reference/revision and digest, and matches its candidate,
claim pair, profile/model/catalog, executable/guard/CBM, credential-source
metadata, resource policy and attempt/session binding to the startup and live
campaign selection. Campaign's `qualified_initial_v4` builder repeats the
semantic binding and stores the acceptance digest in the root's trusted
config identity. `Campaign::install_qualified_initial_authority` accepts only
this checked, non-serializable `InstalledAcceptancePin` and makes a private
process-local `InstalledProfileAuthority`; it never accepts a bare
`AcceptedBoundary`, `TrustedClaudeProfile`, path/hash tuple or wire field as a
grant. A missing, changed, unaccepted or self-referential source refuses before
dispatch. The independent deployment pin is the trust input; file permissions
and a self-asserted startup digest alone are insufficient. The exact real pin
and model remain RP3 owner decisions, so offline RP2 cannot claim a production
installation exists.

`DispatchPermit` remains campaign-owned, private-field, non-Clone and
non-Serde. The campaign tags its private origin when a fresh
`prepare_initial` admission reaches `authorize_dispatch`; recovered admission
and remediation origins are different. Only
`DispatchEffect::Applied { permit, receipt }` from that fresh path can call
`Campaign::consume_qualified_initial(permit) -> FreshInitialEntry`.
Under the existing campaign owner gate this new method rechecks the permit's
owner epoch, root, identity, obligation, request digest, command, exact
dispatched revision, operation receipt and durable may-have-entered slot,
requires initial kind and fresh origin, and binds the installed authority's
acceptance digest/profile to the current selection. It consumes the permit by
value and returns a **different** private-field, non-Clone/non-Serde
`FreshInitialEntry` containing those exact bindings and the process-local
`InstalledProfileAuthority`. It has no locator, DTO, readback or recovered
handle constructor; it exposes read-only binding/authority getters to E1,
never a reusable issuer. Existing `consume_dispatch_permit` may continue
returning public `DispatchedRequest` for controlled callers, but that DTO is
never an RP2 entry argument or an E1 permit issuer. Dispatch replay and recovery return
no fresh-origin permit; cold reopen can reconstruct accepted configuration,
not the lost process-local entry capability.

E1's `QualifiedWriter::execute_applied_initial(FreshInitialEntry)` consumes
the campaign type directly, rechecks every entry binding against its
`QualifiedScope`/`Prepared` and the installed `TrustedClaudeProfile`, then
constructs its private `InitialEntryPermit` **inside E1** immediately before
the existing `execute_initial` path. The production permit constructor stays
private to E1; neither workflow nor campaign can call it. A mismatch refuses
before launch intent or prompt. `Workflow::QualifiedInitialLaunch` merely
retains the entry value through checked episode begin; it is not an issuer.
The old `InitialEntryPermit::offline_peer` remains a test-only RP1 helper and
cannot authorize an installed profile. This is one local launch capability,
not proof of a remote at-most-once inference.

The direct E1 import of `FreshInitialEntry` requires an acyclic dependency
change. Today `slice-campaign` imports E1's concrete
`CheckedExecutionResult`; adding E1→campaign would cycle. Split E1's
**read-only, checked custody owner** into new Rust crate
`review-execution-custody`, still owned and reviewed as E1. It owns the
existing `ExecutionEvidenceRef`, `ExecutionBinding`,
`ExecutionProvenance`, `EvidenceClass`, `EvidenceReaderProfile`,
`CheckedObservation`, `EvidenceReadError`, `ExecutionEvidenceOwner`,
`CheckedExecutionResult`, and controlled/qualified readers. The checked
result's constructor remains private to that custody crate. Its readers alone
make checked results after the same anchored-root, event-chain, artifact,
session, response and provenance verification. `ExecutionEvidenceOwner`
continues returning the **concrete** `CheckedExecutionResult`; campaign J1/J2
still require that type, so a third-party implementation cannot substitute a
home-made checked view or a public DTO.

The split moves only read-side store/anchor/marker/event/artifact helpers,
`ReaderPin`, descriptive `TrustedClaudeProfile`/`QualifiedReaderSelection`,
`Prepared` and terminal/capture record DTOs, local failure DTOs, stored
prompt/command validators, and session manifest verification into custody.
The current E1 crate retains private store mutation, controlled/Claude
writers, provider transport, guarded launch and entry permit. Pure shared
response and result-shape validation used by both sides sits with custody;
writer-only capture/session mutation stays E1. Writers may call a public
**non-authorizing** raw-result validator before publication, then the custody
reader for final readback; they do not gain a public checked-result constructor.
Test-support fixture construction, which currently calls the private
constructor, moves inside the custody crate and remains feature-gated. The
existing E1 crate reexports custody types/readers and test-support under its
current public paths. It does not expose custody store mutation or add a
second journal; read-side code opens the same exact RP1 database and schema.
The production package graph is `slice-campaign → review-execution-custody`,
`review-execution-evidence → {slice-campaign,
review-execution-custody}`, then workflow and permanent host above E1
(arrows mean Cargo dependency). Campaign's E1-backed `owner_join`
integration test moves to a downstream workflow test target to remove the
campaign dev-dependency cycle. Existing controlled, Claude, campaign, SC3
and CE/J1/J2 regressions are mandatory after the split. E1 custody semantics
and campaign/CE authority do not move to a generic interface or new owner.

Passing `DispatchedRequest` or a dispatch receipt to a public E1 issuer is
insufficient: both are constructible/replayable data, and a private workflow
token cannot call E1's private constructor. An apparently smaller neutral
`CheckedExecutionView` interface was also rejected: a caller could implement
that trait and supply a forged qualified J1 result where campaign presently
requires E1's private-constructor checked type. A public E1 tuple constructor,
caller-implemented entry-authority trait, or test feature that mints installed
authority would likewise widen the production entry boundary. The E1 custody
package split is a physical dependency inversion with explicit migration
cost, not a transfer of semantic custody to campaign or workflow.
If implementation cannot keep the concrete checked result and journal writer
sealed across this split, stop for an E1/campaign boundary decision; do not
substitute the rejected generic view or public mint as a compatibility fix.

## One initial route and durable recovery

1. Before the installed startup is sealed, a trusted preflight validates the
   candidate pins and builds the private candidate-only CBM index with RP0's
   `CandidateBoundary`. It durably publishes an immutable boundary manifest
   containing candidate tree/project/root, store/config/MCP/coverage hashes, generation, build
   completion, watched state and pinned binaries. The installed startup pins
   its digest. A narrow `CandidateBoundary::open_frozen` reads that exact
   manifest at host initialization and after restart and
   runs the existing frozen checks and private readback; it neither rebuilds an
   index nor launches a provider. A partial or changed private root refuses.
   Any pre-dispatch replacement uses a fresh private root and new accepted
   binding; after dispatch, the original boundary identity is fixed.
2. Initialize or open the schema-4 campaign and native episode from the pinned
   roots. Reuse H1's admit, phase, candidate and selection sequence with exact
   historical predecessors. The candidate tree, baseline diff, evidence
   catalog, role projection and result schema enter a deterministic bounded
   prompt/request; their exact digests bind the E1 qualified selection. No
   provider-generated text becomes the trusted subject or claim selection.
3. `execute_initial` may hold `FreshInitialEntry` only after
   `Preparation::Applied`, `DispatchEffect::Applied` and campaign's
   `consume_qualified_initial` have consumed the fresh opaque permit under
   its installed-authority check. It carries that value through a durably
   read-back episode begin, then passes it by value to
   `QualifiedWriter::execute_applied_initial`. E1 compares the full scope
   and creates its private one-time `InitialEntryPermit` internally.
   E1 reserves capture/session space,
   records launch intent before the start syscall, verifies the realized
   guarded boundary before prompt release, and uses its bounded initial writer.
   A definite pre-entry refusal is distinct from an uncertain local start.
4. Open `QualifiedReader` using `scope.reader_selection()` and the exact
   published `ExecutionEvidenceRef`; require its `QualifiedProvider` class,
   profile/root/attempt/session/provenance and canonical result subject.
   Install `InitialJoinSelection` with E1's `claude-qualified-v1` record and
   `review-execution-evidence-claude-v1` custody verifier and the
   accepted production grant. J1 remains campaign/CE/episode owned. Install
   `CompletionSelection` from the same claims root and exact accepted
   revision-bound finding/reliance grant, consumer tree, decision scope and
   authority source before J2 finalization. One narrowly scoped finding grant
   may supply both finding publication and later reliance permissions; an
   empty finding set uses J2's no-findings record. The host cannot invent
   claim truth, finding revisions or builder evaluations. It reports the
   checked J1/J2 receipt, pending/conflicting/unresolved stage, or
   `EvidenceUnestablished` outcome exactly as the owners return it.
5. After restart, reopen the exact installed startup, frozen boundary,
   campaign/episode and qualified E1 root. Recover the original dispatch
   receipt and original request; call only `QualifiedWriter::resume/reconcile`.
   A verified complete capture may finalize a terminal without another
   entry, then the checked reader rejoins J1/J2 through their existing
   locators. `launch_intent`, uncertain spawn, failed capture, changed session,
   missing boundary, missing CE/episode owner or unverified output stays
   unresolved. No recovery branch constructs `QualifiedInitialLaunch`, calls
   `execute_initial`, or changes attempt/session/reference IDs.

The host keeps a single workflow worker and the current bounded control
runtime. Status exposes stale/ready/busy observations and selected attempt
status, never a second dispatch queue. `stop_admission` closes new commands
without declaring the in-flight provider cancelled. A caller EOF or blocked
output retains owner work and its original recovery identity. A bounded
per-attempt interrupt may report only local signal/reap facts; remote
cancellation/refund/non-entry is not inferred. The protocol remains initial
only: `status`, `inspect_initial`, `execute_initial`, `recover_initial`,
`stop_admission`; no remediation command or general provider endpoint.

## Storage and runtime budget decision

RP1's accepted 64 MiB aggregate session-storage invariant includes live,
retained, in-progress and orphan storage plus bounded metadata. Its private
cgroup `memory.max` is the page-rounded room left under that limit and
`memory.swap.max=0`; the cgroup charges **the whole Bubblewrap/guard/Claude
process tree as well as session tmpfs data and inode metadata**. The 16 MiB
active tmpfs limit alone does not bound empty-file metadata. Accordingly the
64 MiB cgroup is a storage enforcement mechanism that also imposes a
conservative provider-runtime ceiling. RP2 must not raise `memory.max` from a
startup number while calling the original 64 MiB storage guarantee intact.

For this bounded RP2 route, install an explicit
`strict_aggregate_session_64mib_v1` resource policy. The trusted startup pins
it; E1 derives the actual remaining cgroup budget, writes and reads back
`memory.max`, zero swap, and guard membership/current charge before READY.
There is no independent larger runtime-memory allowance in this policy.
Unavailable/wrong delegated cgroup controls, insufficient room, or a peer
that cannot start under the cap fail closed before a usable result. Tests
exercise wrong/missing policy and oversized live storage. A measured provider
runtime requirement above this cap is **an unqualified installed profile**,
not grounds to weaken the storage claim. A future larger runtime budget
requires an independently enforced aggregate session data+metadata bound and
new hostile qualification before the profile can be accepted. That is a
separate explicit owner decision; RP2 does not assert that the installed
Claude binary fits. The status/receipt identifies storage policy, realized
runtime limit and any capacity refusal separately.

The selected 10-minute provider horizon, 5-second local cleanup, 64 KiB
command, 1 MiB result/observation, 4 MiB stream and 64 MiB retained-session
limits remain bounded selections from the parent plan. A provider CLI may
internally retry an upstream request; the host's one-entry proof is one local
launch/prompt operation, not a remote at-most-once billing or inference claim.
RP0's 4,096-request relay behavior and hard cleanup deadline are not upgraded
to established properties by RP2 status or fixture tests.

## Proposed code ownership

| Surface | Bounded change |
| --- | --- |
| `rust/crates/review-workflow/src/startup.rs`, new `provider.rs`, `initial.rs`, `recovery.rs`, `protocol.rs`, `lib.rs` | Add a separate trusted qualified startup document/profile and installed provider adapter. Share the existing initial stage order and J1/J2 join; keep controlled SC0 contract valid. Bind static profile, role/prompt, fresh schema-4 campaign and claims/J2 selection, and the Applied-only E1 bridge. Host result/recovery remain exact owner projections. |
| `rust/crates/review-execution-evidence/src/claude/{isolation,qualified,writer}.rs` | Narrow `open_frozen` with an immutable candidate/index manifest and checked readback; consume campaign's opaque fresh entry in E1, validate it against prepared scope, and create E1's private permit internally. Preserve RP1 writer/reader/session/store invariants and test-only offline provenance. No retained permit. |
| `rust/crates/work-engine-review-host/src/{worker,control}.rs`, `main.rs` only if profile dispatch requires it | Select the installed workflow at process startup, keep one worker and bounded status loop, report truthful busy/timeout/error counts. No provider work on control runtime. |
| `rust/crates/slice-campaign/src/{lib,application,completion,store,contract,owner_readback,remediation}.rs` and a small `installed_authority.rs` | Add the separate schema-4 qualified-initial config/marker and accepted-installation validator, origin-tagged Applied permit and opaque `FreshInitialEntry`; change E1 imports to the custody crate's **same concrete checked result**, without changing J1/J2 reducers. Existing remediation v4 and schema-3 profiles remain exact. The new campaign code is required, not optional. |
| New `rust/crates/review-execution-custody/{Cargo.toml,src/{lib,contract,reader,store_read,record,process,test_support}.rs,src/claude/{reader,selection,session_read,profile,transport_validation}.rs}`; E1 `src/{lib,contract,reader,store,producer,process,test_support}.rs`, `src/claude/{mod,qualified,reader,session,profile,transport,writer}.rs` | Physically split E1's read-only store and checked-result constructor/reader into an E1-owned custody package; retain writer mutation, launch and private permit in E1. Split shared record/session/transport validation from mutation, reexport original public reader paths, and preserve exact schema/digest/readback. No public checked-result constructor, public journal mutation, generic checked view, or parallel evidence store. |
| `rust/Cargo.toml`, `rust/Cargo.lock`, crate-local manifests for campaign, custody, E1, workflow and host | Register custody, replace campaign's normal E1 dependency with custody, add E1→campaign and E1→custody, and wire offline test-support only in test targets. Move campaign's E1 dev test downstream. Freeze the exact lock delta under its owner; no dependency cycle or production test-support feature. |
| Move `slice-campaign/tests/owner_join.rs` to downstream `review-workflow/tests/campaign_owner_join.rs`; add `review-workflow/tests/{qualified_initial,qualified_recovery}.rs`, `work-engine-review-host/tests/qualified_control.rs`, focused campaign/E1 authority tests, and `docs/rust-review-provider-rp2-validation.md` | Preserve existing E1-backed campaign integration coverage without a dev-dependency cycle. Add permanent-binary, exact-owner, crash-cut, capability and held-provider qualification with frozen nonsecret receipts. |

No lifecycle/S8/S9/UI code or old native Node adapter is in scope. A
shared-workspace/lock edit, if needed, is coordinated with its owner; unrelated
dirty paths are preserved. This proposal does not require a new campaign
schema, CE reducer, episode result shape, background daemon, or generic
multi-provider abstraction.

## Offline acceptance and review evidence

Use test-owned pinned programs, canary credentials where the guard protocol
requires them, private fresh schema-4 roots, and the actual permanent host
binary. `TrustedConfig::qualified_initial_offline_v4` is a feature-gated,
separately marked schema-4 test profile; it can carry only offline authority
and cannot be opened as the production qualified-initial profile. The
accepted-installation validator has one shared binding core for
candidate/claim/selection/profile/program/catalog/credential metadata,
acceptance reference and resource policy. Its production wrapper requires the
independent protected deployment pin and returns campaign's
`InstalledProfileAuthority`; its feature-gated offline wrapper accepts a
test-owned pin and returns a **different** `OfflineProfileAuthority` and
`OfflineInitialEntry`. There is no test constructor, conversion, trait
implementation or flag that yields `InstalledProfileAuthority`,
`FreshInitialEntry`, or a production E1 permit. The offline entry reaches the
same campaign owner-gate dispatch validation and E1 scope/permit consumption
core: `consume_offline_initial` calls the same private campaign slot/origin
validator with an offline acceptance class, and E1's feature-gated
`execute_offline_applied(OfflineInitialEntry)` and production
`execute_applied_initial(FreshInitialEntry)` call one private
`validate_and_consume_entry` before the existing writer path. The distinct
offline type cannot select an installed profile. E1 labels its output
`offline_peer`, which the qualified production reader/J1 join rejects.
Production-loader tests cover absent, wrong and
changed deployment pins as refusals; they make no positive installed-authority
claim. The existing descriptive
`TrustedClaudeProfile::load_installed` alone is never an entry grant.
Test peers use offline identities only;
offline E1 fixture evidence remains `ControlledFixture`, and its current
reader deliberately refuses production custody. A feature-gated test adapter
may inject a synthetic controlled `ExecutionEvidenceOwner` into the **same**
host join function with a test-only claims root, so the complete
campaign/CE/episode/J1/J2 mechanics can be checked without making its output
`QualifiedProvider`. Separate tests route a guarded offline peer through the
actual E1 writer/reader and assert that the production join rejects its
custody. The production qualified join compiles against E1's installed
reader, but only RP3 can observe positive live Claude custody. These are
distinct evidence claims, not one synthetic end-to-end provider proof. The
deterministic gates must cover:

- Exact startup/profile/candidate/index/role/claim/grant/binary drift and
  schema-3 or controlled evidence presented as qualified; arbitrary wire
  fields, replay and locator attempts cannot reach an installed entry.
- One Applied test dispatch, consumed through the campaign's new offline
  capability and E1's **same entry-binding/consumption core**, produces one
  witnessed offline peer start. A constructed `DispatchedRequest`, copied
  dispatch receipt/locator, recovered handle, wrong-origin permit, altered
  acceptance pin/profile/command/revision/session/attempt, double consumption,
  preparation or dispatch replay, restart, disconnect and four competing
  execute commands produce no additional start. Assert compile-time inability
  to convert the offline capability into installed authority or an E1
  production permit. Record the peer witness independently of E1's journal,
  and preserve the original attempt/session/reference.
- Every material E1/J1/J2 cut: before dispatch, after Applied dispatch,
  episode begin, E1 prepared/launch intent/spawn/capture/terminal, CE stages,
  episode result, findings/no-findings and J2 outcome. Complete capture
  reconciles without entry; intent, failed capture and unknown remote effects
  stay unresolved. Check duplicate grant/finding/reliance prevention.
- Qualified reader rejects altered observation, subject/model/session/CBM
  generation, result, session manifest, root, or provenance; CE false and
  unestablished results remain distinct from reported completion. Across the
  custody package split, existing controlled and Claude fixture records retain
  exact reference/claim/custody digests and cold reader behavior; compile-fail
  checks reject external `CheckedExecutionResult` construction or a generic
  checked-view substitute in campaign J1/J2.
- Cold `CandidateBoundary::open_frozen` succeeds only for the original
  durably published manifest. Partial publication, sidecars, changed store,
  candidate, config, coverage, binary or project identity refuse without
  rebuilding or provider launch. Wrong/unavailable cgroup memory or swap
  realization, empty-file metadata attack, oversized capture and storage
  exhaustion refuse before positive evidence.
- One held provider interval long enough to contain the 700 external status
  probes, with cold reconstruction, campaign contention and four simultaneous
  mutation requests (one admitted). Save every latency and p50/p95/p99/max,
  timeout/busy/error counts, actual provider hold duration and exact control
  binary. Gate p99 <= 50 ms, max <= 250 ms, zero status timeouts, exactly one
  peer entry and honest busy refusals. Compilation/setup is excluded from
  latency samples but separately recorded.

On one frozen exact source and lock subject, run focused E1, workflow and
host tests under default and fault-enabled debug/release builds, affected
claim-evidence/episode/campaign/J1/J2/H1 regressions, all-target Clippy and
formatting. Include `cargo metadata --locked` cycle verification and old
E1 public-path compile consumers after the custody split. The SC3-D expensive
debug-case exclusion remains explicit and
receives optimized capacity proof where affected. Preserve raw failure and
success logs, exits, counts, feature sets, toolchain, binary/launcher/profile
hashes and a complete source/lock manifest. Reuse previous RP1 gate evidence
only for an unchanged relevant input closure, labeled historical; it cannot
replace the new host/recovery/held-provider gates.

RP2 acceptance is an **offline mechanism** claim: exact authority wiring,
isolation readback, checked custody, restart behavior and owner joins under
synthetic peers. It cannot establish current installed Claude compatibility,
login/expiry, upstream model or billing route, prompt quality, actual response,
remote at-most-once behavior or same-session inference. The first such initial
observation belongs to separately authorized RP3. A later retained round and
its continuity proof belong to SC3-H. No source retirement, campaign
terminalization or review acceptance follows just from RP2 qualification.

## Decisions retained by their owners

The campaign/acceptance owner must select the exact real candidate, initial
claim pair, authority source, finding/reliance grant and schema-4 profile for
the future installed run. The compiler owner must provide the exact role
projection and any required verification, or select a profile that truthfully
requires none. The provider/credential owner must select the installed
Claude binary/model/catalog, credential source and cost horizon at RP3. If
that binary exceeds the strict RP2 resource policy, the session-storage and
runtime-memory design requires a new accepted bound before a live call.
None of these external selections can be supplied by a test fixture, a wire
request, or a replayed campaign receipt.

## Planning evidence and limits

Tier-2 Verify in graph project `home-bline-code-work-engine`, generation
`2026-10-16T13:39:53Z`, store `uf9a921ac2b51351eg2`, watched. Bounded
searches and exact snippets covered the workflow launch/join/recovery,
host control/worker, E1 installed profile/scope/writer/reader and schema-4/J2
installation. All 17 initially relied-on code/plan paths returned
`content_verified=true`, `no_recorded_issue`, `metadata_match` with no missed
ranges. This is best-effort, not graph completeness. Direct source reads
verified literals, profile choices, cgroup policy and restart behavior; test
and documentation paths added to the final support manifest receive their
own coverage check or are labeled direct-source-only. No provider, credential,
current CLI compatibility or timing probe ran during planning. The frozen
support and exact source-hash manifest are under the private RP2 planning
packet named in the handoff.
