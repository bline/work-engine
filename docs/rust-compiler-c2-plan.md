# Rust compiler C2: verified skill closure

Prepared 2026-10-07. **Planning only; C2 implementation is not authorized by
this document.** The user authorized parallel C2/R2 planning in coordination
message 1210. This plan owns this file only. Its predecessor is accepted C1/R1
commit `8497df02e055c3b93e865cbe75fe4c99eb555eb4`, atop S0
`84be6695d716f026460fbf11aa703e73631e514e`.

The accepted [compiler plan](rust-compiler-implementation-plan.md),
[C0 contract](rust-compiler-c0-contract.md), [C1 result](rust-compiler-c1-result.md)
and [combined qualification](rust-c1-r1-validation.md) retain their original
bytes. Their historical prepublication wording does not undo commit 8497df02.
[Design](../DESIGN.md), [philosophy](../PHILOSOPHY.md) and
[Rust direction](rust-development.md) govern placement and authority. Requested
planning configuration is Astra/xhigh; runtime-effective model identity was not
independently exposed. Future implementation and ordinary remediation remain
`gpt-6-sol`/high under the user's explicit choice.

## Outcome and scope

C2 establishes verified skill compilation through the **existing Node CLI and
manifest hydration consumers**, using Rust and the unchanged real Python AEG
owner. A successful verified result binds canonical Markdown, all declared
bindings, and any role projection to the exact captured bytes consumed. It does
not establish semantic adequacy, provider admission, deployment authority, or an
atomic working-tree snapshot.

The five C0 cases remain P-SUP, P-BLD, P-REP, P-CLM and P-AIR. Preserve
`work-engine.skill-compiler.rust-v1`, the accepted rejection of collection YAML
mapping keys, scalar-key compatibility, BOM/raw-byte hashing, and the domain-local
`compiler-js-canonical-json-v1` codec. A verified operation must never return an
unverified fallback. C1's unverified operation remains available and unchanged.

C2 does not implement C3 manifest projection/satisfaction, C4 extension
attachments, C5 default cutover, binary deployment, generation activation, live
database work, providers, or a Rust replacement for Python's graph semantics.
Existing authored skills, decompositions, projections and runtime manifest are
inputs, not files this slice may refresh to make tests pass.

## Observed integration and the proposed boundary

These are source observations, not newly executed behavior:

| Observed source | Consequence for C2 |
| --- | --- |
| `compileSkill` in [skill-compiler.mjs](../app-server/src/skill-compiler.mjs), lines 390–438, validates/render/projects, reads sources in its verified branch and returns `{ ir, output }` with a Node `Buffer`. | Keep the async facade; route a whole selected operation through Rust. Preserve legacy as the default until cutover authority exists. |
| `verifyLegacySource` (348–367) checks source digest, exact frontmatter prefix, first offset, contiguous section bytes and EOF; `verifySourceBindings` (341–346) hashes every binding. | Extract the corresponding pure checks into the existing compiler core; filesystem capture belongs in the executable. Do not replace the span checks with output-digest equality. |
| `verifyRoleProjection` (369–388) hashes/parses the pinned YAML and compares its entire value with the AEG projection. | Check complete projection equality using the compiler codec, not just role ID or selected fields. Generated evidence remains comparison evidence. |
| [AEG adapter](../app-server/src/agent-environment-graph-adapter.mjs) uses a 30-second deadline and independent 2 MiB stdout/stderr caps, checks exact envelope fields, backend/hash, canonical match, exit and stderr. Its failure promise can settle before child close. | Preserve semantic validation and limits; replace process mechanics with a terminal owner that observes the direct child's exit/reaping before a normal terminal response. |
| Python `project_candidate_role` (883–923) loads the two canonical inputs, requires exact candidate-role equality, validates the environment, renders the closed projection, and hashes `__file__`. `_source_hashes` (562–572) rereads those two inputs and uses constant semantic path labels. | Run this unchanged owner against private copies of captured bytes so its multiple reads share an input identity. No Python source change or graph reimplementation is needed. |
| [compile-skill.mjs](../app-server/scripts/compile-skill.mjs) reads structure/interface from invocation cwd, calls `compileSkill`, then writes output and optionally compares it. | Exercise this real entry point. Publish complete output by same-directory temporary-file replacement; retain argument/base semantics. |
| `hydrateRuntimeRequirements` in [runtime-manifest.mjs](../app-server/src/runtime-manifest.mjs), lines 309–344, dynamically imports `compileSkill` for role and secondary skills. `loadRuntimeManifest` calls hydration; generation bootstrap also calls it. | A trusted compiler selection at the facade reaches hydration without migrating manifest logic or changing generation code. Test both direct hydration and `loadRuntimeManifest`. |

