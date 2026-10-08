# Rust compiler C3: manifest projection and requirement satisfaction

Prepared 2026-10-07. **Planning only.** The user authorized C2/R2 publication
and parallel C3/R3 planning; no C3 implementation, test/build execution,
publication, default cutover, provider entry or activation follows from this
document. Planning owns this file only. The implementation baseline is committed
C2/R2 `bcbc52eb071270dc56d4dc65489b26b269506604`, whose parent is S1
`e7c1d88ec0df20ea6213239b6dc2b36abd9f1872`. Chatboard 1245 records publication;
the prepublication wording in the historical receipts does not undo that commit.

The [compiler plan](rust-compiler-implementation-plan.md),
[C0 contract](rust-compiler-c0-contract.md), [C2 plan](rust-compiler-c2-plan.md),
[C2 result](rust-compiler-c2-result.md) and
[joined integration receipt](rust-c2-r2-integration.md) supply the predecessor
contracts and evidence. [Design](../DESIGN.md), [philosophy](../PHILOSOPHY.md)
and [Rust direction](rust-development.md) govern placement. One retained
`gpt-6-sol`/high builder is recommended for implementation and ordinary repair;
a separate `gpt-6-astra`/xhigh reviewer should challenge the composed result.
This is a recommendation, not a launch or an independence claim.

## Result and boundary

C3 makes Rust the selected implementation of normalized manifest projection,
environment digest construction and pure requirement satisfaction, reached
through the existing synchronous `projectRuntimeManifest` and
`satisfyRuntimeRequirements` APIs. The JavaScript `RuntimeManifest` facade
retains instance projection and freezing; `ManifestRoleRuntime` retains
delivery orchestration and refusal before its adapter is called.
`loadRuntimeManifestDocument`, asynchronous hydration and `loadRuntimeManifest`
remain asynchronous Node entry points. C2's verified skill/Python route remains
the owner of source verification.

A satisfaction receipt establishes that supplied requirements match the supplied
manifest grants, identities and fingerprints under the existing checks. It is
neither a capability grant nor independent proof that source verification ran.
In particular, the current public requirements input contains `verified_sources`
and a digest; anyone controlling that whole input can recompute them. C3 preserves
the trusted-host consumption boundary rather than claiming its DTO is a permit.
Source provenance comes from C2 hydration and the existing generation closure.

Keep legacy as the default and make Rust selection explicit and fail-closed.
C3 does not implement C4 extensions, C5 cutover, a daemon, provider admission,
review-episode semantics, live-store migration, generation activation, instruction
authoring or a replacement for Python AEG. It creates no shared persistence or
process-supervision framework. A common codec already has compiler consumers;
two different bridge lifecycles alone do not justify a universal process owner.

## Current source boundary

These are source observations at the baseline, not new execution results.

| Source | Consequence for the replacement |
| --- | --- |
| [runtime-manifest.mjs](../app-server/src/runtime-manifest.mjs), `projectRuntimeManifest` 194–290 | Resolves three bases, sorts role IDs and grants, preserves skill array order, builds the identity payload, and constructs a `RuntimeManifest`. Move the deterministic algorithm, preserve these distinctions. |
| Same file, normalizers 71–155 and `RuntimeManifest` 157–192 | Authored `reasoning_effort` becomes runtime `effort`; skill names are unique but are not restricted to role-ID syntax. `projectRole` validates instance IDs and returns frozen copies. Keep the latter in Node. |
| Same file, loading/hydration 292–367 | Loader parses YAML with unique keys and no aliases; hydration invokes the async skill compiler once per compiled occurrence. Direct projection does not hydrate. Retain both paths. |
| Same file, satisfaction 369–438 and delivery 453–485 | Requirements digest, capability/effect ceilings, secondary-to-role subset, continuity, fingerprint and exact contract identity are checked before delivery. Secondary skills have independent checks and receipts. |
| [generation bootstrap](../app-server/src/executable-generation-bootstrap.mjs), `roleEnvironmentSource` 157–240 | Hydrates candidate bytes, projects them, and captures document, canonical identity/requirements bases and requirements in the generated role configuration. The generation owner controls this handoff. |
| [generation role environment](../app-server/src/executable-generation-role-environment.mjs), 257–285 | Reprojects with snapshot delivery paths and saved identity/requirements bases. Reading the snapshot does not rehydrate from ambient source. |
| [native review host](../app-server/src/services/slice-campaign/native-review-host.mjs), 238–276 | Loads and directly projects without hydration. R3 owns this host; C3 must not add a hydration assumption or edit it concurrently. |
| [snapshot](../app-server/src/executable-generation-snapshot.mjs), 44–95 | Source entries require regular non-symlink files and record executable mode; copied executable entries become mode 0700. Generated entries are non-executable. A compiler binary belongs in source inventory. |
| [C2 bridge](../app-server/src/rust-compiler-adapter.mjs), 92–155 | Async skill compilation captures and hashes the selected binary before launch. Its 60-second/Python profile is unsuitable as the responsiveness budget for a synchronous pure manifest operation. |

