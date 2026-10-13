# H1 controlled initial host validation

Implementation record, 2026-10-13. This implements the explicitly authorized
SC0-only H1 continuation of the [controlled-host plan](rust-hp3-controlled-host-implementation-plan.md),
using accepted H0 `7068bb029e44e4543e27091fd3c89daab0b5e3c7` and the J2 owner's
finalization exports. It is not broad HP3 completion, provider qualification,
review acceptance, supervisor activation, deployment, migration or retirement.

## Implemented boundary

`review-workflow` composes existing campaign, compiler, CE3, episode and E1
owners. `work-engine-review-host` is the permanent Unix stdio consumer with
`initialize --startup /absolute/startup.json` and `serve --startup ...`.
The startup schema is `review_workflow::startup::StartupDocument`; the actual
process fixture in `tests/support/mod.rs` shows the complete installed selection.
There is no alternate test workflow, fake campaign grant, fixture evidence owner,
direct domain-table write, Node call or per-command episode subprocess.

The profile is exactly `review-host-controlled-initial-v1`, frozen SC0 corpus,
one selected obligation, one initial attempt and one installed startup selection.
Startup pins actual host and peer bytes/modes, source/toolchain/lock artifacts,
target/features, runtime dependencies, gate/candidate/physical/selection inputs,
authority/session, compiler input/options and request input, CE production grant
and custody configuration, and distinct anchored owner roots. Startup pins are
trusted launcher selection, not proof of human authorization or build attestation.
Changed startup bytes also change the campaign writer configuration binding.

The selected compiler mode is `manifest-only-unverified`. The pure manifest
projection is consumed in the actual E1 request. External and embedded runtime
requirements are refused as whole operations; the requirements API does not
establish verified sources in this composition. No unverified skill is relabeled
verified. H1 selects exactly one CE production grant with observation,
establishment and admission-read permissions, without a findings grant.

Only a fresh applied campaign preparation and consumed dispatch permit can
construct the private launch token. H0 applies episode begin and ER1 reads it
back before E1 launch. E1 owns capture and custody. Campaign installs the actual
E1 reader and CE3 port, registers exact result content before the H0 result
write, and joins its historical ER1 read. J2 alone finalizes the outcome.
Recovery replays the original campaign admission receipt to recover its config
digest, verifies the descriptive original dispatch locator with `replay_dispatch`,
and uses E1 resume/reconcile. It never calls execution or authorizes a new launch.
There is no second workflow journal. Partial/unknown roots are preserved.

The controlled peer is same UID with `mutationAuthorized=true`. Actual owner
readback publishes `EvidenceUnestablished`, retaining both original **false**
claim statuses and reasons including `mutation_authorized`; it does not rewrite
false to an unestablished claim status. Findings are empty and consumption
registrations remain unconsumed. The campaign is not terminalized.

## Limits and control behavior

One Tokio control runtime and one workflow OS thread/runtime; synchronous domain
work and compiler work remain on the worker. One command is in flight, with no
queued mutation. An eight-slot output queue reserves a terminal response slot
before work. Status is a bounded cached revision/phase observation explicitly
marked stale while busy; inspection uses authoritative owner readback. The campaign
owner retains its lifetime writer fence; a competing host is refused at open.
No per-operation transaction or CE gate is carried across the child wait.
Stop-admission and EOF finish owned work. Blocked output closes admission; the
writer has a two-second output deadline, and lost response recovery uses original
owner identities. There is no automatic restart, retry or resend.

Command/status envelopes are 64 KiB, detailed responses 1 MiB, startup metadata
1 MiB, combined captured compiler/options/request inputs 4 MiB. Exact composed
request encoding is checked against E1's 1 MiB bound before preparation. Pin reads
also have explicit per-file caps: executables 512 MiB, release metadata/runtime
dependencies 256 MiB, peer source 4 MiB, gate/selection/authority/CE inputs 1 MiB.
E1's stream, result, terminal/root payload, capture/cleanup and database limits
remain authoritative. Oversized replies return capacity refusal, not a truncated
success. This does not qualify broad history or concurrent-domain performance.

