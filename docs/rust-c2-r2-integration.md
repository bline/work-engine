# C2/R2 integration with committed S1

Status on 2026-10-07: the bounded reviewed C2/R2 files are integrated **uncommitted** into `/home/bline/code/work-engine` on `e7c1d88ec0df20ea6213239b6dc2b36abd9f1872` (committed S1). The two historical builder receipts and [isolated validation record](rust-c2-r2-validation.md) retain their original evidence. This record covers the main-checkout overlay, root resolution, preservation audit, clean joined validation, and the coordinator's final gates below. Integration review is clear. No staging, commit, push, provider entry, live DB copy, default compiler cutover, or activation was performed.

## Coordinator final gate and review

The coordinator ran the complete workspace against the integrated main source
and lock with `CARGO_TARGET_DIR=/tmp/work-engine-c2-r2-joined-final-target`:

| Command (Cargo commands use Rust 1.92.0) | Result |
| --- | --- |
| `cargo +1.92.0 test --manifest-path rust/Cargo.toml --workspace --locked` | 122 passed, zero failures or ignored tests. |
| `cargo +1.92.0 test --manifest-path rust/Cargo.toml --locked -p review-episode-store -p review-episode --features test-faults` | 32 passed; one deliberately ignored subprocess helper. |
| `cargo +1.92.0 fmt --manifest-path rust/Cargo.toml --all -- --check` | Passed. |
| `cargo +1.92.0 clippy --manifest-path rust/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings` | Passed. |
| `cargo +1.92.0 build --manifest-path rust/Cargo.toml --workspace --release --locked` | Passed. |
| Four-file Node gate in `/tmp/work-engine-c2-r2-clean-joined`, selecting the coordinator's main-built release compiler | 40 passed, zero failures or skips. |

The Node files were `rust-compiler-adapter.test.mjs`, `skill-compiler.test.mjs`,
`agent-environment-graph-adapter.test.mjs`, and `runtime-manifest.test.mjs`, all
under `app-server/tests/`. The exact selected compiler at
`/tmp/work-engine-c2-r2-joined-final-target/release/work-engine-compiler` has
SHA-256 `13489cce7e98bff78ee7fa98b5632fc453d9cf6eb68bc0a79d21085f6e7c473d`.
The review executable in the same release directory has SHA-256
`c3659927086d41cebc623a64c85f86c75e1e21eb3ffab27125debc67eb871cab`.
These tests do not assert a passing full Node gate in the dirty main checkout;
its reproduced baseline limitation is detailed below.

The retained Astra reviewer independently verified both source aggregates,
the four-package lock delta, unchanged existing versions/checksums, and S1 path
preservation. No material integration finding remained. A separate bounded
inspection confirmed the baseline and main Node logs have the same eleven
failing names and matching source-drift errors. No product source changed in
integration, so the earlier C2/R2 domain review remains bound to identical bytes.

After final tests, the coordinator independently verified all 166 protected
file byte/mode identities, full Git index hash, and HEAD against the preflight
snapshot: all unchanged. Logs, the preservation snapshot, and their checksum
manifest are retained in
`/home/bline/.local/state/work-engine/rust-c2-r2-integration-20261007/`.
This integration used one retained Sol builder and one retained Astra reviewer;
there were no product remediation iterations. Exact token/cost metrics are not
available. Shared integration is complete; local publication remains a separate
user decision. Later host joining, source-owner refresh, and activation are not
established by these gates.

## Exact overlay and root resolution

