# HP3 controlled initial host implementation plan

Planning proposal, 2026-10-08, against published E0/J1/E1 commit
`3b8d1f4b75375c3328f191748619ca7e515d4a22`. The user authorized the next parallel
plan. This document authorizes no implementation, provider call, migration,
deployment, publication or state disposition. The parallel campaign work is
[J2](rust-j2-completion-implementation-plan.md); their common handoff is owned by
the [coordination plan](rust-j2-hp3-parallel-coordination.md).

> **Later annotations.** Blocks headed *Current implementation* were added on
> 2026-10-11 against a working-tree snapshot based on `c72f7a5` (content
> manifest sha256
> `946887f3934c2d5541c41d54ce9fba8d7e1653460d853bac1a788dfa7fa2ac28`). They are
> not part of this plan and do not change its subject. Each points to where a
> symbol the section names is implemented in that later tree. *At subject* means
> a definition of that name existed in the crate at `3b8d1f4` (checked with
> `git grep`); it does not establish the same shape or meaning. *Introduced
> later* means none did.

## Selected result and boundary

Implement a permanent Rust initial-review composition in `review-workflow` and
`work-engine-review-host`, qualified by an actual host process, a controlled Rust
child, and the real campaign, CE3, episode and E1 owners. First make the small
episode write-admission prerequisite **H0**, then implement **H1**. This is a
bounded continuation of HP3, not completion of its original broad roadmap scope.

The first host profile is `review-host-controlled-initial-v1`: Unix/Linux,
one selected obligation and one initial attempt on new private roots, using the
existing **frozen SC0 campaign corpus**. `TrustedConfig::controlled_sc0` currently
requires run `sc0-oracle-c2`, slice 1, plan `sc0-controlled-v1`, its exact accepted
boundary and gate capture. H1 does not generalize that constructor or claim that
arbitrary campaigns are admitted. Startup rejects incompatible input before root
initialization or campaign mutation. The permanent composition and protocol can
later gain separately accepted profiles; today's bounded profile remains named.

E1 records `ControlledProcess`, a same-UID child and `mutationAuthorized=true`.
The joined host must preserve both unestablished production-path claims and their
reasons. After J2 joins, the actual-process outcome is `evidence_unestablished`.
This is a successful qualification of truthful handling, not successful review
acceptance or evidence of an enforced read-only/provider-independent realization.
Do not add a temporary sandbox framework or fabricate a positive observation to
make this gate green. J2's controlled owner fixtures test its separate established
and false branches; those fixtures do not enter the host process qualification.

## Source-backed placement

The inspected implementation supports this division:

| Owner / evidence | Implication for H1 |
| --- | --- |
| `slice-campaign/src/application.rs`: `prepare_initial`, `authorize_dispatch`, `consume_dispatch_permit`, `replay_dispatch` | Campaign issues process-local admission and dispatch capabilities; replay returns descriptive records. H1 must consume a fresh permit before child launch. |
| `slice-campaign/src/owner_readback.rs`: `claims_admission_with_custody`, `prepare_initial_completion`, `complete_initial`, `register_initial_episode_read`, `recover_initial_completion` | Campaign installs the real CE3 port, owns stage registration and joins exact historical episode evidence. Workflow must not duplicate those stages or write campaign tables. |
| `review-execution-evidence/src/producer.rs`: `ControlledScope`, `EvidenceWriter::{create,resume,execute,reconcile}` | E1 owns process capture, immutable evidence and launch uncertainty. It does not grant campaign admission. Its scope is fixed to one exact attempt. |
| `review-episode/src/host_admission.rs` and `in_process_admission.rs` | Native write admission binds an exact descriptor; ER1 provides distinct checked read admission. A checked read does not grant a write. |
| `review-episode-store/src/sqlite.rs`: `init_native_root`, `read_native_selection` | The native root binds its absolute path and actual current executable digest. A new host creates a new root; it cannot relabel a predecessor root. |
| `work-engine-compiler/src/lib.rs`, `manifest.rs`, `verification.rs` | Pure compilation/manifest projection can run in process. Captured-byte comparisons do not establish source capture or an AEG invocation. |
| `app-server/src/services/slice-campaign/native-review-host.mjs`: owner factory and request construction | Current host composition combines compiler, adapter, claims, episode and campaign without any one domain owning the whole workflow. Its authority/subject/session construction is characterization evidence, not a dependency of the successor. |
| `app-server/src/services/slice-campaign/native-review-closure.mjs:239`: `executeInitial` | Episode begin precedes reviewer execution; result and claim admissions follow actual execution. The J1 harness's later begin is not the runtime lifecycle to adopt. |
| `app-server/src/services/slice-campaign/capability-host-runtime.mjs:201` | The actual supervisor currently dispatches native execute/recover/retry/correction/evaluation/remediation to the Node host. H1 does not replace that registration. |