## Validation evidence

External evidence root:
`/home/bline/code/.work-engine-tmp/h1-validation-20261013/`.
The final gate uses its `final/` subdirectory. `freeze.json` binds machine,
compiler, lock, release artifacts/features and probe contract before measurement.
`source-manifest.json` and `source-archive.tar.gz` retain the isolated source and
fixture subject. `artifacts/` retains startup records and content-addressed pinned
bytes. Crash traces and raw latency samples are JSON; Cargo output remains raw
logs. Build/disposable state is under
`/home/bline/code/.work-engine-tmp/h1-build-20261013/`.

The final isolated results and artifact identities are recorded below.

The initial debug permanent-CLI gate passed four tests. The first release gate
passed its then-current workflow tests, four process tests, two control tests,
ten actual crash cuts and two compile-fail examples. Initial Clippy found three
control-style warnings, which were corrected. A later competing-host test assumed
two hosts could open the campaign; the owner correctly refused the second writer
at startup. The test now asserts that earlier refusal. The failed raw run is
retained as `final/release-all.log`; it was not a duplicate-dispatch observation.
The first final debug process run exceeded the test consumer's 30-second response
wait. Debug-only wait was raised to 90 seconds; the full execute/recover test then
passed in 71.14 seconds. Release waits and 50/250 ms control targets are unchanged.
Both `debug-host.log` and `debug-host-confirmed.log` are retained.

## Integration and retained gates

This worker changed only the two new crates and this report. Shared
`rust/Cargo.toml` and `rust/Cargo.lock` remain under another owner's coordination;
this worker did not edit them. The isolated workspace contains the same domain
sources plus the two new members and uses an isolated lock. Required shared
integration is exactly adding `crates/review-workflow` and
`crates/work-engine-review-host` and their package lock entries, followed by the
serial joined workspace gate. The exact unapplied patch is
`final/shared-integration.patch`; `git apply --check` passed against the current
shared files. Do not replace the shared lock with the isolated
subset lock, which intentionally excludes unrelated workspace packages.

Until that handoff and shared-workspace gate occur, qualification is for the
frozen isolated composition, not a completed shared workspace integration. No
staging, commit or publication was performed. No provider/credential access,
active supervisor changes, lifecycle/UI edits or source retirement occurred.

Graph evidence used Tier 2 Verify, project `home-bline-code-work-engine`, with
source fallbacks for concurrently changing campaign files. Final relied source
coverage at generation `2026-10-13T03:50:27Z` reported content-verified and no
recorded gaps for workflow/host, episode application/store and CE schema. This is
best-effort coverage, not a completeness proof. Owner regression receipts and
final test coverage are identified with the completed results below.


## Final isolated gate

All Cargo checks used `--offline --locked` and the isolated manifest
`/home/bline/code/.work-engine-tmp/h1-build-20261013/rust/Cargo.toml`, with target
`/home/bline/code/.work-engine-tmp/h1-build-20261013/target`.

| Gate | Result | Raw evidence under `final/` |
| --- | --- | --- |
| Release H1, both crates, all features, serial | 19 passed, 0 failed: 14 runtime + 5 compile-fail | `release-final.log` |
| Default-feature debug workflow | 8 passed, 0 failed | `debug-workflow.log` |
| Debug actual permanent CLI execute/recover | 1 passed, 0 failed | `debug-host-confirmed.log` |
| E1 release `test-faults`, serial | 20 passed, 0 failed, including existing actual-process cuts | `e1-regressions.log` |
| H1 all-target/all-feature Clippy `-D warnings` | Passed | `clippy-confirmed.log` |
| Owned Rust formatting | Passed | `format-check.log` |