The 46 C2 files and 18 R2 files listed below were copied byte-for-byte from the reviewed isolated checkout `/home/bline/code/work-engine-c2-r2` after rechecking its final aggregates. C2 aggregate: `0cd0ee200d1be0ec3105d4c09fbe49d02ea62583bee1d55fb65150a64805bfa7`; R2 aggregate: `653baf04361871056bd2d28242f8854f92a2c2267a8fc98652e102e1b1da963c`. Each aggregate sorts complete UTF-8 relative path strings lexically and hashes four-byte big-endian path length, path bytes, eight-byte big-endian content length, and file bytes. Eleven target files existed and matched the `34e5723` baseline exactly; 53 were new. None overlapped S1's 19 committed paths or a pre-existing dirty/staged path. The three result documents copied unchanged at overlay time were `docs/rust-compiler-c2-result.md`, `docs/rust-review-episode-r2-result.md`, and `docs/rust-c2-r2-validation.md`; this integration record and the appended validation postscript are new integration documentation.

The main `rust/Cargo.toml` added only members `crates/review-episode-store` and `crates/review-episode`, preserving existing members and the `crates/terminal-ui` exclusion. Its SHA-256 is `3cbc3291ced3c37becd71a31a770640aaf35997d6180bfa43d410f30cee7ea8f`. Cargo resolved **from S1's committed lock** with `cargo +1.92.0 metadata --manifest-path rust/Cargo.toml --format-version 1 --offline`, rather than copying the isolated lock. The old S1 lock SHA-256 was `3f67c9b9fce94f72426b1aa362c51bb8573aa9a1d6b8c5ee4b52aaa1198dbba6`; the integrated lock SHA-256 is `060d0d28780c289c8abe1964866c5e0b8f600cca54a539c47753165012736400`. The lock grew from 102 to 106 packages: only local `review-episode` and `review-episode-store`, `signal-hook 0.3.18`, and `signal-hook-registry 1.4.8` were added. No old package/version/checksum was removed or changed. Among the 102 old package entries, only `work-engine-compiler-cli` gained its declared `rustix`, `signal-hook`, and `tempfile` dependencies; the other 101 package entries remained byte-equivalent in parsed TOML. S1 `lifecycle-core` and `lifecycle-store` dependency entries remained exactly the same. Joined `cargo metadata` reports Apache-2.0/MIT for `signal-hook` (no declared minimum Rust version) and MIT OR Apache-2.0 for `signal-hook-registry` (minimum Rust 1.26); Rust 1.92 built both. Private `cargo-audit v0.22.2` returned exit 0 on the joined lock: 106 dependencies, zero vulnerabilities, no warnings, RustSec database 1294 advisories last updated `2026-10-07T16:40:27+02:00`.

## Preservation evidence

Before the first main-checkout write, `/tmp/work-engine-c2-r2-main-preservation.json` (SHA-256 `1cfbaffca510eee9331eca291189e8da31b4e2945453e54655c2fa939e7d4e48`) recorded HEAD, all 148 dirty/staged/untracked status paths, all 19 committed S1 paths, 166 protected S1/unrelated working-file byte hashes and modes (or symlink targets), the exact 64 overlay paths, and a complete index-entry hash. The index SHA-256 was `9ad185d092777671052048af877b7986c8a840073cf373f4d43924ce32b912dd` before and after overlay. The 166 protected file identities were checked unchanged after copy and joined tests; all committed S1 paths except the intentionally reconciled lock were preserved. Pre-existing UI and other staged entries remain staged exactly as found. The main `skills/slice-builder/SKILL.md` was never edited by this integration.

## Joined gates and baseline qualification

The following integration gates ran from main with `CARGO_TARGET_DIR=/tmp/work-engine-c2-r2-main-integration-target`, with `--locked` for Cargo tests/builds. They used the merged 106-package lock and committed S1 source plus exact reviewed C2/R2 overlay.

