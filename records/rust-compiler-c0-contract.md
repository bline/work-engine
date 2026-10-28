# Rust compiler C0: compatibility and C1 interface contract

Prepared 2026-10-07 under the user's bounded C0 authorization. The accepted
[implementation plan](rust-compiler-implementation-plan.md) is preserved at
SHA-256 `7e64984a4b399578d01c2728d2bb25e62d0e5b352fc9304f1773f45dc4b9a24e`.
The user supplied the review and accepted its direction; this document does not
claim a newly conducted independent review or acceptance of the C0 result.

C0 produces source characterization, corpus specification and the proposed C1
handoff. It creates no fixture files, executable vectors, test harness, crate or
runtime change. C1 and later implementation remain unauthorized. The later
builder is `gpt-6-sol` under the user's explicit direction; C0 remains architecture
and documentation work.

## Decisions made in C0

- C1 implements only unverified skill compilation, with all five currently
  manifest-referenced compiled skills in its compatibility corpus. Source reads,
  Python AEG invocation and verified output begin in C2.
- The versioned local executable accepts source bytes, not source paths or an
  authority flag. Its pure library receives the same bytes without host handles.
- Rust emits `work-engine.skill-compiler.rust-v1` in `ir.compiler`. This accepted
  producer change is explicitly excluded from producer-neutral IR comparison;
  no other field is excluded merely to obtain parity.
- Legacy compiler canonical encoding stays domain-local. The trusted operation
  and format version select it; authored input cannot choose a weaker codec.
- Compatibility source is the exact pinned subject, not a moving working tree.
  Concurrent source drift is its own refusal case, not a reason to refresh pins.

## Evidence and exact source corpus

The source baseline is HEAD `7583e2095725957470e28cc11fc69404386fec63` with
individually observed working-tree hashes. The accepted plan's evidence table
still identifies the compiler, manifest, extension, Python and test source bytes;
their listed hashes were checked again and matched. Its migration-plan hash is
historical because the parent has added coordination text. These observations
are not an atomic snapshot or newly executed behavioral results.

Five input pairs below are referenced by the pinned
[runtime manifest](../app-server/runtime-manifest.yaml). Each `structure.yaml`
and `interface.yaml` is an exact byte input, including comments and whitespace.
The expected output column is the digest declared in structure/manifest and
observed on the matching canonical bytes, except the live builder drift below.
It is an expected C1 assertion, not a compilation result produced in C0.

| Corpus ID and input links | Raw structure SHA-256 | Raw interface SHA-256 | Declared output SHA-256 |
| --- | --- | --- | --- |
| P-SUP: [supervisor structure](../app-server/migrations/skills/slice-supervisor/structure.yaml), [interface](../app-server/migrations/skills/slice-supervisor/interface.yaml) | `7cf2bd8c33a05b8c49a1567118352aee1d8c05d8b83e584bd1d6f84d137e850b` | `62e2261c20e1b605ca281afbc2c5b5811e2b2dfb929de7c4e8ccfe97515c1fcc` | `6b26d4dd8294be02d8fc8f462af9f3c505142fb5c060ef31529f9a28fcf48d9e` |
| P-BLD: [builder structure](../app-server/migrations/skills/slice-builder/structure.yaml), [interface](../app-server/migrations/skills/slice-builder/interface.yaml) | `9ad73f6d2e683ce2891b7bdf7dbfbbccf5f2e8b213bbc7055c054c1b0923b898` | `50a5cf4a3bc69900a4694e3753e245552b308f8809763e653fc2b331d38cb12a` | `ba46408c74cfd396abd68e430c5ff9230835a985333a595292ad1e7620e3da2e` |
| P-REP: [repo-search structure](../app-server/migrations/skills/repo-search/structure.yaml), [interface](../app-server/migrations/skills/repo-search/interface.yaml) | `05fc9c134c0957d6bdf6d8b0dfb94c01fd33da2b4ae28aeb10c604b8a8bd31ff` | `69c795407d84cafeb30c4e4254be106f5df39e73031a417189de5e8835f9d82e` | `cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949` |
| P-CLM: [claim-evidence structure](../app-server/migrations/skills/claim-evidence/structure.yaml), [interface](../app-server/migrations/skills/claim-evidence/interface.yaml) | `96d49135fd31c590dade3a126a91baff2ee17d7b2be175fc608bd618647b783f` | `2181d13d0f8cbe4d3844a10a665e286d3232452e9b9d4c9b28901ea07d7c296d` | `afd233654dbc0b70262eaabf359e5d9f0619e1a36fb4889b301bd32ed6e2800a` |
| P-AIR: [instruction-review structure](../app-server/migrations/skills/agent-instruction-review/structure.yaml), [interface](../app-server/migrations/skills/agent-instruction-review/interface.yaml) | `4d47d42de934b8a5d020294bdf46eca6e591b4ad0566c9903fd8e6f97effeffa` | `18c8c47cd7fb3e2a6778becbbe583f79864f2114113ea730776b48363900fbdc` | `ab652b616869062170f9adca8299e6fe1ee934d3a3e21c8f030a4d37ebebec72` |

