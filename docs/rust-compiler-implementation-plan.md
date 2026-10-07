# Rust compiler implementation plan

Proposed 2026-10-07. This plan covers deterministic skill, runtime-manifest and
run-extension compilation, including the source-verification adapter needed by
current consumers. It is planning evidence, not implementation acceptance,
deployment authority or a new owner for authored instructions. Implementation
starts only after the user accepts a bounded unit and the relevant interfaces.

The [Rust direction](rust-development.md), [migration plan](rust-migration-plan.md),
[design](../DESIGN.md) and [philosophy](../PHILOSOPHY.md) govern the proposal.
Lifecycle and UI remain separately coordinated. The lifecycle integration owner
owns S0, root manifests, lockfile, toolchain, common types and common gates;
this lane requests changes there. It does not create a competing workspace.
UI U0 remains at lifecycle S3 and controlled read-only U1 at S4.

## Result and boundary

The intended result is one Rust compiler library with explicit skill, manifest
and extension models, plus a small executable that resolves inputs and performs
bounded local verification. Existing JavaScript entry points become temporary
compatibility adapters. Declarative YAML/JSON and generated Markdown remain
usable, and supported v1 artifacts retain their required bytes and identities.

Compilation establishes mechanical consistency, source correspondence and
declared ceilings. It does not establish semantic adequacy, instruction quality,
model judgment, production authority, provider admission or campaign acceptance.
The semantic context compiler/verifier and claim inference are different owners;
their use of the word “compiler” does not put them in this lane.

| Current boundary and evidence | Rust disposition and actual consumer |
| --- | --- |
| [compileSkill](../app-server/src/skill-compiler.mjs) parses strict YAML, validates structure/interface, renders exact Markdown and projects requirements; its verified branch reads source bytes and calls AEG. | Pure skill model/rendering in the library; source resolution and AEG invocation in the executable. Preserve the async entry used by [compile-skill CLI](../app-server/scripts/compile-skill.mjs), manifest hydration and extension compilation. |
| [runtime-manifest](../app-server/src/runtime-manifest.mjs) loads YAML, hydrates role/secondary requirements, resolves paths, projects role templates and checks requirement satisfaction. | Move these deterministic transformations/checks; retain the JavaScript `RuntimeManifest` facade and `ManifestRoleRuntime` delivery owner during transition. A satisfaction receipt is evidence of checks, not a grant. |
| [generation bootstrap](../app-server/src/executable-generation-bootstrap.mjs) hydrates requirements and saves the role-environment document; [generation role environment](../app-server/src/executable-generation-role-environment.mjs) reprojects it with snapshot delivery paths. | Both consume the replacement through the manifest facade. Their snapshot, activation, custody and provider behavior stay with their owners. |
| [operator switchboard](../app-server/scripts/operator-switchboard.mjs) calls `loadRuntimeManifest`; [native review host](../app-server/src/services/slice-campaign/native-review-host.mjs) directly projects a manifest without hydration. | Exercise both paths; do not make “all manifests are hydrated” a new assumption. Review orchestration and episode persistence stay outside this lane. |
| [extension compiler](../app-server/src/run-extension-bundle-compiler.mjs) checks the host-supplied admitted revision/allowlists, hashes inputs, compiles skills with `verifySources: false`, and emits an attachment. | Pure extension policy checking and descriptor assembly after resolved input capture. Bootstrap remains attachment/snapshot owner; [registry](../app-server/src/run-extension-registry.mjs) remains tool-dispatch owner. |
| [AEG adapter](../app-server/src/agent-environment-graph-adapter.mjs) invokes Python `project-role-machine` and validates its envelope. [Python owner](../skills/agent-environment-graph/scripts/agent_environment_graph.py) compares the exact canonical candidate and generates the closed projection. | Preserve that Python ownership and protocol. Replace invocation mechanics, not graph semantics, authoring, canonical-role judgment or generated site ownership. |

## Placement and alternatives

Proposed packages, not existing APIs:

- `rust/crates/work-engine-compiler/`: `src/skill.rs`, `manifest.rs`,
  `extension.rs`, `codec.rs`, `error.rs` and `lib.rs`; domain types and pure
  operations, with no filesystem, Git, subprocess, database or provider access.
- `rust/crates/work-engine-compiler-cli/`: binary `work-engine-compiler`,
  `src/main.rs`, `protocol.rs`, `source_snapshot.rs`, `aeg.rs`; bounded request
  handling, source reads, path resolution, Python adapter and explicit output.
