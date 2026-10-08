# Bounded Rust artifact integration with executable generations

## Current disposition

After the C3/R3 responsiveness qualification failed, the user directed planning
toward direct Rust host and consumer replacement and away from large temporary
infrastructure unless there is a specific reason for it. The
[host replacement plan](rust-host-replacement-plan.md) now owns the proposed
next boundary. This earlier Node-manager integration proposal remains a
reviewed alternative, not the next implementation recommendation or an
authorized implementation.

Its requirements for exact artifact identity, captured configuration, generation
selection, draining and controlled reconstruction remain relevant. Its proposed
two-executable packaging layout and Node consumer handoffs should be reassessed
against the selected Rust boundary before implementation. A retained temporary
handoff needs a named consumer it unblocks, a reason direct replacement cannot
reasonably provide that result, bounded scope and a removal condition.

The user has also clarified that Work Engine is unused during this migration.
Keeping the Node system operational is not a requirement, and Work Engine is not
being selected as infrastructure for developing its own replacement. An offline
build and deliberate cutover may therefore replace the transitional deployment
route proposed below. Future executable identity, restart and recovery contracts
still need a permanent owner; zero-downtime coexistence is not an acceptance
condition for this migration. Saved state still requires explicit disposition.

## Original bounded proposal

Prepared 2026-10-07. **Planning only.** The user agreed to plan a small shared
generation integration slice while retaining the Node generation manager. This
session owns that planning work; no separate generation team is assumed. The
coordination reference is chatboard 1254, claim
`a8051157-4cc6-4916-8ae5-cfc08e20769f`. This document authorizes no implementation,
build, test execution, provider entry, database access, activation or commit.

The implementation starting point is joined C2/R2 commit
`bcbc52eb071270dc56d4dc65489b26b269506604`, plus the exact accepted C3/R3 candidate
when available. The reviewed [C3 plan](rust-compiler-c3-plan.md), SHA-256
`cbfdb2172967fe8288b7dd78b4850c899c20c7d4465e81886645aa78e6731421`, and
[R3 plan](rust-review-episode-r3-plan.md), SHA-256
`905d2615d46d7e604e072d5a13b340ea7d88bfc09c2a8df0d448687046ea6a18`, retain their
component contracts. Their bounded implementation was separately authorized;
their final binaries and results are not inferred from planning or work in
progress. [Rust direction](rust-development.md), [model guidance](model-selection.md),
[DESIGN](../DESIGN.md) and [PHILOSOPHY](../PHILOSOPHY.md) govern placement.

## Intended result and boundary

An explicitly configured generation captures two independently selectable Rust
domain executables and a saved trusted selection. The actual worker manifest
consumer and the actual outer native-review host use the captured selections on
startup and reconstruction. Removing mutable build output, changing ambient
backend variables or changing cwd cannot redirect them. A corrupt, missing or
incompatible selected artifact refuses before dependent work.

The Node `ExecutableGenerationManager` retains fencing, draining, worker
construction, compatibility checks, activation, store CAS and retirement. The
new work supplies artifacts and selection to that owner; it does not recreate
its state machine in Rust. Compiler semantics remain C3-owned, episode semantics
and native admission R3-owned, and claims/campaign/provider authority stays with
its current owners. Backend selection grants no review authority.

This slice qualifies provider-disabled scratch deployments. It does not qualify
live startup, migrate a production root, enable RC correction routes, repair
general generation activation ordering, reconcile executable and operational
locations, or implement a broad Rust generation-manager rewrite. R5 retains
episode root adoption, build-binding changes and operational cutover. The
[location v2 draft](../planning/executable-generation-location-reconciliation-plan-v2.md)
and its [review disposition](../planning/executable-generation-location-plan-v2-review-disposition.md)
are unresolved proposals, not accepted authority here. In particular, physical
`workspaceRoot` still supplies operational defaults, and constructing a current
worker with an arbitrary live semantic profile is not known to be inert.

## Observed seams

