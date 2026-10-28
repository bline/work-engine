# Codex Fast against standard tier: matched speed test (2026-10-28)

**Result: Fast's gain depends on other Fast traffic.** With three Fast roots
running (16:49–16:57Z), Fast was 1.09x faster per turn by median, faster in 7 of
10 pairs. With no other Fast thread active (18:55–19:03Z, rerun below), it was
1.49x faster, in 10 of 10 pairs. Fast draws included usage at 2.5x the standard
rate. The Codex speed docs give 1.5x, for GPT-5.6 only.

Approved by the owner on 2026-10-28 ("proceed"). Results:
[`codex-fast-tier-speed-20261028.json`](codex-fast-tier-speed-20261028.json).

## Design

- One fixed, tool-free prompt (write a Python `merge_intervals` with five
  doctests and explain its complexity), `gpt-6.1-sol` at `xhigh`, `codex exec`
  0.164.0, read-only sandbox, a fresh process per run in an empty scratch
  directory.
- 10 pairs, the tiers alternating within and across pairs (standard first in
  even pairs, Fast first in odd) so drift over the 20 minutes cancels.
- Tier set per run with `-c service_tier="default"` or `"fast"`; every run's
  rollout records the requested tier (`default` or `priority`).
- Measures come from each rollout: Codex's own turn `duration_ms` and
  `time_to_first_token_ms`, output tokens, plus the process wall time.
  Every run was one model call.

## Results

| Tier | Runs | Median turn | Median time to first token | Median wall | Median output tokens | Median output tok/s |
|---|---:|---:|---:|---:|---:|---:|
| standard | 10 | 20.89 s | 17.31 s | 24.84 s | 707.5 | 33.74 |
| Fast | 10 | 19.56 s | 16.07 s | 24.80 s | 704.5 | 35.68 |

Per pair, standard time over Fast time (above 1 means Fast was faster):

| Measure | Median | Range | Fast faster |
|---|---:|---|---:|
| turn duration | 1.09 | 0.95–1.26 | 7/10 |
| time to first token | 1.07 | 0.97–1.26 | 7/10 |
| process wall time | 1.03 | 0.95–1.19 | 7/10 |
| output tokens per second | 1.07 | 0.91–1.26 | 7/10 |

Time to first token is about 83% of each turn at `xhigh`: most of the turn is
reasoning before the first visible token, and Fast shortened it about as much
as the rest.

## Cost

20 runs plus one check pair: $0.33 standard and $0.33 Fast at list price; $1.16
with Fast weighted at 2.5x.

## Limitations

- One prompt, one model and effort, short single-call turns (about 700 output
  tokens). Long agentic turns with large cached context are not measured here.
- n = 10 pairs on one afternoon; provider load can change the gap.
- The 2.5x quota rate is the documented rate, not measured here.

Scripts (not committed; in `~/cbm-work/fast-tier-20261028/`): `matched_run.py`
sha256 `c129864a5517f9fb…`, `matched_analyze.py` `5a9b094de83d4983…`.

## Rerun with no other Fast traffic (2026-10-28 18:55–19:03Z)

The owner turned Fast off in every session but S10, and the rerun started only
after no thread had been at `priority` for 10 consecutive minutes (S10's tree
included). Same prompt, model, effort, pairs and alternation; each run recorded
the other `priority` threads active in the previous 5 minutes: none, in 20 of 20.
Results: [`codex-fast-tier-speed-quiet-20261028.json`](codex-fast-tier-speed-quiet-20261028.json);
script `matched_run_quiet.py` (sha256 `f536b0bd16a79434…`, the first run's script plus that check).

| Tier | Runs | Median turn | Median time to first token | Median wall | Median output tokens | Median output tok/s |
|---|---:|---:|---:|---:|---:|---:|
| standard | 10 | 19.53 s | 15.98 s | 29.91 s | 714 | 36.15 |
| Fast | 10 | 13.27 s | 10.85 s | 23.54 s | 691 | 51.49 |

| Measure | First run (3 Fast roots) | Rerun (no other Fast) |
|---|---|---|
| turn duration | 1.09 (0.95–1.26), 7/10 | **1.49 (1.19–1.66), 10/10** |
| time to first token | 1.07 (0.97–1.26), 7/10 | 1.51 (1.15–1.70), 10/10 |
| process wall time | 1.03 (0.95–1.19), 7/10 | 1.19 (1.06–1.86), 10/10 |
| output tokens per second | 1.07 (0.91–1.26), 7/10 | 1.41 (1.23–1.62), 10/10 |

**Reading.** Standard turns took about the same time in both runs (20.89 s,
then 19.53 s), while Fast turns went from 19.56 s to 13.27 s. A general drop in
provider load would have sped up both, so the change points to contention for
Fast itself: concurrent Fast work erodes its gain. Whether that limit is per
account or provider-wide is not established; interleaving one Fast session
alone with three Fast sessions at the same moment would separate the two. Wall
time gains less because process start-up is a fixed cost. n = 10 pairs per run,
one prompt, two times of day.
