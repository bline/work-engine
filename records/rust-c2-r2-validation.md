# C2/R2 joint validation

Status on 2026-10-07: implementation, bounded source review, and isolated combined
validation complete; shared-workspace integration pending. Work is preserved in
`/home/bline/code/work-engine-c2-r2`, detached at plan commit
`34e5723c98b454bde80448452f35945203b1cdf9`. This record is owned by the coordinating
session. Neither implementation is accepted, integrated into the shared branch,
or activated by this record. The shared S1 workspace remains separately owned.

The user authorized the two plans and parallel implementation, with
`gpt-6-sol` for implementation. Both retained builders resumed on that model
after a temporary usage-limit failure. Review uses a separate
`gpt-6-astra` reviewer; this is not a provider-independence claim.

## Initial combined gate

All commands ran from the isolated checkout using Rust 1.92.0 and
`CARGO_TARGET_DIR=/tmp/work-engine-c2-r2-final-target`. Cargo test, Clippy,
and release commands used `--manifest-path rust/Cargo.toml --locked`.

| Gate | Observed result |
| --- | --- |
| `cargo +1.92.0 test --workspace` | 79 passed, including the compile-fail documentation test. |
| `cargo +1.92.0 test -p review-episode-store -p review-episode --features test-faults` | 27 passed; one intentionally ignored subprocess helper. |
| `cargo +1.92.0 fmt --manifest-path rust/Cargo.toml --all -- --check` | Passed. |
| `cargo +1.92.0 clippy --workspace --all-targets --all-features -- -D warnings` | Passed. |
| `cargo +1.92.0 build --workspace --release` | Passed. |
| Node compatibility suite below | 37 passed, no skips or failures. |
| `git diff --check` | Passed on tracked changes. |

The Node command was:

```sh
WORK_ENGINE_C2_TEST_BINARY=/tmp/work-engine-c2-r2-final-target/release/work-engine-compiler \
node --test app-server/tests/rust-compiler-adapter.test.mjs \
  app-server/tests/skill-compiler.test.mjs \
  app-server/tests/agent-environment-graph-adapter.test.mjs \
  app-server/tests/runtime-manifest.test.mjs
```

That compiler binary's SHA-256 was
`13489cce7e98bff78ee7fa98b5632fc453d9cf6eb68bc0a79d21085f6e7c473d`.
The complete-hydration diagnostic was 924 ms; this is an observation, not a
performance guarantee. Joined lifecycle-host responsiveness remains later
integration work.

The initial R2 review subject was
`fef9313893b07847a63f7dad3780572fbb75b32570587fee4dd4c8115239763f`,
over 18 source/test/manifest files: sorted relative paths encoded as four-byte
big-endian path length, path bytes, eight-byte big-endian content length, and
content bytes. Root files and result receipts are excluded. The subject includes
the final concurrent-output-drain test-helper correction. Subsequent remediation
requires a new subject and affected gates; the results above do not qualify
later edits automatically.

Root manifest SHA-256:
`3cbc3291ced3c37becd71a31a770640aaf35997d6180bfa43d410f30cee7ea8f`.
Root lock SHA-256:
`15eaab1e8f96c8a59627e91a28d2c2b28547517140598d7ac1cad75cd3f7a2d3`.
The [C2 builder receipt](rust-compiler-c2-result.md) separately records its
dependency, source, fixture, and focused-gate evidence.

## Initial review and remediation scope

The initial R2 source review found three P2 defects, sent to the retained Sol
builder for remediation:

1. Public request fields allow admitted envelope data and executed arguments to
   diverge in direct library use. Require a coherent request and regression.
2. Import must preserve `sqlite_sequence` presence and value for empty history,
   including a retained zero or nonzero counter.
3. The first rejected import diagnostic must include row/table/identity/sequence
   context where available, rather than only the underlying validation reason.

The existing reverse Node check reopens JS-origin imported rows and checks a
count. It does not establish legacy Node parity for newly Rust-produced rows;
the builder is adding that qualification.

At the initial gate, C2 source review, R2 remediation, retained re-review, and new
affected gates remained required. The later sections record their completion.
Coordination with S1 before shared-root integration remains required.
No C3/R3 work, live database import, provider execution, backend-default change,
deployment, push, or activation is authorized by this validation record.

## Remediation gate

