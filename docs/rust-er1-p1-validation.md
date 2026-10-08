# ER1 and P1 implementation validation

The user authorized parallel ER1 and P1 implementation after the bounded plans.
Both implementation workers used the requested `gpt-6-sol/high`. These changes
remain uncommitted on the shared branch. J1/J2 campaign integration, HP3 execution
custody, providers, live-root migration and activation were outside this slice.

## ER1: checked read-only episode access

ER1 adds `open_read_only` and `read_checked` under a validated typed native scope,
with exact revision/transition resolution and private checked results. The store
opens SQLite read-only and rechecks native root, executable, marker, schema and
storage identity. It does not initialize, migrate or checkpoint the store.

The new admission port deliberately refuses the generic execution route. Its
supported read surface is `read_checked`; the retained reviewer accepted this as
consistent with the plan. Readback does not create episode write authority.

Real native descriptor tests cover committed WAL frames visible to an already
open reader, v2 evidence admissions and tampering, scoped refusal, historical
readback and write denial. The initial WAL fixture did not prove the intended
condition; a test-only revision corrected it and closed the reviewer finding.

## P1: current-read validation work

P1 shares committing-receipt envelope checks and performs one current-state
validation pass and decode instead of a duplicate historical pass. Historical
replay retains its checks. No cache across reads, schema change, index, history
pruning or weaker integrity contract was introduced.

The eight-preparation attribution workload reduced snapshot decodes and revision
hashes from 18 to 9, progress passes from 2 to 1 and slot lookups from 32 to 16.
The retained baseline decoded 630,938 bytes; the final candidate counter run
decoded 316,045 bytes. Different generated path lengths prevent interpreting
those byte totals as an exact paired ratio. Query plans were retained; indexes
remain deferred.

Final timings use retained V2 Rust source, release executables, exact invocation
and operation order, raw monotonic samples, process time/RSS, and copied generated
SQLite roots with integrity checks and per-file hashes. All roots were created
through real campaign APIs at the same anchored path. No provider was called.
Artifact copying occurs after timed operations; process wall time includes it.

The earlier preparation comparison is diagnostic only: its original harness
bytes were overwritten before archival. The first selection run retained source
but not generated roots. Neither is used as final retained performance evidence.
V2 closes those evidence-retention gaps. Background desktop/UI load remained;
these are observed-machine measurements, not exclusive quiet-machine or host SLO
qualification. One early preparation40 diagnostic overlapped an S5 gate by less
than one second; that pair remains a coverage diagnostic.

## Measured outcomes and acceptance

Repeated dispatched-history comparison used three alternating baseline/candidate
pairs at 16 attempts, ten warm operations and thirty measured samples per case
per run (90 samples per case at the final milestone). This is fewer than the
plan's proposed five pairs. Median current read was 83.605 ms baseline versus
42.197 ms candidate. Earliest/latest recovery medians were 87.992/162.699 ms
versus 46.466/123.121 ms. Raw samples, p95, ranges and paired ratios are retained
in `p1/metrics-dispatched16-v2.json`.

Latest dispatch replay was approximately 1–2% slower in all three candidate
pairs. Its cause remains unresolved under background load. **At the initial handoff, P1 performance acceptance remained pending:** the
plan's no-repeatable-regression condition had not been established. Source correctness and evidence completeness acceptance
do not waive this condition. The subsequently authorized [focused follow-up](rust-p1-replay-regression-validation.md)
did not reproduce the specific suspected slowdown and closes it with measured,
scoped acceptance. The initial observations below remain unchanged; no weaker
acceptance policy or universal performance claim is implied.

| Retained diagnostic | Result |
| --- | --- |
| Selection-width matrix, including 129 obligations and omitted variants | 24/24 processes completed; 30 current-read samples each |
| Preparation 40, one baseline/candidate pair | Both completed; one warm and three measured samples per milestone |
| Preparation 64 | Both completed at 64 |
| Dispatch 64 | Both timed out; last completed API-read milestone 32 baseline / 40 candidate |
| Preparation 129 | Both timed out; last completed API-read milestone 40 / 64 |
| Dispatch 129 | Both timed out; last completed API-read milestone 16 / 32 |

Large diagnostics used one warm and one measured sample, 60 seconds per process;
all eight were attempted in 424.3 seconds within the 480-second total budget.
There was no byte-capacity refusal. Timed-out copies can contain operations after
the last completed API read, so a SQLite integrity pass does not qualify their
later semantic state. No complete 129-attempt history is qualified here.

All completed/partial roots are retained. Inspecting five copied databases
changed only their SQLite shared-memory files after the initial hashes; original
hashes and refreshed post-inspection hashes are recorded, with unchanged DB/WAL
bytes. Counter instrumentation overhead was not calibrated; headline release
integration binaries exclude those test-only counters.

## Review and validation

Two retained review findings concerned evidence quality: ER1's original WAL test
and P1 workload/retention coverage. Both are closed. No source correctness defect
remains from this review. Review was a retained same-provider agent assessment;
no provider-independence claim or Claude review is made.

Final combined gates passed on pinned Rust 1.92.0: locked all-feature tests in
debug and release, each reporting 75 executable tests and seven compile-fail
documentation tests (including helper entrypoints), serialized fault tests,
all-target/all-feature Clippy with warnings denied, and package formatting.
Sources and shared Cargo inputs were unchanged across the final gates. Ignored
performance workloads ran separately and are not routine correctness passes.

The first combined Clippy gate found unused shared test helpers. A fresh
`gpt-6-sol/high` worker added three narrowly scoped lint allowances and a comment.
The retained reviewer verified this semantic-neutral delta. Both complete test
profiles and lint/format gates then passed again. The final helper hash differs
from the measured V2 subject only by that recorded lint delta.

The focused final attribution run retains raw counts and an observed-at-run
binary hash; the subsequent rebuild replaced that target binary. The headline
V2 measurement binaries are separately retained.

Workflow: two parallel implementation workers, one fresh lint-remediation worker
and one retained reviewer; two evidence findings closed and one lint-fix cycle.
No Claude calls occurred. Token/cost totals and complete retrieval statistics
were not exposed; no values are inferred. The main avoidable rework was initial
evidence cleanup: future measurements should preserve source and generated roots
before the first run. Wall time is recorded in the final receipt.

Evidence root:
`/home/bline/.local/state/work-engine/rust-er1-p1-implementation-20261008/`.
The shared Cargo.lock includes other owners' work; `er1-lock.patch` records only
the review-episode dev-dependency edges for already-pinned rusqlite and serde.
