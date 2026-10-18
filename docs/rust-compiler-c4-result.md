# C4 Rust extension attachment

C4 now produces a Rust-owned, versioned attachment. The operator clarified on
2026-10-18 that the earlier JavaScript extension consumer was never released
and need not run again. No Node bridge, legacy Buffer codec, generated-file
normalization, historical attachment migration, or import of previous extension
work is part of this result. This is a fresh Rust attachment profile.
The earlier C4 candidate and its review remain in the retained evidence; this
document describes the revised implementation.

The pure `work-engine-compiler` crate compares an authored bundle with a
separate host revision and capability, adapter, and provider allowlists. It
requires a sealed run with no projected credentials, revision-bound run fields,
namespaced roots, no prohibited effects or authority requests, sealed-cache
dependencies, and admitted unique registry entries. Capability, effect,
adapter, and provider lists contain text values. It compiles skill sources
without source verification and retains `verified_sources: false` and the
`below_core` default. The attachment does not grant runtime or production
authority.

The attachment has `schema_version: 2`, profile
`work-engine.run-extension.rust-v1`, a descriptor, and a SHA-256 digest of
the descriptor's canonical JSON. The descriptor also has `schema_version: 2`.
Each compiled skill contains `output_bytes` as a JSON array of integers 0–255
and an independent raw output SHA-256. This representation survives ordinary
JSON serialization without a Node-specific byte type. The frozen sealed
fixture attachment digest is
`f8755809381fa4a79dfb75c6f2de40d118cd353ee140960d410752901b182b05`.
The C4 attachment codec orders object keys by UTF-16 units and serializes
`serde_json` numbers without casting integers to binary64. Distinct admitted
large integers therefore have distinct attachment identities. The existing
C2/C3 JavaScript-compatible codec remains separate. Skill-source admission
hashes and skill IR input hashes both use raw UTF-8 source bytes.

The one-shot CLI uses protocol version 4. `compile_extension_bundle` captures
authored structure and interface files through the existing bounded source
snapshot reader. `verify_extension_attachment` accepts a serialized attachment
and independently supplied host admission in a fresh process; it checks the
profile, descriptor digest, canonical lists, sealed policy, dependencies,
registry, unverified skill state, byte ranges and output hashes. Its success
reports attachment identity and bundle ID, not activation. A valid digest
establishes consistency of supplied bytes, not provenance from a trusted
compiler or authorization to dispatch effects. The consumer must still pin
the producer and mediate runtime capabilities under its own accepted contract.

Repeated references to one captured source count toward a 64 MiB materialized
input limit before each copy. Accumulated compiled output is limited to 512 KiB
before JSON byte projection. Compile requests are limited to 4 MiB, successful
compile responses to 8 MiB, and serialized verification requests to the 16 MiB
envelope limit. Before reporting compile success, the CLI measures the actual
serialized attachment together with the parsed host admission and a maximal
valid verification request ID. Compact numeric input may expand during JSON
serialization, so the response cap alone is insufficient. Limits fail
explicitly with `resource_limit`.
Source capture retains lexical path resolution, including absolute and `..`
paths, subject to the existing snapshot limits. Opaque admitted run,
dependency, and registry fields remain in the descriptor.

The focused library and CLI tests cover deterministic fixture identity,
serialization into a fresh Rust verifier, tampering and rehashed forbidden
changes, stale inputs, unavailable host admission, duplicate registry entries,
repeated-path memory bounds, and cumulative output bounds. The existing C2/C3
compiler operations retain their public protocol behavior. C4 does not add a
runtime registry, activation path, or deployment; consumer selection and
predecessor disposition are separate decisions.