| Gate | Result |
| --- | --- |
| `cargo +1.92.0 test --manifest-path rust/Cargo.toml -p lifecycle-core -p lifecycle-store -p work-engine-compiler -p work-engine-compiler-cli -p review-episode-core -p review-episode-store -p review-episode --locked` | Pass, 113 focused tests including two R2 compile-fail doctests; actual S1/C2/R2 process and SQLite fixtures. |
| `cargo +1.92.0 test --manifest-path rust/Cargo.toml -p review-episode --features test-faults --locked` | Pass, 12 default command tests, six fault/crash tests, two doctests. |
| `cargo +1.92.0 fmt --manifest-path rust/Cargo.toml -p lifecycle-core -p lifecycle-store -p work-engine-compiler -p work-engine-compiler-cli -p review-episode-core -p review-episode-store -p review-episode -- --check` | Pass. |
| `cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p lifecycle-core -p lifecycle-store -p work-engine-compiler -p work-engine-compiler-cli -p review-episode-core -p review-episode-store -p review-episode --all-targets --all-features --locked -- -D warnings` | Pass. |
| `cargo +1.92.0 build --manifest-path rust/Cargo.toml -p work-engine-compiler-cli -p review-episode --release --locked` | Pass; main-target compiler binary SHA-256 `564cedce124bc07be70a684a14731fb1302a8a806509d750e5e43a0123c377dd`; R2 binary `c3659927086d41cebc623a64c85f86c75e1e21eb3ffab27125debc67eb871cab`. |
| `WORK_ENGINE_C2_TEST_BINARY=/tmp/work-engine-c2-r2-main-integration-target/release/work-engine-compiler node --test app-server/tests/rust-compiler-adapter.test.mjs` | Pass, C2 fixture-bound 10/10 from main. |

The four-file Node gate from the **dirty main checkout** yielded 29/40 (log `/tmp/work-engine-c2-r2-main-node.tap`, SHA-256 `30175ecd6c1f678f16778c7fdc40aa42446c9cce49f3f3d9ca8c23b1d207da39`). Its 11 failing legacy/manifest test names are **exactly the same** as the 11 failures when the existing three legacy test files run on disposable `e7c1d88` with only main's dirty builder source bytes copied in: 19/30 baseline (log `/tmp/work-engine-c2-r2-baseline-node.tap`, SHA-256 `2cf0a8d21e0335a24bee27f5f0381d73288fb0f3d220507df024795f8e5daa22`). Each failure is `legacy source digest mismatch` or a downstream assertion intercepted by that mismatch. Main's dirty `skills/slice-builder/SKILL.md` hashes `a26420e183547b73162b87030976617ad23c6f3f044f22388e64f049fff1c28d`; committed `e7c1d88` and the pinned C0 success oracle hash `ba46408c74cfd396abd68e430c5ff9230835a985333a595292ad1e7620e3da2e`. C2 did not refresh this source-owned closure.

A second disposable detached checkout, `/tmp/work-engine-c2-r2-clean-joined`, contains precisely `e7c1d88` plus the same 64 source files and the exact integrated root manifest/lock; its C2/R2 aggregates and root hashes match main. Its ignored `node_modules` symlink points to main's already installed dependencies, with no Node manifest or lock edit. A release compiler built from that checkout in `/tmp/work-engine-c2-r2-clean-joined-target` hashes `55c9a8afedb559a54ced88f4cade1840f37471fd4fd8d3fb1116751e2819eece`; the exact four-file Node command with `WORK_ENGINE_C2_TEST_BINARY` set to that binary **passed 40/40**. This clean joined gate qualifies the reviewed C2/R2 code with S1's committed inputs. The dirty-main 11-failure baseline remains an explicit, separate source-owner limitation, not an integration regression or a reason to rewrite unrelated work.

The source inventory below excludes `rust/Cargo.toml`, `rust/Cargo.lock`, and all result documentation. No further source changes are expected before parent final gates.

### C2 source inventory (46)

