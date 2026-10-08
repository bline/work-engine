# HP1 direct episode library validation

Completed 2026-10-07 under the user's authorization to proceed alongside lifecycle
S2/S3. HP1 establishes direct-library readiness and successful full-history
handling. It does not establish host responsiveness, retire a Node consumer,
select a default backend or authorize operational cutover.

## Change and review

The episode application now accepts typed operations and returns typed results
for all seven existing operations. Rust consumers do not need a JSON
serialize/parse round trip. The application encapsulates its store/admission
port, and writes preserve exact encoded-result capacity checks before commit,
historical identities, replay, writer fencing and possible-commit reconciliation.
The compiler already exposes the needed pure APIs; HP1 changes no compiler code.

A fresh read-only Astra review found two medium issues. The retained Sol builder
reproduced and corrected both, and the same reviewer verified closure:

- Direct construction and wire parsing now share domain-field validation;
  malformed identity, revision and recovery fields refuse consistently.
- Store opening anchors the checked root before marker/database access, so
  successful and uncertain-write recovery contexts retain the actual absolute
  root across a working-directory change.

This is separate-context, same-provider review; no provider-independence claim
is made. No material findings remain within the reviewed HP1 delta.

## Exact joined subject

The qualification checkout is `/home/bline/code/work-engine-hp1-joined`, based
on published S2 commit `b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2`, which includes
S3 `634161c2f54dc4fcf9dcb53936951ecefe457235`. It combines that baseline with the
frozen C3/R3 candidate and HP1 candidate 2. All 38 published S2/S3 paths, including
the root manifest and lockfile, remain byte-for-byte and executable-bit identical.
The original C3/R3 source bindings also remain unchanged.

| Subject | Binding |
| --- | --- |
| HP1 candidate 2, 16 paths | `5fdd14d5d6d7aa8cfbf6c8977ae2f7a51ec8c35ba0a006471a624b8f450aee76` |
| Joined candidate 2, 45 product paths | `7b1b684d172528b55b57f6e9b739f9945e5bd223cd3459be3082294a3efed27f` |
| Joined manifest file | `cc6b7e8eadbf60dc40f0ae7e55eaedbcbb2a818d7dc2abe10787312171ca8cce` |

The aggregate algorithm sorts relative paths and hashes each UTF-8 path with its
four-byte big-endian length, followed by content with its eight-byte big-endian
length. Separate manifest entries bind executable bits. Three planning support
documents and the declared `node_modules` symlink are excluded from product
source; the symlink points to existing dependencies in the main checkout.

## Final coordinator gates

| Gate | Result |
| --- | --- |
| Locked joined Rust workspace tests, including doctests | 178 passed, 0 failed, 3 diagnostic tests ignored |
| Episode/store fault-feature tests | 43 passed, 0 failed, 4 ignored: three diagnostics and one existing helper |
| Five affected Node compatibility files, serial file execution | 41 passed, 0 failed, 0 skipped |
| Whole-workspace formatting | Passed |
| Whole-workspace Clippy, all targets/features, warnings denied | Passed |
| Locked default compiler/episode and fault-enabled episode release builds | Passed; executable hashes retained |
| Explicit full-history release probe | Passed under the declared 30-second diagnostic watchdog |
| Final source, lifecycle-path and working-tree scope audit | Passed; no unexpected paths or changed bound source |

The final gates ran after both review fixes. Candidate-1 results are retained
separately rather than added to these totals. Build and test temporary files use
`/home/bline/code/.work-engine-tmp/hp1-parent/`, with both `CARGO_TARGET_DIR` and
`TMPDIR` set there. Builds used Rust 1.92.0, one Cargo build job, incremental
compilation disabled and `RUSTFLAGS=-Cdebuginfo=0`.

## Large-history result and limits

The final candidate returned all four rows of the exact **31,207,216-byte**
history, SHA-256
`08be22b5348c34fdf0ba9ada1b8f9e013c56597efa7920cf786ff7d7f6ed8a66`.
The selected wire reply is 31,207,425 bytes. The direct application call measured
**362 ms**; a separate history encoding measured 84 ms, and the complete probe
process took about 0.916 seconds. Process high-water RSS after the direct call
was 220,920 KiB; that includes fixture preparation. Allocator peak was not
observed. This is one bounded diagnostic, not a latency distribution or service
completion guarantee.

Candidate-1 stage measurements identified integrity parsing/validation as the
dominant cost. They remain attributed to candidate 1; its earlier 322 ms direct
measurement is not a final-candidate result. HP1 does not remove validation,
truncate or paginate the result, substitute a cache, or add a separate daemon.

The final direct call exceeds the former 250 ms child deadline. The original
C3/R3 HOST failure remains failed evidence. The future Rust host must select an
explicit supported completion budget and qualify control responsiveness before
HP3 acceptance; moving the call into Rust alone does not settle that question.

## Evidence and next boundary

Durable evidence is under
`/home/bline/.local/state/work-engine/rust-hp1-20261007/`: the candidate-2 freeze
and source manifests, immutable archives, retained-review result,
`parent2-rust-results.json`, `parent2-post-results.json`, raw logs,
`parent2-test-totals.json` and `final-source-audit.json`. Source checks after
execution confirm the tested bytes remain frozen.

The implementation used one retained `gpt-6-sol`/high builder, one fresh
`gpt-6-astra`/xhigh source reviewer and one retained remediation cycle. Two medium
findings were resolved. There were no Claude calls. Token/cache usage is not
available. An initial `/tmp` linker quota failure was infrastructure-only;
final builds and probes used local disk. The first coordinator scope audit
omitted the already-declared dependency symlink from its allowlist; explicit
link/target verification corrected that bookkeeping error without source changes.

No main runtime source, shared Cargo root/lock or lifecycle files were edited.
No ordinary commit, push, live provider call, live-state migration or activation
was performed. The next planned work is the bounded HP2 claims/campaign contract
and implementation planning, with parallel ownership once their shared types
are agreed. HP1 is ready as that library handoff; it is not the completed Rust
review host or full Node retirement.
