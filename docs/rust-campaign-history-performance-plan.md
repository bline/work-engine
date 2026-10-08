# Campaign history performance: bounded next slice

Prepared 2026-10-08 against committed base
`d4fc06027dc2dc11d91187da11a096c857148c25`. Planning/read-only diagnosis only;
no product changes, builds, tests, profiling, provider calls or commits were made.
Future implementation is assigned `gpt-6-sol/high`. Planner thread:
`01a11996-0597-7f50-bd7d-9e2187a811cd`; effective model identity is not independently
observable. This proposal preserves the [SC2 contract](rust-campaign-sc2-implementation-plan.md)
and [core validation limits](rust-ce3-sc2-core-validation.md).

The recommended next slice is **P1: reproducible measurement plus per-read
validation deduplication**, contingent on measurement identifying useful savings.
It does not select a new history representation, cache or schema. The intended
result is less repeated work with the same evidence checks and readable history.
Absolute host latency/throughput acceptance remains for the host owner; no user
SLO is inferred here.

## Evidence and limits

| Evidence | What it establishes |
| --- | --- |
| Candidate-3 `attempt-boundary-interim.log` | Debug test `accepted_129_independent_attempts_remain_readable` ran beyond 60 seconds. It contains no completed result or per-slot timing trace. |
| Candidate-3 manifest and completion receipt | Builder-reported interruption after more than two minutes at 40 slots. The stress source was removed; the final manifest does not freeze that attempted test. This is a reason to measure, not a reproduced baseline or qualified 129-attempt history. |
| `src/store.rs:339–423` | `read` decodes current state, checks SQL revision and an exact JSON commit receipt, invokes `replay_result` (which decodes again and validates historical progress), then validates current progress. This duplicated work is source-backed; its share of latency is unmeasured. |
| `src/store.rs:446–738,892–905` | Both progress passes fetch slots and preparation receipts, decode/hash saved snapshots, and validate dispatch snapshots when present. Slot reads also verify store binding. Current-only active-slot and dispatch-presence checks differ from historical checks. |
| `src/application.rs:1794–1835,2188–2285` | Preparation checks replay before current state; recovery reconciles, replays exact history and reads current state. One public operation can therefore trigger several validation paths. |
| Campaign migrations 0001/0002 | Operation ID and slot keys are indexed by their constraints. Exact-current-receipt lookup compares `identity_key,result_json`; prior-revision existence extracts JSON. No measured query plan or cost is claimed. |

Source paths above are under `rust/crates/slice-campaign/`. Repeatedly decoding
snapshots containing growing prefixes suggests increasing cumulative work; static
structure alone does not establish a complexity-dominant runtime bottleneck.
SQLite lookup, canonical hashing, filesystem checks, serialization and fixture
construction are competing explanations to separate in measurements.

## P1 workload and measurement gate

Use the current controlled-SC0 configuration and real campaign APIs/SQLite in
private disposable roots below `/home/bline/code/.work-engine-tmp`. Generate the
corpus in Rust from retained fixtures, recording exact source/fixture/config hashes
and commands. Construction uses admit, candidate binding, selection, preparation
and dispatch APIs. Direct SQL edits belong only to negative tamper cases, never
positive corpus creation. No provider entry occurs.

| Corpus | Observations and scope |
| --- | --- |
| Selection-only, 0/1/2/8/16/32/40/64/129 obligations | Separates selection width from persisted attempts. Preserve selected and omitted variants; 129 selection entries already passed a narrower correctness test. |
| Independent preparations at those nonzero counts | One initial attempt per selected obligation, in deterministic order. Record every completed milestone, current JSON bytes, receipt/slot counts and total receipt-result bytes. 129 attempts is a target experiment, not already supported by evidence. |
| Same prepared corpus with dispatch recorded for each attempt | Exercises saved dispatch snapshots, exact historical replay and recovery. Consume permits as data locally; no execution owner/provider is invoked. |
| Reopen and old-operation readback | At each completed milestone measure first read after reopen, repeated current reads, earliest/latest preparation replay, exact reconciliation and recovery; verify retained old receipts/revisions. Reopen is not an OS cold-cache claim. |
| Byte-capacity and semantic regressions | Preserve current encoded-capacity limits and distinguish capacity refusal from performance timeout. Existing private retry/evaluation tests remain semantic evidence; they are not a public real-owner performance workload until the join supplies those routes. |