- Tests belong to each package under `tests/`; the core owns
  `tests/fixtures/v1/`. A temporary `app-server/src/rust-compiler-adapter.mjs`
  provides only the necessary Node interoperation, not another semantic engine.

A single Rust library keeps mutually used v1 models/codecs together without
creating a universal instruction IR. Separate private validated types distinguish
role skills from secondary skills, source verification from unverified output,
and requirements from manifest grants. Closed wire vocabularies become exhaustive
enums; genuinely open text remains text. Validation errors carry domain/code/path
and useful source spans instead of spreading exception-string logic.

Alternative placements were considered against the consumers above. Putting this
inside lifecycle would couple independent compilation to runtime authority. Moving
the Python AEG owner would turn a bounded compiler migration into a separate graph
service rewrite. One crate per current JavaScript file adds packaging without a
separate ownership need. Native Node bindings or WASM avoid some process startup
cost but add ABI/toolchain and host-I/O complexity before there is evidence it is
needed. A one-shot executable is the proposed initial bridge; its cost is measured
at real compilation boundaries rather than assumed to be faster.

The core takes resolved bytes and explicit path identities; it does not discover
the current directory or repository. The adapter reads declared inputs, and binds
verification to the exact bytes consumed. Hash comparison and contiguous UTF-8
byte-span checks are pure operations. AEG closure is a separately attributed
observation from the existing owner. A request cannot obtain a verified result
merely by deserializing `verified_sources: true`. Typed states prevent accidental
mixing; they do not prove the trustworthiness of an external process by themselves.
Revision/allowlist inputs come from the existing host, not from an inferred Git
status or a bundle's self-asserted authority.

## Compatibility to freeze before replacement

| Dimension | Required outcome and boundary case |
| --- | --- |
| Skill v1 | Preserve ordered sections, exact frontmatter/body bytes, LF and final-newline behavior, raw input/output SHA-256, byte offsets, role-free omissions and `experimental_non_authoritative` status. Reject duplicate YAML keys/aliases, unsupported fields, invalid authority references and ceiling contradictions. Cover Unicode/multibyte source spans. |
| Source verification | Match canonical prefix, contiguous spans through EOF, every source-binding digest, pinned projection digest, complete projection equality, Python backend digest and canonical-role match. Unverified extension compilation stays unverified and cannot satisfy production requirements. |
| Encoding | Name the legacy compiler encoding profile explicitly, e.g. proposed `compiler-js-canonical-json-v1`: recursively sorted JavaScript UTF-16 keys, JavaScript scalar rendering, no trailing newline. Use separate domain wrappers for requirements, manifest environments and extensions even when bytes coincide. Unknown profile/version fails. No migration to the new common codec is implicit. |
| Different digest inputs | Skill inputs hash raw UTF-8 bytes; `extensionDigest(structureSource)` hashes the canonical JSON string, including quotes/escaping. A manifest loaded from disk hashes source bytes; direct projection without `sourceSha256` hashes the document. These are not interchangeable. |
| Extension bytes | The descriptor contains compiled `Buffer` output. Existing canonicalization traverses its enumerable byte-index keys, whereas JSON serialization and structured cloning can produce different representations. Capture the compile, persistence, restart and registry-recheck forms separately before choosing the bridge encoding; a proposed base64 wire field alone is not compatible. |
| Manifest identity | Keep identity base, delivery base and requirements base distinct. Preserve role/skill sorting, defaults, exact contract input, compiled fingerprints, capabilities/effects and continuity checks. Relocation to a snapshot changes delivery paths without changing canonical environment identity. |
| Provenance | Rust reports its own implementation identity. Proposed IR `compiler` is `work-engine.skill-compiler.rust-v1`, an explicit C0 compatibility delta; retain v1 IR shape otherwise. Compare output/requirement hashes exactly while excluding only this accepted producer field. Record binary/build identity in the transport envelope, outside historical digest payloads. Never claim the Rust process was the JavaScript bootstrap or rewrite Python's backend identity. |
| Errors | Preserve rejection classes and refusal before delivery/publication. Exact incidental exception text, invalid-input coercions and JavaScript object layout are not automatically product invariants. Any widened/narrowed admitted input set is separately described and accepted rather than hidden as “parity.” |