P-SUP/P-BLD contain role profiles; P-REP/P-CLM/P-AIR are role-free. In C1 all
five use unverified compilation. Their source paths, historical
`source.repository_revision`, `source.producer`, declared binding digests and
optional projection digest remain authored data; C1 does not verify them or
rewrite them to the compiler build revision.

For C2, the same structures precisely enumerate the source closure:

| Cases | Canonical source and additional declared bindings |
| --- | --- |
| P-SUP/P-BLD | `skills/<skill>/SKILL.md`; [environment](../docs/agent-environments.yaml) `38002804a2ef3966e58ecef5af50f9376f302a08964ba9d139e8a562aa30e97b`; [invariants](../docs/workflow-invariants.md) `124e27108241e42b6fe5f42c499cd5153a14e3c122e9bf41993f96a378f1b4e9`; respective generated projection below. |
| P-SUP projection | [slice-supervisor.yaml](../docs/agent-environment-views/slice-supervisor.yaml) `1071afffd9b6f0162d3e6ffab3e6bc478c0635408ded599f2e8d7c02744f9c6c`. |
| P-BLD projection | [slice-builder.yaml](../docs/agent-environment-views/slice-builder.yaml) `6f7e3644b3f555aafb5a25ea2aec2d487ad3d853a3aecba97ae4111ce819d8cb`. |
| P-REP | [skill](../skills/repo-search/SKILL.md); [openai.yaml](../skills/repo-search/agents/openai.yaml) `fa5513bd8685bf7ab885008307c84056b2b5445150d176fbf210b805fb6b087c`; [backend capabilities](../skills/repo-search/references/backend-capabilities.md) `e0458fc1ba9773d7f5bf4853c2e4dfad7bb82615c71f4f2d5a34a13ce9652f91`. |
| P-CLM | [skill](../skills/claim-evidence/SKILL.md); [claim contract](../skills/claim-evidence/references/claim-evidence-contract.md) `e0e8d452b5e64cc3432ec727e47d4f8a2fee3c4e4e37d8e2dc318c4b6923d0f3`; [openai.yaml](../skills/claim-evidence/agents/openai.yaml) `bd967b550a6a9a439241361d504ece223fbb11fc0ac6e4f74dbfbebe47084054`. |
| P-AIR | [skill](../skills/agent-instruction-review/SKILL.md); [finding contract](../skills/agent-instruction-review/references/finding-contract.md) `ad0099e0744011aeb5662f8e630d3cf7b70411635b09a8b9dc7e856df625f69a`; [openai.yaml](../skills/agent-instruction-review/agents/openai.yaml) `b1505aab3948e89ef90ea70cc3b2c7435baa3d5b16d4d4d8f44765584140f0d5`. |

These declared binding hashes matched direct byte observations during C0 except
the builder's live skill. At observation it was
`373c87fe0a99c86608a4ce40d6f2296a09712e0d4f1b5e48fe85668a2318e118`, reflecting
the separately authorized model-adapter repair. Reading the baseline Git blob
for `skills/slice-builder/SKILL.md` yielded the declared `ba46408c…` digest.
Therefore P-BLD uses that exact immutable blob for its expected output; a
verified compile against the observed live bytes is a source-predicted rejection
at `verifyLegacySource`, before AEG invocation. Neither outcome was executed.
The source owner handles any future decomposition/projection/manifest refresh;
C0 leaves all those files untouched.

Later extension corpus inputs are [sealed-bundle.json](../app-server/tests/fixtures/run-extension-bundle/sealed-bundle.json)
(`71f4efa7f0447ab915992a155b3c7e54126041cd8636d34c006a76d5b22965d8`),
[rejected-bundle.json](../app-server/tests/fixtures/run-extension-bundle/rejected-bundle.json)
(`74454bdff4ddc4b732b0898c67c830bad465e31df7afb8783de833b0775d0a25`) and
[linguistic-dry-run.json](../app-server/tests/fixtures/run-extension-bundle/linguistic-dry-run.json)
(`38ec858cffa13fec033810be681093091c6c31b8faaf150cebd08595f690e13a`).
The latter two use the test fixture's shallow inheritance plus merged `run` map;
`extends` is test loading behavior, not a new compiler feature.

