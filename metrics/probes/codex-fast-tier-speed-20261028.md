# Codex Fast against standard tier: matched speed test (2026-10-28)

**Result: Fast was 3–9% faster by median (n = 10 pairs), at 2.5x the quota rate.**
Fast was faster in 7 of 10 pairs. The Codex speed docs give 1.5x only for
GPT-5.6; they give no figure for the GPT-6 models.

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