| Current source | Integration consequence |
| --- | --- |
| `app-server/src/executable-generation-bootstrap.mjs`, inventories 18–82 and `roleEnvironmentSource` 157–240 | Bootstrap hydrates and projects in the outer process, then saves the document, identity/requirements bases and exact requirements. Selection must reach this first compiler call as well as worker reconstruction. |
| `app-server/src/executable-generation-snapshot.mjs`, 42–103, 114–210 | The source inventory records regular non-symlink bytes, size and executable bit; copied executables become 0700. Generated files become non-executable 0600. A Rust binary belongs in source inventory, never `generatedFiles`. |
| Bootstrap 282–398 | The current workspace is captured and compared with persisted generation identity before selection; the worker forks a snapshot entry with inherited environment and separate cwd. Existing recovery is not an arbitrary snapshot-only boot facility. |
| `app-server/src/executable-generation-role-environment.mjs`, 257–287 | Worker projection uses snapshot delivery paths and saved identity/requirements bases, without rehydrating ambient source. Preserve this distinction and saved requirement values. |
| `app-server/src/default-executable-generation-worker.mjs`, 68–162 | Role-environment construction happens at module initialization. Scratch-only stores, explicit fake transports and an absent/live-disabled semantic profile are necessary for the bounded proof. |
| Bootstrap 439–451; `app-server/scripts/app-server-proxy.mjs`, 263–280 | The host-effect factory runs in the outer proxy, after worker activation, and currently receives only workspace/state roots. Inventorying an R3 binary in the worker alone does not select it in this host. |
| `services/slice-campaign/capability-host-runtime.mjs`, 30–82; `native-review-host.mjs`, 238–276 | Capability-host construction injects native owners. The native factory derives runtime source from its own module origin and loads `app-server/runtime-manifest.yaml` and `app-server/reviewer-profiles.yaml` once into its manifest/profile registry. It needs explicit compiler and episode selection plus captured configuration identity through an owned handoff. |
| Bootstrap 453–479 | One outer host runtime survives ordinary worker replacement, and the snapshotter reuses `source.generatedFiles` captured at bootstrap. A changed binary, outer-host module or cached configuration input cannot be treated as a worker-only reload. Same-byte artifacts at a different snapshot path are a different case. |

These are bounded source observations, not a claim that every host module or
runtime dependency is already in a snapshot. Node, its native/runtime features,
installed package resolution, the outer JavaScript closure, and any C2 Python
verification remain explicit runtime dependencies until individually bound.

## Artifact and selection contract

Use one narrow versioned packaging record, proposed
`app-server/generated/executable-rust-artifacts.json`, outside semantic manifest
and environment payloads. It describes the closed source inventory and trusted
backend selection, including the inputs cached by the outer host and bootstrap;
it is not an authority token. Its schema rejects unknown versions,
duplicate/conflicting domain selections, absolute or escaping inventory
paths and unsupported launch profiles. The selected record is captured as a
generated non-executable file and therefore contributes to generation identity.

The record's `capturedConfigurationInputs` identifies each consumer, its actual
inventory-relative source path, size and SHA-256. Include the outer host's
`app-server/runtime-manifest.yaml` and `app-server/reviewer-profiles.yaml`, plus
the bootstrap's selected manifest and any other source configuration used to
construct its saved role-environment values. If the two manifest consumers use
different paths, record both; a default filename does not establish equality.
Bind the saved `executable-role-environment.json` digest and its source-input
identities as well. Record actual read origins separately from semantic identity
bases. Module and binary equality alone cannot establish configuration equality.

Each domain is explicit: compiler `legacy` or `rust`, episode `javascript` or
`rust`. An omitted packaging record retains the current unselected behavior.
Once a record selects a domain, neither ambient environment nor an absent field
may select or redirect it. Cover all four combinations in integration evidence.
If explicit legacy is not yet representable through C3's trusted API, that is a
C3 handoff, not permission to rely on clearing environment variables.

| Runtime artifact | Proposed source-inventory path | Consumer selection |
| --- | --- | --- |
| Package `work-engine-compiler-cli`, binary `work-engine-compiler` | `app-server/bin/work-engine-compiler` | `{ binaryPath, expectedSha256, protocolVersion: 3, launchProfileId: 'manifest-sync-v1' }` supplied as `compilerSelection` to C3 projection/loading; retain it for satisfaction on the constructed manifest. |
| Package `review-episode`, binary `work-engine-review-episode` | `app-server/bin/work-engine-review-episode` | R3 `reviewEpisodeRust: { binaryPath, expectedSha256, root, timeoutMs }`; profile `native-host-v1`, envelope 2, codec `review-episode-js-json-v1`. |