## Implementation ownership

Paths below are a proposed bounded implementation grant, not files changed during
planning. One retained builder owns this complete vertical slice and its fixes.

| Owner | Exact scope |
| --- | --- |
| C2 core | `rust/crates/work-engine-compiler/src/{lib,skill,error}.rs`; new `src/verification.rs` and `tests/verified_skill_v1.rs`. Reuse codec/parser/rendering; keep C1 fixtures and public unverified results compatible. No filesystem/process dependency enters this crate. |
| C2 executable | `rust/crates/work-engine-compiler-cli/Cargo.toml`, `src/{main,protocol}.rs`; new `src/{source_snapshot,aeg}.rs`; `tests/protocol.rs`; new `tests/{verified_skill,aeg_process}.rs` and `tests/fixtures/c2/`. Process helper code stays private to this executable. |
| C2 interoperability | New `app-server/src/rust-compiler-adapter.mjs`; bounded routing in `app-server/src/skill-compiler.mjs`; bounded publication/cancellation changes in `app-server/scripts/compile-skill.mjs`. New `app-server/tests/rust-compiler-adapter.test.mjs`; bounded coverage additions in existing skill, AEG-adapter and runtime-manifest tests; fixture data under `app-server/tests/fixtures/compiler-c2/`. |
| C2 receipt | New `docs/rust-compiler-c2-result.md`, recording exact subjects, commands, differential results, process observations and limitations. |
| Shared integration owner | Any `rust/Cargo.toml`, `rust/Cargo.lock`, common gate inventory or toolchain change, serialized with S1 and R2. No new workspace member is requested. |
| Retained owners | Python script and canonical source refresh; C3's manifest algorithms/generation inventory; C4's extension compiler; review R3's native-review-host changes; all lifecycle/UI code. C2 does not edit these implementations. |

The only JavaScript additions are necessary transition glue and consumer tests.
Rust owns new source verification and child-process mechanics. There is no actual
second consumer with an accepted common supervision contract, so C2 does not
extract a supervisor framework or depend on lifecycle internals.

## Captured source and trust rules

The source profile remains C0's trusted local Linux/POSIX operation. Resolve
`workspaceRoot` once at the Node host boundary. Relative authored source,
binding and projection paths resolve lexically against it as `path.resolve`
does; absolute paths and legitimate `..` remain supported. Do not apply realpath
normalization to IR identity, introduce a workspace jail, or reject an existing
symlink merely because it points outside the root. These paths grant no authority
and cannot choose an executable, Python script or AEG input configuration.

The executable captures one bounded regular-file byte buffer per resolved lexical
path and reuses it for all references to that path. Each declaration still gets
its own digest check; conflicting expected digests fail. Different lexical aliases
are not silently collapsed into a canonical identity. File opening follows
symlinks as the existing read does; use nonblocking opening and file-type checks
to refuse FIFOs/devices/directories rather than allow a source read to hang. If
an observed change during capture prevents relating the consumed buffer to the
expected digest, refuse. A successful result attests the captured bytes, not
continued equality of the live path after capture.