### Identity and encoding contract

Resolve default bases once at the Node call boundary, with the current Linux
lexical `path.resolve` behavior. `baseDirectory` selects delivery skill paths;
`identityBaseDirectory` selects the canonical role contract, identity skill paths
and thread `cwd`; `requirementsBaseDirectory` resolves required contract paths.
Relative, absolute and legitimate `..` paths remain supported. No realpath,
filesystem access or new confinement policy enters the pure Rust operations.
Pass explicit absolute bases, including the cwd-resolved `sourcePath`, across
the bridge. Rust implements the tested lexical POSIX resolution over those inputs.
Changing ambient cwd after construction cannot change satisfaction meaning.

Use the existing domain-local `compiler-js-canonical-json-v1` implementation in
`work-engine-compiler/src/codec.rs`: UTF-16 key ordering, JavaScript scalar
rendering, retained array order, UTF-8 bytes and no final newline. Add named
manifest/environment/satisfaction wrappers, without migrating to a common
repository digest. The protocol selects that profile by its trusted operation
version; authored inputs cannot select a codec.

Preserve these exact digest inputs:

- Direct projection uses the canonical document hash unless the caller supplies
  `sourceSha256`; loaded manifests retain the loader's source identity. YAML
  parsing/source decoding is unchanged by C3. No Rust reserialization substitutes
  for the loaded source hash.
- Environment identity hashes the role template with `roleContract` replaced by
  its canonical `{path}` and `skills` replaced by identity-normalized skill rows.
  The role's `runtimeRequirements` is included. Secondary attached requirements,
  activated paths and added `identityPath` fields are absent from those identity
  skill rows. Preserve this asymmetry rather than making all requirements part
  of identity. Delivery-only relocation leaves the environment revision equal.
- Requirements hash every unsigned requirement field, including otherwise opaque
  supported JSON fields; `sha256` alone is removed. Unknown requirement fields
  are not silently discarded before hashing. The receipt hashes exactly its
  current unsigned fields, with `skill_id` omitted for role satisfaction.
- Binary/protocol/build identity is transport evidence outside all three legacy
  payloads. Adding a staged executable changes generation identity as expected;
  it does not justify changing a role-environment digest or binding fingerprint.

The Rust-selected wire supports JSON data: own string-keyed records, arrays,
finite JavaScript numbers and valid Unicode scalar strings, with absent fields
distinct from explicit null. A bounded preflight rejects cycles, accessors,
non-data prototypes, functions, symbols, bigint, undefined values, nonfinite
numbers and lone surrogates before JSON encoding can drop or coerce them.
This is the proposed explicit direct-object input profile; do not label such
JavaScript-only values as parity successes. Legacy remains unchanged. Preserve
current supported defaults and omissions, including nullish optional fields;
do not add blanket closed-schema checks to requirements where none exists.

## Implementation ownership

The following is the proposed bounded implementation grant, not changes made by
this planning pass. C3 and R3 may work in parallel on disjoint files, using one
integration owner for shared workspace and composed gates. S2/S3 own their
lifecycle packages and root dependency handoffs independently.