Persist inventory-relative artifact paths, expected SHA-256, size, executable
mode, domain/protocol/profile and build-provenance reference. Resolve to absolute
paths only against the validated selected snapshot. Reject symlink components,
nonregular files, path escape, changed bytes/mode and profile/platform mismatch.
The owner-private source stage is stable during capture; perform final checks
against the snapshot, not only the input record. Domain adapters retain their
own exact-byte capture and response checks. No runtime discovery through `PATH`,
`rust/target`, an arbitrary environment binary or an external fallback is added.

The build record binds the immutable source subject and any exact candidate
patch/tree, package and dependency graph, complete `Cargo.lock` digest, root and
package manifest digests, toolchain file and observed `rustc -Vv`/Cargo identity,
target triple, build profile, features, relevant build flags, build command,
artifact digest/size/mode, and actual native runtime dependencies. Default release
and fault-enabled artifacts are distinct. Production selection rejects the fault
profile. The initial profile is the qualified Linux target actually built; no
portable or statically linked claim follows from the word "Rust". Observe ELF
interpreter/linked libraries and bind their supported runtime contract, or fail
when unavailable. Pin the Node executable/version and required package versions
and bytes in the proof receipt; include `yaml` wherever the selected manifest
module imports it, independently of whether a semantic profile is present.

Build and transport identity remains outside the historical compiler manifest,
role-environment, requirements and review-episode digest inputs. A binary change
changes generation/package identity; it does not silently redefine semantic
identity, repair a role fingerprint or rehash an old episode. Preserve delivery,
identity and requirements bases exactly, with actual physical origin recorded
separately. Existing C2 hydration needs its own trusted compiler selection; C3's
v3 selector alone does not prove that the v2 source-verification route was pinned.
Its Python/runtime and source-verifier closure remain attributable prerequisites,
not concealed transitive dependencies.

## Staging, reconstruction and host handoff

The bounded build/stage mechanic consumes an exact component handoff and copies
the two selected release artifacts into an owner-private clean source staging
root. Cargo output is an input to staging, never the runtime location. It verifies
the source artifacts before and after copy and records source/build identities.
Use the repository's pinned toolchain and locked release builds in isolated target
directories. No component builder's evidence binary is overwritten by a joined
build. Shared manifests/lock remain under the S2/S3 integration owner's serialized
handoff; adding the small Rust staging/test package below needs registration, not
a dependency upgrade or a new deployment framework.

Bootstrap receives a trusted `rustArtifactManifestPath` option; the proxy may
expose one explicit corresponding flag. Parse and preflight it before launching
the delegate or constructing a role environment. The configuration is outside
authored role/capability inputs. Validate each selected domain and the scratch
native root marker before hydration, worker spawn or outer host-store creation.
On a stored generation, validate its saved selection and compare it with the
candidate before allowing a changed selection to cross construction. A missing
saved record cannot silently adopt the current shell's backend choice.

At capture, include both selected source binaries, the selection/provenance data,
the recorded configuration source bytes, all newly imported worker adapter
modules, and their required package files in the explicit inventory. Derive the
exact transitive module list from the frozen C3/R3 source and give it a bounded
verification test. Do not treat a successful
import from an ancestor `node_modules` as proof of closure. Preserve the existing
snapshot manifest format unless a demonstrated missing property requires a
separately reviewed format change.

After capture or stored-snapshot reconstruction, a small Node selection resolver
validates saved data against the actual inventory and returns frozen selections.
Worker projection gets the compiler selector through saved role configuration;
it never uses an absolute build-directory path saved at staging time. The outer
bootstrap's hydration/projection gets the equivalent trusted staged selection,
including the separate C2 route. Compare observed selected bytes and semantic
results across those two locations.

Extend the existing host-effect factory argument with a validated, frozen
selection context containing selected generation/source identity, snapshot root,
resolved compiler selector, resolved review selector, captured configuration
identities and the captured record digest. The proxy forwards it through
`createSupervisorCampaignCapabilityHostRuntime` to `nativeReviewOwnersFactory`;
R3 applies its native-factory changes after an
explicit serialized handoff. The outer native manifest projection receives C3's
selector, and the review episode service receives R3's selector. This context
cannot originate in a reviewer capability request.

The R3 builder has reported the additive `compilerSelection` native-owner option
implemented in its isolated candidate and forwarded to `projectRuntimeManifest`.
That is a supplied integration seam, not final reviewed composition evidence;
the generation/proxy/capability-host forwarding remains this slice's work.