- `app-server/scripts/compile-skill.mjs`
- `app-server/src/runtime-manifest.mjs`
- `app-server/src/rust-compiler-adapter.mjs`
- `app-server/src/skill-compiler.mjs`
- `app-server/tests/fixtures/compiler-c2/legacy/agent-environment-graph-adapter.mjs`
- `app-server/tests/fixtures/compiler-c2/legacy/skill-compiler.mjs`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/agent-instruction-review/interface.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/agent-instruction-review/structure.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/claim-evidence/interface.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/claim-evidence/structure.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/repo-search/interface.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/repo-search/structure.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/slice-builder/interface.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/slice-builder/structure.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/slice-supervisor/interface.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/migrations/skills/slice-supervisor/structure.yaml`
- `app-server/tests/fixtures/compiler-c2/root/app-server/runtime-manifest.yaml`
- `app-server/tests/fixtures/compiler-c2/root/docs/agent-environment-views/slice-builder.yaml`
- `app-server/tests/fixtures/compiler-c2/root/docs/agent-environment-views/slice-supervisor.yaml`
- `app-server/tests/fixtures/compiler-c2/root/docs/agent-environments.yaml`
- `app-server/tests/fixtures/compiler-c2/root/docs/workflow-invariants.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/agent-environment-graph/scripts/agent_environment_graph.py`
- `app-server/tests/fixtures/compiler-c2/root/skills/agent-instruction-review/SKILL.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/agent-instruction-review/agents/openai.yaml`
- `app-server/tests/fixtures/compiler-c2/root/skills/agent-instruction-review/references/finding-contract.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/claim-evidence/SKILL.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/claim-evidence/agents/openai.yaml`
- `app-server/tests/fixtures/compiler-c2/root/skills/claim-evidence/references/claim-evidence-contract.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/repo-search/SKILL.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/repo-search/agents/openai.yaml`
- `app-server/tests/fixtures/compiler-c2/root/skills/repo-search/references/backend-capabilities.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/slice-builder/SKILL.md`
- `app-server/tests/fixtures/compiler-c2/root/skills/slice-supervisor/SKILL.md`
- `app-server/tests/rust-compiler-adapter.test.mjs`
- `rust/crates/work-engine-compiler-cli/Cargo.toml`
- `rust/crates/work-engine-compiler-cli/src/aeg.rs`
- `rust/crates/work-engine-compiler-cli/src/main.rs`
- `rust/crates/work-engine-compiler-cli/src/protocol.rs`
- `rust/crates/work-engine-compiler-cli/src/source_snapshot.rs`
- `rust/crates/work-engine-compiler-cli/tests/aeg_process.rs`
- `rust/crates/work-engine-compiler-cli/tests/verified_skill.rs`
- `rust/crates/work-engine-compiler/src/error.rs`
- `rust/crates/work-engine-compiler/src/lib.rs`
- `rust/crates/work-engine-compiler/src/skill.rs`
- `rust/crates/work-engine-compiler/src/verification.rs`
- `rust/crates/work-engine-compiler/tests/verified_skill_v1.rs`

### R2 source inventory (18)

- `rust/crates/review-episode-core/src/command.rs`
- `rust/crates/review-episode-core/tests/transitions.rs`
- `rust/crates/review-episode-store/Cargo.toml`
- `rust/crates/review-episode-store/src/import.rs`
- `rust/crates/review-episode-store/src/integrity.rs`
- `rust/crates/review-episode-store/src/lib.rs`
- `rust/crates/review-episode-store/src/sqlite.rs`
- `rust/crates/review-episode-store/tests/cas.rs`
- `rust/crates/review-episode-store/tests/compatibility.rs`
- `rust/crates/review-episode-store/tests/recovery.rs`
- `rust/crates/review-episode/Cargo.toml`
- `rust/crates/review-episode/src/admission.rs`
- `rust/crates/review-episode/src/lib.rs`
- `rust/crates/review-episode/src/main.rs`
- `rust/crates/review-episode/src/protocol.rs`
- `rust/crates/review-episode/tests/command.rs`
- `rust/crates/review-episode/tests/crash.rs`
- `rust/crates/review-episode/tests/support/mod.rs`