| Owner | Proposed paths and responsibility |
| --- | --- |
| C3 pure core | New `rust/crates/work-engine-compiler/src/manifest.rs` and `tests/manifest_v1.rs`; bounded `src/{lib,error,codec}.rs` exports/errors/domain wrappers; fixtures under existing `tests/fixtures/v1/manifest/`. No filesystem, process, database, provider or lifecycle dependencies. |
| C3 executable | `rust/crates/work-engine-compiler-cli/src/{main,protocol}.rs`, new `src/manifest_protocol.rs` and `tests/manifest_protocol.rs`; retain v1/v2 behavior and source-verification mechanics. No new package or dependency is currently needed. |
| C3 facade and bridge | `app-server/src/runtime-manifest.mjs`; new `app-server/src/rust-manifest-adapter.mjs`, `app-server/tests/rust-manifest-adapter.test.mjs`, bounded existing `runtime-manifest.test.mjs` additions and `app-server/tests/fixtures/compiler-c3/`. The C2 async bridge remains compatible; extract only small private mechanics if actually useful and exercised by both compiler adapters. |
| C3 result | New `docs/rust-compiler-c3-result.md`, binding immutable subjects, selected and observed executable identities, exact differential vectors, process/generation observations, HOST measurements and remaining handoffs. |
| R3 | `app-server/src/services/slice-campaign/native-review-host.mjs`, `app-server/src/index.mjs`, review bridge/service files and `supervisor-native-review-hosting-integration.test.mjs`. C3 supplies its interface and fixture cases; R3 makes any required host injection/call-site edits serially. |
| Generation owner | `executable-generation-bootstrap.mjs`, `executable-generation-role-environment.mjs`, source inventory/snapshot/worker deployment configuration and existing generation worker/snapshot tests. C3 requests the exact artifact/selector handoff below; it does not independently claim bootstrap or activation. |
| Shared integration owner | Root manifests/lock, toolchain, common gate window, one frozen HOST profile and joined evidence. Package qualification remains bound to the agreed lock. |
| Retained source owners | Canonical skills, decompositions, projection pins, runtime manifest and executable-source maintenance proposal decisions. C3 does not refresh these inputs to obtain passing tests. |

## Pure models, capture and transport

The core exposes private-construction validated manifest/template types and a
separate satisfaction receipt type. Parsing validates shape; projection creates
the normalized value; matching returns a receipt or a typed refusal. Deserializing
a transport record does not create a source-verified skill or runtime authority.
Keep structurally invalid manifest input distinct from valid grants that fail a
requirement. The Node facade still rejects a non-`RuntimeManifest` satisfaction
argument, preserves `instanceof`, role-ID ordering, `projectRole` output shape,
deep freezing and synchronous return/throw behavior.

Capture the document, explicit options and requirement values as one bounded
detached request before process entry. No input object is mutated. The synchronous
call cannot observe later event-loop mutation, and the child receives no caller
object references. This is a request-value capture, not a repository snapshot.
Projection and satisfaction perform no authored file reads and spawn no Python
or provider. The async hydration step continues to use C2's captured-source path.

Add **protocol v3** without altering the closed v1/v2 requests. One stdin/EOF
request yields one complete response and process exit. Proposed exact fields:

| Operation | Request fields in addition to `schema_version: 3`, bounded nonempty `request_id`, `operation` |
| --- | --- |
| `project_runtime_manifest` | `document`, `options`; options have exactly `base_directory`, `identity_base_directory`, `requirements_base_directory`, `source_path`, `source_sha256`, `runtime_requirements_by_role`. Nullable source values remain explicit. |
| `satisfy_runtime_requirements` | `manifest`, `role_id`, `requirements`, `skill_name`; `manifest` is the immutable facade data (`manifestId`, `source`, `roles`, `requirementsBaseDirectory`), and null `skill_name` means role satisfaction. |

The satisfaction decoder validates the supplied normalized manifest data and
reconstructs the typed projection; it must not accept a caller's claimed internal
validated-state flag. Transport input is still a trusted host's statement of
grants, not a new admission mechanism. An exported `RuntimeManifest` constructor
remains usable with supported normalized data; C3 does not require that instances
were previously issued by this process or cached by an opaque Rust handle.