The outer JavaScript host is still loaded by the proxy. Its qualified closure
includes modules/imports and the non-module configuration captured into the
manifest/profile registry. This slice checks their actual origins and captured
byte identities at startup, and tests without ancestor-package fallback. It does
**not** describe the entire outer host as snapshot-executed or sealed. A host-code change requires
a fresh qualified outer process; runtime imports from a mutable unrelated tree
leave the claim unmet. Broader relocation or inert-host construction is separate
work. If the exact selected host cannot be exercised from a controlled source
and scratch targets without those changes, report that bounded integration
block rather than claim fixture packaging as production-consumer evidence.

Within this bounded profile an unchanged selection can survive a worker reload:
compare domain/backend, artifact bytes/mode, protocol/profile, runtime contract
and native root binding, qualified outer modules and captured configuration
inputs, not expanded absolute snapshot paths. Before reusing bootstrap's
`source.generatedFiles`, read and compare the current configuration source
identities against the initial captures; comparing a cached generated value to
itself cannot detect drift. Missing or changed input refuses before candidate
construction. The worker may use the successor snapshot's same-byte artifact
while the existing outer host
keeps its original validated path. Retain that referenced artifact directory
until the outer host closes and record both physical origins. A changed selected
binary, root binding or qualified outer-host closure, including either cached
YAML input, refuses before candidate construction through the bootstrap/snapshotter
boundary; it is not silently
activated with the old outer service. This is a bounded unsupported-handoff
refusal, not a replacement deployment policy or a new manager state machine.
A configuration change requires a fresh qualified outer process and the existing
generation/semantic compatibility checks; it does not authorize automatic
acceptance, rebinding or configuration refresh in the retained host. Refusal
preserves the old worker configuration, outer manifest/profile registry and
saved generation identity.

## Native-root and failure boundaries

R3 initializes only a new empty private scratch root with the selected binary's
`init-native --root ABS`, then opens it through `native-host-v1 --root ABS`.
Its `.review-episode-native-host-v1` marker binds canonical root, profile,
protocol 2, domain codec, executable SHA-256 and selection digest. Generation
integration validates that binding and saves the expected root identity; it
does not recreate or edit the marker. Each R3 operation still carries its
host-derived request/authority bindings over inherited read-only FD 3. Packaging
does not create those admissions or expose the descriptor to the reviewer.

Restarting the same root with identical artifact bytes is supported. A new
review binary on that root is a binding mismatch, even if its semantic codec
is unchanged. Do not reinitialize, rewrite the marker, open a legacy writer or
adopt a copy to make startup succeed. R5 owns that later transition.

Use existing `ExecutableGenerationStartupError` classification, with a bounded
artifact/selection detail such as missing, digest/mode mismatch, unsupported
platform/profile, invalid inventory or root-selection conflict. Refusal reports
the expected and observed artifact identities and stage without secrets. It
does not yield a projection, episode result or provider admission, and does not
retry through legacy. A post-launch episode timeout or malformed reply may mean
an unknown commit; keep R3's operation identity and reconciliation semantics.
Generation startup failure cannot turn it into "no write" or a reviewer finding.

An invalid candidate leaves the existing saved generation/selection and native
marker untouched. Do not repair mismatches by deleting durable generation state.
Scratch rollback can explicitly select legacy on a different new scratch root;
the Rust root remains preserved for exact readback. Closing processes and
reselecting an earlier artifact is not proof of safe rollback after new writes.
Live rollback and activation remain outside this slice.

## Bounded file ownership

Recommend one retained `gpt-6-sol`/high builder to own this integration through
ordinary remediation after plan acceptance, and one separately established
`gpt-6-astra`/xhigh reviewer for the composed candidate. This recommendation does
not launch agents or establish cross-provider independence. Requested and
observed model configuration belongs in the eventual result record.

