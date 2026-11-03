# Manual host efficiency observation, 2026-11-03

The latest host is associated with less root-authored orchestration code than
the preceding host in these two sessions. This is useful evidence for the
narrower benefit of reducing generated glue. It does **not** establish net
time or cost savings against manual orchestration, or the original roughly
40% root-time saving hypothesis.

The [observations and source audit](manual-host-efficiency-20261103.json) cover
13 closed fresh native reviewer launches across four host versions. These are
unequal engineering tasks, including follow-up reviews and one instruction
review. The cohort was defined for this retrospective before calculating its
aggregates; it was not a trial specified before the tasks ran. No additional
paid agents or product executions were launched for this analysis.

All figures below are **medians per review**, not totals or bookkeeping-only
model turns. Cost means speed-adjusted API-equivalent USD under the repository's
recorded pricing scenario, not the subscription bill.

| Host version | Reviews | Inline Python bodies | Associated root calls | All root calls in window | Root window cost | Window minutes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Initial, `fe560261` | 3 | 14 | 9 | 14 | $0.4644 | 6.34 |
| Receipt repair, `9a8536e1` | 1 | 9 | 7 | 12 | $0.5325 | 9.32 |
| Large-state repair, `e52f391d` | 5 | 10 | 11 | 19 | $1.5731 | 22.32 |
| Current, `b25c9bdf` / bundle `b03ad3a8` | 4 | 4.5 | 7 | 11.5 | $0.6744 | 9.40 |

Relative to the immediate predecessor, the current sample has 55% fewer inline
Python bodies and 36% fewer associated root calls at the median. Those are
descriptive differences between these samples, not estimated treatment effects.
The cost and elapsed-time differences must not be advertised as savings: the
children, review subjects, concurrent work and root contexts differ. The
initial sample was already cheaper and faster than the current sample.

Removing the dispatch cell, which sometimes also authors the packet, changes
the Python medians from **9 to 4** and the associated-call medians from **10 to
6**. Removing the instruction review leaves three current code reviews, with
medians of **5 Python bodies, 7 associated calls and $0.5011 root window cost**.
The generated-code reduction therefore does not depend entirely on that one
lighter review or on counting packet preparation inside a dispatch cell.

Separating the roots also preserves the direction for generated Python:

| Root | Preceding version | Current version |
| --- | --- | --- |
| Backend/conductor, `01a195d4…` | 3 reviews; Python 10, associated calls 11 | 1 review; Python 5, associated calls 6 |
| UI/auth, `01a19b6d…` | 2 reviews; Python 8.5, associated calls 9.5 | 3 reviews; Python 4, associated calls 7 |

The UI current group includes the instruction review. With that review removed,
its two code reviews have medians of 5 Python bodies and 8.5 associated calls.
There is only one current backend review, so that stratum provides little
precision. Restricting both versions to non-delta code reviews gives three
reviews each: Python **8 to 5**, associated calls **8 to 7**, but all root
window calls **11 to 13**. A broad claim that every class of review now needs
fewer model calls would be unsupported.

Manual state inspection has not disappeared. The associated Python bodies that
read host state or wrap `inspect` have medians of **2 before and 2.5 now**. In
the current four reviews, roots still submitted 19 inline Python bodies,
containing 50,901 source characters. Source cells continue to construct identity
and entry-gate records, update assignments and budget bindings, and prepare
collection and closure records. Some combine deterministic work with authored
authority or acceptance text. This analysis does not label all of that code
mechanical or treat deleting its judgment content as an optimization.

## What was measured

The frozen cutoff is **2026-11-03T18:43:36.504Z**. Sources are the two roots:

- Backend: `01a195d4-b7a7-77f2-ba65-277669c0ec97`, rollout prefix 20,842
  lines / 97,879,855 bytes.
- UI: `01a19b6d-db5b-7a50-b4d1-7f69e34bb20e`, rollout prefix 9,197
  lines / 37,964,132 bytes.

The input inventory contains 39 host operations in 21 state-file snapshots.
The main cohort includes every closed, non-retained operation whose ID contains
`review` and which has a recorded native session. All 13 have captured results;
12 have accepted closure dispositions and one initial-version delta review has
an abandoned disposition. That abandoned case is retained rather than silently
selecting successful outcomes. Its latest receipt remains unacknowledged in
the frozen state. Closure disposition here is the root's recorded decision,
not an independent assessment of product quality.

The remaining inventory is reported separately in the JSON: 23 worker
operations (22 closed and one running), one running fresh review, one closed
pre-admission review without a native session, and one closed retained review
attempt without a new native session. Worker operations were not pooled into
the review comparison because their long windows include implementation,
integration and overlapping activity. Running or not-yet-admitted operations
do not contribute invented zero-cost completions. Later operations, including
work that closes after the cutoff, are outside this observation.

An operation's window starts at its first associated dispatch tool cell within
15 minutes before native start. Two older backend launches execute custom
wrappers rather than literal host commands; their manually checked source
lines and input hashes are recorded in the private input and public result.
All 13 dispatch boundaries were checked against their source cells. Packet
preparation before dispatch is excluded unless batched in the same submitted
cell. The additional figures excluding that cell reduce this batching bias.

The window ends at the **first closed task receipt's persistence time**. It
does not extend to later objective accounting, final acknowledgment or user
reporting. This avoids counting a 39-minute later receipt tail in one older
review while using ordinary task completion for the others. It also means this
is not a full phase-cost or delivered-receipt measurement. Receipt modification
time records local persistence, not acceptance or delivery to the root.