Success fields are exactly `schema_version`, `request_id`, `operation`, `status`,
`producer`, `result`; use `work-engine.manifest-compiler.rust-v1` as producer.
`result` is the normalized manifest data or the existing satisfaction receipt.
Error fields replace `result` with `error: {code,path,message}`. Require strict
UTF-8/JSON, no duplicate keys, extra fields, trailing values or truncated envelope;
check version, correlation, operation, producer, complete result shape, exit zero,
no signal and empty stderr before returning success. Error envelopes also require
valid correlation/shape and a consistent failure exit. A malformed error is a
transport failure, never a normal negative satisfaction result.

Use stable domain codes `invalid_manifest`, `invalid_requirements`,
`requirements_unsatisfied`, alongside protocol/selection `unsupported_version`,
`invalid_request`, `resource_limit`, `binary_unavailable`, `binary_mismatch`,
`protocol_error`, `timeout`, `process_failed`, `cleanup_unconfirmed`.
Domain input/refusal exits 2; host/process/internal failure exits 1. Node maps
structural errors to `TypeError` and satisfaction refusal to `Error`, carrying
code/path. Preserve meaningful rejection class and pre-delivery consequence;
incidental wording and multiply-invalid first-error order are not new invariants.
No failed bridge request silently reruns legacy.

Selection remains trusted host configuration using the existing
`WORK_ENGINE_COMPILER_BACKEND=legacy|rust`, absolute
`WORK_ENGINE_COMPILER_RUST_BINARY`, and expected
`WORK_ENGINE_COMPILER_RUST_BINARY_SHA256`. The new bridge supplies an immutable
host-visible observation of selected path, observed digest, protocol 3 and launch
profile ID; this observation is outside legacy return/digest payloads. R3 consumes
it through its owned host wiring if needed. Author documents cannot choose a
binary, codec, command, environment variables or process mode.

For an offline synchronous call, read one bounded regular local executable,
check its expected digest, copy captured bytes to an owner-only temporary
executable, recheck the copy, and invoke that copy. A subsequent write to ambient
`target/` cannot change the launched bytes. Preserve the C2 distinction between
a captured binary and a sealed generation. For sealed operation the source of
that capture is the generation-owned pinned artifact, never an ambient fallback.
Use nonblocking open/file-type checks so FIFOs/devices are refused; require an
executable-size ceiling before allocation and bounded complete reads.

Proposed C3 local resource profile: 4 MiB request and result each, JSON depth 64,
64 KiB stderr, 128 MiB executable capture, and a 100 ms direct-child deadline.
These new pure-operation limits do not change C2's 16 MiB/60-second/Python limits.
The bounded synchronous one-shot can use `spawnSync` with explicit `SIGKILL`
timeout and buffer limits because these two selected operations own no descendants.
On timeout, overflow, bad framing, signal or nonzero exit, return no projection or
receipt and observe the direct-child terminal result before ordinary return.
Success-looking stdout before failure is discarded. Remove private staging on
all normal paths; host SIGKILL may leave unadvertised scratch, never valid success.
Do not claim hard real-time guarantees under host/kernel failure, cancellation
callbacks while the event loop is blocked, or arbitrary descendant supervision.
The HOST gate, rather than this deadline alone, decides whether this bridge is
acceptable at consumers.

## Generation handoff and available offline work

C3's pure/facade implementation and pinned offline vertical can proceed after its
implementation grant while the generation owner completes its handoff. Full C3
exit additionally needs the controlled-generation proof; a local fixture that
imitates packaging is not that proof. Missing generation work blocks that claim,
not unrelated offline progress, and does not create a new user permission flow.

Request one concrete owner-produced inventory/configuration contract:

1. A regular staged compiler artifact under the closed source root, relative
   artifact path, SHA-256, executable bit, source/build/lock/toolchain identities,
   supported protocol versions and launch-profile ID. A proposed conventional
   relative path is `app-server/bin/work-engine-compiler`; the generation owner
   freezes the actual path before its inventory changes. The bridge module and
   its runtime imports must be in that same closed inventory.