| Owner | Proposed edits and handoffs |
| --- | --- |
| Generation integration | `app-server/src/executable-generation-bootstrap.mjs`: selection preflight, source/configuration inventory, saved record, first compiler selection, outer factory context and configuration-drift check before cached role config reuse. `app-server/src/executable-generation-role-environment.mjs`: resolve saved selector while preserving bases/requirements. New `app-server/src/executable-generation-rust-artifacts.mjs`: strict record validation and existing-runtime glue. |
| Generation entry/glue | `app-server/scripts/app-server-proxy.mjs`: explicit trusted input and preflight/forwarding. `app-server/src/services/slice-campaign/capability-host-runtime.mjs`: additive forwarding to the native owners factory. Bounded worker changes only if needed to transport saved config; avoid changing lifecycle construction semantics. |
| Snapshot owner, same builder | `app-server/src/executable-generation-snapshot.mjs` only if a demonstrated validation gap needs a small fix; its existing source executable support is the starting implementation. `app-server/tests/executable-generation-snapshot.test.mjs` and `executable-generation-worker.test.mjs`: meaningful compatibility additions. No manager/store algorithm rewrite. |
| Rust integration mechanics | New small package `rust/crates/generation-artifacts/{Cargo.toml,src/lib.rs,src/main.rs,tests/generation_integration.rs}` for bounded build-output staging/provenance and the actual-process proof driver; use existing pinned JSON/hash dependencies. It is a build/test tool, not a third selected runtime service, scheduler or durable state owner. |
| Transitional real-consumer driver | New `app-server/tests/fixtures/generation-rust/host-driver.mjs` and exact fixture data. Node glue invokes real bootstrap/worker/native-host APIs with controlled transports; Rust owns new process orchestration, failure injection and observations. No parallel Node staging framework. |
| C3 retained owner | `runtime-manifest.mjs`, `rust-manifest-adapter.mjs`, `rust-compiler-adapter.mjs` and compiler packages. Supply final module closure, explicit legacy/Rust trusted selector, v2 hydration propagation and v3 selector observations. Any missing API is an agreed bounded C3 edit, not a concurrent generation edit. |
| R3 retained owner | `native-review-host.mjs`, `native-review-closure.mjs`, review `rust-adapter.mjs`, `app-server/src/index.mjs` and episode packages. Apply compiler/review selection at the native factory, preserving current source/identity semantics and RC fences. Supply final marker/admission contract, module closure and observed cached-configuration inputs. |
| Shared root owner | `rust/Cargo.toml`, `rust/Cargo.lock`, toolchain/package-lock changes if any. Register the small package after S2/S3 handoff; no speculative new dependency. Recheck exact joined lock rather than assuming main equals either lane. |
| Integration receipt | New `docs/rust-generation-integration-result.md`: exact candidate/build/closure identity, matrix observations, failures and remaining scope. Parent owns `docs/rust-migration-plan.md`; this planner owns only this plan. |

The new package is justified by the Rust-first build/test direction and its
generation ownership. It should contain only mechanics required by the proof;
there is no common process, persistence or artifact registry framework to build.
Its initial paths and schema are concrete proposed placement, subject to plan
review; component adapter semantics stay in their existing owners.

## First vertical and acceptance evidence

Start with one clean exact-candidate source fixture, immutable compiled-role
inputs, a new generation-state root and a new R3 native root. No live profile,
credentials, provider transport, production DB copy or shared operational target
is available to this proof. Construct the real bootstrap and real worker from
their captured inventory, while the real outer native host uses controlled
campaign/claims stores and R3's controlled peer. Invoke one compiled manifest
projection/satisfaction path through the worker and one admitted review-episode
operation through the outer host. Observe actual executable digests, paths,
protocols/profiles, process exits and downstream consumer results in each
location. Calling an adapter directly or fabricating matching metadata is not
the generation proof.

Close the worker and outer host, then start a new OS process against the same
controlled source fixture and saved state. Prove that saved selections reconstruct
without ambient binary resolution and read back the exact episode and manifest
identities. Remove the isolated mutable Cargo target, change cwd and set hostile
ambient backend/binary values between runs. Keep the closed staged source itself
available because the current bootstrap revalidates it. Separately exercise a
same-byte worker successor at another snapshot path and show both consumers
still use the same content/profile selection. Do not call a fixture-generated
snapshot an actual consumer restart unless the real bootstrap path consumed it.