For every operation: hash the original structure/interface bytes; preserve the
strict UTF-8 input profile; validate the existing schema/authority rules; capture
the canonical Markdown and every declared binding; verify hashes and all canonical
span/prefix checks. Role-free compilation ends verification there and launches no
Python child. Role compilation additionally captures the pinned projection and
the host-selected Python script, invariant catalog and role environment. The host
defaults are the existing script under `skills/agent-environment-graph/scripts/`
and the two canonical files under `docs/`; custom locations are trusted launch
configuration, never inferred from YAML binding labels.

For a role, create a private owner-only temporary directory. Write only three
compiler-chosen filenames for the captured script, invariants and environment;
never join authored paths into staging destinations. Use exclusive regular files,
no staging symlinks, restricted permissions and exact byte/hash checks. Invoke
the captured script with the existing `project-role-machine --invariants ...
--environments ...` protocol. Its observed relevant source closure is those two
files plus its own script; stdlib and installed PyYAML remain interpreter/runtime
dependencies, not newly claimed captured source. Select an explicit trusted
interpreter and isolate import search from authored workspace/PYTHONPATH (the
proposed launch uses `-I -B` and a qualified environment with PyYAML available).

The current script's `_source_hashes` always emits `../workflow-invariants.md`
and `../agent-environments.yaml`, independent of staging location. Preserve those
semantic labels. Require its returned hashes to equal the two captured input
hashes, and `backend_sha256` to equal both the captured script hash and the host's
expected script digest. Validate the full closed-projection envelope and compare
the entire projection with the captured pinned YAML. The real Python owner alone
establishes canonical-role equality. Do not replace its rejection with a local
approximation or use generated projection bytes as authority.

Mutation after capture may leave the result valid for the captured identity;
mutation before or during capture may fail its expected hash. Test both outcomes
explicitly. Private copies prevent Python's parse and later hash reads from
accidentally referring to different live bytes. This is not an atomic repository
snapshot, a hostile same-user filesystem sandbox, or a sealed executable generation.

Core construction should separate validated skill, source-checked skill and final
verified result with private fields; no deserialization or public boolean setter
can construct the latter. Pure check inputs contain captured bytes and explicit
identities. A checked AEG observation is created only on the executable's real
process path; tests may provide a private fixture peer. Rust types prevent
accidental mixing, while the trusted host/process observation establishes external
provenance. Arbitrary caller-supplied JSON cannot certify that Python ran.

## Protocol, backend selection and cancellation

Keep protocol v1 exactly as C1: one stdin/EOF unverified request, existing success
shape and limits. Add explicit **protocol v2**, still one request and one terminal
response per process. Its exact request fields are `schema_version: 2`, nonempty
bounded `request_id`, `operation` (`compile_skill_unverified` or
`compile_skill_verified`), `structure_source_base64`, and
`interface_source_base64`. Unknown/duplicate fields and unknown versions fail;
there is no verified bit, projection envelope, codec, command or source path in
the request. A verified operation is a request to perform checks, not a proof.

Host-only launch configuration supplies the absolute source root, absolute Python
executable, script path plus expected SHA-256, and the invariant/environment paths.
The v2 response has exact fields `schema_version`, `request_id`, `status`, and
either `ir`, `output_base64`, `verification` on success or `error` on failure.
`verification` records mode, captured source identities/hashes and the Python
observation for roles; it is outside all historical IR/digest payloads. A role-free
success records no Python observation. The Node host separately records its
observed Rust executable digest; a binary's self-reported name is not that evidence.

Retain C1's 16 MiB request/result and 4 MiB decoded-source limits. Proposed new
capture bounds are 4 MiB per file, 256 distinct paths and 64 MiB aggregate captured
bytes, including AEG inputs; classify excess as `resource_limit`. Retain Python's
30 seconds and independent 2 MiB stdout/stderr limits. Bound compiler stderr at
2 MiB and the Node one-shot request at 60 seconds with a 5-second cleanup grace.
Record these as local resource profile values accepted with C2, not YAML semantics.
Do not accept a truncated result. Boundary tests use smaller explicit fixture
limits without widening the public request.

