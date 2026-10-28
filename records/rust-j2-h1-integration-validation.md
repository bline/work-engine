# J2/H1 joined integration validation

Integration record, 2026-10-13, following explicit human authorization to integrate
J2 then H1 immediately, preserve S8/unrelated changes, validate serially without
providers, and return publication to the parent. Base HEAD is
`7068bb029e44e4543e27091fd3c89daab0b5e3c7` (accepted H0).
This record supersedes the pending shared Cargo gate in the earlier H1 report;
it does not change the accepted controlled profile or authorize activation.

## Frozen subjects and preservation

Evidence root: `/home/bline/code/.work-engine-tmp/j2-h1-integration-20261013/`.
The integration claim and S8 notice were posted before changes (board 1575,
resource `rust-j2-h1-cargo-integration`). No conflicting Cargo claim existed.

All seven J2 source/test files plus its report, and all eighteen H1 files, exactly
matched their delivered receipts before integration. `accepted-owned-hashes.json`
is the 26-path freeze. Existing Cargo HEAD/index/worktree bytes, their staged and
unstaged diffs, complete index entries, staged-path list and binary index are
retained under `preintegration/`. The freeze contains 2,488 index entries and
70 preexisting staged paths, none overlapping the 26 accepted J2/H1 files.
The only integration source edits add two
workspace members and their two package lock entries. Existing S8 members,
package entries and lifecycle-service dependency changes remain byte-preserved.
No source remediation, staged mutation, commit or push was performed by this
worker. The parent alone owns publication.

Two distinct subjects are qualified:

1. `shared/` is the exact full workspace snapshot after the additive Cargo edits,
   including preexisting unpublished S8 registrations and source. It freezes
   validation against concurrent lifecycle edits without operating that lifecycle.
2. `candidate/` is HEAD plus only accepted J2/H1 files and their minimal Cargo
   additions. It excludes unpublished S8 changes. This is the intended code/config
   publication subject; the final integration report adds documentation only.

`shared-source-manifest.json` and `candidate-source-manifest.json` bind every
captured source/fixture byte. Both Cargo configurations resolve locked and offline.
`dependency-closure-comparison.json` proves equality of all 129 packages reachable
from the five selected test packages, including resolved edges/features and
complete local-package source bytes. The candidate also receives its own locked
build and actual host test gate; closure comparison is not substituted for those.

`Cargo-applied-to-dirty.patch` is the exact additive worktree change.
`Cargo-owned-against-HEAD.patch` is the publication delta: apply it to HEAD Cargo
bytes, not by staging the whole dirty working files. `cargo-identities.json`
distinguishes the worktree configuration from the publication candidate.

| Cargo file | Integrated shared workspace SHA-256 | Publication candidate SHA-256 |
| --- | --- | --- |
| `rust/Cargo.toml` | `257a76f7fddd9705f14bbd7d34d2e9036f65680a5b8a37dda3f3fa378063a071` | `4d7a13318ae89695d4b320770b83a2469fd5534a5ce203c28b009a68e6a67fc0` |
| `rust/Cargo.lock` | `7d02afdc7b5e3d8ca249c2e7fc3e86844ee9d546551b77921f209846e2cc6562` | `aed46285aacbcaf89f4ac6122c947d8c6fc944ce4a109d8b0514ba78b0cb58b9` |

The publication boundary is exactly 29 paths: the accepted 26, these two Cargo
files, and this integration report. `authorized-paths.txt` lists every path; the completed byte/hash manifest
accompanies the final receipt. Any eventual commit must preserve all unrelated index entries
and leave the preexisting S8 Cargo changes uncommitted unless separately authorized.

## Serial provider-disabled gates

All commands use `--offline --locked`, the selected frozen manifest, and
`CARGO_TARGET_DIR=/home/bline/code/.work-engine-tmp/h1-build-20261013/target`.
Builds and tests run serially so no running native-bound test executable is
rebuilt. Rust owner fixtures and controlled Rust peers are the only execution
adapters; no live provider, credential, service activation or S8 operation occurs.

Release packages are `review-episode`, `slice-campaign`,
`review-execution-evidence`, `review-workflow`, and `work-engine-review-host`,
with `--all-features -- --test-threads=1`. Release artifacts and the external
control probe contract are frozen before execution under `shared-evidence/` or
`candidate-evidence/`. These directories preserve exact installed startup pins,
actual host/peer bytes, crash outcomes and independent peer-entry counts.
The controlled final outcome remains `EvidenceUnestablished`, retaining original
claim statuses/reasons; it is not review acceptance or provider qualification.

The shared release gate passed **146 tests, zero failures, three existing
ignored performance tests**. The shared debug owner gate passed **112 tests,
zero failures, three existing ignored tests**. The fresh debug permanent host
execute/recover gate passed **1 test**. All-target/all-feature Clippy with
`-D warnings` and selected-package formatting passed.