`review-workflow` depends on `work-engine-compiler`, `slice-campaign`,
`claim-evidence`, `review-episode`, `review-episode-core`,
`review-episode-store`, and `review-execution-evidence`; the executable depends
on workflow and the chosen bounded protocol/runtime dependencies. No domain
depends back on workflow. E1 remains the process/evidence implementation, not a
second workflow. Lifecycle, UI, a generic task broker and a shared storage
framework are unnecessary for this single concrete consumer.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/slice-campaign/src/application.rs::Campaign::consume_dispatch_permit`, public as `slice_campaign::Campaign::consume_dispatch_permit`; at subject.
> - `rust/crates/slice-campaign/src/owner_readback.rs::Campaign::claims_admission_with_custody`, public as `slice_campaign::Campaign::claims_admission_with_custody`; at subject.
> - `rust/crates/review-execution-evidence/src/producer.rs::ControlledScope`, public as `review_execution_evidence::ControlledScope`; at subject.
> - `rust/crates/review-episode-store/src/sqlite.rs::init_native_root`, public as `review_episode_store::init_native_root`; at subject.
> - `rust/crates/review-episode-store/src/sqlite.rs::read_native_selection`, public as `review_episode_store::read_native_selection`; at subject.

## H0: exact episode admission without a per-command subprocess

The current `NativeHostAdmission::from_inherited_fd` reads a private, read-only,
unlinked FD-3 descriptor, limited to 64 KiB. Its immutable fields bind request ID,
operation, grant/selection, request/args/authority/result/evidence/content digests,
principal and observed revision. One inherited descriptor cannot admit both
`begin` and `record_result`: the latter depends on actual E1 bytes and CE3
admissions produced after startup. J1's existing join harness invokes itself as
an episode writer for each command. Retaining that as the permanent host route
would reproduce the bridge that the host replacement is meant to remove.

The proposed minimal episode-owned export is:

```rust
NativeHostAdmission::from_trusted_descriptor(
    selection: NativeRootSelection,
    canonical_descriptor: &[u8],
) -> AppResult<NativeHostAdmission>
```

Factor common bounded canonical parsing and exact selection checks out of
`from_inherited_fd`; retain FD provenance checks on that existing transport.
The new constructor is **trusted in-process composition**, not a wire grant or
a claim that arbitrary linked Rust code is untrusted. It preserves all current
request/principal/digest/CAS checks in `precheck` and `admit`. Validate the native
root selection against `read_native_selection` before issuing admission, so
current executable, root and marker identity remain enforced. This export does
not accept an unchecked alternate root, relax ER1 or create a general role port.

Workflow's private startup capability is the concrete consumer. Only it builds
descriptors from the installed authority, current campaign/dispatch facts and
registered exact result content; command input never supplies descriptor bytes,
principal, authority, root or executable selection. The startup capability is
not serializable or reconstructible from an operation reference. Its allowed
writer actions are the selected initial `begin` and `record_result` transitions;
ordinary episode validation and observed-revision CAS remain authoritative.
Recovery may replay those same exact downstream transitions after owner readback,
but cannot create a new reviewer dispatch permit.

H0 owns only `review-episode/src/host_admission.rs` and
`review-episode/tests/trusted_native_admission.rs` (plus an export only if module
visibility requires it). It changes no episode store schema and no native marker.
Test equal direct/inherited descriptor behavior, actual host executable/root
mismatch, changed request/result/evidence/content, principal/write scope,
noncanonical or oversized bytes, stale observed revision, and ER1's continuing
write refusal. The descriptor transport provenance differs deliberately; the
admitted command semantics do not. Freeze H0's exact export/checkpoint before H1
depends on it. It can proceed alongside J2; it is not a campaign prerequisite.

## Trusted startup and the permanent consumer

The permanent binary exposes `initialize --startup <absolute-file>` and
`serve --startup <absolute-file>`. `initialize` creates only the selected empty
private controlled roots through their owner APIs; `serve` opens those exact
roots and runs a bounded versioned JSON-lines interface on stdin/stdout. An
operator or a Rust consumer can keep one host process and issue correlated
commands. The actual-process gate is that consumer using the same installed
binary and protocol, not a test-only alternate workflow. HP4 later connects the
real supervisor to this interface or its accepted successor; H1 supplies no Node
shim, socket service, daemon manager or parallel deployment arrangement.

The startup document is an immutable, bounded selection made by the trusted
launcher/operator. Its path/hash is not proof of human authority by itself.
It pins the accepted controlled profile and boundary; campaign identity,
repository and subject workspace; gate capture/manifest/receipt; candidate and
physical profile; supervisor selection and both required claims; raw compiler
inputs and options with their path bases; controlled child absolute path/bytes,
arguments and selected session; claims source/config/grant and custody profile;
native episode authority; all private root locations; and deterministic command,
transition and attempt IDs. It also binds host source archive, toolchain, lock,
target/features, host executable bytes/mode and runtime dependencies. The host
validates actual files/bytes and cross-bindings, not just hash string syntax.

Use a versioned startup record owned by workflow, installed read-only before any
dispatch. Admission references remain descriptive data. Existing campaign and
claims trusted constructors still validate their own selected profiles. Startup
uses no ambient PATH, credentials, backend selectors, mutable `target/` fallback
or arbitrary request-selected program. Changes in bytes/configuration require
a different selection and new roots or a later accepted migration; same-byte
relocation of the executable may work if all owner bindings still match, but
episode-root relocation is rejected by its existing absolute-path binding.

For compiler consumption, call `project_runtime_manifest` and
`satisfy_runtime_requirements` directly on the captured bounded document/options,
and consume the projection in reviewer-request construction. If skill compilation
is selected, call the pure compiler and retain its exact source/input binding.
This profile may use schema-checked/unverified controlled skill inputs only when
startup declares that limited mode; it must refuse any whole operation requiring
source-verification/AEG evidence that the composition cannot establish. It cannot
mark `compile_skill_unverified` output verified. General compiler C2 external
verification and its possible Python dependency remain outside this profile.

Initialization uses real `Campaign::{initialize,admit,advance,
bind_existing_candidate,bind_selection}`, the existing subject/gate verification
and native episode initialization. CE bootstrap follows E1 root creation during
the first admitted operation, when its actual reader can be installed; `serve`
does not pretend a missing E1 reader already exists. No harness inserts review-ready
rows or issues replacement admission objects. Partial initialization is visible
and cannot be silently wiped: reopen/reconcile existing exact owners, or report
the unresolved owner/binding. Never treat an existing unknown directory as empty.
If interruption leaves a marker without a valid database/schema and the existing
owner cannot open it, return that exact partial initialization refusal and
preserve the directory. H1 does not repair markers, drop/recreate databases or
invent an initialization journal; disposition or a different fresh root needs
the startup/state owner's explicit decision.
The actual host executable, rather than a test executable, initializes episode
roots so normal restart can satisfy their native executable binding.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/work-engine-compiler/src/manifest.rs::project_runtime_manifest`, public as `work_engine_compiler::project_runtime_manifest`; at subject.
> - `rust/crates/work-engine-compiler/src/manifest.rs::satisfy_runtime_requirements`, public as `work_engine_compiler::satisfy_runtime_requirements`; at subject.
> - `rust/crates/work-engine-compiler/src/skill.rs::compile_skill_unverified`, public as `work_engine_compiler::compile_skill_unverified`; at subject.

