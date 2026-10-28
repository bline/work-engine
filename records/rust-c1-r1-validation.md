# C1/R1 combined validation

Recorded 2026-10-07. **Bounded C1 and R1 are complete and accepted by the
coordinating session.** The user authorized these slices after C0/R0 and
selected `gpt-6-sol` for implementation and remediation. Changes remain
uncommitted. This record does not authorize C2/R2, production adapters,
state migration, provider execution, deployment or cutover.

## Implemented boundary

- [C1](rust-compiler-c1-result.md): pure unverified skill compilation and a
  bounded one-request executable. Five pinned skills match legacy Markdown
  bytes and producer-neutral IR. The accepted producer is
  `work-engine.skill-compiler.rust-v1`; the user accepted rejection of YAML
  collection mapping keys while preserving scalar keys.
- [R1](rust-review-episode-r1-result.md): pure episode values, historical
  JavaScript-compatible codec, state/result validation and reducer. It retains
  the selected R0 replay, unresolved-question and admission corrections.
  Trusted host admission, durable CAS, claims joins and the owning RC gates
  remain later integration work.

S0 was published as `84be6695d716f026460fbf11aa703e73631e514e` on
`rescue-detached-20260911`. Coordination-board message 1191 transferred bounded
registration of `work-engine-compiler`, `work-engine-compiler-cli` and
`review-episode-core`. The workspace now has ten explicit private members and
continues to exclude the standalone terminal UI. The seven S0 member sources,
toolchain and historical `FOUNDATION_GATE.md` were unchanged at this gate.
Lifecycle S1 deferred shared writes until this session's final handoff.

## Executed final checks

The coordinating parent ran these commands after the final BOM correction;
all exited zero on Rust 1.92.0:

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml --workspace --locked
cargo +1.92.0 fmt --manifest-path rust/Cargo.toml --all -- --check
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo +1.92.0 build --manifest-path rust/Cargo.toml --workspace --release --locked
```

The suite passed **42 tests: S0 14, C1 16, R1 12**. This includes the S0
negative compilation cases and three tests that execute the compiler CLI.
Unit/doc targets with zero cases are not added to that count. The unchanged
legacy R0 characterization directory previously passed all 14 JavaScript tests.

The compiler worker separately ran cargo-audit 0.22.2 against this exact
56-package lockfile and 1,294 advisories: zero vulnerabilities and warnings.
It checked the excluded terminal UI with its own locked manifest before the
parser substitution; the root parser dependency does not enter that independent
lockfile. These two checks are attributed to the worker, not a fresh parent
audit or UI qualification. Existing dependency resolutions were retained;
the maintained YAML parser and the explicit Zlib license-inventory addition
are documented in the C1 record. No legal audit is claimed.

## Review and exact subjects

Retained read-only Astra review reported no remaining material findings for
C1/R1 registration and core implementation. Its final C1 coverage correction
required executable BOM and merge-key vectors. The BOM test exposed a parser
bug: the fix strips only the leading BOM for parsing while hashing the original
bytes. Literal `<<` scalar keys remain supported; alias and mapping-value
cases have distinct rejection assertions. The retained reviewer confirmed
closure against the final subjects below. This is same-provider review;
no cross-provider independence is claimed. The reviewer ran no tests/builds.

| Subject | SHA-256 |
| --- | --- |
| `rust/Cargo.toml` | `6c593393b67b61814f3a47b152fc20191dc974b1200559752e20f2719cf43365` |
| `rust/Cargo.lock` | `15241cbdfb8bd9150e4705f5adfcfd9884e86d79305220198ef59698faa8a877` |
| C1 result record | `0630aa67fba1ffc1f90386ef6e2a36175a418be34faa1a3ae8b4675347f73b8d` |
| R1 result record | `a405df31a83ac7d8b9d6fbff39a1ae127290c4dbc4350d4002797889b08c76dc` |
| Compiler `src/skill.rs` | `df9a3e6f8f5d636b0117e57325462f72d323a692e5230505b9a8d1fd907d7002` |
| Compiler `tests/skill_v1.rs` | `21d4c19eada68293b72491bf71469404c4b0e9ca0cb3f91c19cd66c5aef5ba37` |
| Final workspace-build `rust/target/release/work-engine-compiler` | `a22f9214e03e72ce564d73ee3fdd4154bb094e4bbe915715c235224189a51ec9` |

The binary hash identifies the artifact produced by the full-workspace release
command above. A later Cargo build can overwrite that path; focused package
builds recorded in C1 produced different artifacts. This is not a staged or
activated executable identity.

The approved compiler/episode plans and frozen C0/R0 contracts retain their
original hashes, recorded in the linked result records and migration plan.
Root identities above bind this completed gate before later S1 dependency
changes; they are historical qualifications, not a claim about future HEAD.

## Workflow observations

This resumed integration used two implementation workers requested as
`gpt-6-sol`/high (replacement C1 worker and retained R1 worker), followed by
one retained Astra reviewer. The runtime did not expose an independently
attestable effective model identity. Claude calls: zero in this resumed
integration. Token/cache use and complete elapsed/retrieval totals were not
exposed and are not estimated.

One final integration review/correction cycle added the missing BOM/merge
execution evidence and fixed the resulting BOM test failure. Final parent
tests had no failures. Earlier preparation and remediation are recorded in
the individual lane results; this count does not describe their entire history.
Graph-guided source review used targeted coverage checks with direct reads for
untracked lockfile metadata. Parent reads were limited to coordination,
qualification and documentation rather than repeating worker reconnaissance.

The S0 handoff resolved the remaining build dependency without a second
workspace. Future slices should retain executable parser-edge fixtures and
bind artifact hashes to the exact Cargo build selector. Serialized source and
manifest handoffs also kept the combined gate stable while S1 prepared work.