2. Saved trusted selection pointing only within the reconstructed snapshot,
   bound to its inventoried digest/protocol/profile. The generation owner decides
   whether this is represented in existing generated configuration or launch
   wiring; C3 supplies the consumer schema. Missing/corrupt/mismatched selected
   executable refuses construction. No environment override may redirect a
   sealed generation to `rust/target`, PATH or an arbitrary external binary.
3. Continued propagation of all three existing path bases and exact hydrated
   requirement values. Candidate physical bytes, canonical identity paths and
   operational destinations remain separate. Do not replace identity with a new
   source worktree path to simplify packaging.
4. A provider-disabled generation test using owner-controlled scratch roots:
   capture, reconstruct, project, satisfy, terminate and reconstruct again with
   identical manifest/environment/receipt identities and the observed pinned
   binary. Mutate/remove ambient build output and change cwd between runs;
   behavior remains pinned. Corrupt/missing/non-executable or mismatched snapshot
   artifact refuses before delivery. No activation or live semantic-store access.

The uncommitted [location reconciliation v2 proposal](../planning/executable-generation-location-reconciliation-plan-v2.md)
and [review clarifications](../planning/executable-generation-location-plan-v2-review-disposition.md)
describe distinct physical source and preserved realization, requirements
propagation, inert preparation and activation ordering. They are proposals with
unresolved implementation acceptance, not new authority for C3. V1's placement
disposition rejects blanket location-independent identity. C3 neither implements
those broader lifecycle changes nor claims that existing worker construction is
inert for live profiles. Its controlled test uses fake transports, scratch stores
and no live semantic profile. If owner-controlled packaging cannot be isolated
without those changes, record the missing handoff and leave full C3 exit unmet.

## Vertical-first execution and compatibility matrix

First prove one complete pinned P-SUP path: C2 async hydration against the
immutable `compiler-c2/root` closure, Rust v3 projection, JavaScript facade
`projectRole`, Rust v3 satisfaction and exactly one fake delivery. Compare the
full normalized projection and receipt bytes/hashes with a fixture-only copy
of the baseline manifest implementation. Pair it with excess capability and
wrong contract-path cases that call the fake adapter zero times. Record actual
binary digest/version and child exit; importing the Rust adapter alone is not
evidence of Rust execution. This precedes broad fixture expansion.

| Boundary | Required evidence |
| --- | --- |
| Projection | Pinned manifest and synthetic minimal roles; stable sorted role IDs/capabilities/effects, retained skill order, duplicate names/grants, unknown fields, identifier rules, exact contract inclusion, compiled-secondary metadata, continuity/defaults, thread-option mapping and rejected authored `effort`/`ephemeral`. |
| Identity | Different delivery, identity and requirements roots; absolute/relative/`..` and changed cwd; equal environment identity on delivery relocation; changed canonical paths/grants/instructions/role requirements change identity. Secondary attached requirements retain the current digest omission. Loaded source hash and direct-document hash remain distinct. |
| Codec | Existing C1 vectors plus manifest/receipt payload vectors: UTF-16 non-BMP ordering, numeric keys, escaping, finite numeric rendering, absent/null, array order and opaque unsigned requirement fields. Compare bytes and hashes, not only parsed JSON equality. |
| Satisfaction | Role and every compiled secondary occurrence: missing/excess capabilities, secondary grant outside role, effect outside ceiling/prohibited, continuity mismatch for role only, stale fingerprint, absent compiled requirement, wrong contract kind/path/activation requirement, absent skill, unverified source marker and digest tampering. Every refusal is before fake delivery. |
| Hydration/direct | All six pinned occurrences: supervisor; builder and builder repo-search; reviewer repo-search, claim-evidence and agent-instruction-review. Preserve role/secondary profile and ID checks. Direct native-review projection stays unhydrated; do not manufacture requirements or invoke Python there. |
| Facade | Synchronous object/receipt or throw, async loader/hydration still promises; `instanceof`, frozen object graph, role/instance validation, public supported constructor data, no caller mutation, and existing output fields. Unsupported direct-object profile refuses explicitly before serialization. |
| Process | Missing/wrong-hash/wrong-version executable, capture mutation, request/result exact limit and overflow, excess nesting, duplicate/trailing/malformed/invalid-UTF-8 JSON, wrong correlation/producer/operation, malformed error, stderr, signal, nonzero exit after success bytes, stalled reader and timeout. Observe direct-child absence/exit and scratch cleanup on actual-process failures. |
| Generation | Owner handoff above; restart with the same inventoried binary; executable mode preserved; replaced ambient binary irrelevant; identity/delivery paths remain distinct; no provider, activation or live store. |
| HOST composition | Native host initialization/direct projection and manifest-backed delivery with C3 alone, R3 alone and both, plus legacy baseline; measure cold/warm normal, burst and failure/control-route behavior under one frozen profile. |