The R2 builder addressed all three P2 findings and the reverse-parity evidence
gap. The coordinator independently matched the new 18-file subject
`bf09ea4c279963f4a2f417f2bcfc5d5908efb160452f185a3c35e62c45d6e089`
using the same encoding above. The new request compile-fail test, direct-library
admission test, empty-history sequence cases (absent/zero/nonzero), diagnostic
context case, and Rust-produced row read through legacy Node all passed.

Repeating the same combined Cargo commands after remediation produced **84
workspace tests passed**, **32 fault-feature tests passed** (one deliberately
ignored subprocess helper), and clean formatting, all-target/all-feature Clippy,
and locked release build. The resulting review executable at
`/tmp/work-engine-c2-r2-final-target/release/work-engine-review-episode` has
SHA-256 `c3659927086d41cebc623a64c85f86c75e1e21eb3ffab27125debc67eb871cab`.
Root hashes and the compiler executable hash above remain unchanged.

C2 review found invalid UTF-8 replacement in the CLI and rejection of valid
BOM-bearing Buffers in the adapter. Both were fixed, and the parent Node suite
passed 39 tests, including their regressions. Review then identified the same
replacement-decoding gap in manifest hydration. The C2 builder's ownership was
expanded to `app-server/src/runtime-manifest.mjs` solely for the input-reading
boundary required by the accepted C2 hydration path; manifest algorithms remain
outside this change. That fix is complete. The final parent Node gate passed
**40 tests**, including invalid bytes in all four role/secondary YAML reads
through both hydration and manifest loading. The final hydration diagnostic was
788 ms. Rust source did not change for these C2 encoding fixes.

## Final source binding

For unambiguous reproduction, final aggregates sort complete relative POSIX path
strings by their UTF-8 bytes, then use the length/content encoding above. Earlier
`fef931...` and `bf09ea...` aggregates used Python `Path` component ordering. The
reviewer and builder reconciled this methodological difference; no R2 file drift
occurred between the passing gates and final review.

| Final subject | Files | SHA-256 |
| --- | --- | --- |
| C2 owned implementation/test/fixture files, including the hydration input boundary | 46 | `0cd0ee200d1be0ec3105d4c09fbe49d02ea62583bee1d55fb65150a64805bfa7` |
| R2 owned files, same bytes as the passing remediation gate | 18 | `653baf04361871056bd2d28242f8854f92a2c2267a8fc98652e102e1b1da963c` |

C2 includes the explicit implementation/test paths in its builder receipt and
all 29 files below `app-server/tests/fixtures/compiler-c2/` (27 source fixture
files and two legacy oracle modules). R2 includes every file in its two new
crate directories and the two core paths in its receipt. Root manifests/lock
and documentation are excluded from both aggregates; their evidence is bound
separately. Final tracked whitespace validation also passed.

Two Sol builders executed parallel implementation and remediation. One separate
Astra reviewer covered both bounded domains and retained the findings through
fixes: three P2 findings in each domain, plus the R2 reverse-parity qualification
gap. The retained reviewer closed all six findings and the parity qualification,
with no remaining material finding in the bounded C2/R2 scope. No Claude
reconnaissance or provider call was made in this continuation. Exact aggregate
token and cost metrics are unavailable; they are not estimated here.

## Integration handoff

The reviewed plans are committed on the shared branch as `34e5723c98b454bde80448452f35945203b1cdf9`.
Implementation files remain uncommitted in the isolated checkout. No shared
branch, index, or S1 implementation was changed by this continuation. Chatboard
message 1229 reports S1 Candidate 2 awaiting retained review, so shared workspace
ownership has not been handed back.

After S1 releases the shared workspace, integrate the bounded C2/R2 files while
preserving S1 and unrelated user edits; reconcile the two member additions and
dependency lock with S1's final root, then run the joined workspace gates. That
future gate must bind the integrated root; this isolated result does not stand
in for it. Production host joining, combined lifecycle/review responsiveness,
generation staging, and activation retain their later owners and scope.

## Subsequent main-checkout integration

After the S1 handoff and commit `e7c1d88ec0df20ea6213239b6dc2b36abd9f1872`,
the reviewed C2/R2 files were overlaid on the main checkout and their root
dependencies were resolved from S1's committed lock. The exact overlay,
preservation audit, baseline-qualified Node results, and joined gates are in
[the integration receipt](rust-c2-r2-integration.md). This postscript records a
later event; the isolated validation and handoff above remain historical.