Golden vectors include non-BMP versus BMP key ordering, escaping/control characters,
numeric rendering and absent/null fields where actually admitted. Unknown-field
handling is versioned per format; extension records currently retain some opaque
fields, so blanket `deny_unknown_fields` is not a substitute for characterization.
Current revision inclusion and forbidden-effect checks are compatibility facts,
not proof of stronger repository binding or an exhaustive security policy.

## Slices and ownership

The implementation owner can use the [model guidance](model-selection.md): one
retained Sol/high builder for a clear slice, Astra/xhigh for unresolved boundary
judgment, and a separate reviewer where the owning acceptance requires review.
This document does not launch those roles or establish review independence.

### C0 — Interface and corpus preparation, independent of lifecycle execution

This is code-free source characterization and fixture specification, not a new
test harness or Rust workspace. It can proceed before S0 under planning authority:
record supported v1 cases, exact material input revisions, observed versus intended
behavior, the producer-identity delta and the source/path trust boundary in this
document. Fixture execution/capture and executable test infrastructure begin in
C1 under its implementation grant. No paid/provider run is needed.

Request from the S0 owner: package names/member registration, toolchain/MSRV,
dependency/features agreement for YAML, Serde, hashes, typed errors and tests,
and ownership of any already-shared values. Ask that owner to accept the compiler
gate contribution. Keep the legacy compiler codec domain-local initially;
neither a new common digest abstraction nor a generic runtime/store is prerequisite.
YAML library selection is open until its duplicate-key, alias, scalar and source
span behavior passes the characterized v1 cases; this plan does not guess a version.

Exit is an accepted interface/file boundary for C1. Root workspace edits remain
with S0. Before that handoff the compiler lane can refine this plan and corpus
specification, but does not make its own root manifests or dependency lockfile.

### C1 — Pure skill compiler and bounded executable

Depends on accepted C0 and S0 package registration/interfaces, not S1–S10 runtime
completion. First implementation unit: the two proposed package manifests under
S0 registration; core `lib.rs`, `skill.rs`, `codec.rs`, `error.rs`; executable
`main.rs`, `protocol.rs`; `tests/skill_v1.rs`, `tests/codec_v1.rs` in the core and
`tests/protocol.rs` in the executable; fixture directory `tests/fixtures/v1/`.

Implement parsing, validation, rendering and unverified requirements as pure
operations. The executable accepts one versioned request via stdin/EOF, writes one
bounded result envelope on stdout, keeps diagnostics separate, and has no arbitrary
command or source-verification flag that can impersonate a verified result.
All transport byte encodings are explicit. Unknown versions and incomplete or
oversized requests produce no successful artifact. No new daemon is required.

Acceptance: existing role and role-free corpus renders byte-exactly; negative
schema/authority cases fail; deterministic repeated output and codec vectors pass;
no externally supplied verified bit can construct a verified result. Rust tests
invoke the existing JavaScript implementation only as a pinned migration oracle
on identical fixtures. Differential agreement supplements contract assertions;
it does not make an incidental legacy defect correct. This first unit is useful
and independently testable, but is not completion of verified skill, manifest or
extension replacement; those are explicitly assigned to C2–C4 below.

### C2 — Verified skill closure through the real CLI and hydration consumer

Depends on C1. Own executable `source_snapshot.rs`, `aeg.rs`,
`tests/verified_skill.rs`, `tests/aeg_process.rs`; temporary Node adapter plus
bounded edits to [skill entry](../app-server/src/skill-compiler.mjs) and
[CLI entry](../app-server/scripts/compile-skill.mjs). Existing compiler/AEG tests
gain backend coverage through these entry points. Python source remains unchanged.

Resolve canonical source/binding/projection paths and read bytes once per captured
input. Invoke the configured Python owner with its existing machine protocol;
check full envelope, script digest and projection equality. Preserve current
30-second and per-stream 2 MiB bounds unless an explicit interface revision changes
them. Own the child through exit/reaping, including timeout, output overflow,
stdin failure and caller cancellation. Return no verified artifact on uncertainty.
No reusable supervision crate is required; request extraction only if another
actual consumer and the foundation owner justify a shared mechanics contract.