Freeze baseline manifest source/hash and full oracle payloads in C3 fixtures,
alongside explicit bases and exact C2 immutable source revision. No test may
quietly refresh the live builder closure. The dirty-main Node result remains
29/40 with eleven pre-existing source-digest failures reproduced on S1 plus only
the dirty builder; clean joined C2/R2 passed 40/40. Historical builder pin is
`ba46408c74cfd396abd68e430c5ff9230835a985333a595292ad1e7620e3da2e`, dirty bytes
`a26420e183547b73162b87030976617ad23c6f3f044f22388e64f049fff1c28d`.
Live cutover needs a separately accepted source-owner refresh and new closure
evidence. Immutable C3 gates do not resolve that obligation.

## One composed HOST responsiveness gate

C3 and R3 planners agree to one integration-owner record, proposed
`docs/rust-c3-r3-host-profile.json`, rather than independent passing budgets.
This record is a future validation artifact, not created or accepted here.
The profile fixes hardware/OS/Node/build identities, fixture hashes, role/skill
counts, request/history sizes, cold/warm cases, burst/concurrency, sample count,
control-route and timer probe interval, and maximum permitted failures before
measurements are collected. Use at least 1,000 external probe samples per steady
condition; retain cold and fault maxima separately rather than hiding them in
an aggregate percentile. Record all observations, including resource refusals.

Proposed initial engineering targets are p99 control-route delay at most 50 ms
and maximum at most 250 ms. **These are proposed targets, not an existing product
requirement or an observed pass.** Before the joined gate, the host consumer owner
compares them with actual timeout/cancellation/service needs and freezes the
accepted numeric profile and load. Smaller real deadlines take precedence; if
the owner changes a target, record the reason before rerunning rather than
raising a threshold to conceal a failure. R3 separately accounts for its SQLite
busy wait inside the same maximum; its offline five-second ceiling is not a
host latency allowance.

Run the actual Node host in a child and send monotonic-timed external probes to
its bounded fake control route while manifest operations and R3 store/admission
operations execute on their real selected bridges. Also record an in-process
timer's delay after it can run again; the external observer catches complete
event-loop stalls. Measure legacy/legacy, C3-only, R3-only and both; include
startup, largest admitted corpus, repeated secondary satisfaction, sustained
bursts, R3 contention, compiler timeout, malformed result and controlled restart.
Use fake providers and scratch stores. Record end-to-end route latency, maximum
uninterrupted stall, per-call child time and total hydration separately, with
errors and exact binary digests. A mean or a pure Rust benchmark is insufficient.

Exceeding the accepted profile leaves synchronous joined acceptance unmet.
Ordinary bounded optimization may proceed without changing contracts. If meeting
the profile requires changing synchronous public APIs, caching away required
validation, weakening failure bounds or adding a long-lived process, return that
specific boundary decision to its consumer owner. Do not quietly turn the API
async or declare timeout termination proof of responsiveness. C3's offline
semantic evidence remains useful even if the host route requires revision.

## Gates, exit and failure limits

Implementation begins with agreed C3 authority/file ownership and the existing
joined lock. No new dependencies are proposed. Any necessary dependency change
goes to the shared integration owner, serialized with S2/S3/R3. Use an isolated
target directory and an agreed stable source window for joined gates; never
overwrite another lane's evidence executable with a workspace build.

