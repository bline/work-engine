# J2H1-R2 remediation and validation

This bounded repair addresses R2 from the retained fresh review of
`c4952af0542763de365ead11ba69d70a1347d831`. The review report is retained at
`/home/bline/code/.work-engine-tmp/j2-h1-review-20261013/review-report.json`.
R1 belongs to the workflow owner. This report does not accept the review,
publish a commit, or authorize a provider operation.

## Reproduction and change

The baseline plus the new regression returned a committed initial aggregate
after changing only one CE `operation_admissions.admission_sha256` to a different
syntactically valid SHA-256. The canonical claims state, finding revisions,
grants and campaign receipts stayed intact. The baseline regression failed with
`initial aggregate accepted changed reliance: Committed`; it used two findings,
one committed invalid disposition and one pending disjoint disposition.

`initial_outcome.rs` now uses one private `verify_committed_outcome_owners`
check for initial outcome recovery, fresh evaluation preparation, evaluation
recovery and evaluation publication. In addition to the existing finding-owner
checks, it checks every committed evaluation used by the current aggregate:

- Reconstruct its original reliance command from the registered stage and
  admission; compare CE's derived locator to the saved original locator.
- Reconcile through CE and compare the complete exact reliance and locator
  against both the evaluation record and checked stage receipt.
- Require that exact finding revision and reliance in the fresh current
  projection, then ask CE to verify projection freshness.

The check runs before new evaluation effects and again after downstream effects
before the aggregate commit, including the newly completed evaluation. The
second check runs outside the campaign mutex because projection admission reads
campaign state. The following campaign CAS still requires the exact last checked
revision. Pure historical state validation remains separate from live owner
reconciliation.

Changed exact owner records produce conflicting progress. Missing committed
reliance is pending; unreadable/inconsistent owner state is unresolved. Existing
partial-effect receipts and original locators remain available. Recovery does
not recreate missing committed reliance, repair corrupted owner bytes, issue a
permit, change verdict/status semantics or consume terminalization commitments.

## Regression coverage

Three added owner-join regressions cover a changed valid admission hash, a
missing admission row and an unavailable real CE database. Each exercises:

- Initial aggregate recovery with one disposition and with both dispositions.
- Recovery of the affected committed evaluation and completion of a pending
  disjoint evaluation, requiring no new effects while prior evidence is bad.
- Recovery of an already committed disjoint evaluation whose own reliance is
  intact but whose returned aggregate uses the damaged prior reliance.
- Restoration of the exact test-owned bytes followed by successful recovery
  through original locators, byte-equivalent original publication receipts,
  reported aggregate and exactly two original reliance operations.

The missing-row test intentionally makes CE's owner read inconsistent; the
application refuses that state rather than treating it as safely absent. These
tests do not claim arbitrary concurrent-store mutation is transactionally
atomic across CE and campaign.

## Validation custody

All builds and tests use an isolated archive of the named baseline plus only
the two R2 Rust files, Rust 1.92.0, `--locked --offline`, an independent target
directory and task-local temporary roots. No shared executable is rebuilt while
its tests run. Shared Cargo files and unrelated source are untouched.

Evidence directory:
`/home/bline/code/.work-engine-tmp/j2-r2-20261013/`.
`frozen-source.json` records exact repaired source and baseline Cargo hashes.
Raw logs are under `logs/`. The first reproduction compile attempt lacked the
archive's external fixtures and used direct equality on a receipt type without
`PartialEq`; these test setup errors were corrected before the actual baseline
failure in `baseline-reproduction2.log`.

The optimized full `slice-campaign --features test-faults` suite passed
**58 tests, 0 failures, 1 existing ignored performance workload**, including
four compile-fail doctests. It includes the existing actual-child crash ledger,
fixture finding/evaluation cuts, post-owner-failure receipt preservation,
historical publication corruption, original-ID replay and P1 current-read
checks. The earlier focused release run passed all three new regressions.
Clippy passed for all package targets with `test-faults` and warnings denied,
using a separate lint target so running debug test executables were untouched.
The full default debug suite passed **46 tests, 0 failures, 1 existing ignored
performance workload**, including all three new regressions and four
compile-fail doctests. Both suites ran with `--test-threads=1`; debug and release
used distinct executables, and lint used a separate target. Owned-file rustfmt
and diff whitespace checks passed. No source changed after the final suites
started. Raw-log hashes, exact commands and report/source hashes are recorded in
`validation-receipt.json` in the evidence directory.

| Repaired file | SHA-256 |
| --- | --- |
| `rust/crates/slice-campaign/src/initial_outcome.rs` | `bf4a33f2749fbad70894ad8f5d7f59aaedc5bbc6cbf4ad2dec62577d9baa9663` |
| `rust/crates/slice-campaign/tests/owner_join.rs` | `b785300ba9c788e149f4a4926652a87021314ce0856a5995227345dff0d32d24` |

Tier 2 graph evidence used `home-bline-code-work-engine`, confirmed root and
generation `2026-10-13T04:51:15Z`. Exact symbol discovery, CE reconciliation
snippet and both call directions for `build_view` identified the live versus
historical paths. Sparse/heuristic clone edges were not treated as authoritative;
exact source supplied the owner-injected behavior. Coverage checked the two
owned Rust files plus CE application/store, all content-verified without a
recorded gap. This is bounded best-effort evidence, not an exhaustive audit.
Final coverage also included the campaign application, CE reliance contract and
package manifest at generation `2026-10-13T05:03:05Z`; all seven paths were
content-verified with no recorded gaps. The raw receipt is `coverage.json`.

Only `slice-campaign/src/initial_outcome.rs`, `tests/owner_join.rs` and this report
are owned by this repair. There are no CE, Cargo, provider, SC3, migration or
activation changes. Parent integration and retained reviewer re-evaluation are
still required before publication.