Add stable error classes for `source_unavailable`, `source_mismatch`,
`aeg_unavailable`, `aeg_failed`, `aeg_protocol`, `cancelled`, `timeout`, and
`cleanup_unconfirmed`, alongside existing codes. Input/source inconsistency exits
2; host/process/internal failure exits 1. Return no partial IR/output. A successful
response requires valid correlation/version/producer, expected output encoding and
hash, complete envelope consumption, exit zero, no signal, and empty stderr. Unknown
fields or extra JSON after the envelope fail closed. Do not promise incidental
legacy exception wording or first-error order for multiply invalid inputs.

Use a bounded nonblocking I/O loop for stdin, stdout and stderr, with monotonic
deadlines and cancellation flags installed before child work. This avoids a blocked
writer/read thread preventing cancellation or reaping. Timeout, overflow, write
failure, broken pipe, malformed result, nonzero exit and signal all enter one
cleanup path: stop admitting success, close input, terminate the owned Python
child if live, observe its wait result, finish bounded pipe handling and remove
private staging before returning a normal terminal result. Do not treat `kill()`
success as observed exit. Success-looking bytes before nonzero exit are failures.

The Node adapter launches the compiler as a dedicated POSIX process group; Python
inherits that group. Normal caller abort (`AbortSignal`) or CLI SIGINT/SIGTERM is
forwarded to Rust, which cancels and reaps its direct Python child. On the outer
deadline/failed cleanup, Node kills the whole group, awaits its direct compiler
child, and reports failure with cleanup evidence. The unchanged real Python owner
does not launch child processes on this route; this is not an arbitrary process-tree
service. Uncatchable compiler death or host loss cannot establish Python reaping:
do not fabricate that observation or verified success. Test orderly cancellation
separately from emergency termination; retained descendant pipes must not make
the host hang. No claim of remote-effect settlement is involved.

The proposed host selector is `WORK_ENGINE_COMPILER_BACKEND=legacy|rust`, with
explicit absolute binary path and expected SHA-256 when selecting Rust. Missing,
unknown or mismatched selected configuration fails; there is no silent backend
fallback. Default remains legacy. Copy the selected binary bytes after their digest
check to an owner-only private executable for the invocation, so a later write to
ambient `target/` cannot change what is launched. Record build selector, source path,
observed digest and launched-copy digest in the gate evidence. This local captured
binary is not C3's sealed generation artifact or a complete dynamic-runtime attestation.
Exact selector/config variable names are implementation API details to freeze in
the v2 fixtures before the vertical gate.

`compileSkill` keeps its async signature and `{ ir, output: Buffer }` result,
with an optional host cancellation signal. Convert base64 to an actual Buffer and
validate the envelope, output and expected operation before returning it. Existing
legacy test adapter injection must not silently switch a Rust-selected request back
to legacy; Rust adversarial fixtures use a private executable test seam. Preserve
valid Node string input; reject unsupported lone-surrogate/invalid-byte inputs
explicitly under C0's profile rather than silently reinterpret them.

## Real consumers and vertical-first sequence

1. After plan acceptance and shared dependency handoff, prove one immutable P-SUP
   role through `node app-server/scripts/compile-skill.mjs`, selected Rust binary,
   captured inputs, **real unchanged Python**, and exact Markdown/IR comparison.
   Pair it with one source mismatch that produces no destination replacement.
   The test records executable/script identities and direct-child completion.
   This first composed result precedes broad fixture expansion or abstraction.
2. Complete adversarial process/source vectors and role-free behavior. Keep a
   deterministic private peer seam for failures; it never enters production request
   JSON or claims to be the real canonical owner.