Future focused commands, after these tests exist, include:

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli --locked
cargo +1.92.0 fmt --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli -- --check
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p work-engine-compiler -p work-engine-compiler-cli --all-targets --all-features --locked -- -D warnings
cargo +1.92.0 build --manifest-path rust/Cargo.toml -p work-engine-compiler-cli --release --locked
node --test app-server/tests/rust-manifest-adapter.test.mjs app-server/tests/runtime-manifest.test.mjs app-server/tests/rust-compiler-adapter.test.mjs
```

Node gates select the exact release binary/hash and immutable C2/C3 fixture
roots. Legacy oracle and Rust-selected runs are separate, attributed modes.
The generation owner contributes its focused worker/snapshot gate and R3 the
native-host gate. One integration owner runs the joined HOST test and proportionate
workspace tests/fmt/Clippy/release against exact agreed source/lock bytes; these
commands and predecessor counts are not evidence that new gates have passed.

Full C3 exit requires exact projection/receipt compatibility on the supported
profile, the first real vertical, all material refusal/process cases, unchanged
C1/C2 behavior, owner-controlled generation/restart evidence, the accepted joined
HOST budget, and no unresolved material separate-review finding. The result
receipt distinguishes offline-core readiness, generation handoff and joined
host acceptance so an unavailable prerequisite cannot be labeled completion.
Retain the same builder and separate reviewer through bounded remediation.

All failures return no new projection/receipt and cause no adapter delivery;
there is no write to live state to roll back. Offline tests discard scratch by
their owners. An explicit legacy selection can be tested as a separate run;
it is never automatic fallback after a Rust failure. Reverting deployed
generation selection, changing bindings, refreshing pins or activating a successor
belongs to the existing owners and is outside this plan. No C4 or default/live
cutover follows from C3 acceptance.

The remaining handoffs are concrete: generation owner freezes artifact inventory
and snapshot selection; R3 owns any host wiring change; the integration/consumer
owner freezes and measures the single HOST profile. The source owner retains the
live builder refresh obligation. These are coordination inputs and exit limits,
not newly invented authority or reasons to stop authorized offline planning.

## Evidence limits

Tier **Verify**, graph project `home-bline-code-work-engine`; initial parent
generation `2026-10-07T19:16:32Z`, child `index_status` confirmed the matching root
and ready state. Exact runtime-manifest discovery returned 36 symbols with no
remaining page. Both-direction depth-one traces for projection, hydration and
satisfaction were untruncated; material snippets and direct source reads establish
the boundaries above. Generation bootstrap and role-environment inbound traces
included tests and were untruncated. Test discovery corrected guessed nonexistent
bootstrap/role-environment test filenames to the actual worker tests; no negative
whole-repository caller or test-coverage claim is made.

Coverage for all 28 relied-on governing/source/test/proposal paths at generation
`2026-10-07T19:33:11Z` reported no recorded issue and matching metadata. This is
best-effort evidence, not completeness or runtime proof. Proposal documents were
read as proposals; no historical review result was promoted to C3 authority.
No product tests, compilation, Python, provider, live-state access, activation,
staging or commit was performed for this plan.

| Baseline source | SHA-256 |
| --- | --- |
| `app-server/src/runtime-manifest.mjs` | `b694ad405c13ac7932534a214066587c0b3c9995cc6d42556f7fb0ba0901cf3e` |
| `app-server/src/executable-generation-bootstrap.mjs` | `d1341f128d3d5f2f4c9368e92fd70991b21206d8e9a61db57c1afc5c8bbe8047` |
| `app-server/src/executable-generation-role-environment.mjs` | `ba5256d1984268c118c411fa5285a7d8c5fefa2f6138bc00ef7e088dd1cc8514` |
| `app-server/src/executable-generation-snapshot.mjs` | `d9a2c4c8f2f1a5cc6efc2f5a7329b17b5e22aceb1212a23b715222f9e3b389af` |
| `rust/crates/work-engine-compiler/src/codec.rs` | `ca940efcf512b70fc9511e51ed74a906613afcb116a4e59224eba1aa29306063` |