A provenance check found Cargo had reused the first debug host test artifact
with its earlier fixture path embedded. Its result is retained in
`shared-debug-host.log` but is superseded for exact startup binding by
`shared-debug-host-fresh.log`. Snapshot mtimes were refreshed without changing
any source bytes, the local crates rebuilt, and the repeated host test passed.
The fresh retained startup record pins the integrated shared Cargo.lock. All 37
shared release startup records also pin that exact lock. The full debug owner
suite uses unchanged accepted owner/test bytes; the 129-package closure equality
is retained. No executable was rebuilt while its process was running.

The candidate build was similarly forced fresh before its separate actual-host
qualification. `startup-lock-binding-check.json` records observed pins so a
passed test invocation is not silently treated as evidence for another startup
configuration. Candidate release qualification passed 19 tests, zero failures, including all
eleven actual-host crash cuts; every recovery preserved the independent peer-entry
count. All 37 candidate startup records pin the publication candidate lock. The
candidate no-default-features build check also passed.

## Scope and remaining operational boundaries

The package/publication integration is separate from S8 validation and any S9
state disposition. The parent must hand S8 the resulting commit and both Cargo
identities before its next validation. This worker supplies the frozen identities
and checked selective patch; a Git commit hash cannot be reported until the
parent creates it. No live supervisor route, provider attestation, deployment,
root migration/disposition, rollback authority or source retirement is inferred.

Tier 2 graph checks reported verified/no recorded gaps for the selected existing
source and Cargo manifest. The graph did not track Cargo.lock, so exact captured
bytes, structured Cargo metadata and locked builds provide the direct fallback.
The graph generation was `2026-10-13T03:54:11Z`, latest coverage record
`2026-10-13T04:07:38Z`; coverage is best effort, not a completeness guarantee.


## Completed gate and publication handoff

| Exact subject / gate | Passed | Failed | Existing ignored | Raw log |
| --- | ---: | ---: | ---: | --- |
| Shared full-workspace release, all five packages/all features | 146 | 0 | 3 | `shared-release-faults.log` |
| Shared debug episode/campaign/E1/workflow owners | 112 | 0 | 3 | `shared-debug-owners.log` |
| Fresh shared debug actual permanent host | 1 | 0 | 0 | `shared-debug-host-fresh.log` |
| Publication candidate release workflow/host, all features | 19 | 0 | 0 | `candidate-release-host.log` |

`shared-clippy.log`, `shared-format.log` and `candidate-default-check.log` retain
passing lint/format/default-build checks. The ignored tests are the two existing
episode history performance tests and the existing campaign history performance
test; none is newly ignored or used to waive a failed gate. No joined test failed
and no accepted product source required remediation. The superseded cached debug
host run is retained and explicitly distinguished above.

Both actual release subjects exercise the permanent CLI, original campaign
admission, H0 begin/readback before E1 entry, real E1/CE3/ER1 joins, and J2 outcome
publication. Each has eleven crash cuts and no additional peer entry during
recovery. Each measured workload includes one successful outcome and three busy
refusals, with zero timed-out/unresolved outcomes or missed probes. Separate
behavior tests cover timeout, backpressure, EOF and changed-owner/input refusal.

| Release subject | Worst scenario p99 | Maximum observed reply | Successful operation time |
| --- | ---: | ---: | ---: |
| Shared | 0.589 ms | 1.348 ms | 3,829 ms |
| Candidate | 0.541 ms | 1.501 ms | 3,814 ms |

Each operation includes the selected 3,000 ms peer delay. Each subject uses 700
external samples (200 cold, 100 capture, 200 SQLite contention, 200 four-request),
5 ms delays and a continuously draining Rust consumer. Both satisfy p99 ≤50 ms
and maximum ≤250 ms. Memory was not observed and no broader workload bound is
claimed. Raw samples remain in each evidence directory; `gate-summary.json`
records counts and log hashes.

Release host and controlled peer bytes were frozen before measurement:

- shared `work-engine-review-host`: `d350f469d4bffe0aa7334244459f2c79dedf8f5acd49cc97b9427893d8bd224f`.
- shared `review-host-controlled-peer`: `d9f152831ec6fa9e472f200701aac9b5d1487b937c639a4bca1298b8aa4e9353`.
- candidate `work-engine-review-host`: `d6af96759ec8750260259d0e21c3d52cc25407d4b4c244ec2190c38e15d0b694`.
- candidate `review-host-controlled-peer`: `60492e7969b695c2ec7be75468ba213a8d4a3856f5785b6b4a87fc836e9cdb9f`.

The exact final 29-path manifest is `publication-path-manifest.json`; it records
both worktree and publication hashes for Cargo, and identical hashes for the
accepted J2/H1 files and this report. `validation-receipt.json` binds that manifest,
all gate results, snapshot source identities, preservation checks and this report.
`Cargo-owned-against-HEAD.patch` passed `git apply --cached --check` against the
unchanged index; that check did not stage anything.

Implementation/integration validation is complete. **Publication and the S8
commit handoff remain with the parent.** The code/config candidate was tested;
there is no resulting commit hash yet in this worker's receipt. The parent can
publish only the 29 authorized paths with the candidate Cargo bytes, preserve the
70 existing staged paths and S8 working deltas, then provide S8 the resulting
commit plus both Cargo identities above before further validation. No operational
gate is waived by this handoff.