The async JavaScript `compileSkill` signature and `Buffer` result remain usable.
Backend selection is trusted host/build configuration, never authored YAML, and
fails explicitly if the selected binary is absent or mismatched; no silent legacy
fallback. Tests may still inject an AEG peer, but that fixture capability is not a
request field or production authority. A normal role compile uses the real Python
owner in local tests. CLI output replacement occurs only after successful complete
compilation; interruption cannot leave a partial destination advertised as success.

Acceptance: compile-skill and manifest hydration actually traverse Rust in bounded
tests; canonical role divergence, source drift, generated-evidence-as-authority,
missing paths, invalid envelopes and process failures all refuse verified output.
Cover every compiled skill in the pinned runtime manifest, not just slice-builder
and repo-search. Relate captured source bytes to the exact result; concurrent
workspace mutation either rejects or preserves that captured identity, without
claiming an atomic filesystem snapshot that this component does not own.

### C3 — Manifest projection and requirement satisfaction at current APIs

Depends on C2 and a generation-owner handoff for compiler artifact inventory.
Own core `manifest.rs`, `tests/manifest_v1.rs`; executable protocol support;
bounded facade changes in `runtime-manifest.mjs` and its existing tests. Coordinate
edits to `executable-generation-bootstrap.mjs` with its owner. The native-review
host, switchboard and generation-role consumer should retain their call contracts;
any necessary call-site edits are specifically claimed before implementation.
For the proposed pair of lanes, review-episode R3 owns changes to
`native-review-host.mjs`; a compiler-driven adjustment there is an explicit request
to that owner and integrates serially. The compiler lane owns its manifest facade,
not concurrent edits to the review host.

Move normalized projection, environment digest construction and pure satisfaction
checking into Rust. Keep role-instance projection and delivery orchestration in
the JavaScript facade until their owning domain migrates. Existing synchronous
`projectRuntimeManifest`/`satisfyRuntimeRequirements` use a bounded synchronous
one-shot bridge during this transition; async source hydration stays async. Measure
startup cost and event-loop blocking on representative manifests. If unacceptable,
return the API transition decision to consumers rather than silently making a
sync API async or adding a long-lived controller. No provider is invoked by a
manifest compile or a satisfaction check.

Generation tests pin the bridge and executable in the closed source inventory,
using a staged executable path/build digest agreed with the generation owner.
[Snapshot code](../app-server/src/executable-generation-snapshot.mjs) already copies
source executable modes; `generatedFiles` entries are non-executable. Do not resolve
a mutable ambient `target/` binary from a sealed generation. Stage/read/hash the
compiler as a source artifact and preserve its exact launch/build identity.

Acceptance: direct, loaded, hydrated and snapshot manifests agree; malformed,
missing, excess, stale-fingerprint and wrong-path requirements reject before the
fake delivery adapter is called. All compiled secondary skills stay independently
bounded by role grants. Relocation and changed ambient cwd preserve expected
identity; changed canonical inputs change it. A controlled generation reconstructs
the same result using the pinned executable after restart. No live activation.

### C4 — Extension compilation and attachment interoperability

Depends on C2 plus the C3 sealed executable handoff. Own core `extension.rs`,
`tests/extension_v1.rs`; executable protocol/source-loading support; bounded edits
to `run-extension-bundle-compiler.mjs` and existing extension tests. Coordinate the
bootstrap integration test with that owner; registry and extension lifecycle remain
their existing owners. Keep the existing extension contract codec for remaining
JavaScript consumers until its replacement has equivalent wire evidence.

Compare host-admitted revision/allowlists, sealed-run constraints, dependency and
registry checks; assemble attachments from unverified skill compilation. Preserve
`below_core`/current descriptor semantics without adding authority. The bridge
reconstructs the exact Node byte representation required by legacy digest consumers.
Acceptance includes attachment digest/canonical bytes, serialization/restart and
actual registry recheck, a rejected stale input, prohibited/unmediated requests,
duplicate registry entries and unavailable dependencies/providers. A discovered
legacy round-trip defect is a separately accepted compatibility correction, not
permission to normalize old stored attachments. Bootstrap recovery and registry
execution are tested with local fixtures and fake effects only.

### C5 — Consumer cutover and explicit predecessor disposition

Depends on C2–C4 acceptance and consumer-owner approval for the chosen release.
Compiler owner prepares the exact adapter/default-selection diff, binary closure,
source inventory, compatibility report and rollback selection; generation/release
owner controls activation. Cut over whole compilation operations, with one selected
producer per result, rather than blending old/new partial outputs.