For each milestone, separately time corpus growth (each preparation/dispatch),
current read, same-operation replay, reconciliation and recovery. Do not average
fixture Git setup, compilation or corpus construction into read latency. Retain
construction time because real campaign growth is itself an operational cost.
If any requested count cannot fit the unchanged byte contract, retain the exact
refusal and last readable state; do not shrink fields or raise limits silently.

Before timing, freeze toolchain/lock/features/profile, executable and source hashes,
hardware/OS/storage, SQLite configuration, workload sizes and operation order.
Release build is the primary comparison; debug reproduces the original symptom
separately. Run one workload process with serial fault tests and no competing
build/profile run. Keep default durability/busy settings and real filesystem checks.
A bounded run budget is a harness safety limit, not a product SLO: choose and
record it before running (initial proposal: ten minutes per corpus construction).
An interrupted run is censored/incomplete, never a passing sample.

Use monotonic wall time plus process CPU and peak RSS where available. Record raw
samples, not just ratios: proposal is five alternating baseline/candidate runs,
ten untimed warm reads and thirty measured operations per available read case.
Report medians, p95 with sample count, range and paired run ratios. New roots are
API-created at the same anchored paths for corresponding runs; compare semantic
results under recorded root/epoch differences, never rewrite those identities.
Retain successful receipts and validation outputs before disposing of scratch.

Use package-private, test-only counters/timers in an internal unit workload using
the same corpus recipe to attribute query counts, decoded
snapshot bytes, hash calls/bytes, progress passes and binding checks. Measure an
uninstrumented binary for headline latency and report instrumentation overhead.
Capture `EXPLAIN QUERY PLAN` for the two receipt lookups without changing indexes.
No new generic profiler framework or public diagnostic capability is needed.

**Decision gate:** baseline must run the newly frozen reproducible corpus, and the
profile must identify the duplicate pass/decoding as material enough to justify
P1. If it does not, retain the measurements and revise the bounded proposal; do
not implement a cache or index merely to produce a change. A performance-success
claim requires fewer attributable duplicate operations plus a repeatable latency
improvement outside observed baseline variability, with no repeatable regression
in the other measured operations. Ambiguous/noisy timing yields an inconclusive
performance result, even if correctness passes. Report the measured supported
corpus explicitly; this does not qualify all byte-valid history or host responsiveness.

## Candidate implementation and alternatives

| Route | Benefit hypothesis and boundary | Disposition |
| --- | --- | --- |
| Share validation within one `Store::read` | Fetch/check the exact committing receipt against current bytes, validate its envelope and prior revision, decode/hash that snapshot once, run one full current-progress pass. Historical `replay_result` keeps historical semantics. Reuse already fetched slot data within that call only if its identity remains exact. | Smallest candidate after P1 measurement. |
| Index exact receipt/revision lookup | May reduce receipt scanning if query plans and timings identify it. An index/reference is a locator; result bytes, envelope, hashes and current SQL revision still require validation. | Defer. Stored locator/schema changes require their own new-root/version/compatibility decision and measured benefit. No whole-JSON index proposed by default. |
| Validated immutable history retained across calls | Could avoid decoding old prefixes, but the current private SQLite store is not a proof that rows cannot change. Would need an owned immutable identity, tamper detection and exact invalidation/read-consistency contract. | Defer. Revision/mtime/inode alone cannot establish unchanged historical contents. No generic cache/store framework. |

Implement an explicit private current-versus-historical validation mode or separate
private helpers with equivalent checks; callers cannot request weaker validation.
A refactor may extract the receipt-envelope check from replay and reuse it for the
current row, but simply removing `replay_result` loses required evidence. Conversely,
returning its historical result directly loses current active-slot checks. Validate
that the receipt identity matches the caller/current row, not only its own snapshot.
Retain current binding checks in P1; reducing their frequency is not necessary to
remove duplicate decoding and needs a separately justified consistency boundary.
No validation result survives a read, mutation, reopen, owner epoch or external
owner call. P1 neither holds campaign SQL transactions over CE/episode calls nor
changes campaign-gate → CE lock ordering.