## Case specification: known source behavior and future executable assertions

**S** means direct implementation source supports the outcome; **T** means an
existing test expresses the assertion, not that it passed now. **V** marks a new
executable vector to construct later. C1 fixtures record exact input bytes,
expected output bytes or rejection code, and source/test references. Parser
cases whose result is not settled below are characterization cases, not silently
accepted or rejected values. There is no generated expected-IR file in C0.

### C1 skill cases

Evidence: [skill implementation](../app-server/src/skill-compiler.mjs) and
[skill tests](../app-server/tests/skill-compiler.test.mjs), including the named
tests “vertical compiler path”, “pinned slice-builder decomposition”,
“schema v1 rejects duplicate keys”, and “repo-search compiles byte-exactly”.

| ID | Exact input or mutation and expected assertion | Basis |
| --- | --- | --- |
| S01 | P-SUP, P-BLD, P-REP, P-CLM, P-AIR unmodified: output matches the pinned canonical Markdown; every result has `verified_sources: false`; role-free IR omits `role_profile`, `role_projection`, role-projection input hash and requirements `role_id`. | S/T; C1 |
| S02 | Synthetic two-section `vertical` test, retaining `must_require` and `observation_limits`: preserve section order and complete relation maps. The test's injected fake AEG response is not production verification evidence. | S/T; C1 pure portion |
| S03 | Duplicate `schema_version` key; extra `whole_document`; root sequence/null; wrong schema version/status: reject without returning partial IR/output. | S/T; V root/version cases |
| S04 | An alias reference in either input: reject; an unused anchor declaration is a separate parser vector, not automatically a prohibited alias. Multiple YAML documents, tags, merge keys and non-string keys need explicit characterization against the pinned parser. | S configured alias limit; V |
| S05 | Empty/duplicate source-binding, section or boundary IDs; missing/unknown binding kind; absent source-bindings; fewer than two sections: reject. Empty text content is allowed; empty required text is not. | S/T |
| S06 | `generated_evidence` as boundary authority, missing referenced section, or mediation mapped to the lifecycle section: reject. Preserve the exact boundary-kind/section-kind and enforcement vocabularies in source. | S/T |
| S07 | Required capability absent from ceiling or role `may_invoke`; effect outside role mutation targets; permitted/prohibited overlap; duplicate entries; unsupported continuity: reject. Permute valid capability/effect lists and expect sorted projected requirements. | S; V |
| S08 | Empty or duplicate `must_require` entries, non-map relation extensions, empty observation entity/limit; malformed mutation/mediation relation objects: reject. Optional `must_require`/`observation_limits` omitted or null follow current omission behavior. | S/T |
| S09 | Absent or null `role_profile`: role-free result; `{}`: reject missing projection/relation structure. A secondary skill cannot become a role simply by naming a role-like skill ID. | S/T |
| S10 | Bad section digest, level 0/7/noninteger, negative span, or `end_byte <= start_byte`: reject in C1. A positive-length span with the wrong absolute offset may still compile unverified; canonical contiguity/EOF is C2's assertion. | S/T |
| S11 | Synthetic valid sections containing `é`, non-BMP text and combining marks: count UTF-8 bytes, preserve text without Unicode normalization, match section/output hashes. Paired surrogate escape handling and non-BMP map-key ordering get distinct vectors. | S; V |
| S12 | Body with/without trailing LF crossed with `final_newline` true/false: true adds one LF only if missing; false removes one trailing LF only. Do not trim all whitespace/newlines. `line_ending: crlf` and a description containing LF reject. | S; V |
| S13 | YAML comments, mapping order, whitespace or BOM changed without changing parsed meaning: semantic output stays equal where parser-admitted, while raw input hashes change. No input normalization before hashing. | S; V |
| S14 | Change only historical source revision/producer or a declared binding digest to another syntactically valid value: unverified compilation does not consult Git/files or claim the assertion true; preserve the changed metadata. | S; V |
| S15 | Finite integer offsets around 2^53, negative zero, exponent forms, escaped control characters, and empty versus whitespace-only required strings: characterize JavaScript numeric/string semantics. Do not substitute Rust integer parsing, trim rules or parser defaults silently. | S predicates; V exact vectors |