No database import is needed for compilation. Existing IR, manifest snapshots and
extension attachments remain historical artifacts with their original identities.
Read old formats through their named profiles; regenerate a new artifact only
under its consumer's authority. Retain legacy code until the controlled consumer
corpus and release rollback are accepted; then remove superseded algorithms and
disposition internal helper tests against public behavior. The Python AEG owner,
extension dispatch/lifecycle, manifest delivery, and generation manager are retained
by design, not forgotten migration work. A later Rust host can remove the Node
bridge without changing the compiler's model or taking over those owners.

Rollback selects the previous pinned executable/Node closure for subsequent
compilations and preserves failed candidate diagnostics. It does not rewrite
existing evidence, rehash persisted attachments or reactivate a generation by
itself. If new artifacts are unreadable by the predecessor, cutover is not
rollback-ready until an explicit dual-read/disposition strategy is accepted.

## Validation commands and open decisions

These are future commands after the named targets exist; none was run to write
this plan. The S0 owner incorporates accepted selectors in the common inventory.

```bash
cargo test --manifest-path rust/Cargo.toml -p work-engine-compiler --locked --test codec_v1 --test skill_v1
cargo test --manifest-path rust/Cargo.toml -p work-engine-compiler-cli --locked --test protocol
cargo test --manifest-path rust/Cargo.toml -p work-engine-compiler-cli --locked --test verified_skill --test aeg_process
cargo test --manifest-path rust/Cargo.toml -p work-engine-compiler --locked --test manifest_v1 --test extension_v1
node --test app-server/tests/skill-compiler.test.mjs app-server/tests/agent-environment-graph-adapter.test.mjs app-server/tests/runtime-manifest.test.mjs app-server/tests/run-extension-bundle.test.mjs
node --test --test-name-pattern='sealed extension|executable role snapshots|role generations pin' app-server/tests/executable-generation-worker.test.mjs
node --test app-server/tests/executable-generation-snapshot.test.mjs
cargo clippy --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli --all-targets --locked -- -D warnings
```

Tests specify the Rust backend and exact built binary in their fixture setup once
that proposed interface is accepted; commands alone do not establish which backend
ran. Preserve selected/observed executable identity with differential results.
Acceptance also reports unresolved mismatches, fixture coverage and launch timing;
test counts or passing old tests alone do not establish migration completion.

Remaining owner decisions are the bounded C1 grant, S0 registration/dependencies,
accepted producer identifier, supported source-path/platform contract, per-request
limits, and generation binary staging. Current `path.resolve` behavior permits
paths outside the workspace; confinement must be decided explicitly without
breaking legitimate `../skills` identity paths. C0/C1 distinguish supported inputs
from incidental acceptance before tightening a boundary. Review-episode work has
no compiler dependency unless a concrete consumer is identified; neither lane
waits for a speculative universal runtime or persistence package.

## Evidence baseline and limits

Read-only planning used HEAD `7583e2095725957470e28cc11fc69404386fec63` plus the
working-tree bytes below. Hashes identify inputs read, not an atomic whole-repository
snapshot. Concurrent lifecycle/UI/documentation edits were preserved. The migration
plan input predates its added coordination section; its hash below preserves the
assessed input rather than claiming current equality. Structural evidence used
Tier 2 Codebase Memory project `home-bline-code-work-engine`, generation
`2026-10-07T06:51:24Z`: complete relevant search/trace pages for compiler entry points,
both directions where material, exact snippets and direct source/test inspection.
Coverage checks on cited code, tests and governing paths recorded no gaps and
matching metadata. This is best-effort, not an exhaustive caller or dynamic-import
audit. A spurious Python `Path.resolve` graph edge was disregarded after the exact
snippet; Python projection semantics are attributed to source, not that edge.