## Correctness, ownership and acceptance

The resulting current read still checks SQL revision, canonical snapshot hash,
exact commit receipt/result bytes, root/config/identity/kind/content/prior/result
bindings, preparation revision exclusion from hashing, full historical attempts,
per-obligation predecessors and uniqueness, latest active slot, dispatch binding,
completion/evaluation semantics and capacities. Exact old replay remains readable
after later dispatch/retry while supplying no fresh admission/dispatch capability.
History is never pruned, truncated or replaced by a summary for this optimization.

Negative gates mutate SQL revision, current state hash, committing receipt,
preparation/dispatch result, slot active/dispatch fields and per-obligation chain.
Include coherently rehashed but semantically invalid state, missing prior receipt,
wrong identity/config, and tampering between successive reads and after reopen.
Test valid historical replay whose slot subsequently changed so stricter current
checks are not accidentally applied to old state. Verify same-operation replay,
content conflict, stale revision/epoch, recovery ambiguity and admission refusals.
Existing process-fault, unsupported-schema, lease and capacity gates remain passing.
Compare exact bytes on repeated reads of the same root and semantic dispositions
across equivalent fresh roots; randomized root IDs prevent naive cross-root hash
comparisons. No timing assertions in routine correctness tests.

| Owner | Exact proposed edit scope |
| --- | --- |
| P1 worker, Sol/high | `rust/crates/slice-campaign/src/store.rs`; new `tests/history_performance.rs` for an ignored explicit-run Rust workload; new `tests/history_integrity.rs` for public-store tamper/read/replay regressions; `tests/support/mod.rs` only to reuse corpus setup; this plan's eventual result/update. Test-only internal counters/unit checks stay in `store.rs`. |
| P1 integration owner | Accept exact source/fixture/config/run manifest, retained raw samples and baseline/candidate comparison; serialize any needed package manifest edit. No dependency or workspace lock change is expected. |
| Episode/join lane | Episode scoped readback and campaign application/admission/completion join. It owns `application.rs` changes and new joined tests. P1 does not change these files in its first slice. |

Episode-only implementation and P1 corpus/profiling can proceed in parallel with
separate file ownership and build directories, but timing runs require a quiet
machine. Serialize campaign join integration after the P1 store/test checkpoint,
or reverse that order explicitly and regenerate the baseline. In either order,
one named integration owner reconciles `store.rs` validation with new completion
fields, freezes the combined subject and reruns package plus joined fault gates.
Do not infer that individually passing lanes qualify their composition. Any new
need for application-level deduplication is deferred to that serialized decision.
Lifecycle/UI work and workspace manifests remain under their existing owners.

After implementation: locked package tests in debug and release with fault feature
and `--test-threads=1`, compile-fail gates, all-target/all-feature Clippy with
warnings denied and package formatting. Run the ignored profiling workload
explicitly in release, retain its executable hash and full invocation, and rehash
sources after gates. Completion reports correctness and performance separately,
including unfinished milestones, noise and retained review findings. These are
proposed future gates; none ran during this planning task. HP3 provider custody,
positive real-owner retry, host responsiveness and live-root changes remain pending.

## Planning provenance

Tier 2 Verify, graph project `home-bline-code-work-engine`, parent generation
`2026-10-08T03:33:30Z`; this lane's source coverage generation
`2026-10-08T03:35:49Z`, with manifest/output follow-up at `2026-10-08T03:40:28Z`.
Relevant symbol searches and depth-one bidirectional validation trace exhausted
pages. Heuristic edges into grok/claim-codec/terminal-UI were not relied on; exact
source supports the call claims. All cited repository paths have coverage checks.
SQL partial ranges (0001: 8,13,17,25,33; 0002:16) were read with both whole files.
Other paths reported metadata match/no recorded issue, a best-effort signal rather
than completeness proof. Retained logs are external filesystem evidence.

Source/log hashes, coverage and limitations are retained in
`/home/bline/.local/state/work-engine/rust-episode-join-performance-planning-20261008/history-performance-evidence.json`.
Parent owns chatboard resource `rust-host:episode-join-performance:planning`, claim
`6b855b7d-38f5-41f2-9020-137504b0c2bf`. This document and that evidence file are this
lane's only writes.