| Matrix | Required observation |
| --- | --- |
| Four backend combinations | Explicit legacy/JavaScript, compiler-only, episode-only and both; selected Rust observations match pinned artifacts. Ambient values cannot turn an explicitly unselected domain on. |
| Startup and restart | First startup, persisted reconstruction and new host OS process; saved selector/inventory/digests agree, exact semantic identities survive, no provider entries. |
| Origin and closure | Worker import and binary paths come from the selected snapshot; outer host origins and qualified closure are recorded separately. Missing adapter/package dependency fails without ancestor/source-tree resolution. Node/native runtime mismatches refuse or remain explicitly unqualified. |
| Relocation and reload | Removed Cargo output and changed cwd do not matter; same-byte snapshot relocation with unchanged captured configuration is accepted. Changed selected bytes/profile/root or outer closure, including cached configuration, is explicitly unavailable for worker-only reload, before candidate effects. |
| Cached-configuration drift | Independently change the outer host's `runtime-manifest.yaml` and `reviewer-profiles.yaml` while binary and JavaScript bytes stay identical; also cover a distinct bootstrap-selected manifest path. Invoke the real reload path and observe refusal before candidate construction/effects. Verify unchanged saved generation, worker role configuration, outer manifest/profile registry and entry counters; cached generated bytes cannot conceal the changed input. A fresh outer process remains subject to existing compatibility checks. |
| Artifact failures | Missing, truncated, wrong digest/size, non-executable, symlink, FIFO/device, escaping path, malformed/unknown selector, unsupported target/profile and wrong protocol. No worker delivery, host operation or fallback writer. Observe refusal timing against counters/store snapshots. |
| Identity separation | Equal compiler document, environment and receipt bytes/hashes under saved identity/requirements bases; delivery relocation changes no historical semantic hash. Changed semantic input still changes/refuses through existing owners. C2 hydration and outer direct unhydrated projection retain their distinct contracts. |
| Root safety | Same R3 marker/build reopens; different binary/profile/root marker refuses without DB/marker mutation or JS opening. Existing R3 unknown-outcome reconciliation remains available and no RC-dependent route becomes enabled. |
| Candidate isolation | Real current construction with controlled scratch state and fake transports; record all writes, timers and entries that occur. An invalid selected artifact fails before those consumers. Evidence does not claim live-profile inertness or solve the proposal's activation-order defect. |
| Compatibility | Focused existing generation snapshot/worker tests and C3/R3 real-consumer regressions on exact clean fixtures. Preserve unrelated dirty builder source/pin drift; report its known eleven baseline Node failures separately if rerun. |

Freeze the exact build/lock, selectors, test inputs, controlled roots and observer
versions in the result. A test of a fake packaging implementation cannot replace
the actual bootstrap/native-host observations. The shared C3/R3 HOST profile
remains the qualification owner for p99 control-route delay at most 50 ms and
maximum at most 250 ms. Record packaging/preflight/reconstruction overhead
separately; keep it out of every manifest/episode hot call unless a demonstrated
invariant requires a per-call check. If the integration changes a measured
consumer path, requalify that path under the same frozen profile and disclose
failures; do not invent a more permissive budget.

Future gates are the Rust package's focused staging/process tests, the existing
Node snapshot/worker and component integration tests, locked release builds,
format/Clippy checks appropriate to changed Rust packages, and the affected
joined HOST cases. A command name or prior component test count is not new
evidence. Separate review focuses on selection completeness, both process
locations, artifact/runtime closure, root binding, early effects and reload
split-brain. Completion requires all material findings resolved or an explicit
truthful boundary disposition; offline component readiness and generation
qualification remain separately reported.

## Remaining handoffs and evidence limits

Before implementation, accept this bounded slice and freeze the final component
subject. C3 supplies the trusted selector for v2 hydration and explicit legacy,
plus final observed v3 metadata. R3 supplies its final native-factory injection
and root binding (the compiler option above is reported implemented). The
generation builder then owns the exact inventory, staged artifact selection and
two-location proof. S2/S3's integration owner serializes
package registration/root-lock changes. These are concrete handoffs, not a
requirement to wait for the complete lifecycle migration. Binary digests,
platform/runtime dependencies and the exact joined source/lock are execution
facts to freeze before qualification, not placeholders to infer now.

Planning used Tier 2 Verify on `home-bline-code-work-engine`, confirmed ready;
coverage metadata generation `2026-10-07T19:42:07Z`, recorded
`2026-10-07T20:10:02Z`. The parent supplied verified generation/manager/snapshot
and host paths. This pass used exact symbol discovery, an untruncated
both-direction bootstrap trace, the role-source snippet and direct source
verification of the material factory/configuration seams. Broad test discovery
was narrowed to the two named existing tests; no exhaustive test claim follows.
Exact-path coverage for relied-on source and planning documents reported matching
metadata and no recorded gaps. `rust/Cargo.lock` was reported untracked by the
graph, so lock/build facts remain a future exact-byte handoff; no lock-content
claim is made. An initially guessed `app-server/package.json` was absent; the
actual dependency source is root `package.json`. Graph evidence is best-effort,
not a claim of complete runtime closure.

Only this planning document was written. No product code, tests, build artifacts,
database, provider, live runtime, branch or index was changed by this planning
work. The plan claims no present deployment or activation state.
