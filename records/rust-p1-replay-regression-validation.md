# P1 replay regression follow-up

The user authorized targeted work on the unresolved replay slowdown from
[ER1/P1 validation](rust-er1-p1-validation.md). The retained implementation worker
used `gpt-6-sol/high`; the retained reviewer continued the same assessment.
No product correction was justified by the resulting evidence. The only code
change adds an opt-in focused mode to the existing ignored performance harness.
Nothing in this session was committed or pushed.

## Outcome and acceptance

The specific suspected 1–2% latest-dispatch replay slowdown was **not reproduced**
in a stronger, counterbalanced comparison at 16 dispatched attempts. Scoped
performance acceptance is supported for that measured workload. This closes the
specific suspected regression; it does not prove zero regression everywhere.
The original mixed-operation workload remains different, and its small observed
shift is not retrospectively declared noise.

The paired candidate/baseline geometric mean latest-replay ratio was **0.99993**,
with an approximate 95% interval **0.99331–1.00659**. Four pair ratios were above
one and four below. Baseline-first and candidate-first geometric means were
1.00015 and 0.99971. The evidence combines equivalent corpora, unchanged replay
checks, counterbalanced order, mixed signs and precision excluding the earlier
1–2% effect magnitude. Acceptance does not rely on nonsignificance alone and
introduces no permitted slowdown threshold.

Earliest replay's ratio was 1.00084 (interval 0.98922–1.01259). The current-read
control retained the optimization: ratio 0.50925 (interval 0.50202–0.51658),
about 49% less latency. These controls have fewer samples per run than the primary
endpoint and are not separate universal performance claims.

## Reproducible comparison

The predeclared protocol used eight adjacent pairs in order BC, CB, CB, BC, BC,
CB, CB, BC. Each release process created a fresh 16-dispatch SC0 corpus through
public campaign APIs, reopened it, warmed latest replay ten times, and timed
60 latest replay calls. It then warmed and measured earliest replay and current
reads. Serialized result equality was checked outside the timed closures.

Each pair was the statistical unit. The interval is a Student-t interval on
eight paired log median ratios (seven degrees of freedom), exponentiated; it is
approximate, with limited power. Individual calls were not treated as independent
replicates. No slow runs were discarded or replaced. All 16 processes and eight
pairs completed, with matching corpus shape, within 40 seconds per process and
a 640-second total cap; process time summed to about 174 seconds.

Both frozen Rust 1.92 release trees contain 393 files and differ only in
`store.rs`. Identical harness, support and dependency inputs were used. Exact
executables, source, commands, raw samples, order, load, and all 16 generated
roots are retained. The reviewer recomputed the primary statistics from raw logs.
Background desktop activity remained; no exclusive quiet-machine claim or host
SLO is made. S5 was independently published as `b4c6df9b` during preparation;
this did not change the frozen subjects or this session's product files.

Source diagnosis found the same receipt decode, envelope/prior-revision checks
and historical progress validation in both replay paths. P1 extracted the
receipt checks into a helper, without adding replay queries or history walks.
Inlining/layout was a hypothesis, not an established cause. Restoring duplicate
checks without evidence would be speculative, so product code remains unchanged.

## Validation and retained evidence

Final locked all-feature `slice-campaign` tests passed in debug and release:
30 executable tests and two compile-fail documentation tests in each profile.
All-target/all-feature Clippy with warnings denied and package formatting passed.
Sources and shared Cargo inputs were unchanged during the gates. The 16 explicit
release workload executions passed separately. Two final rustfmt line wraps in
the live harness are recorded as a semantic-neutral delta from measured source.

Evidence:
`/home/bline/.local/state/work-engine/rust-p1-replay-regression-20261008/`:

- `focused-design.json`, `design-review.json`: predeclared protocol and review.
- `subjects-manifest.json`, `artifact-manifest.json`: retained subject/artifact hashes.
- `run-ledger.json`, `analysis.json`, `runs/`: invocation, raw outcomes and roots.
- `review.json`, `final-gates/receipt.json`, `final-receipt.json`: acceptance and validation.

Prior ER1/P1 evidence remains intact. Larger censored histories are not qualified
by this follow-up. J1/J2, HP3, providers, migration and activation remain outside
scope. The accepted campaign source can now be used for separately planned join
work without changing those authority boundaries.

Workflow: one retained implementation worker and one retained reviewer; no
product repair, no automatic timing rerun and no Claude calls. Token/cost totals
were not exposed. Freezing corpus identity and evidence retention before the
first run avoided the archival rework encountered in the earlier P1 experiment.