## H1 operation and capability lifecycle

The finite wire vocabulary is `status`, `inspect_initial`, `execute_initial`,
`recover_initial`, and `stop_admission`. Requests include version, request ID,
selected obligation, exact operation identity and expected campaign revision
where mutation requires it. `inspect_initial` returns bounded authoritative
owner readback; `status` reports host activity with the last observed revision
and explicitly identifies an in-flight/stale snapshot. References in responses
are data and never permits. `stop_admission` acknowledges a closed frontend
admission gate; it neither cancels a committed operation nor settles an effect.

The command-specific route is:

1. Parse and bound the whole command. Check the installed profile, expected
   selection/candidate/obligation and capacity before acquiring domain work.
   Unsupported retry, correction, remediation, evaluation, publication,
   terminalization, arbitrary history or provider operation refuses before any
   domain effect. Do not perform the supported prefix of an unsupported request.
2. `prepare_initial` issues `AdmissionHandle`; `authorize_dispatch` consumes it
   and durably records the dispatch before returning `DispatchPermit`.
   `consume_dispatch_permit` rechecks current epoch/revision/slot and returns
   the selected request/command. Only this fresh applied branch reaches launch.
   Replayed or uncertain dispatch returns reconciliation data, never a new launch.
3. Construct exact native authority and controlled reviewer request from the
   installed compiler projection, subject and campaign facts. Bind E1's
   `ExecutionBinding` and `ExecutionProvenance` to the actual original dispatch
   operation/revision, prepared-request digest, episode/attempt/profile/session.
   Before child launch, apply and read back the selected episode `begin` using
   H0, its exact dispatch-bound authority and begin transition. This records the
   admitted episode before execution; it does not assert that execution occurred.
   A failed/uncertain begin stops launch and preserves the already committed
   dispatch as unresolved. If begin committed and the host dies before E1,
   reconstruction reads that exact begin but never recreates the lost launch
   capability. This preserves the native closure's runtime ordering rather than
   copying the J1 harness's deferred episode writes.
   Install `ControlledScope` and create E1's separate writer and checked reader.
   Neither an arbitrary `DispatchedRequest` data value nor a serialized dispatch
   record can reach the workflow's private launch function.