3. Exercise `hydrateRuntimeRequirements` and `loadRuntimeManifest` under the same
   explicit Rust host selection, using an isolated pinned C0 manifest/source tree.
   Cover every compiled occurrence: supervisor and builder role environments,
   builder's repo-search, and reviewer's repo-search, claim-evidence and
   agent-instruction-review. Five unique skills do not mean only five consumer
   occurrences. Existing role/secondary identity/fingerprint checks remain in Node.
4. Run focused gates, retain a separate permitted reviewer for the composed result,
   remediate with the same Sol builder, then coordinate whole-workspace qualification
   and a result receipt. No default selection or live activation follows automatically.

The immutable C1 Markdown/structure/interface fixtures remain the compatibility
oracle. C2 adds the missing source closure from exact C0 pins/Git blobs, including
P-BLD's historical canonical skill. It does not use a fresh live builder file as
the success oracle. The live-source drift already recorded by C0 is a rejection
case until its source owner independently refreshes and accepts the closure.
Python script bytes are pinned to the observed owner hash below. Real Python tests
record interpreter and PyYAML identities without claiming a new provider run.

For CLI publication, fully compile first, write a unique same-directory temporary
file, then rename a completed file into place; cleanup on ordinary failure/abort.
An existing destination stays intact when compilation fails. SIGKILL may leave an
unadvertised temporary file but cannot expose a partial final file. Preserve the
existing `--compare` behavior: a valid compiled file may be published and mismatch
sets a nonzero exit; the summary must not misrepresent the comparison as passing.
Keep argument resolution relative to invocation cwd and `--workspace-root` limited
to source verification. The user accepted on **2026-10-07** the explicit CLI
compatibility delta: reject an output symlink and preserve permissions on an
existing regular output file. Detect/recheck output type during publication so
an intervening symlink is not silently accepted; never follow its referent for
replacement. Source-file symlinks remain supported under the capture rules above.

## Acceptance matrix

Each refusal asserts no verified result and the relevant consumer consequence;
each actual-process failure additionally asserts direct-child exit observation
where the owning supervisor remains alive. Mock-only tests cannot prove lifecycle.

| Group | Required executable cases |
| --- | --- |
| Compatibility | All five pinned skills: exact Markdown bytes, complete producer-neutral IR (exclude only the accepted producer field), verified requirements digest, role-free omissions, repeated determinism. C1 parser/BOM/scalar-key/collection-key and v1 protocol tests remain passing. |
| Canonical source | Wrong source hash; wrong frontmatter with a matching authored digest; first-offset gap; middle gap/overlap; bad section bytes; incomplete EOF; multibyte boundaries; every binding's digest; missing/unreadable source; duplicate path with incompatible declarations; oversized/nonregular input. |
| Trust and projection | Generated evidence as authority; divergent candidate with matching local projection; wrong pinned projection hash; complete projection differs outside role fields; modified canonical environment/invariants; wrong backend/hash/status/version/match; absent/extra/wrong-type envelope fields; malformed/duplicate/trailing JSON; forged source hashes. |
| Capture/path | Relative, absolute and `../` paths; symlink read behavior and retarget between distinct captures; cwd independence with explicit bases; mutation before capture rejects, mutation after capture preserves captured identity; script replacement after capture cannot alter invoked bytes; staging names cannot be influenced by authored traversal. |
| AEG processes | Missing interpreter/script/PyYAML; spawn and early-stdin failures; stalled reader; timeout; stdout/stderr overflow independently; exact-limit success; success bytes plus stderr/nonzero exit/signal; SIGTERM/SIGINT cancellation; cleanup on output parse failure; no Python invocation for role-free/unverified operations. |
| Rust bridge | Missing/wrong-hash/wrong-version binary; request correlation mismatch; malformed/oversized/truncated response; wrong output hash/base64; unsupported Unicode; no silent fallback; event-loop remains responsive while compiling; Buffer identity; actual child exit before promise resolution; cancellation and emergency group termination. |
| CLI/hydration | Real CLI success and compare mismatch; existing-output preservation on failed compile; accepted output-symlink rejection (including intervening type change) and regular-file permission preservation; interruption before/after rename; temp-file cleanup; every manifest occurrence through Rust; wrong secondary ID/fingerprint or role profile refuses hydration; unverified requirements cannot pass the unchanged satisfaction check; no fake delivery on a rejected result. |

