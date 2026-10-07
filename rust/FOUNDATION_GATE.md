# S0 Rust foundation handoff

This document describes the **implemented S0 contract revision**, not a running
lifecycle service. The accepted S0 slice is `s0-attempt-1` / `s0-plan-v1` in
`rust-context-lifecycle-implementation-20261007`. Its 37-path boundary is frozen
in the accepted plan (SHA-256
`26faf3980c3e7850e2b976b1f6b1ce65b0d57d462c0286f610e5c87238b1f6b6`).
S0 has no provider entry, SQLite implementation, executor implementation,
controller, migration, service activation, U0 fixture/schema, or U1 observation.

## Reproducible base and contract version

The root `Cargo.toml` is SHA-256
`a5a4a4939f78b0a52c24b724d5c52171fbc1589d228ba95a1f8904d7abc1a4c1`;
`Cargo.lock` is
`538487febae20503446d5cd1ddc206eecdab876a00a42c6948a275648111f858`;
`rust-toolchain.toml` is
`1d76a06616a42a984bad672eeb24ca39a0496e95a8b624e21df0d420d1557734`.
The virtual workspace has exactly seven explicit members: `work-engine-types`,
`lifecycle-core`, `lifecycle-store`, `lifecycle-runtime`, `lifecycle-provider`,
`lifecycle-wire`, and `lifecycle-client`. Edition is 2024, resolver 3, minimum
Rust 1.92, pinned toolchain 1.92.0. All packages are private (`publish = false`).
The direct normal dependency pins are Serde 1.0.229, serde_json 1.0.151,
sha2 0.10.9, serde_json_canonicalizer 0.3.2, and thiserror 2.0.21;
trybuild 1.0.121 is dev-only. The only explicitly changed Cargo profiles are
overflow checks in dev and release and release `panic = "unwind"`.

`work-engine-types` owns the trusted closed `CodecContract` registry. Protocol
v1 command and snapshot JSON use the `jcs-rfc8785-v1` codec, an envelope binding
domain, kind, schema version and payload, and SHA-256 of exact canonical UTF-8.
Binary artifacts and lifecycle controlled-text input use distinct
`exact-bytes-v1` contracts and hash exact bytes. Callers must verify
the declared domain/kind/schema/codec against a trusted contract; they cannot
select a historical codec by digest trial. Unsafe large JSON integer values
are rejected; exact counters and decimal quantities use validated strings.
The fixture pins RFC 8785 ordering, escapes and numeric formatting, positive
exact-byte binary/text digests, and distinct historical claim-evidence and
workspace-coordination bytes. No historical digest is migrated or recomputed.

The S0 controlled-text effect carries a typed input ID, exact UTF-8 text and
trusted text-input digest in its plan. The store-owned, non-clone
`AuthorizedEntry` carries that same typed input across the claim boundary.
The executor must derive `TextTurnRequest.input` from the claimed entry;
the provider request retains the input ID, bytes and digest. S1 must commit
and revalidate the plan-to-entry association; S2 must implement the transfer.
The interfaces and negative digest test establish the data path, not durable
admission or a working provider.

## Exact existing interface files

Hashes below identify **every existing S0 public interface source file** and
its package manifest. Tests and fixture are identified by command in the next
section; their full hashes are in the implementation receipt. New later-lane
files are *not* implicitly authorized by this handoff.