4. Await the real `EvidenceWriter::execute`. E1 remains sole owner of launch
   intent, child handles, bounded transport capture and terminal custody. The
   controlled Rust peer receives only its bounded request and stdio; no database
   handle, descriptor, credential or admission capability is inherited. Same-UID
   filesystem access still exists and must be described truthfully.
5. Install `InitialJoinSelection` and the actual `EvidenceReader` via campaign's
   concrete CE3 path. Prepare and advance J1 completion using the checked evidence
   reference. Create the episode result content from exact native result bytes
   and both checked CE admissions; register its target with campaign before
   executing that exact result transition. The `begin` already committed before
   execution; use H0 for `record_result`,
   the real episode application/result validator and ER1 for checked historical
   readback. Owner joins remain separate from campaign outcome publication.
6. At the serial J2 handoff, call `finalize_initial_completion(identity, expected,
   locator, outcome_operation_id, owners)` and its matching
   `recover_initial_outcome`. Return its typed progress and exact references.
   H1 does not independently set outcome, consume claims, publish findings or
   reinterpret J2's pending/unresolved/false statuses.

   J2 maps a valid result with `EvidenceUnestablished` to
   `AttemptOutcome::FailedWithResult` and obligation `failed`, retaining the exact
   richer claim outcome. This settles initial-result processing only; it makes
   no provider-failure or retry inference. A malformed/missing valid E1 result
   instead leaves the original dispatch/completion unresolved and never enters
   that finalization branch. If the response containing the outcome locator was
   lost, use J2's proposed read-only `locate_initial_outcome(identity,
   obligation_id, outcome_operation_id)` against original persisted receipts;
   an exact absence is not authority to dispatch again.

The J2 names are proposed interfaces owned by its checkpoint, not exports already
present at the baseline. H1 can compile and qualify through `OwnerReadsJoined`
against published J1 during parallel development. Its *combined exit* includes
J2; there is no placeholder successful finalizer or duplicate campaign reducer.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/slice-campaign/src/completion.rs::DispatchedRequest`, public as `slice_campaign::DispatchedRequest`; at subject.
> - `rust/crates/review-execution-evidence/src/contract.rs::ExecutionBinding`, public as `review_execution_evidence::ExecutionBinding`; at subject.
> - `rust/crates/review-execution-evidence/src/contract.rs::ExecutionProvenance`, public as `review_execution_evidence::ExecutionProvenance`; at subject.
> - `rust/crates/review-execution-evidence/src/reader.rs::EvidenceReader`, public as `review_execution_evidence::EvidenceReader`; at subject.

