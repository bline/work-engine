# Context-lifecycle cost comparison, 2026-11-14

Recorded 2026-11-14 UTC. Status: **observed operational improvement** in five
continuing root sessions, using a frozen API-equivalent pricing scenario.

After native context refresh, scoped document loading, and context-pressure
delivery changes, equal-root mean cost per model call was **16.6% lower**.
This is a meaningful observed reduction. The comparison does not isolate the
effect of any one change or establish a causal or whole-project saving.

## Scope and method

The same five development roots cover research, interface, organizational,
development-vertical, and Pi work. A call is a model response recorded by the
usage ledger, rather than a completed task or a tool invocation. All model
calls in each selected root and period are included, including preparation and
recovery. Children, reviewers, the observer session, and implementation costs
are outside this comparison.

| Period | UTC window on 2026-11-14, start inclusive/end exclusive | Calls |
| --- | --- | ---: |
| Before | 01:30–03:00 | 160 |
| After, first hour | 06:35–07:35 | 688 |
| After, full observed period | 06:35–14:45 | 1,747 |

The original 02:00–03:00 baseline had no Pi calls. It was extended to 01:30
after discovering the missing overlap, rather than selecting on effect size;
the original comparison was retained as a sensitivity check. The first-hour
and full-period comparisons were both retained. Release timing defines
exposure; it does not prove uniform adoption of each change on every call.

For each root, compute its mean cost per call separately in each period, then
take the arithmetic mean of the five root means. Each root has weight 1/5 in
both periods, so a shift toward a busier root does not determine the result.
Change is `100 * (after / before - 1)`. Input cost includes uncached input,
cached input, and cache writes; total cost also includes output.

## Results

| Equal-root mean | Before | After, full period | Change |
| --- | ---: | ---: | ---: |
| API-equivalent USD per call | 0.02986560 | 0.02491029 | −16.59% |
| Input API-equivalent USD per call | 0.02137862 | 0.01892204 | −11.49% |
| Input context tokens per call | 158,174 | 120,660 | −23.72% |

The first after-hour gives a similar cost result: **−16.61%**. Pooling all
calls instead of giving roots equal weight gives **−15.85%** over the full
period. Four roots have lower cost per call; the interface root increases by
3.88%. Output tokens per call also fall 29.44%, while uncached input tokens per
call rise 23.28%; the total reduction cannot be attributed entirely to smaller
context.

The original four-root-overlap baseline gives 12.6% lower total cost per call;
an alternative five-root 03:00–04:40 baseline gives 15.9% lower. Matching exact
root/model/effort/service-tier cells gives 17.37% lower, but covers only 67/160
before calls and 470/1,747 after calls. Task and output mix remain uncontrolled
under every comparison.

## Interpretation

The result supports continued investment in context efficiency and provides a
baseline for subsequent comparisons. It does not measure cost per accepted
outcome, subscription quota, an invoice, campaign-wide savings, or project ROI.
There was no randomized control; tasks, output, and effort changed between
periods. The three changes were observed together and their individual effects
are unestablished. Further planned gains in the
[efficiency ledger](../efficiency-ledger.md) are separate hypotheses and are
not additive to this result.

## Evidence and reproducibility

[Published aggregates](context-cost-comparison-20261114.json) contain each
root's call count, total cost, input cost per call, context mean, and output
mean for all three periods. They permit recomputation of the primary and
first-hour results and the pooled comparison without publishing transcripts
or session identifiers. They do not independently validate the underlying
private telemetry.

The source analysis was retained at
`/home/bline/cbm-work/context-cost-comparison-20261114/`: `analyze.py`,
`analysis-config.json`, `RESULTS.json`, `SUMMARY.json`, and the frozen
`ledger-0130-1445/`. These are local provenance references, not dependencies
for reading the public aggregates. The full ledger snapshot had 8,029 calls
across 99 Codex transcripts; its cumulative totals matched all 99, with no
recorded issues, fallback calls, or unknown models. The selected comparison
uses only the root calls in the windows above.

The fixed scenario is `usage-ledger-standard-api-v1`, using
`speed_adjusted_api_usd`; it supplies consistent comparison units rather than
current price advice. Aggregate metadata preserves its rates and scenario
digests along with the analyzer, ledger, call-data, and result digests. The
source calculation also passed a separate Decimal reaggregation.