The supplied text validators are not a Markdown/YAML authoring sanitizer. For
example, `text()` checks string length rather than trimming, and the frontmatter
renderer rejects LF in description but does not quote all frontmatter values.
C1 preserves characterized v1 semantics; any later hardening is an explicit
compatibility change, not an incidental side effect of a stricter Rust DTO.
Raw invalid UTF-8 and lone surrogate strings are outside the C1 byte-input
profile below; existing Node replacement/encoding behavior requires a later
compatibility disposition before a legacy consumer is switched.

### Reserved cases for C2–C4, specified now but not implemented by C1

| ID | Input/case and expected assertion | Source/test owner |
| --- | --- | --- |
| V01 | P-BLD immutable closure verifies; observed live builder drift refuses before AEG. Alter canonical prefix, first-section start, middle contiguity, EOF, any binding bytes or projection bytes: refuse verified output. | S; [skill tests](../app-server/tests/skill-compiler.test.mjs), C2 |
| V02 | Candidate role differs, pinned complete projection differs, envelope schema/status/backend/digest/match is wrong, success has stderr or malformed JSON: refuse verification. | S/T; [AEG tests](../app-server/tests/agent-environment-graph-adapter.test.mjs), C2 |
| V03 | Python spawn/stdin error, timeout, independent stdout/stderr overflow, nonzero exit or signal: no verified result; child exit/reaping observed. No provider call. | S/T failure checks; new reaping assertion, C2 |
| M01 | Pinned runtime manifest with explicit canonical, delivery and requirements bases; hydrated versus direct projection without hydration remains distinct. Shift only delivery base to a snapshot and retain canonical environment identity. | S/T; [manifest tests](../app-server/tests/runtime-manifest.test.mjs), C3 |
| M02 | Change process cwd after projection: secondary satisfaction retains requirements-base meaning. Missing capability, excess capability/effect, wrong continuity/path/fingerprint, unverified requirements or modified requirements digest refuses before fake delivery. | S/T; manifest tests, C3 |
| M03 | Duplicate skill names, unsupported `thread_options.ephemeral`/`effort`, missing exact contract input, role used as secondary or wrong secondary skill identity: reject. Authored `reasoning_effort` maps to runtime `effort`; do not rename authored input. | S/T; manifest implementation/tests, C3 |
| X01 | Sealed fixture with host revision `2af529971c1b5660de4caad7092cd270dcd162eb` and the test's exact allowlists: expect unverified P-REP skill, sealed networking, no credentials and admitted run-local registry. | S/T; [extension tests](../app-server/tests/run-extension-bundle.test.mjs), C4 |
| X02 | Rejected fixture; mismatched revision/input digest; unknown adapter/provider; network dependency; duplicate registry key; unavailable capability: reject before activation. | S/T existing subset; V remaining cases, C4 |
| X03 | Compile attachment → clone → JSON persistence/restart → registry digest recheck: separately record Buffer, typed-array and JSON-object forms. No assumption that all forms hash alike; do not bless a failing legacy round trip as new success. | S representations; V, C4 |
| X04 | Extension unknown opaque fields, source-revision substring matching and an effect not on the current forbidden list: characterize exact retention/rejection. Current checks are not a proof of stronger repository or authority confinement. | S; V, C4 |

## Byte, identity and producer rules

Use `Hraw(bytes)` for SHA-256 of the exact bytes. The domain-local
`compiler-js-canonical-json-v1` profile means the existing recursive encoder:
sorted JavaScript UTF-16 object keys, array order retained, JavaScript scalar
serialization, UTF-8 output and no appended newline. Domain wrappers keep skill
requirements, manifest environments and extensions distinct even where that
algorithm is shared. This profile is not claim-evidence's newline/code-point codec.