## Runtime topology and responsiveness

Use one Tokio control/stdio task and one bounded workflow worker on a separate
OS thread with its own Tokio runtime. That worker owns synchronous campaign,
claims and episode operations and awaits E1's asynchronous child I/O. This keeps
the existing synchronous E1 SQLite calls and domain gates off the control runtime
without rewriting E1 or introducing a worker-process farm. Construct owner handles
on that worker; channels carry bounded commands/observations, not SQLite handles
or grants. Pure compiler work
also runs on that worker. No domain lock, SQLite transaction or CE lease crosses
the child wait. CE3 retains its existing campaign-gate/evidence-read/CE-transaction
order; locks are released before subsequent campaign CAS publication.

The initial limit is one in-flight workflow command, no queued mutation and a
small separately bounded control/output queue. Busy commands refuse before
admission. Reserve a terminal response slot before starting work; output/client
disconnect cannot make an admitted effect disappear. A slow or disconnected
consumer stops new admission, preserves exact durable owner identities and does
not trigger replay. Bounded status responses stay available while the workflow
worker is busy, under the declared continuously draining consumer assumption.
Authoritative inspection may report busy rather than block the control path.

Proposed wire limits: 64 KiB command/status envelopes and 1 MiB detailed reply;
oversized readback refuses explicitly without truncating a success. Retain E1's
published 1 MiB request/result/observation, 4 MiB streams, 16 MiB terminal and
64 MiB root payload limits, its 5-second capture and 2-second cleanup bounds and
5-second database busy wait. Bound startup to 1 MiB metadata, 4 MiB total captured
compiler/request inputs and at most one attempt. Reconcile exact encoding overhead
against every underlying owner's limit before freezing the profile. Full history
is not exposed in H1, so these limits do not claim the broad HOST history case.

Preserve the roadmap control target for this finite workload: external p99 at
most 50 ms and maximum at most 250 ms, measured during actual child capture,
domain contention, cold reconstruction and four simultaneous requests (one
admitted, others visibly refused). Freeze machine, sampling interval, sample
count, draining-consumer behavior and release artifacts before the run. Report
full successful operation time, refused/timed-out/unresolved counts, missed probes,
queue/backpressure and memory where observed. The broad large-history completion
budget and concurrent-domain performance qualification remain unresolved successor
work; do not use an error-only workload as evidence of successful operation speed.

## Reconstruction and failure semantics

Startup selection supplies identities and exact input bytes, never saved permits.
Campaign records and original receipts provide original revisions and dispatch
facts; E1 provides immutable capture; CE3 and ER1 provide their own effects. A
small immutable workflow startup record is sufficient; do not introduce a second
workflow journal/state machine that can contradict those owners. Derive fixed
operation locators from startup IDs and checked owner receipts, preserving each
owner's codec. Missing reconstruction input is an explicit unresolved result.

