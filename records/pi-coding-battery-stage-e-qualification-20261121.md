# Pi coding battery — Stage E offline qualification

Evidence through November 21, 2026 (UTC). This report summarizes retained
executions behind the [Stage E ledger](../planning/openrouter-lane-e1a-battery-20261120.md#status--ledger).
It identifies what ran and what a public checkout can verify. It is not a
publication of the private handoff, a replay bundle, or an independent rerun.

**Result:** the historical Completions loopback settled. On a later, exact
source revision, the synthetic Responses battery also settled, and a separate
failed-terminal control refused without retry. These tests used the real Rust
Host, Pi SDK, Docker, and lifecycle store against scripted local HTTP endpoints.
They did not use provider credentials or call an external inference service.
In both successful batteries, `workspace_acceptance` was `not_run`.

## Source and executable identities

| Subject | Git identity and build provenance | `coding-qualification` SHA-256 |
| --- | --- | --- |
| Historical Completions `loopback-3` | Checkout HEAD `d410c9e1108d18e262fe60fcbe7a79b458e842a0` at launch (local reflog and run time), **with uncommitted Stage E edits**. No immutable full build tree was captured at that launch. The later preservation commit `efc2664ee075143632a6ae7660fb2869cd23cf2c` is not a substitute for that provenance. The receiver bytes copied into the fixture do match that later commit. | `a4638aae459b02906c186562ec50d32821481eb3fa4fb4d0f25d63d10ca5624f` |
| Initial Responses probe | Exact Git source `efc2664ee075143632a6ae7660fb2869cd23cf2c`, tree `392fd33f2867153a5c56decc44ea53615960adcb`, with the private lock reconciliation described below. Its positive probe did **not** settle. | `6a31f7ecf88241ec842a3c9f538c4c0131a94fa9b313f9c8c25f6373b313db8f` |
| Paired Responses follow-up | Exact Git source `acfd467f192f935e59d12985ad570f6bd4f966c2`, tree `f2ad8699f9a1139df48ad03becb2b1128ea43f20`, with the same private lock reconciliation. Stage E range starts at `2c3f00cdc97ebca652815a160826a908b60b2fef`. | `0f7c8f2eb453283abb23d475ed6bb6d1071ae972575e002656adbe3d17259053` |

The paired follow-up is the second parent of integration commit
`c183115a15135b3c7acc5f635bd5a5cb870d1c69` (composed tree
`1dbf31574cacf670a9eb2cf1db8b42030852aa33`). The tests ran on the Stage E
source input above, **not** the composed tree. No full-suite result is claimed.

Common loopback job pins, read from the retained job inputs:

| Input | Recorded pin |
| --- | --- |
| Docker client executable (SHA-256) | `a429e235ef670ea83357a5c8c7451f0a69d485a6fee49f9032fd938a0ab4969d` |
| Container image | `node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da` |
| Pi SDK runtime inventory (manifest SHA-256) | `cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985` |
| Mounted offline toolchain inventory (manifest SHA-256) | `ed85380d9328cbe52df383a135a019f5d095c0c85f43cca93a88c85e392a27ae` |
| Receiver shim, Completions (SHA-256) | `150165cc15e1310ed202ed21c276e64a09852c0a69bee3982f9319fa6a62877a` |
| Receiver shim, paired Responses (SHA-256) | `d262af9882f08e1ae3369750e1f0fa1e29cf6f9831960f03849617bceb12b33a` |

These are job pins, not a claim about the Docker daemon binary or the compiler
binary that built the Host. The mounted toolchain is a fixture input, not a
producer-build provenance record.

## Commands and observed outcomes

Commands below are transcribed from retained launch records. Private paths are
replaced with placeholders and supervision wrappers are omitted. The fixture
scripts and their inputs are not part of the public checkout.

### Historical Completions — November 21, 05:56–05:58 UTC

From the dirty repository root, the release build and loopback ran in sequence;
the targeted coding regression was a separate command. These Cargo commands did
not use `--locked`.

```sh
cargo build --release --manifest-path rust/crates/pi-pressure-context-host/Cargo.toml --offline --bin coding-qualification &&
  python3 -I "$FIXTURE/run_loopback.py" "$FIXTURE/loopback-3"
cargo test --release --manifest-path rust/crates/pi-pressure-context-host/Cargo.toml --offline --test coding
```

The fixture copied the producer and invoked it as `coding-qualification
--execute job.json` (absolute paths in the original) under a credential-free
environment. Loopback and tests exited 0; **18 coding tests passed**. The
battery completed five requests, carried a 72,955-byte second request and a
149,966-byte history observation, executed three shell tools across one real
reset, and reached `CompletedSettled` in `context:new1`. Host reported
`component_passed`, no failures, and removal of both container classes.

A separate offline `cargo check -j1 -p work-engine-context-status --offline`
passed in a network-disabled container on the D1 baseline
`1e21e71c5dea67d143ebd7696d7884789018ee69`. That smoke check did not build or test
a battery-produced candidate and was not workspace acceptance. Live D1 did not
execute or produce a score.

### Responses precursor — same wire, incomplete positive

At `efc2664e`, the final producer build used the full archived source and
`cargo build --manifest-path crates/pi-pressure-context-host/Cargo.toml --locked --offline --bin coding-qualification`
from its `rust/` directory after reconciling the nested lock. The corrected
synthetic fixture (SHA-256
`f388b6c2f5eb987e47e5a515b27ff0a7688a9897ca4608b9eb52cc85034ee560`)
ran as `/usr/bin/python3 -I run_responses.py responses-positive-2 positive`
and `/usr/bin/python3 -I run_responses.py responses-failed-terminal-2 terminal-failure`.
The positive exited 1: request 1 and two tools completed, but the receiver's
legacy 45,000-byte zstd decode bound stopped request 2 before it reached the
local endpoint. The negative exited 0 at the fixture level after observing
expected producer refusal, one uncertain reservation, no tools and no retry.
Neither run was a completed Responses battery.

### Exact follow-up — November 21, 23:09–23:19 UTC

The build checked a clean `acfd467f` and exported it with `git archive`. In the
archive only, it replaced `rust/crates/pi-pressure-context-host/Cargo.lock`
(tracked SHA-256 `c71e0cde32fbcc394ccaabde8b7e764b7639c27c16ac731d4fff29b8d9cd1111`)
with the previously reconciled lock (SHA-256
`30aa6f7315b54037dd1d797b93dc683ca7c14990e0c3c2367025ef1cab0a2cc5`). This
added `slice-campaign-identity` to `slice-campaign`'s dependencies and one
local package entry (`0.1.0`, depending on `review-episode-core`, `serde`,
`serde_json`, `thiserror`). No registry resolution changed; the tracked lock
was not edited. The test job checked that the private lock stayed unchanged.

From the archive's `rust/` directory, with `env -i HOME="$HOME"
PATH="$HOME/.cargo/bin:/usr/bin:/bin" LC_ALL=C` for each Cargo call:

```sh
manifest=crates/pi-pressure-context-host/Cargo.toml
cargo metadata --manifest-path Cargo.toml --no-deps --format-version 1 --locked --offline
cargo metadata --manifest-path "$manifest" --no-deps --format-version 1 --locked --offline
cargo build --manifest-path "$manifest" --locked --offline --bin coding-qualification
install -m 0500 "$SHARED_TARGET/debug/coding-qualification" "$FIXTURE/coding-qualification"
cargo test --manifest-path Cargo.toml --locked --offline -p work-engine-manual-host --lib pi_docker
cargo test --manifest-path "$manifest" --locked --offline --test coding
```

Both metadata checks had to report the same configured shared target before
building. The job exited 0: **176 Host tests passed, 5 ignored, 30 filtered
out; 18 coding tests passed**. The scoped Host regression refused
receiver-supplied workspace evidence while preserving genuine Host attachment.

The runners verified the copied producer and the fixture script SHA-256
`2cca1022f492df50ef753ff3a6c5a3ab2867f45ff3dea459f30018b57872e3c6`
before executing these two separate commands from the fixture directory:

```sh
/usr/bin/python3 -I run_responses.py responses-positive positive
/usr/bin/python3 -I run_responses.py responses-failed-terminal terminal-failure
```

- **Positive:** runner and producer exited 0. Five matched SSE streams, three
  tool executions joined to their returned call IDs, and 72,957 bytes reached
  the loopback server on request 2 with the predecessor tool results. The
  run reset into a new Pi generation, rehydrated without tool access, and
  continued to `CompletedSettled`. Host reported `component_passed` and no
  failures.
- **Failed-terminal control:** runner exited 0 **because the producer refused
  with exit 1**. A synthetic `response.failed` inside HTTP 200 left exactly
  one uncertain attempt and its reservation intact, with no retry or tool
  execution. Host reported `not_established`, not `component_passed`.

Both controls removed tool and inference containers. Neither ran workspace
acceptance. Synthetic usage and reservations are not provider billing evidence.

## Receipt digests

These SHA-256 values were recomputed from retained files. The Host receipt
identifier is the hash of the raw stored receipt; it differs from the hash of
the `coding-host-result.json` wrapper. The files are not included here.

| Run | Raw Host receipt SHA-256 (receipt ID) | Host result wrapper SHA-256 |
| --- | --- | --- |
| Completions `loopback-3` | `3ee6ec9f6d04a83d3888cfe52fc3d72b13444e7b9ee0f9f764d3598d354e7527` | `f2c43cf438d2d0dda60593cfc263a2dbd730739ae1213cf946f80af88aaa7429` |
| Initial Responses positive | `e3b88e9ba28d104e52fcbf6fbb88277fac7970f16cead382743ab01e20f35b45` | `e43a9aaec743dd7ec4d734d09b2da7bdf3b92dd6b73931fc839e04c4e26a0def` |
| Initial Responses failed-terminal | `8d168109057cbfb39a220fe8c15396a76535652f4738e1ffe21ebcc72b124169` | `404a49587971eb0e7519d649a5d467d34c6075b5666fdc302b5b51ef3c0dbae4` |
| Follow-up Responses positive | `8565b1f944537fc9693666661809efc7b149668b72f4931194e35822b80e569b` | `f1eda4ac7e3d4350d6e3ae6838217252226e5b9e60c7d787b2c23acbe6eda018` |
| Follow-up Responses failed-terminal | `b822ba8ba69aac9b04c13b0c239704234477dccde94790e581eb738a888ece4b` | `ea74774801e0df12ff23bc450b5a0852bc4674c1838df38b6448250c3f9b2aca` |

The Completions `loopback-3` job JSON SHA-256 is
`7a9a983cc7c10141184608523f8393e0a7e365c5cb25c77b864cfa292346dc54`.
The follow-up Responses job JSON hashes are
`e21ac1830ef9c69480f140bbe96e55e9535c300368627fb688d5698fb06d2928` (positive) and
`af58967482945e63e1f1b4704beea48e3524d2ef0b2db8a8db699bae93ffcd47` (failed-terminal).

`coding-continuity-result.json` SHA-256 for the completed Completions run:
`5151c3f96dc1a6aaa73951c03bb7c08501e66417a85bedfcc530fd3bc32f4022`;
for the completed Responses run:
`6e2a9b552499edd5a8ce9988016035ce0e74cfe0c312ec6ae7f8d0b49f1bd84d`.

## What a cold reviewer can verify

- **From Git:** inspect the exact Stage E source range and merge parents, check
  the receiver and tracked lock hashes, and read the regression tests. For
  example, `git rev-parse c183115a^2 acfd467f^{tree}` should return the follow-up
  commit and tree above. `git show acfd467f:rust/crates/work-engine-manual-host/receiver/pi-coding.mjs | sha256sum`
  should return the paired Responses shim pin.
- **Can rerun separately:** the scoped Rust tests, on a fresh checkout of the
  named revision with the required dependencies and the documented nested lock
  reconciliation in a disposable copy. The test counts above are observations
  from the original job, not a claim that a clean clone with an empty Cargo
  cache can run offline or that a rebuild will yield identical binary bytes.
- **Cannot reproduce from this checkout alone:** the integrated loopbacks.
  The original dirty build input was not captured as an exact tree. A public
  follow-up replay would need the sanitized synthetic runners, pinned runtime
  and toolchain inputs (or a reproducible way to assemble them), and an expected
  assertion set. Receipt hashes let a reviewer check byte identity **if the
  original files are supplied**; they do not by themselves prove execution or
  independence.
  A new run would produce its own receipts.

**Not established by Stage E:** checked build/test evidence tied to a single
workspace export, independent reviewer custody and checked evidence access,
aggregate spool and encoded-evidence limits for live use, a selected live
route, provider behavior or cost, or production readiness.
