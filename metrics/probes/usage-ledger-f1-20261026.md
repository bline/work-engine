# F1 result: call-level usage ledger (2026-10-26)

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md) §4 F1.
Script: [`metrics/scripts/usage_ledger.py`](../scripts/usage_ledger.py), sha256
`b6180b04ced39babe03595a1529f341f28d0e39500a15b4354cd3cffad6da169`.
Build output, not committed (large): `~/cbm-work/usage-ledger/since-20261001/`
(`calls.jsonl`, `quota.jsonl`, `level1.json`, `issues.jsonl`, `summary.json`).
Committed copies: [`usage-ledger-build-summary-20261026.json`](usage-ledger-build-summary-20261026.json)
and [`usage-ledger-reconcile-trial-20261026.json`](usage-ledger-reconcile-trial-20261026.json).

**Status: accepted and frozen 2026-10-27 by the owner.** The acceptance gate was
reproduction of every arm of the four-builder trial. The Claude level-1 differences
for interactive sessions stay **open** and are classified below.

## Frozen evidence

| Item | sha256 |
|---|---|
| `metrics/scripts/usage_ledger.py` | `b6180b04ced39babe03595a1529f341f28d0e39500a15b4354cd3cffad6da169` |
| Trial input `anthropic-builder-baselines-20261027/measurement/final-refresh-v2.json` | `cffb785b9d20805f0c92ff55e97001fe8f790bb6c8a6be4d743a705df2947463` |
| [`usage-ledger-reconcile-trial-20261026.json`](usage-ledger-reconcile-trial-20261026.json): byte-identical when rerun | `b2c292734a9b1403ef1c4c46affe7d8779f8297f0d95a632f71c7274a6337200` |
| [`usage-ledger-build-summary-20261026.json`](usage-ledger-build-summary-20261026.json), the frozen build's `summary.json` | `92b39de20065d30fdd595f45acb269d69653d56e5668fb0fb4ddfd2be869207e` |
| [`usage-ledger-f1-inputs-20261027.json`](usage-ledger-f1-inputs-20261027.json): path, size and sha256 of all 1,040 input files (1,915,018,116 bytes) | `3323c6f69a097c4bd184b394e9eefc4474b19ded6acae722fb6abcccd7d01cfe` |

Frozen build outputs, kept outside the repository because of their size
(`~/cbm-work/usage-ledger/f1-frozen-20261027T0430Z/`):

| File | Bytes | sha256 |
|---|---:|---|
| `calls.jsonl` | 91,203,443 | `29d1a6ed5f0da06561d85a33938a2b6da7529621b277b21134f69eba0230ffc7` |
| `quota.jsonl` | 36,054,713 | `107b87ff47fcb19fcf6de25743ff23f93b3512a8723beaeb8bf53de96779e7af` |
| `level1.json` | 732,828 | `32a8ab20ff0d315f25fb0afe1de7d5c266fecc89ae3b285c0ef0602a96c2324f` |
| `issues.jsonl` | 3,434 | `6ba2526e18a7a6c554e9fadbbedcf4934b751266519166909c9bd387d7917da1` |

To reproduce:

```bash
python3 -I metrics/scripts/usage_ledger.py reconcile-trial \
  ~/.local/state/work-engine/anthropic-builder-baselines-20261027/measurement/final-refresh-v2.json
python3 -I metrics/scripts/usage_ledger.py build --since 2026-10-01T00:00:00Z \
  --until 2026-10-27T04:30:00Z --out DIR \
  --streams $(jq -r '.streams[].path' metrics/probes/usage-ledger-f1-inputs-20261027.json)
```

The `--until` bound fixes the calls and quota rows. Level-1 comparisons read each
session's full file, so they reflect the inputs as of the manifest. A later build
can differ if live sessions grow or if source files are deleted. Compare the
inputs against the manifest first.

## Build