| Material input | SHA-256 of observed bytes |
| --- | --- |
| [AGENTS](../AGENTS.md) | `04675bbabbf39576e4590cc8dfa2d681f7a630572ccd5f88bc62ebc422e4d9b8` |
| [Design](../DESIGN.md) | `2213ab9a32925e7792353489599201bf71f3268eb8981cc192f87c57e2952095` |
| [Philosophy](../PHILOSOPHY.md) | `299b0ad8192f3cc442f6491d2908f421f22333024ffb1423cae6856b69c3a307` |
| [Rust direction](rust-development.md) | `aca53736538773d86ca5c0e9381752814fa1f59ff0a23dd12cff249e37c36f37` |
| [Migration plan](rust-migration-plan.md) | `911b647cd75772ab136c8cef85e98737a0d6f42be22a71d2e539f6fce412b0fe` |
| [Model guidance](model-selection.md) | `f5c594118b0a151056cea38281b7db8f8b0904de02eea87d586d2697fdfa452c` |
| [Lifecycle implementation plan](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md) | `02c8ad62b958708b9d841eacfd29dda9601096f2bca9ba4cfb713d78a4d8baae` |
| [Skill compiler](../app-server/src/skill-compiler.mjs) | `abb2cdf31c2d6a2f4a19dfc062cbaef6b96ece6fd74721b0bf170dd5ef23cbeb` |
| [Manifest implementation](../app-server/src/runtime-manifest.mjs) | `6cbee5e351e3193ac591c2d9501399dc9e66faa9f489ec9f80f64aa58a4cca30` |
| [Extension compiler](../app-server/src/run-extension-bundle-compiler.mjs) | `da0ad00e4a6316e939d95923be2d336c98a383386c9a2cee44ef3d65b10ccb28` |
| [Extension contract](../app-server/src/run-extension-bundle-contract.mjs) | `6c7b75c807b3fdb6f7b9e817a74aa7b65b8204014b832baa33cdc7d7ad98017c` |
| [AEG adapter](../app-server/src/agent-environment-graph-adapter.mjs) | `fad4815833bad8d0bd61992137719cc444141b11de121430c6f70704c7b849eb` |
| [Python AEG](../skills/agent-environment-graph/scripts/agent_environment_graph.py) | `24583ce14385728da3459acd848368fbb7768258bdb80dfefa3f00cef7545450` |
| [Generation bootstrap](../app-server/src/executable-generation-bootstrap.mjs) | `d1341f128d3d5f2f4c9368e92fd70991b21206d8e9a61db57c1afc5c8bbe8047` |
| [Generation snapshot](../app-server/src/executable-generation-snapshot.mjs) | `d9a2c4c8f2f1a5cc6efc2f5a7329b17b5e22aceb1212a23b715222f9e3b389af` |
| [Generation role environment](../app-server/src/executable-generation-role-environment.mjs) | `ba5256d1984268c118c411fa5285a7d8c5fefa2f6138bc00ef7e088dd1cc8514` |
| [Extension registry](../app-server/src/run-extension-registry.mjs) | `c96c6072fe6e47c964c5b48dd8d5e101ac8e45c91a6e2b85889a78fceca6ee81` |
| [Native review host](../app-server/src/services/slice-campaign/native-review-host.mjs) | `21a6f438510e32002e2726ba657d8a1892386e5cbec27e16137e0662bf12b467` |
| [Runtime manifest](../app-server/runtime-manifest.yaml) | `d50e88a2b0cfe2ba58819de56e95c23ec2369655abc838c94f00b9a9dab9e70d` |
| [Skill tests](../app-server/tests/skill-compiler.test.mjs) | `73b087fdb4a2ef3ca63d9229ecb221c3665560e2246e9865b2d3c83ad64356bc` |
| [Manifest tests](../app-server/tests/runtime-manifest.test.mjs) | `5011ccf363ff3d57ea0eba4d11dc6852858e0946ab2a460bc28dba9c97f7eac9` |
| [Extension tests](../app-server/tests/run-extension-bundle.test.mjs) | `1cfb65c0529120af4894ae1c0c869cb214af1c45d6b1ff469c8eeaaf5c655a7d` |
| [AEG tests](../app-server/tests/agent-environment-graph-adapter.test.mjs) | `7a214349936f2ea5448477b6fbafdbb463cf54a6e8a0156dfa045eaf2da167e0` |
| [Generation worker tests](../app-server/tests/executable-generation-worker.test.mjs) | `1527f9455519c3ce905f1918cf550374c5f63db5a82cb9a5dcef96898b227b68` |
| [Generation snapshot tests](../app-server/tests/executable-generation-snapshot.test.mjs) | `2fb8ab21c4f072b304864e46e51ab431f9a3497a0896e437ba19b31e89712630` |

Documentation validation for this proposal checks local references, whitespace and
scope only. No compiler, unit suite, provider workload, source migration, workspace
scaffold, runtime activation or commit was performed during planning.