Tests needing filesystem/process control belong in Rust where practical; existing
Node boundary tests are the necessary compatibility evidence. Record elapsed
wall time for standalone role compilation and complete hydration to expose startup
cost, without inventing a performance threshold or claiming a speedup.

## Dependencies, shared workspace and gates

C2 requires accepted C1, C2 plan/implementation authority, and serialized lock
qualification. It does not depend on S1 completion, R2 implementation, a reusable
persistence layer or a shared supervisor. S1 currently owns root integration;
board 1212 reports a changed S1 lock and explicitly promises no shared persistence
owner. Do not overwrite that lock with C1's historical qualified bytes.

Proposed new direct CLI dependencies are `tempfile = "=3.27.0"`,
`rustix = { version = "=1.1.5", features = ["fs", "event"] }`, and
`signal-hook = "=0.3.18"`. The first two versions were observed in the current
S1-era lock; this is not a C2 qualification. Keep existing compiler dependencies
and pure-core graph unchanged. No Tokio, SQLite, lifecycle crate or new Node
package is needed. The foundation owner reviews these exact pins/features and
resolves/qualifies the resulting lock, including signal-hook's transitive inputs,
license inventory, advisories and MSRV. Do not hand-edit transitive lock entries.
Use library signal registration rather than custom unsafe signal handlers;
[signal-hook flags](https://docs.rs/signal-hook/0.3.18/signal_hook/flag/index.html)
support bounded polling, while [tempfile](https://docs.rs/tempfile/3.27.0/tempfile/struct.Builder.html)
provides private staging and explicit cleanup. These API references support the
mechanics choice, not an assertion that dependency checks have passed.

Coordinate the root claim/handoff before dependency changes. C2 and R2 can edit
disjoint owned files after their separate grants, but one integration owner batches
their lock requests. Focused gates bind an exact shared lock revision; final
workspace test/fmt/Clippy/release gates require a stable source/manifest window
with S1/R2. Avoid whole-workspace formatting or a build that overwrites another
lane's evidence binary outside that window. Use an isolated target directory and
record its exact selector/hash for local compiler gates. Existing terminal-UI
exclusion and member registration remain intact.

Future commands, **not run during planning**, after the named tests exist:

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli --locked
cargo +1.92.0 fmt --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli -- --check
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli --all-targets --locked -- -D warnings
cargo +1.92.0 build --manifest-path rust/Cargo.toml -p work-engine-compiler-cli --release --locked
node --test app-server/tests/rust-compiler-adapter.test.mjs app-server/tests/skill-compiler.test.mjs app-server/tests/agent-environment-graph-adapter.test.mjs app-server/tests/runtime-manifest.test.mjs
```

Node fixtures bind the exact built executable/hash and pinned source root; a
command alone is not evidence that Rust ran. Legacy oracle runs use the pinned
JavaScript source and record actual Node/YAML versions. If old tests rely on the
drifted live builder, qualify the immutable fixture path and report the known
live rejection separately; never suppress a new regression as baseline drift.
The joint owner then runs the required workspace gates against the agreed stable
subject and publishes their exact scope and result.

C2 exit requires the real CLI/Python vertical, complete source/process matrix,
all pinned hydration occurrences, unchanged C1 behavior, no unresolved material
review finding, qualified dependencies and a truthful result receipt. The receipt
separates selected/observed binary/script identities, captured-source hashes,
runtime identities, passing gates, fixture limitations, cleanup observations and
any unqualified live source closure. C3/C4/C5 remain unauthorized by that exit.

## Decisions still requiring an owner

The concrete proposed C2 scope, v2 transport, local resource profile and dependency
request above are ready for the user's bounded implementation decision; this plan
does not grant it. The shared owner must accept the dependency/qualification
handoff. A production-source refresh belongs to its source owner and is not a
prerequisite to the immutable C2 fixture gate. Sealed binary inventory and consumer
cutover remain C3/C5 owner decisions.

The CLI output decision is **settled by the user's 2026-10-07 choice**: reject
output symlinks and preserve existing regular-file permissions. This accepts that
specific compatibility delta; it neither confines source paths nor rejects source
symlinks, and it does not authorize C2 implementation. No further output-path
authority question is pending.

## Evidence and planning limits

Tier **Verify**, project `home-bline-code-work-engine`; parent recovery generation
`2026-10-07T16:59:31Z`; child `index_status` confirmed the correct root and ready
state. Targeted name searches completed every relevant page (`has_more: false`).
`compileSkill`, `hydrateRuntimeRequirements` and Python `project_candidate_role`
were traced both directions at depth 1; Python `render_role_projection` was traced
outbound. Results were untruncated. Exact snippets and direct source reads support
the integration table. Graph heuristics suggesting unrelated `resolve`, `get` and
`sorted` targets were discarded after source inspection, not treated as dependencies.
A first combined file-pattern search returned no matches; separate exact-path
queries resolved the candidates. No negative whole-repository caller claim is made.

Coverage checked all 28 relied-on governing/code/test/config paths at generation
`2026-10-07T17:07:46Z` (recorded `17:07:47Z`): no recorded issue and matching metadata,
except evolving `rust/Cargo.lock` reported `not_tracked`. Its relevant package
entries were read directly; no lock completeness or stable final hash is claimed.
The generation's unrelated episode fixture and S1 SQL parse gaps were not relied
on. Coverage is best-effort, not proof of source completeness or runtime behavior.

| Material subject | Observed SHA-256 |
| --- | --- |
| Accepted compiler plan | `7e64984a4b399578d01c2728d2bb25e62d0e5b352fc9304f1773f45dc4b9a24e` |
| C0 contract | `2a817ee4074fe0338a931148fcd5e38d1b9b14ab916cebca65b8582dacba3a5a` |
| C1 result | `0630aa67fba1ffc1f90386ef6e2a36175a418be34faa1a3ae8b4675347f73b8d` |
| Combined C1/R1 validation | `4c03bf943e0e9d06fce4ae716aec42444dd1868b27a8081f571b17370df5a7f3` |
| Node skill compiler | `abb2cdf31c2d6a2f4a19dfc062cbaef6b96ece6fd74721b0bf170dd5ef23cbeb` |
| Node AEG adapter | `fad4815833bad8d0bd61992137719cc444141b11de121430c6f70704c7b849eb` |
| Node CLI | `92b684d03d56bc1ed2894930a6fdf01b9f9216245e0665d564592ea74423adee` |
| Node manifest implementation | `6cbee5e351e3193ac591c2d9501399dc9e66faa9f489ec9f80f64aa58a4cca30` |
| Python AEG owner | `24583ce14385728da3459acd848368fbb7768258bdb80dfefa3f00cef7545450` |
| Runtime manifest | `d50e88a2b0cfe2ba58819de56e95c23ec2369655abc838c94f00b9a9dab9e70d` |
| C1 core skill implementation | `df9a3e6f8f5d636b0117e57325462f72d323a692e5230505b9a8d1fd907d7002` |
| C1 executable protocol | `f7e70c4e6659e488c0bbad59736d1296ac2d19e923292b35af0452a3a3d3b17b` |

Only this plan was written. Read-only board, graph, source/hash and documentation
checks were used; no product/test execution, AEG invocation, provider, live store,
workspace mutation, staging or commit occurred. Planning does not establish an
independent review or any new gate pass.