`build --since 2026-10-01T00:00:00Z --until 2026-10-27T04:30:00Z` read:
- 650 Codex rollouts, compressed `.jsonl.zst` included;
- 313 Claude transcripts;
- 77 stream-json files (found with `find -maxdepth 5`).

It produced:
- **110,680 calls**;
- **98,885 quota readings**;
- 1 Codex call counted under the fallback rule (token_count events, no per-response
  record);
- 0 calls with an unknown model;
- 15 issues: 12 `conflicting_native_counter`, 3 `malformed_line`.

Unpriced, because no frozen rate exists:
- Claude: Fable 5.1, Haiku 5.5, Opus 5, Opus 4.8.
- Codex: gpt-5.6-sol, gpt-6-luna.

These calls carry `usd: null` and are counted, not estimated. Pricing them needs a
sourced rate.

## Reconciliation

| Level | Comparison | Result |
|---|---|---|
| 1, Codex | Per-thread sum of per-response usage against the provider's cumulative `thread_token_usage` | **642 of 642 exact.** Two forked threads (`01a1294e`, `01a11443`) start their counter at the source thread's total. With that starting offset removed they match exactly (216,394,989 and 81,459,105 input tokens). |
| 1, Claude | Per-session messages (transcript, copies, `subagents/`) against the CLI's `cost-state.modelUsage` | **148 of 241 session-model pairs exact.** Differences classified below. |
| 2 | Every actor of `anthropic-builder-baselines-20261027/measurement/final-refresh-v2.json`, recomputed from raw sources | **25 of 25 actors exact** in calls, every counter and dollars (19 Codex actors, 2 historical root-coordination windows, 2 native builders with and without subagents). Builder-only, builder-plus-reviewers, coordination and arm totals reproduce for all four arms; shared overhead $35.266662 reproduces. |
| 3 | Current Codex window (resets 2026-11-02 06:25Z) against the session-scratch figures | Astra 3,966 calls / 63.36M weighted tokens against 3,968 / 63.25M; Sol 1,772 / 25.59M against 1,773 / 25.59M; Terra 61 / 0.82M, equal. Modelled $685.36 against $684. The residual comes from deduplication by response_id instead of (thread, timestamp, counters) and from window-boundary assignment. |

Level 2 shows that the ledger's readers reproduce the trial's method. Both use the
same counting rule, so level 2 does not independently validate that rule. Level 1
does, against provider counters.

## Claude level-1 differences (open)

| Class | Pairs | CLI `costUSD` |
|---|---:|---:|
| exact | 148 | $340.32 |
| model absent from transcript (small background calls) | 33 | $1.19 |
| CLI counter exceeds transcript | 50 | $435.96 |
| mixed | 9 | $454.18 |
| transcript exceeds counter | 1 | $0.10 |

The two kinds of transcript differ sharply by request source (`api-request.querySource`):

| Transcripts | Requests | Main thread | Subagents | Headless SDK | Compaction markers |
|---|---:|---:|---:|---:|---:|
| 39 with a gap | 7,829 | 4,278 | 3,364 | 187 | 60 |
| 147 exact | 4,245 | 80 | 5 | 4,160 | — |

So headless `claude -p` sessions, the kind experiment arms use, reconcile. Long
interactive sessions do not. The cause is `not established`. Candidates:
- compaction or other requests that write no assistant message;
- subagents isolated in worktrees, whose transcripts land in another project
  directory;
- counters carried across a resume.

**Until this is resolved:**
- Cost for interactive Claude sessions (coordinators, this session) is reported
  both ways: transcript-derived and from the CLI counter.
- The budget uses the larger of the two.
- The CLI counter's dollars are its own list-price estimate. The ~$890 by which
  the counter exceeds the transcripts in the "counter exceeds" and "mixed" classes
  is **not established as additional billed or quota-consuming expenditure.** It
  may be real requests the transcripts omit, or counter carry-over.