The final H1 command was `cargo test --release -p work-engine-review-host
-p review-workflow --all-features -- --test-threads=1 --nocapture`, with the
manifest/target/offline/locked arguments above and `H1_EVIDENCE_ROOT` set to
`final/`. The default host also passes the no-default-features build check.
No whole unoptimized host suite or shared workspace suite is claimed.

The crash test exercises eleven actual host deaths: before preparation, after
preparation, after dispatch, after checked episode begin, E1 launch intent,
E1 terminal custody, each of the two CE establishment commits, episode result,
J1 joined commit, and J2 outcome commit. Every restarted process uses the same
pinned executable/startup. The independent peer-entry file stays at zero before
entry and one after entry; recovery never increments it. The before-preparation
case reports not-dispatched; retained preparations/dispatches remain unresolved
without a launch capability. Terminal custody cases recover the exact final
controlled outcome. `crash-*.json` preserves each observed result and counts.

External release control results (microseconds), with a continuously draining
Rust observer and a 5 ms delay between probes:

| Scenario | Samples | p99 | Maximum | Missed |
| --- | ---: | ---: | ---: | ---: |
| cold | 200 | 656 | 842 | 0 |
| capture | 100 | 439 | 440 | 0 |
| contention | 200 | 539 | 695 | 0 |
| four | 200 | 590 | 1283 | 0 |

All are below 50,000 µs p99 and 250,000 µs maximum. The successful four-request
workload took 3,805 ms including the selected 3,000 ms peer delay: one committed
outcome, three busy refusals, zero timeout/unresolved results and one peer entry.
Separate tests qualify child timeout, EOF, stop-admission, output backpressure,
SQLite contention followed by successful completion, and competing writer refusal.
Memory was not observed; no memory/performance bound is inferred. Raw samples,
startup identity and counts remain in `probe-*.json` and `operation-timing.json`.

The installed test host/peer have mode `0500` (build outputs were `0775`). Exact
release artifact hashes:

- `work-engine-review-host`: `a3a5e6839fc96b741883bed76369715d5fb00db2d9c1ddd08d7a268b1ca231cb` (9012872 bytes).
- `review-host-controlled-peer`: `60492e7969b695c2ec7be75468ba213a8d4a3856f5785b6b4a87fc836e9cdb9f` (608576 bytes).

Isolated lock: `0891381bf06b714232c1161ead19704f0565bf0473a027df01a61182ed9d42b8`.
Final source-manifest digest: `24a9b6f23f0ba7cca6b873b29fe735f4190e93731295480d971a009220bb95ff`.

`validation-receipt.json` records owned file hashes, raw-log hashes, counts and
freeze identity. The frozen source tree exactly matches the delivered owned
crate files. Startup metadata/bytes are retained by content hash in `artifacts/`;
the complete isolated source/fixture archive is retained separately. Runtime
pins include Git, timeout, the loader and the transitive shared libraries reported
by `ldd` for host/peer/Git/timeout. These are measured build inputs and launcher
pins, not a proof against arbitrary privileged or same-UID mutation.

H0's accepted owner gate remains its published 47-pass serial test-faults receipt
in [rust-h0-implementation-validation.md](rust-h0-implementation-validation.md).
J2's final owner receipt is
`/home/bline/code/.work-engine-tmp/j2-logs/validation-receipt.json`: 43 debug tests,
54 optimized test-faults tests, plus its later owner-store-loss test in both
profiles, all passing (one existing ignored test in each full suite).
All seven source/test hashes in J2's manifest match the isolated host snapshot;
`final/j2-source-match.json` records the comparison. That suite includes J1's
existing owner-join crash regressions. The H1 process cuts independently qualify
this composition. J2 report:
[rust-j2-implementation-validation.md](rust-j2-implementation-validation.md).

Test/manifest coverage subsequently reported no recorded gaps with verified
content at graph generation `2026-10-13T03:54:11Z`; this report was being written
and was checked directly after graph metadata indicated that change.