| Owner | Existing file and SHA-256 | Public contract now exposed |
| --- | --- | --- |
| Shared | `crates/work-engine-types/Cargo.toml` `eddc4601fef3392d78fc47c1076112c862c30d03de01a48af90f8d7d85f937cf` | Exact normal dependency inputs. |
| Shared | `crates/work-engine-types/src/lib.rs` `f8e10df2bcf5e722599c9297ae95efcd974a743f31ce5980838dfaea17c297ba` | Reexports identity and codec values. |
| Shared | `crates/work-engine-types/src/identity.rs` `9630a98a28f15ef139c019c69a5f1c81a2769f2793ac9fe52973d301339d8334` | `IdValue`, `IdError`, strict ASCII identity parse. |
| Shared | `crates/work-engine-types/src/codec.rs` `803fd5b895e6406a0afd3509a733329193b0b5e5d18b9f7f950510bf30f80a08` | `CodecContract`, `PayloadKind`, `Digest`, `CodecError`; v1 canonical and exact-byte digests, declaration check. |
| S1 | `crates/lifecycle-core/Cargo.toml` `63129c8eb4a42d9ae74296442769b031ce1c526796e2af02360a1e31c9ce55fe` | Core depends on shared types only. |
| S1 | `crates/lifecycle-core/src/lib.rs` `53d7e8d16f63ebd8754be7493f81e2d68b6fae6ee1d193dfe19fb4c9d69b6c52` | Core contract reexports. |
| S1 | `crates/lifecycle-core/src/contracts.rs` `4f1b898f05ea159db4d7706af6b20aa094333423d7af6e0d5b03111fa959e7ae` | Distinct lifecycle IDs; `CommandRequest`, typed `ControlledTextInput`/`EffectPlan`, `ExecutionOutcome`, `EffectSettlement`, `EffectObservation`, typed clocks/revisions/errors. |
| S1 | `crates/lifecycle-store/Cargo.toml` `d2652cabc4fea5a9c224deece5c301786b476eba9ef7fb0bbed3e7919ccb1e2f` | Store port depends on core. |
| S1 | `crates/lifecycle-store/src/lib.rs` `532f837059ead8243943ce28e24452af8707840f1f62d4832f43aee0aeb42f49` | Opaque input-bearing `AuthorizedEntry` and `CapturePermit` with private fields; `LifecycleStore` transactional signatures and `StoreError`. No permit constructor or store implementation. |
| S2 | `crates/lifecycle-runtime/Cargo.toml` `123f7403d1d540ab27bb83212d7b680f0988ef872918be58b92afa6c6135acc5` | Runtime port depends on core, store and provider. |
| S2 | `crates/lifecycle-runtime/src/lib.rs` `28222a9f7fb0154b2cbcbb9bab419b533a0f24f748715a3fd9bcd3a3b5866437` | `LifecycleExecutor`, `TaskExit`, `RuntimeError`; consumes store-owned entry. |
| S2 | `crates/lifecycle-provider/Cargo.toml` `74d345711d06f94cc598b1a285d348fcc74f5533a44536d2f4075de2b53d2ef9` | Provider port depends on core only. |
| S2 | `crates/lifecycle-provider/src/lib.rs` `662d1eedaa7f0382a6fa5389b69d9092cbbad40389c77fe610fcb26524135eab` | `TextTurnPort`, `OperationProfile`, input-bearing `TextTurnRequest`, `TextTurnObservation`, `ProviderError`; no adapter. |
| S3 | `crates/lifecycle-wire/Cargo.toml` `1c183aab4cd1efa49af730303c2f835ee0d6ee2c92d5db28c6f74e75808467e8` | Public wire normal graph has shared types, Serde and error support only; core is test-only. |
| S3 | `crates/lifecycle-wire/src/lib.rs` `825297d18874902d8405384bc4780c52709eb0071c544c8d408c7f4f3b75e6c9` | `PROTOCOL_VERSION = 1`, `CommandDto`, strict `parse_command`, `WireErrorCode` including stale/conflict, `CursorV1`, `WaitResultV1`. DTO is data, not a grant. |
| S3 | `crates/lifecycle-wire/src/json.rs` `664e5cfb7cebf5a88b312c90a782b3da097e6c84783a2601232c2407449fa16f` | Recursive duplicate-key, malformed Unicode and unsafe-number rejection before `Value` collapse. |
| S3 | `crates/lifecycle-client/Cargo.toml` `8999a5514aad1ea4bcc7d46282439655573285f2e873391683330c868bd4c154` | Client normal graph depends on wire only. |
| S3 | `crates/lifecycle-client/src/lib.rs` `b72f16b15058c9400743d8f8ad3def9c8757c9995931e768fa39b0b5f7f8cb08` | Read-only `ObservationTransport`, `ReadClient`, `SubjectRef`, `SnapshotHeaderV1`, `ClientError`; no mutation method. |