| Artifact/field | Hash input and effect of the accepted Rust producer change |
| --- | --- |
| `input_sha256.structure` / `.interface` | `Hraw` of decoded source bytes; no YAML reserialization. Unchanged for identical input bytes. |
| `input_sha256.role_projection` | The declared projection digest when a role profile exists, including in unverified compilation; it is not a C1 observation of that file. |
| `output_sha256` / `compiled_skill_sha256` | `Hraw` of rendered Markdown; unchanged when output bytes match. |
| Requirements `sha256` | Canonical JSON of requirements without its own `sha256`. Includes `verified_sources`; does not contain `ir.compiler`. Producer rename alone leaves it unchanged; false→true verification does not. |
| Manifest environment/satisfaction hashes | Preserve the existing unsigned payloads and path bases. Compiler implementation identity is not inserted into those payloads. This does not promise equality when actual requirements, grants or paths change. |
| Extension input digest | SHA-256 of canonical JSON **string**, not raw source bytes. Sealed fixture's P-REP structure/interface pins are `32c59cfe7e5e2a384563826a75de4d918e694bb85de021851337518818eb5ec3` / `10045eed4aa1ed780427702a7a7d4e0e1be085d2b2f761e72929cd51ad22c816`, distinct from the raw hashes above; these fixture declarations still await executable verification. |
| Extension descriptor | Compiled-skill subset excludes `ir.compiler`; that rename alone should not alter descriptor identity. Output-byte representation still needs X03 qualification. |
| Entire IR / CLI summary | `compiler` changes from `work-engine.skill-compiler.bootstrap-v1` to `work-engine.skill-compiler.rust-v1`. A digest of the entire serialized IR/summary therefore changes; historical artifacts are not rewritten. |
| Python and build provenance | C2 preserves the Python owner's actual `backend`/`backend_sha256`; Rust does not inherit that identity. The invoking host records the Rust binary's observed digest separately from self-reported producer text. A new binary/source inventory can change generation identity. |

New codec vectors K01–K05 cover U+1F600 versus U+E000 key order; `"10"` versus
`"2"` keys; escaped quotation/backslash/control characters; absent versus null and
array order; numeric boundary rendering around 1e-6, 1e21 and negative zero.
Their exact expected serialized bytes are to be captured and checked in C1,
not inferred to be qualified because this document names them. Historical Buffer
encoding belongs to X03/C4, not an arbitrary generic JSON value in C1.

## Source, path and host trust boundary

The C1 input profile is explicit valid UTF-8 bytes, no normalization and no
filesystem lookup. Input base64 is only transport; decoded bytes are hashed and
parsed. Rust parser differences cannot silently expand admitted YAML semantics.
Declared paths/revisions in a structure stay data even if absolute or containing
`..`. They neither choose a binary nor trigger a read in C1.

For later compatibility adapters, preserve lexical POSIX path identity on the
current Linux target. `workspaceRoot` resolves canonical source, binding and
projection reads; manifest directory resolves authored compiler inputs and default
delivery paths; `identityBaseDirectory` determines canonical identity paths;
`requirementsBaseDirectory` resolves requirement contract paths. The Node CLI's
structure/interface/output/compare arguments currently resolve from invocation cwd,
while `--workspace-root` controls verification. Resolve those defaults once at
the host boundary and pass explicit bases, rather than consulting ambient cwd
inside pure operations. Preserve legitimate `../skills` paths.

Existing `path.resolve`/`readFile` behavior is not workspace confinement and may
follow symlinks. C0 chooses no new root jail or `realpath` normalization: either
would change supported identity/read behavior. Compatibility execution is a
trusted local host operation under that host's filesystem authority, not a public
file-reading service. A remote/untrusted API or confinement change needs its own
owner decision. These paths never select Python executable/script locations,
Rust binary, allowlists, credentials, registry handlers or backend implementation;
those belong to host configuration. C1 has none of those host capabilities.

C2 observes each consumed byte buffer and hashes that buffer; matching source
metadata does not prove canonical authority beyond the owning source contract.
Its source-capture procedure cannot claim an atomic repository snapshot. Immutable
generation/source closure stays with the existing owner. Missing/unreadable bytes,
drift and unavailable Python remain explicit failures; no downgrade to unverified
output satisfies a caller that requested verification.

## C1 public boundary and exact proposed file ownership

The proposed binary name is `work-engine-compiler`. Its initial one-request
stdin/EOF protocol is version 1 with exactly these request fields:

```json
{"schema_version":1,"operation":"compile_skill_unverified","structure_source_base64":"...","interface_source_base64":"..."}
```

No caller-supplied compiler ID, codec, verified bit, source root or command field
is admitted. Base64 is the standard padded alphabet; decode then validate UTF-8.
Default bounds are 16 MiB request and result envelopes and 4 MiB per decoded
source. These are explicit local resource limits, not new YAML validity rules;
oversize input reports `resource_limit` without truncation. A host may use a
different explicit bound without changing identity bytes or semantic validation.

Success is one JSON envelope containing `schema_version: 1`, `status: "ok"`,
`ir` and `output_base64`, followed by LF, with exit 0. Error is one envelope with
`schema_version: 1`, `status: "error"`, and `error` containing `code`, `path`
(nullable JSON pointer) and `message`; optional source span identifies input,
start/end byte offsets. Input failures exit 2; internal execution failure exits 1.
Diagnostics use stderr, never prefix/suffix a successful stdout envelope.

