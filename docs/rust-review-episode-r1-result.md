# Review-episode R1 pure core result

Status: R1 pure core implemented and package-qualified on 2026-10-07. S0
published its foundation handoff at revision `84be6695`; the later serialized
workspace integration registered `review-episode-core` as a library member.
This result qualifies the pure core and its historical fixtures, not a store,
CLI, trusted host bridge, claims join, campaign transition or cutover. Parent
integration still owns the whole-workspace gate.

The crate is bounded to
[`rust/crates/review-episode-core`](../rust/crates/review-episode-core), with
no I/O or dependency on lifecycle, SQLite or the shared JCS codec. The
domain-local `review-episode-js-json-v1` codec stores JSON keys and string
values as UTF-16 code units, preserves unpaired surrogates, sorts keys by those
units, emits JavaScript escapes and number formatting through `ryu-js`, and
hashes exact canonical bytes without a final newline. Its parser requires
four ASCII hexadecimal digits after `\u` and limits nesting to 256 levels;
deeper historical rows require refusal/quarantine with source bytes retained,
not a normalized rewrite.

Typed identity, authority, writer, status, phase and command values feed
v1/v2 state validation, implementation-review v1 result validation and the
pure reducer. Authority and state keep raw and validated fields private and
coherent; the API exposes read-only accessors. Error kinds distinguish
`ResultContract`, `Authority`, `RevisionConflict`, `InvalidCommand` and
`StateIntegrity`. A later trusted host must still supply and validate real
authority, and a store must enforce durable CAS.

Frozen R0 observations remain separate from the selected target changes.
Current-writer replay is idempotent; a replaced writer cannot replay to read
current state. Evidence succession retains unresolved questions in
remediation. Old admissions remain in history, but a later v2 result without
new admissions is `evidence_unestablished`; this status does not prove claim
establishment or applicability. `evidenceAdmissions: null` has the same
admission-list effect as omission under JavaScript `?? []`, while their raw
command bytes produce distinct transition digests. Finding attribution remains
immutable while permitted status and remediation evidence evolve.

The three Rust test files consume frozen R0 codec, v1/v2 state and command
fixtures and the existing implementation-review result fixtures. They assert
historical canonical bytes/revisions, invalid syntax and numeric boundaries,
state integrity refusal, all reducer actions, writer replacement, retirement,
replay/CAS fences, typed errors, unresolved questions, stale/null admissions,
finding lineage and 24 generated sequences with varied action/phase paths.
Their observed result is 12 passing integration tests: codec 3, contract 2,
transitions 7; unit and doc tests have zero cases. This is executable core
evidence, not production host or cross-store evidence.

All commands below ran from the repository root using toolchain 1.92.0 and
`rust/Cargo.toml`; each exited 0 after the final source/test changes:

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml -p review-episode-core --locked
cargo +1.92.0 fmt --manifest-path rust/Cargo.toml -p review-episode-core -- --check
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml -p review-episode-core --all-targets --locked -- -D warnings
cargo +1.92.0 check --manifest-path rust/Cargo.toml -p review-episode-core --release --locked
cargo +1.92.0 build --manifest-path rust/Cargo.toml -p review-episode-core --release --locked
```

Exact SHA-256 input inventory at this qualification:

| File | SHA-256 |
| --- | --- |
| `rust/FOUNDATION_GATE.md` | `c00ab7e47f02e5389dbd4504e5970f29e1f2c07baf0d258ded589c891b48ef65` |
| `rust/Cargo.toml` | `6c593393b67b61814f3a47b152fc20191dc974b1200559752e20f2719cf43365` |
| `rust/Cargo.lock` | `15241cbdfb8bd9150e4705f5adfcfd9884e86d79305220198ef59698faa8a877` |
| `rust/crates/review-episode-core/Cargo.toml` | `70f570686f9327be18d42911c15fcf5e3ef4a289343ab2ca7d50a3e62c9b034f` |
| `src/codec.rs` | `ae2f70cd8ab4ae10db609f47f384734baecab62312c335dfb049b762d2571407` |
| `src/command.rs` | `704fd1eaedc83a527f1189a1877cbcd66cbf49f4e977d61cd5237073ce1c323e` |
| `src/identity.rs` | `5426f5721cecea2816d25aa81f818a2dd5022e7842af8f69da42c2d76c3601df` |
| `src/implementation_review_v1.rs` | `ea08c9a554ca2ab29a9073b17f958ab0e756d5ffe56e3b4a6d0d795c6643bdbe` |
| `src/lib.rs` | `e380dd1f80a7789b5229bb641c2c6347c687e3a0cc7c42722ca4d34adc3b693f` |
| `src/reducer.rs` | `ad07b24fb6296353695995d027d55143605695d380c687aed1e07ca882096d7a` |
| `src/state.rs` | `d0ae41c6ce721e3b6311822adbffa62490e1d31a1a5909a266a7e592dbdb57d3` |
| `tests/codec.rs` | `e8cdd62e765fd14ea2d899bf662a8351fdec4d90f95f3665cb3a46a9ac046587` |
| `tests/contract.rs` | `9e64875f67b2d903351ca497bd8974b048b6f5b93f7cb6d7769cd3545094956b` |
| `tests/transitions.rs` | `6b9c46fae0692e741f3db9632906b13729830634ed30e856217aeee82d1a79d3` |

The member manifest uses workspace `sha2 = 0.10.9` and `ryu-js = 1.0.3`.
The root manifest and lock hashes above reflect the later ten-member
integration, not the seven-member S0 baseline recorded in
`FOUNDATION_GATE.md`; this lane did not edit either shared file.

R1 does not establish trusted host admission, reviewer independence, claim
establishment, SQLite atomicity/recovery, the interrupted three-store joins,
process bridge behavior or cutover. R2 must qualify durable store/CAS and
history; R3 must qualify the admitted synchronous application/CLI and host
bridge. The R0 selected corrections require their owning RC acceptance and
integration gates before the affected operation can pass full cutover.

The [reviewed plan](rust-review-episode-implementation-plan.md) remains at
SHA-256 `dd17594b61ca7cc70f853b6c92024ffe1472006e4c8ac2530b63df513297515d`;
the frozen [R0 contract](rust-review-episode-r0-contract.md) remains at
SHA-256 `a6a8fe6cc6f714b9b2aaaa6db50d13888cac0cc8088e882f169e50a484d09862`.