The two complementary counts have different meanings:

- **All root window calls/cost** includes every normalized root call ending in
  that interval, including judgment and unrelated or concurrent work.
- **Associated calls/code** requires the submitted tool cell to name the
  operation, native identity, exact request/packet/result path or host state
  directory; bounded relative names also require the run directory. It is an
  association, not exclusive causal attribution. Shared-state reads and mixed
  cells can concern more than one activity.

Inline Python is counted from decoded literal `cmd` strings containing Python
heredocs in submitted tool cells. These are authored source bodies, including
templates and conditional branches, not proof of actual execution. Repeated
bodies count as occurrences; unique hashes are also supplied. Existing Python
scripts invoked by filename, dynamically constructed commands, JavaScript-only
bookkeeping and packet text written outside the window are not counted as
inline Python. No code from a rollout is executed by the analyzer. Literal
extraction is a reproducible proxy for generated orchestration code, not a
reproduction of the original study's mechanical-versus-authored classifier.

## Cost, time and verification

Normalization reuses [usage_ledger.py](scripts/usage_ledger.py) at SHA-256
`ed026f40f93801c9fffab4a9d5d5fc32745fd1d94a6137edd22ed875b5a67291`.
Its existing model/tier, cache and long-input pricing rules remain intact.
The scenario is `usage-ledger-standard-api-v1`; full pricing identity, source
hashes, call IDs, usage counters and source lines are in the JSON. None of these
API-equivalent prices establishes an actual invoice or quota consumption.

The reader normalized 2,755 calls over the complete frozen root prefixes with
no reader issues. Reconstructed token counters match the provider's cumulative
counters for both roots, with zero delta in every recorded token category.
The selected windows contain 203 distinct root calls, with no unknown prices.
All selected root calls use observed `gpt-6.1-sol/xhigh/default`.

Root context still confounds cost. The preceding-version per-review median
contexts range from about 68k to 429k tokens; current contexts range from 109k
to 377k. Some calls cross the reader's 272k long-input pricing boundary.
Median native child duration also changes from 351.09 to 244.69 seconds.
No matched subject, manual arm, context-controlled cost or total-actor saving
is established by this comparison.

Recorded call latency is time from the latest preceding input to the usage
record, and can include queueing or other elapsed time. The JSON includes raw
latency and a version clipped to the window boundary; three older window calls
have no latency observation. Neither proxy measures isolated active thinking
time, and neither is used to turn these samples into root-hours saved.

Validation verified both frozen rollout hashes, all 21 state snapshot hashes
and 170 referenced receipt content hashes. The original six positively
identified custom state inspections at backend rollout lines 10915, 10922,
10930, 10998, 11029 and 11051 are all recovered with the expected classification.
That calibration caught and corrected an omitted host-state-directory alias in
the first analysis pass. There are 2,583 submitted root tool cells, of which
2,552 have an unambiguous single-call mapping. Every associated cell in the
13 selected reviews maps to one call, with zero unresolved `cmd` values in
those cells. This does not prove exhaustive classification of every form of
bookkeeping.

The [validation record](manual-host-efficiency-validation-20261103.json)
records independent count/cost reconciliation and calibration hashes. Six
analyzer tests, Clippy with warnings denied and formatting checks pass. An
identical-input rerun produces identical observation bytes. The new example
was source-read outside the main graph generation; the existing usage reader
and crate manifest had content-verified coverage. No graph completeness claim
is needed for the rollout measurements.

## Prior control and practical conclusion

The [original manual-versus-hosted pilot](deterministic-bookkeeping-host-pilot-20261102.md)
remains the only direct control pair. Both arms needed five root calls. The
older hosted arm had root cost $1.348320 versus $1.238968 manually, and elapsed
time 118.17 versus 100.55 seconds: about 9% more modeled root cost and 18% more
wall time. It was tiny, unblinded, used unequal root contexts and predates the
current host. It neither establishes savings nor settles current-host
performance. These new observational samples do not replace that control.

The defensible conclusion is that the latest ordinary-use sample requires less
generated orchestration code than the immediately preceding sample. Evidence
for net cost/time savings against manual work remains insufficient, and the
original 22.4/52.8-hour mechanical share is still an unverified hypothesis.
Retaining the current host is consistent with its demonstrated execution
repairs and this directional result. A numeric savings claim would require
comparable tasks using the current host and a manual control, including setup,
final acknowledgment, repairs, all actors and a quality outcome. This record
does not authorize a new paid comparison.

To reproduce the observation on this machine, use the frozen private manifest
at `/home/bline/.local/state/work-engine/manual-host-efficiency-20261103/input.json`:

```bash
cargo run --locked --manifest-path rust/Cargo.toml \
  -p work-engine-manual-host --example efficiency_report -- \
  /home/bline/.local/state/work-engine/manual-host-efficiency-20261103/input.json \
  /tmp/manual-host-efficiency-rerun.json
```

The [Rust example](../rust/crates/work-engine-manual-host/examples/efficiency_report.rs)
validates frozen rollout/state hashes and annotated dispatch hashes. Its source,
input and normalized-call hashes are embedded in the observation. Raw rollouts,
tool bodies and the full input manifest remain private; only hashes,
references, counters and selected usage rows are committed. The private input
binds 39 operations to their frozen states and receipt index; normalization
uses `build_codex` restricted to the two frozen rollout paths, from
`2026-10-31T00:00:00Z` to the cutoff, with the reader's existing deduplication
and metadata validation. No runtime host, skill activation, peer state or
budget was changed by this analysis.