Initial codes are `invalid_request`, `unsupported_version`, `unsupported_operation`,
`resource_limit`, `invalid_encoding`, `invalid_yaml`, `invalid_structure`,
`invalid_interface`, and `internal_error`. No successful partial IR accompanies
an error. Exact legacy exception prose is not the contract. Do not promise which
of several simultaneous invalid fields is diagnosed first. C1 tests use one
decisive mutation per rejection case and assert code/location/consequence.

| C1 owner | Exact paths after S0 registration; not created in C0 |
| --- | --- |
| Compiler builder | `rust/crates/work-engine-compiler/Cargo.toml`; `src/lib.rs`, `src/skill.rs`, `src/codec.rs`, `src/error.rs`; `tests/skill_v1.rs`, `tests/codec_v1.rs`; `tests/fixtures/v1/` beneath this package. |
| Compiler builder | `rust/crates/work-engine-compiler-cli/Cargo.toml`; `src/main.rs`, `src/protocol.rs`; `tests/protocol.rs` beneath this package. |
| Lifecycle S0 integration owner | `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/rust-toolchain.toml`, common package definitions and common gate inventory. Compiler submits requests, not parallel edits. |
| Retained external owners | No C1 changes to production JavaScript/Python, YAML/skills, native review host, lifecycle/UI code, build staging or generation inventory. Review-episode R3 owns any later native-review-host change; generation owner controls binary staging/activation. |

Future C1 fixture layout is `tests/fixtures/v1/skills/<corpus-id>/` with exact
structure/interface input bytes, expected Markdown and producer-neutral expected
IR, plus `cases.json` identifying version, inputs/digests and source/test basis.
Synthetic mutations/codecs live under `synthetic/` and `codec/`. This is a
specification only: C0 adds no manifest, fixture directory or expected-output file.
C1 captures the legacy oracle from the pinned JavaScript source and records the
actual Node/YAML identity when it runs. C0 observed [package-lock.json](../package-lock.json)
SHA-256 `412ffab5bac4ac8dcf7579c13fdc6231a93ec94d74d61330fa28e14fc8af3e29`,
declaring YAML 2.9.0; this does not attest the installed runtime or its behavior.
No Python/AEG fixture execution is necessary for C1's unverified operation.

## Foundation requests and readiness

The concrete S0 request is to register the two named packages and binary, pin
toolchain/MSRV/edition and agreed dependency/features, and accept the C1 gate
selectors already specified in the reviewed plan. Needed facilities are YAML
parsing with compatible key/alias/scalar behavior, JSON/Serde, SHA-256, base64 and
typed errors; property-test support is a development dependency if used. C1 needs
no Tokio, SQLite, lifecycle-wire/client, shared process supervisor or new common
codec. Dependency choice/version remains S0's pinning responsibility; compatibility
cases above are the compiler owner's acceptance evidence for that choice.

Domain digest/value newtypes remain compiler-owned. Reuse an already accepted
common byte/value representation only without collapsing source/output/requirements
domains; do not require S0 to invent one for C1. A narrow compiling package
registration and dependency contract is sufficient; full lifecycle delivery is
not a prerequisite.

C0 now supplies the corpus, compatibility deltas, source/path boundary, proposed
protocol/error surface and exact C1 ownership request. C1 is ready for interface
agreement, not launch: the remaining gates are acceptance of this C0 handoff,
S0's registration/dependency revision and a separate bounded C1 implementation
grant. Parser edge cases are explicit C1 characterization work, not missing C0
runtime evidence. Current live builder drift does not block immutable C1 fixtures;
the source owner's refreshed production closure is required before claiming a
current-root verified consumer is qualified in C2 or later.

Documentation checks resolve local links, verify whitespace and preserve the
reviewed-plan hash. Graph evidence remains Tier 2, project
`home-bline-code-work-engine`; additional corpus paths had matching metadata and
no recorded gaps at generations `2026-10-07T07:12:40Z` / `07:14:24Z`. The lockfile
reported untracked freshness, so its YAML entry and exact bytes were read directly;
no reindex was needed for this non-code claim. Coverage is best-effort. No product
script, test suite, compiler, AEG, provider, fixture capture or workspace scaffold
ran to produce this C0 contract; read-only coordination and documentation checks
are the only executed support operations.