The UI remains a separate nested workspace: root `members` are explicit and
`exclude = ["crates/terminal-ui"]`. Its unchanged standalone manifest hash is
`a56779f4a31bc63bef0bcfe1499fbbdf1e22f7704f77b3a8e77a774d67911b67`
and lock hash is
`1789b5c493dba5c6bbd7f09afbfe32c8017fedf61f3b2c0045acf4ce72113d02`.
The root toolchain file nevertheless affects rustup selection inside that
nested directory; `rustup show active-toolchain` there reported 1.92.0 and its
standalone locked check passed. No UI file, lock or manifest was edited by S0.

## Package-focused commands and proof limits

Run from repository root. The listed commands use deterministic local inputs
only; they cannot enter a provider. The `compile_fail` cases use trybuild and
must fail to compile for the intended private/type boundary.

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p work-engine-types --test codec_vectors --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-core --test identity_types --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-core --test compile_fail --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-store --test compile_fail --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-wire --test foundation_flow --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-client --test public_api --locked
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-client --test compile_fail --locked
cargo +1.92.0 check --manifest-path rust/Cargo.toml -p lifecycle-runtime -p lifecycle-provider --locked
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p lifecycle-core -p lifecycle-store --all-targets --locked -- -D warnings
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p lifecycle-runtime -p lifecycle-provider --all-targets --locked -- -D warnings
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p lifecycle-wire -p lifecycle-client --all-targets --locked -- -D warnings
```

These are the exact initial S1/S2/S3 package commands against existing
interfaces. S1 owns later core/store reducer, durable claim/application,
SQLite transaction and recovery tests; S2 owns runtime/provider execution,
process and controlled-peer conformance; S3 owns public DTO/schema/client,
version/cursor/reconnect tests and U0 fixture delivery. Those implementations,
tests and any new file names require their own later accepted manifest. Root
Cargo/lock/toolchain/shared-type changes require cross-lane integration
ownership. S4 owns the real store-to-executor-to-observer path and U1.

For an authorized S0 candidate gate, use the exact candidate inventory
and hashes in its receipt, then `cargo fmt --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `cargo build --workspace --release --locked`,
metadata/tree isolation, UI standalone compatibility, RustSec audit and
`git diff --check`. The parent/supervisor selects the gate;
these commands are not a gate pass merely because this handoff exists.

## Provider-disabled and dependency inventory

S0 has zero supported live-provider selectors, credentials, provider
implementations, binaries, offline-admin entrypoints, or fixture-provider
startup paths. `OperationProfile` and `TextTurnPort` are interface types only.
`cargo metadata --locked` lists seven workspace members, with no UI or C1/R1
package. The normal/build dependency tree for `lifecycle-client` contains
wire/types, not core/store/runtime/provider; the types tree has no Tokio,
SQLite or provider SDK. Future S2/S5 composition must replace this absence
inventory with exact feature/config selectors and verify disabled defaults.

The implementation-stage dependency check scanned 48 lockfile crate entries
with cargo-audit 0.22.2 against 1,290 fetched RustSec advisories and reported
zero vulnerabilities (exit 0). All resolved third-party package metadata
declares one of MIT, Apache-2.0, BSL-1.0, Unicode-3.0 or Unlicense in its SPDX
expression. This is metadata/license qualification, not a legal audit or a
claim about future dependency versions. The final gate should re-run advisory
and lock/profile checks against its exact candidate.