| Observation after crash/restart | Required behavior |
| --- | --- |
| No committed dispatch; exact preparation exists | Inspect/reconcile the original preparation. H1 recovery does not authorize a launch; any future launch route needs a separately fresh owner grant. |
| Dispatch committed but process-local permit lost, even when E1 root/attempt is absent | Report dispatch may have entered / unresolved. No E1 recreation followed by execution, no inferred provider non-entry and no freed campaign slot. |
| E1 prepared/launch-intent/spawn/capture exists without committed terminal custody | Return E1's exact failure/uncertainty and bounded retained facts; no result reconstruction from requested bytes. |
| E1 terminal committed, acknowledgement lost | Reopen checked reader/reconcile original reference, then continue downstream completion without another invocation. |
| CE commit, episode commit or joined/outcome campaign commit with lost acknowledgement | Reconcile exact original owner operation/transition and content. Preserve downstream effects if a later campaign CAS conflicts. Do not substitute current revision for original admission. |
| Exact native episode target absent | Distinguish registered-but-not-written from unavailable/corrupt readback; replay only the registered downstream write under its exact descriptor/CAS. |
| Changed root, binary, source, configuration, result, observation or selection | Refuse/conflict before new effects; unavailable custody remains unresolved, never absent. |

`EvidenceWriter::resume`/`reconcile` and `recover_initial_completion` are the
recovery route, never `execute`. A terminal failure's local cleanup is not remote
settlement. E1's definite pre-entry observation does not grant retry; H1 has no
retry command. After EOF, stop admitting and allow owned bounded work to finish;
abnormal host death retains the uncertainty described above. No automatic process
restart or provider resend is part of this service.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/slice-campaign/src/owner_readback.rs::Campaign::recover_initial_completion`, public as `slice_campaign::Campaign::recover_initial_completion`; at subject.

## File ownership, parallel boundary and qualification

| Work | Exact proposed path boundary |
| --- | --- |
| H0 | `rust/crates/review-episode/src/host_admission.rs`; `rust/crates/review-episode/tests/trusted_native_admission.rs` |
| H1 composition | New `rust/crates/review-workflow/Cargo.toml`; `src/{lib,startup,initial,recovery,protocol}.rs`; `tests/{startup_binding,authority_boundary,initial_composition}.rs` |
| H1 executable/consumer | New `rust/crates/work-engine-review-host/Cargo.toml`; `src/{main,control,worker}.rs`; `tests/{controlled_host,host_recovery,control_responsiveness}.rs`; `tests/support/` Rust controlled peer/consumer and bounded fixtures |
| Shared integration | Integration owner alone registers workspace/lock/dependency edges and the exact J2/H0 imports. S6/S7 currently owns shared Cargo files; reconcile with that owner before editing. |
| Validation record | `docs/rust-hp3-controlled-host-validation.md`, created only by the later implementation task with actual results |

No campaign, claims or E1 source edit belongs to H1. A newly discovered missing
API returns to the integration/domain owner; it is not repaired by bypassing the
owner or silently widening this manifest. H1 tests may consume published fixture
data but not campaign test-only grants or E0 fixture evidence. No dependency on
unaccepted lifecycle S6/S7 source or UI work is introduced.

J2 and H0/H1 have disjoint product files and can implement concurrently. Freeze
the J2 finalization signature/data contract before the host's final adapter;
H0 must pass its owner gate before host write integration. Final real-owner
integration, feature/build reconciliation and joined acceptance are serial.

Meaningful exits require:

1. A release host initialized through its permanent CLI, with real stores and
   compiler projection, consumes real campaign admission, launches the controlled
   Rust peer exactly once, checks custody, commits/re-reads the native episode,
   and reaches J1 join. The final J2 composition persists
   `evidence_unestablished`, both exact claims/reasons and no unauthorized finding
   consumption or campaign terminalization. Parent fixtures cannot supply a fake
   host/campaign or write tables to make this path succeed.
2. Actual host-process cuts before/after campaign dispatch, E1 launch intent,
   E1 terminal, each CE establishment, episode result, J1 joined commit and J2
   outcome commit. Restart the same pinned executable from durable inputs only.
   Independently count peer entry/output: it never increases during recovery.
   E1's five and J1's nine published cuts remain regressions; host cuts establish
   this composition, rather than being replaced by those component results.
3. Wrong/stale campaign revision, selection, subject, session, operation ID,
   changed replay bytes, competing writers, wrong executable/root marker,
   malformed/oversized input/result and read-only profile misrepresentation all
   refuse or preserve uncertainty according to the relevant owner. Prove the
   whole unsupported operation has no effects. Compile-fail tests cover private
   workflow startup/launch capability construction and serialization; runtime
   checks reject controlled evidence used as a qualified-provider profile even
   in all-feature builds.
4. The actual binary remains responsive under the frozen bounded control profile,
   including busy refusal, consumer backpressure, child timeout, database
   contention and orderly admission stop. An external Rust observer measures
   replies, not an in-process clock around a cached method.
5. Focused H0/H1 and affected owner regression tests, locked offline debug/release
   and fault-feature builds, formatting and all-target Clippy pass. Save exact
   source/configuration/profile/toolchain/lock/features/executable identities and
   raw gate/crash/probe logs under an implementation-specific external evidence
   root. Build/disposable roots use `/home/bline/code/.work-engine-tmp/`.

## Decisions for acceptance and retained dependencies

Accepting implementation must explicitly select the SC0-only initial host scope,
H0's trusted constructor, the permanent stdio CLI, one-worker concurrency and
bounded profile/limits, exact startup trust boundary and the truthful controlled
outcome. It must also accept that H1 does not finish the original HP3 roadmap.
The parent coordination document owns the exact J2 API checkpoint and shared
Cargo integration. Acceptance of this proposal would not authorize live providers,
credential access, changing the active supervisor, deployment or cleanup.

| Deferred owner/scope | Remaining consequence |
| --- | --- |
| Later HP3 host slice | Retained-session remediation, supported findings/evaluation host commands, correction/succession, exact large-history success and its accepted completion budget; broaden only when the relevant domain owners are ready. |
| Campaign profile owner | General admission beyond frozen SC0, retry evidence consumption and exact supported whole-operation profiles. |
| HP4 | Actual supervisor/capability dispatch, real provider/credential adapter, realized read-only and provider identity evidence, permanent executable startup/restart/deployment owner and enabled route closure. H1 startup validates its selected process; it is not that complete deployment owner. |
| R4/R5 and HP5 | Import versus authorized archive/clean start, old-root fencing, rollback/roll-forward, whole-root authority transfer and obsolete-source/export cleanup. |
| Compiler verification | Actual selected external verification/AEG dependencies and broader role profiles when required; pure projection does not retire them. |
| Lifecycle and UI | Their separately accepted context/effect accounting and consumer integration; child exit never settles lifecycle operations. |

The controlled CLI path uses linked Rust compiler/episode/claims/campaign code
and E1. It performs no Node review call or per-command episode spawn. That is the
only replaced consumer route claimed here. The Node native host, capability
registrations, real provider adapter, scripts and other roots remain preserved
and unretired. Their deletion requires the later bounded exhaustive consumer
inventory and state/operational authority. Offline development requires neither
dual running nor an availability bridge.

## Evidence and planning limitations

Tier 2 Verify used graph project `home-bline-code-work-engine`, generation
`2026-10-08T19:06:54Z`. Exact-path coverage for all relied-on source/documents
reported `metadata_match` / `no_recorded_issue`; this is best-effort, not proof of
completeness. The later exact native-closure check used generation
`2026-10-08T19:15:30Z` and also reported matching metadata/no recorded issue.
Narrow campaign/E1/compiler/episode/native-host searches were
exhausted. A broad compiler search returned 60 of 69 rows and was replaced by an
exhausted 43-row pure-library scope; it supports no whole-compiler absence claim.
Bidirectional depth-one traces of campaign dispatch consumption and the Node
owner factory were untruncated. Injected/dynamic edges are incomplete, so exact
source establishes the material composition; a zero-caller result establishes
no absence. One guessed legacy path failed and was corrected by graph discovery.

The plan is architectural inference from these APIs, not runtime evidence.
Hashes, exact coverage receipts, query/trace summary, actual planner thread ID,
baseline comparisons and plan digest live under
`/home/bline/.local/state/work-engine/rust-j2-hp3-planning-20261008/hp3/`.
Only this plan and that external evidence are owned by this lane. No product
edits, builds, tests, providers or Git publication were performed.
